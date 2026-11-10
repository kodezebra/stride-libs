//! Comparing the pairing token without leaking how much of it matched.

use crate::pairing;

/// Whether `supplied` is the token `expected`.
///
/// Both sides are normalized first, so a token typed with spaces or lower case
/// still matches. The comparison itself visits every character and accumulates
/// differences instead of returning on the first one: the caller is on a LAN,
/// but a token check is cheap enough to do properly.
pub fn matches(expected: &str, supplied: &str) -> bool {
    let (Some(expected), Some(supplied)) =
        (pairing::normalize(expected), pairing::normalize(supplied))
    else {
        return false;
    };
    if expected.len() != supplied.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in expected.bytes().zip(supplied.bytes()) {
        difference |= left ^ right;
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_code_matches_in_any_handwriting() {
        let token = "0123456789";
        assert!(matches(token, token));
        assert!(matches(token, "01234 56789"));
        assert!(matches(token, "0123-4567-89"));
        assert!(matches("ABCDEFGHJK", "abcdefghjk"));
    }

    #[test]
    fn a_different_code_does_not() {
        assert!(!matches("0123456789", "0123456780"));
        assert!(!matches("0123456789", "012345678"));
        assert!(!matches("0123456789", ""));
        assert!(!matches("0123456789", "not a token"));
    }

    #[test]
    fn an_empty_expectation_never_matches() {
        assert!(!matches("", ""));
        assert!(!matches("", "0123456789"));
    }
}
