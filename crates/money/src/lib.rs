//! Money as integer minor units.
//!
//! An amount is a whole number of minor units plus the currency it belongs to:
//! `1234` is 12.34 US dollars and 1234 Ugandan shillings, and only the currency
//! knows which. Nothing here is a float, so a total is exact and a database
//! column can be an `INTEGER`.
//!
//! The crate is arithmetic, formatting and parsing only. It has no idea what a
//! sale is, where a total is stored, or which country label to show: apps own
//! their tables and their own country list, and ask this crate only "what
//! currency does this country use?".

mod format;

pub use format::ParseError;
pub use iso_currency::{Country, Currency};

/// An amount in one currency, as a whole number of minor units.
///
/// `Copy` because an amount is a value: passing it around never needs a clone,
/// and no arithmetic method mutates in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Money {
    minor: i64,
    currency: Currency,
}

impl Money {
    /// An amount in `currency`.
    ///
    /// `minor` is minor units: cents for USD, whole shillings for UGX. Use
    /// [`Money::parse`] for human input and this for values that are already
    /// integers.
    pub const fn new(minor: i64, currency: Currency) -> Self {
        Self { minor, currency }
    }

    /// The amount in minor units.
    pub const fn minor(self) -> i64 {
        self.minor
    }

    pub const fn currency(self) -> Currency {
        self.currency
    }

    /// Decimal places this amount is written with: 0 for UGX, 2 for KES.
    pub fn scale(self) -> u32 {
        scale(self.currency)
    }

    /// The amount as the customer reads it: `USh 1200`, `KSh 123.45`, `$12.50`.
    pub fn format(self) -> String {
        format::format(self)
    }

    /// Read an amount the way a cashier types it.
    ///
    /// Accepts an optional sign, grouping commas, spaces or underscores, and a
    /// decimal point. The number of decimals is the currency's exponent, so
    /// `"1234"` is 1234 UGX but 123400 KES. The symbol is never part of the
    /// input: one till has one currency, and accepting `$` would let a typo
    /// change the amount.
    pub fn parse(text: &str, currency: Currency) -> Result<Self, ParseError> {
        format::parse(text, currency)
    }

    /// Add two amounts of the same currency.
    ///
    /// `None` on a currency mismatch or overflow. Adding shillings to dollars
    /// is a bug in the caller, not something to convert silently.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        if self.currency != other.currency {
            return None;
        }
        self.minor
            .checked_add(other.minor)
            .map(|minor| Self::new(minor, self.currency))
    }

    /// Multiply by a whole quantity: `qty × unit price`.
    pub fn mul_qty(self, qty: i64) -> Option<Self> {
        self.minor
            .checked_mul(qty)
            .map(|minor| Self::new(minor, self.currency))
    }

    /// Apply a rate in basis points (`1000` = 10.00%), rounded half away from
    /// zero to the currency's smallest unit.
    ///
    /// Basis points rather than a float percentage because tax rates are
    /// stored exactly: 7.5% is `750`, not `0.075`.
    pub fn apply_bps(self, bps: u32) -> Option<Self> {
        let product = i128::from(self.minor) * i128::from(bps);
        let rounded = round_div(product, 10_000);
        i64::try_from(rounded)
            .ok()
            .map(|minor| Self::new(minor, self.currency))
    }

    /// Round to the nearest multiple of `step` minor units, half away from zero.
    ///
    /// Cash rounding lives here, not in the currency: whether a till rounds to
    /// the nearest 100 UGX is a shop's decision, and a currency table that
    /// hard-codes it would be wrong for the next shop.
    pub fn round_to_step(self, step: i64) -> Option<Self> {
        if step <= 0 {
            return None;
        }
        let rounded = round_div(i128::from(self.minor), i128::from(step)) * i128::from(step);
        i64::try_from(rounded)
            .ok()
            .map(|minor| Self::new(minor, self.currency))
    }

    /// Split the amount by weight without losing or inventing a minor unit.
    ///
    /// Every share is the floor of its exact proportion; the leftover minor
    /// units go one each to the largest remainders, ties broken by position.
    /// The shares always sum back to `self`, which is what makes this usable for
    /// splitting a bill or a payment across lines.
    ///
    /// An empty slice, or one whose weights are all zero, yields no shares:
    /// there is no meaningful way to divide by nothing.
    pub fn allocate(self, weights: &[u64]) -> Vec<Self> {
        let total: u128 = weights.iter().map(|weight| u128::from(*weight)).sum();
        if weights.is_empty() || total == 0 {
            return Vec::new();
        }

        let magnitude = u128::from(self.minor.unsigned_abs());
        let mut shares = Vec::with_capacity(weights.len());
        let mut remainders = Vec::with_capacity(weights.len());
        let mut assigned = 0_u128;

        for (index, weight) in weights.iter().enumerate() {
            let product = magnitude * u128::from(*weight);
            let share = product / total;
            shares.push(share);
            remainders.push((product % total, index));
            assigned += share;
        }

        // Largest remainder first; the index tie-break keeps the result stable,
        // so the same bill always splits the same way.
        remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut leftover = magnitude - assigned;
        for (_, index) in remainders {
            if leftover == 0 {
                break;
            }
            shares[index] += 1;
            leftover -= 1;
        }

        let sign = i128::from(self.minor.signum());
        shares
            .into_iter()
            .map(|share| {
                // Exact: a share is at most |minor|, and applying the original
                // sign lands back inside i64 (i64::MIN included).
                let minor = (share as i128 * sign) as i64;
                Self::new(minor, self.currency)
            })
            .collect()
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.format())
    }
}

/// The currency a country tenders in.
///
/// A country can have several currencies in ISO 4217 (historical ones, funds,
/// metals). The tender is the one with no special flags; `None` means the
/// country has no regular currency in the data, not "pick something".
pub fn tender_for(country: Country) -> Option<Currency> {
    Currency::from_country(country)
        .into_iter()
        .find(|currency| currency.flags().is_empty() && currency.exponent().is_some())
}

/// A currency by its ISO 4217 code, if it is a tender.
///
/// `None` for codes that are not money (metals like `XAU`, the test code
/// `XXX`): those have no minor unit, so an amount in them cannot be stored as
/// an integer.
pub fn tender_from_code(code: &str) -> Option<Currency> {
    Currency::from_code(code).filter(|currency| currency.exponent().is_some())
}

/// Decimal places a currency is written with.
///
/// Metals and `XXX` have no exponent. They are not tenders, and
/// [`tender_for`]/[`tender_from_code`] never hand one out; two decimals is the
/// printable fallback for a currency a caller built by hand.
fn scale(currency: Currency) -> u32 {
    currency.exponent().map_or(2, u32::from)
}

/// Integer division rounded half away from zero.
fn round_div(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder.abs() * 2 >= denominator.abs() {
        quotient + numerator.signum()
    } else {
        quotient
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn currency(code: &str) -> Currency {
        tender_from_code(code).unwrap_or_else(|| panic!("{code} is not a tender"))
    }

    #[test]
    fn a_country_maps_to_the_currency_it_tenders_in() {
        assert_eq!(tender_for(Country::UG), Some(currency("UGX")));
        assert_eq!(tender_for(Country::KE), Some(currency("KES")));
        assert_eq!(tender_for(Country::US), Some(currency("USD")));
    }

    #[test]
    fn non_tender_codes_are_refused() {
        // `XAU` is gold by the ounce; there is no minor unit to count.
        assert_eq!(Currency::from_code("XAU").map(|c| c.exponent()), Some(None));
        assert_eq!(tender_from_code("XAU"), None);
        assert_eq!(tender_from_code("XXX"), None);
        assert_eq!(tender_from_code("NOPE"), None);
    }

    #[test]
    fn zero_decimal_currencies_keep_whole_units() {
        let amount = Money::new(1_200, currency("UGX"));
        assert_eq!(amount.scale(), 0);
        assert_eq!(amount.format(), "USh 1200");
        assert_eq!(Money::parse("1200", currency("UGX")), Ok(amount));
        assert_eq!(
            Money::parse("12.00", currency("UGX")),
            Err(ParseError::TooManyDecimals)
        );
    }

    #[test]
    fn two_decimal_currencies_print_the_minor_units() {
        assert_eq!(Money::new(12_345, currency("KES")).format(), "Ksh 123.45");
        assert_eq!(Money::new(5, currency("KES")).format(), "Ksh 0.05");
        assert_eq!(Money::new(0, currency("KES")).format(), "Ksh 0.00");
    }

    #[test]
    fn glyph_symbols_glue_and_letter_symbols_take_a_space() {
        assert_eq!(Money::new(1_250, currency("USD")).format(), "$12.50");
        assert_eq!(Money::new(1_250, currency("EUR")).format(), "€12.50");
        assert_eq!(Money::new(1_250, currency("GBP")).format(), "£12.50");
        assert_eq!(Money::new(1_250, currency("TZS")).format(), "Tsh 12.50");
    }

    #[test]
    fn the_sign_goes_before_the_symbol() {
        assert_eq!(Money::new(-1_250, currency("USD")).format(), "-$12.50");
        assert_eq!(Money::new(-1_200, currency("UGX")).format(), "-USh 1200");
    }

    #[test]
    fn parsing_accepts_what_a_cashier_types() {
        let kes = currency("KES");
        assert_eq!(Money::parse("123.45", kes), Ok(Money::new(12_345, kes)));
        assert_eq!(Money::parse(" 1,234.5 ", kes), Ok(Money::new(123_450, kes)));
        assert_eq!(Money::parse("1_234.05", kes), Ok(Money::new(123_405, kes)));
        assert_eq!(Money::parse("-12", kes), Ok(Money::new(-1_200, kes)));
        assert_eq!(Money::parse("+12", kes), Ok(Money::new(1_200, kes)));
        assert_eq!(Money::parse(".5", kes), Ok(Money::new(50, kes)));
        assert_eq!(Money::parse("7.", kes), Ok(Money::new(700, kes)));
        assert_eq!(Money::parse("0", kes), Ok(Money::new(0, kes)));
    }

    #[test]
    fn parsing_refuses_what_would_be_a_guess() {
        let kes = currency("KES");
        assert_eq!(Money::parse("", kes), Err(ParseError::Empty));
        assert_eq!(Money::parse("   ", kes), Err(ParseError::Empty));
        assert_eq!(Money::parse("abc", kes), Err(ParseError::Invalid));
        assert_eq!(Money::parse("1.2.3", kes), Err(ParseError::Invalid));
        assert_eq!(Money::parse("$12", kes), Err(ParseError::Invalid));
        assert_eq!(Money::parse("1.234", kes), Err(ParseError::TooManyDecimals));
        assert_eq!(
            Money::parse("1.00", Currency::from_code("XAU").unwrap()),
            Err(ParseError::NotATender)
        );
        assert_eq!(
            Money::parse("99999999999999999999", kes),
            Err(ParseError::Overflow)
        );
    }

    #[test]
    fn parsing_and_formatting_agree_on_the_value() {
        // The symbol is presentation, not input: round-trip the number the
        // cashier would type, not the string the customer reads.
        for (code, text, minor) in [
            ("UGX", "1200", 1_200),
            ("UGX", "-1200", -1_200),
            ("KES", "123.45", 12_345),
            ("KES", "-0.50", -50),
            ("USD", "0.00", 0),
        ] {
            let currency = currency(code);
            let amount =
                Money::parse(text, currency).unwrap_or_else(|err| panic!("{code} {text}: {err}"));
            assert_eq!(amount, Money::new(minor, currency));
            assert_eq!(amount.minor(), minor);
        }
    }

    #[test]
    fn adding_across_currencies_is_refused() {
        let ugx = Money::new(100, currency("UGX"));
        let kes = Money::new(100, currency("KES"));
        assert_eq!(
            ugx.checked_add(Money::new(50, currency("UGX"))),
            Some(Money::new(150, currency("UGX")))
        );
        assert_eq!(ugx.checked_add(kes), None);
        assert_eq!(
            Money::new(i64::MAX, currency("UGX")).checked_add(Money::new(1, currency("UGX"))),
            None
        );
    }

    #[test]
    fn quantities_multiply_exactly() {
        let price = Money::new(340, currency("KES"));
        assert_eq!(price.mul_qty(3), Some(Money::new(1_020, currency("KES"))));
        assert_eq!(price.mul_qty(0), Some(Money::new(0, currency("KES"))));
        assert_eq!(
            Money::new(i64::MAX, currency("KES")).mul_qty(2),
            None,
            "overflow must be an error, not a wrapped total"
        );
    }

    #[test]
    fn basis_points_round_half_away_from_zero() {
        let kes = currency("KES");
        // 10% of 10.00 is exactly 1.00.
        assert_eq!(
            Money::new(1_000, kes).apply_bps(1_000),
            Some(Money::new(100, kes))
        );
        // 50% of 0.01 is 0.005, which rounds up to 0.01.
        assert_eq!(
            Money::new(1, kes).apply_bps(5_000),
            Some(Money::new(1, kes))
        );
        // 50% of 0.03 is 0.015, which rounds up to 0.02.
        assert_eq!(
            Money::new(3, kes).apply_bps(5_000),
            Some(Money::new(2, kes))
        );
        // A refund rounds the same way, away from zero.
        assert_eq!(
            Money::new(-1, kes).apply_bps(5_000),
            Some(Money::new(-1, kes))
        );
        // 7.5% of 12.34 is 0.9255, which rounds to 0.93.
        assert_eq!(
            Money::new(1_234, kes).apply_bps(750),
            Some(Money::new(93, kes))
        );
    }

    #[test]
    fn rounding_to_a_cash_step_is_half_up() {
        let ugx = currency("UGX");
        let round = |minor| Money::new(minor, ugx).round_to_step(100);
        assert_eq!(round(1_234), Some(Money::new(1_200, ugx)));
        assert_eq!(round(1_250), Some(Money::new(1_300, ugx)), "half goes up");
        assert_eq!(round(1_249), Some(Money::new(1_200, ugx)));
        assert_eq!(round(-1_250), Some(Money::new(-1_300, ugx)));
        assert_eq!(Money::new(1_234, ugx).round_to_step(0), None);
    }

    #[test]
    fn allocation_never_loses_a_minor_unit() {
        let kes = currency("KES");
        let shares = Money::new(100, kes).allocate(&[1, 1, 1]);
        assert_eq!(
            shares,
            vec![
                Money::new(34, kes),
                Money::new(33, kes),
                Money::new(33, kes)
            ]
        );
        assert_eq!(shares.iter().map(|share| share.minor()).sum::<i64>(), 100);

        let weighted = Money::new(1_000, kes).allocate(&[3, 7]);
        assert_eq!(weighted, vec![Money::new(300, kes), Money::new(700, kes)]);

        let refund = Money::new(-100, kes).allocate(&[1, 1, 1]);
        assert_eq!(
            refund,
            vec![
                Money::new(-34, kes),
                Money::new(-33, kes),
                Money::new(-33, kes)
            ]
        );
    }

    #[test]
    fn allocation_needs_a_weight_to_divide_by() {
        let kes = currency("KES");
        assert!(Money::new(100, kes).allocate(&[]).is_empty());
        assert!(Money::new(100, kes).allocate(&[0, 0]).is_empty());
    }

    #[test]
    fn display_matches_format() {
        let amount = Money::new(1_200, currency("UGX"));
        assert_eq!(amount.to_string(), amount.format());
    }
}
