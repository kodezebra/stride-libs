//! The code a till shows and a phone types.
//!
//! Crockford's base 32: ten characters from an alphabet that leaves out I, L,
//! O and U, so `1`/`I`/`l` and `0`/`O` never depend on the font. Fifty bits of
//! OS randomness is far more than someone on the shop Wi-Fi can work through,
//! and short enough to type without reading it out twice.

/// The characters a code can contain.
pub const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Ten characters is 50 bits.
pub const LEN: usize = 10;

/// A fresh code, upper-case and canonical.
///
/// An empty string means the OS could not produce randomness. That is a broken
/// machine, not a reason to invent a predictable token; the caller treats it as
/// a failure.
pub fn generate() -> String {
    let mut bytes = [0u8; LEN];
    if getrandom::fill(&mut bytes).is_err() {
        return String::new();
    }
    bytes
        .iter()
        .map(|byte| ALPHABET[(byte & 0x1f) as usize] as char)
        .collect()
}

/// Turn what someone typed into the canonical form, or `None` if it is not a
/// code.
///
/// Case, whitespace, `-`, `_` and the ambiguous letters are forgiven: `O`
/// becomes `0`, `I`/`L` become `1`, and `U` becomes `V`. Anything else is
/// rejected rather than guessed at.
pub fn normalize(raw: &str) -> Option<String> {
    let mut code = String::with_capacity(LEN);
    for character in raw.chars() {
        let character = match character {
            ' ' | '-' | '_' | '\t' | '\r' | '\n' => continue,
            'o' | 'O' => '0',
            'i' | 'I' | 'l' | 'L' => '1',
            'u' | 'U' => 'V',
            character => character.to_ascii_uppercase(),
        };
        if !ALPHABET.contains(&(character as u8)) {
            return None;
        }
        code.push(character);
    }
    (code.len() == LEN).then_some(code)
}

/// Whether `raw` is a canonical code.
pub fn is_valid(raw: &str) -> bool {
    normalize(raw).as_deref() == Some(raw)
}

/// The code in two groups of five, for reading aloud.
pub fn spaced(code: &str) -> String {
    match normalize(code) {
        Some(code) => format!("{} {}", &code[..5], &code[5..]),
        None => code.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_code_is_canonical_and_the_right_length() {
        let code = generate();
        assert_eq!(code.len(), LEN);
        assert!(is_valid(&code), "{code}");
    }

    #[test]
    fn typing_forgives_case_and_separators() {
        assert_eq!(normalize("abcd-efgh-!k"), None, "! is not in the alphabet");
        assert_eq!(normalize("0123-4567-89").as_deref(), Some("0123456789"));
        assert_eq!(
            normalize("oi1l2u34 56").as_deref(),
            Some("01112V3456"),
            "O -> 0, I/L -> 1, U -> V"
        );
    }

    #[test]
    fn a_short_or_wrong_character_is_not_a_code() {
        assert_eq!(normalize("0123"), None);
        assert_eq!(normalize("012345678!"), None);
        assert!(!is_valid("012345678"));
        assert!(!is_valid("0123456789 "));
        assert!(is_valid(&normalize("0123 4567 89").unwrap()));
    }

    #[test]
    fn spacing_is_for_people_only() {
        assert_eq!(spaced("0123456789"), "01234 56789");
        assert_eq!(spaced("01234-56789"), "01234 56789");
        assert_eq!(spaced("nonsense"), "nonsense");
    }
}
