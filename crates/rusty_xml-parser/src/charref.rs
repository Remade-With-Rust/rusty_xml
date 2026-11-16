//! Numeric character references: one value parse for every caller (#70, #71).
//!
//! The XML content and attribute parser, entity expansion, DTD entity values
//! and the HTML decoder all turn `&#...;` digits into a code point. They used
//! to do it four ways, and a check added to one (XML's `Char` production)
//! never reached another: a reference inside an entity let U+0000 into an
//! attribute value. The digits-to-value step lives here once; each caller
//! keeps its own policy on top of it: XML refuses a value that is not a
//! `Char`, with its own error codes; HTML substitutes as libxml2's HTML
//! parser does.

/// Why digits have no value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bad {
    /// No digits, or a byte that is not a digit of the radix (a sign, an
    /// uppercase `X` marker, anything else).
    Digits,
    /// More than a `u32` holds.
    Overflow,
}

/// The run of `radix` digits (10 or 16) at the start of `bytes`, in one
/// pass: how many bytes it spans, and its value. The run is consumed whole
/// even past an overflow, so a caller always knows where the digits end. An
/// empty run is `(0, Err(Bad::Digits))`. A value past U+10FFFF that still
/// fits a `u32` is returned as it is, for the caller's policy to judge.
#[inline(always)]
pub(crate) fn scan(bytes: &[u8], radix: u32) -> (usize, Result<u32, Bad>) {
    // A u64 clamped just past u32::MAX: one compare per digit, as the HTML
    // decoder's clamped sum had, and an overflow is still told apart from a
    // value past U+10FFFF. Checked u32 arithmetic cost the HTML numeric
    // branch +0.3% parse Ir on entity-dense fragments.
    const PAST: u64 = u32::MAX as u64 + 1;
    let mut v: u64 = 0;
    let mut n = 0;
    while let Some(d) = bytes.get(n).and_then(|&b| (b as char).to_digit(radix)) {
        v = (v * u64::from(radix) + u64::from(d)).min(PAST);
        n += 1;
    }
    let r = if n == 0 {
        Err(Bad::Digits)
    } else if v == PAST {
        Err(Bad::Overflow)
    } else {
        Ok(v as u32)
    };
    (n, r)
}

/// The code point that `digits` write in `radix`, where `digits` must be the
/// whole text between the marker and the `;`: decimal `[0-9]+`, hexadecimal
/// `[0-9a-fA-F]+`. Anything else, a sign or an uppercase `X` included, is
/// `Bad::Digits`.
#[inline]
pub(crate) fn value(digits: &[u8], radix: u32) -> Result<u32, Bad> {
    match scan(digits, radix) {
        (n, r) if n == digits.len() => r,
        _ => Err(Bad::Digits),
    }
}

#[cfg(test)]
mod tests {
    use super::{scan, value, Bad};

    #[test]
    fn values_and_failures() {
        assert_eq!(value(b"65", 10), Ok(65));
        assert_eq!(value(b"0041", 16), Ok(0x41));
        assert_eq!(value(b"fFfE", 16), Ok(0xFFFE));
        assert_eq!(value(b"110000", 16), Ok(0x11_0000));
        assert_eq!(value(b"4294967295", 10), Ok(u32::MAX));
        assert_eq!(value(b"4294967296", 10), Err(Bad::Overflow));
        assert_eq!(value(b"100000000", 16), Err(Bad::Overflow));
        assert_eq!(value(b"", 10), Err(Bad::Digits));
        assert_eq!(value(b"+65", 10), Err(Bad::Digits));
        assert_eq!(value(b"X41", 16), Err(Bad::Digits));
        assert_eq!(value(b"4a", 10), Err(Bad::Digits));
        assert_eq!(value("\u{e9}".as_bytes(), 16), Err(Bad::Digits));
        assert_eq!(scan(b"65;", 10), (2, Ok(65)));
        assert_eq!(scan(b"41x", 16), (2, Ok(0x41)));
        assert_eq!(scan(b";", 10), (0, Err(Bad::Digits)));
        assert_eq!(scan(b"99999999999999999999;", 10), (20, Err(Bad::Overflow)));
    }

    /// The same results `u32::from_str_radix` gave the old callers on every
    /// digit string they could pass (no sign: they scanned digits first).
    #[test]
    fn agrees_with_from_str_radix_on_digit_strings() {
        for radix in [10u32, 16] {
            for s in ["0", "9", "00000000000065", "10ffff", "ffffffff", "fffffffff", "4294967296"] {
                if s.bytes().all(|b| (b as char).is_digit(radix)) {
                    let old = u32::from_str_radix(s, radix).ok();
                    assert_eq!(value(s.as_bytes(), radix).ok(), old, "{s} radix {radix}");
                }
            }
        }
    }
}
