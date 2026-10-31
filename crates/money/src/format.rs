//! Turning amounts into text and text into amounts.
//!
//! Both directions are currency-aware: the number of decimals comes from the
//! currency's ISO exponent, so `1200` is `USh 1200` in Uganda and `KSh 12.00`
//! in Kenya. Nothing here guesses a locale or a thousands separator for output.

use std::fmt;

use crate::{Currency, Money};

/// Why [`Money::parse`] refused a string.
///
/// A parse error is user input, not a crash: the till shows the cashier what is
/// wrong and keeps the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Nothing but whitespace and grouping characters.
    Empty,
    /// A character that cannot be part of a plain amount, or two decimal points.
    Invalid,
    /// More decimals than the currency has minor units.
    TooManyDecimals,
    /// The amount does not fit in an `i64` of minor units.
    Overflow,
    /// A code with no exponent (a metal, `XXX`): not something to store.
    NotATender,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Empty => "enter an amount",
            Self::Invalid => "not a valid amount",
            Self::TooManyDecimals => "too many decimal places for this currency",
            Self::Overflow => "amount is too large",
            Self::NotATender => "this currency has no minor unit",
        };
        f.write_str(text)
    }
}

impl std::error::Error for ParseError {}

/// Write an amount with its symbol and the currency's decimals.
pub(crate) fn format(money: Money) -> String {
    let currency = money.currency();
    let scale = crate::scale(currency);
    let sign = if money.minor() < 0 { "-" } else { "" };
    let magnitude = money.minor().unsigned_abs();

    let digits = if scale == 0 {
        magnitude.to_string()
    } else {
        let factor = 10_u64.pow(scale);
        format!(
            "{}.{:0width$}",
            magnitude / factor,
            magnitude % factor,
            width = scale as usize
        )
    };

    // `$12.50` reads as one token; `USh 1200` needs the gap or the letters run
    // into the number. A symbol with any letter in it is a word, not a glyph.
    // ISO's generic `¤` is not a symbol a customer should ever see: fall back to
    // the code, the same way as for a currency with no symbol at all.
    let symbol = currency.symbol().to_string();
    let (symbol, space) = if symbol.is_empty() || symbol == "¤" {
        (currency.code().to_owned(), " ")
    } else if symbol.chars().any(char::is_alphabetic) {
        (symbol, " ")
    } else {
        (symbol, "")
    };

    format!("{sign}{symbol}{space}{digits}")
}

/// Read a plain decimal amount in `currency`.
pub(crate) fn parse(text: &str, currency: Currency) -> Result<Money, ParseError> {
    let scale = currency.exponent().ok_or(ParseError::NotATender)? as u32;

    // Grouping is cosmetic and never changes the value: a cashier may type
    // `1 234,56` on a keyboard whose comma is a decimal mark, but the one
    // decimal point this parser accepts is `.`, and commas only group.
    let cleaned: String = text
        .chars()
        .filter(|ch| !matches!(ch, ',' | '_' | ' ' | '\u{a0}'))
        .collect();
    if cleaned.is_empty() {
        return Err(ParseError::Empty);
    }

    let (negative, body) = match cleaned.as_bytes()[0] {
        b'-' => (true, &cleaned[1..]),
        b'+' => (false, &cleaned[1..]),
        _ => (false, cleaned.as_str()),
    };
    if body.is_empty() {
        return Err(ParseError::Empty);
    }

    let mut parts = body.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() {
        return Err(ParseError::Invalid);
    }
    if whole.is_empty() && fraction.unwrap_or_default().is_empty() {
        return Err(ParseError::Invalid);
    }

    let mut minor: i128 = 0;
    for ch in whole.chars() {
        let digit = ch.to_digit(10).ok_or(ParseError::Invalid)?;
        minor = minor
            .checked_mul(10)
            .and_then(|value| value.checked_add(i128::from(digit)))
            .ok_or(ParseError::Overflow)?;
    }
    minor = minor
        .checked_mul(10_i128.pow(scale))
        .ok_or(ParseError::Overflow)?;

    let fraction = fraction.unwrap_or_default();
    if fraction.chars().count() > scale as usize {
        return Err(ParseError::TooManyDecimals);
    }
    for (index, ch) in fraction.chars().enumerate() {
        let digit = ch.to_digit(10).ok_or(ParseError::Invalid)?;
        let place = 10_i128.pow(scale - 1 - index as u32);
        minor += i128::from(digit) * place;
    }

    if negative {
        minor = -minor;
    }

    i64::try_from(minor)
        .map(|minor| Money::new(minor, currency))
        .map_err(|_| ParseError::Overflow)
}
