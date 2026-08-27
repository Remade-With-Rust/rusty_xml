//! Character classes transcribed from libxml2 v2.15.3 `chvalid.h` / `chvalid.c`.
//! `xml_is_char` uses the `xmlIsCharQ` formula, not the range tables.

use crate::chvalid_tables::{
    XML_IS_BASE_CHAR_LRNG, XML_IS_BASE_CHAR_SRNG, XML_IS_COMBINING_LRNG, XML_IS_COMBINING_SRNG,
    XML_IS_DIGIT_LRNG, XML_IS_DIGIT_SRNG, XML_IS_EXTENDER_LRNG, XML_IS_EXTENDER_SRNG,
    XML_IS_PUBID_CHAR_TAB,
};

/// Binary search matching `xmlCharInRange` in `chvalid.c`.
#[doc(alias = "xmlCharInRange")]
pub fn xml_char_in_range(val: u32, short: &[(u16, u16)], long: &[(u32, u32)]) -> bool {
    if val < 0x10000 {
        if short.is_empty() {
            return false;
        }
        let mut low: i32 = 0;
        let mut high: i32 = short.len() as i32 - 1;
        let v = val as u16;
        while low <= high {
            let mid = ((low + high) / 2) as usize;
            if v < short[mid].0 {
                high = mid as i32 - 1;
            } else if v > short[mid].1 {
                low = mid as i32 + 1;
            } else {
                return true;
            }
        }
        false
    } else {
        if long.is_empty() {
            return false;
        }
        let mut low: i32 = 0;
        let mut high: i32 = long.len() as i32 - 1;
        while low <= high {
            let mid = ((low + high) / 2) as usize;
            if val < long[mid].0 {
                high = mid as i32 - 1;
            } else if val > long[mid].1 {
                low = mid as i32 + 1;
            } else {
                return true;
            }
        }
        false
    }
}

/// `xmlIsCharQ`.
#[doc(alias = "xmlIsChar")]
pub fn xml_is_char(c: u32) -> bool {
    if c < 0x100 {
        (0x09..=0x0a).contains(&c) || c == 0x0d || c >= 0x20
    } else {
        (0x100..=0xd7ff).contains(&c)
            || (0xe000..=0xfffd).contains(&c)
            || (0x10000..=0x10ffff).contains(&c)
    }
}

/// `xmlIsBlankQ`.
#[doc(alias = "xmlIsBlank")]
pub fn xml_is_blank(c: u32) -> bool {
    c == 0x20 || (0x09..=0x0a).contains(&c) || c == 0x0d
}

/// `xmlIsBaseCharQ`.
#[doc(alias = "xmlIsBaseChar")]
pub fn xml_is_base_char(c: u32) -> bool {
    if c < 0x100 {
        (0x41..=0x5a).contains(&c)
            || (0x61..=0x7a).contains(&c)
            || (0xc0..=0xd6).contains(&c)
            || (0xd8..=0xf6).contains(&c)
            || c >= 0xf8
    } else {
        xml_char_in_range(c, XML_IS_BASE_CHAR_SRNG, XML_IS_BASE_CHAR_LRNG)
    }
}

/// `xmlIsDigitQ`.
#[doc(alias = "xmlIsDigit")]
pub fn xml_is_digit(c: u32) -> bool {
    if c < 0x100 {
        (0x30..=0x39).contains(&c)
    } else {
        xml_char_in_range(c, XML_IS_DIGIT_SRNG, XML_IS_DIGIT_LRNG)
    }
}

/// `xmlIsCombiningQ`.
#[doc(alias = "xmlIsCombining")]
pub fn xml_is_combining(c: u32) -> bool {
    if c < 0x100 {
        false
    } else {
        xml_char_in_range(c, XML_IS_COMBINING_SRNG, XML_IS_COMBINING_LRNG)
    }
}

/// `xmlIsExtenderQ`.
#[doc(alias = "xmlIsExtender")]
pub fn xml_is_extender(c: u32) -> bool {
    if c < 0x100 {
        c == 0xb7
    } else {
        xml_char_in_range(c, XML_IS_EXTENDER_SRNG, XML_IS_EXTENDER_LRNG)
    }
}

/// `xmlIsIdeographicQ`.
#[doc(alias = "xmlIsIdeographic")]
pub fn xml_is_ideographic(c: u32) -> bool {
    if c < 0x100 {
        false
    } else {
        (0x4e00..=0x9fa5).contains(&c) || c == 0x3007 || (0x3021..=0x3029).contains(&c)
    }
}

/// `IS_LETTER`.
pub fn xml_is_letter(c: u32) -> bool {
    xml_is_base_char(c) || xml_is_ideographic(c)
}

/// `xmlIsPubidCharQ`.
#[doc(alias = "xmlIsPubidChar")]
pub fn xml_is_pubid_char(c: u32) -> bool {
    if c < 0x100 {
        XML_IS_PUBID_CHAR_TAB[c as usize] != 0
    } else {
        false
    }
}

/// XML 1.0 5th edition NameStartChar (default). `old10` uses Letter | '_' | ':'.
pub fn xml_is_name_start_char(c: u32, old10: bool) -> bool {
    if c == b' ' as u32 || c == b'>' as u32 || c == b'/' as u32 {
        return false;
    }
    if old10 {
        return xml_is_letter(c) || c == b'_' as u32 || c == b':' as u32;
    }
    (c >= b'a' as u32 && c <= b'z' as u32)
        || (c >= b'A' as u32 && c <= b'Z' as u32)
        || c == b'_' as u32
        || c == b':' as u32
        || (0xc0..=0xd6).contains(&c)
        || (0xd8..=0xf6).contains(&c)
        || (0xf8..=0x2ff).contains(&c)
        || (0x370..=0x37d).contains(&c)
        || (0x37f..=0x1fff).contains(&c)
        || (0x200c..=0x200d).contains(&c)
        || (0x2070..=0x218f).contains(&c)
        || (0x2c00..=0x2fef).contains(&c)
        || (0x3001..=0xd7ff).contains(&c)
        || (0xf900..=0xfdcf).contains(&c)
        || (0xfdf0..=0xfffd).contains(&c)
        || (0x10000..=0xeffff).contains(&c)
}

/// XML 1.0 5th edition NameChar (default).
pub fn xml_is_name_char(c: u32, old10: bool) -> bool {
    if c == b' ' as u32 || c == b'>' as u32 || c == b'/' as u32 {
        return false;
    }
    if old10 {
        return xml_is_letter(c)
            || xml_is_digit(c)
            || c == b'.' as u32
            || c == b'-' as u32
            || c == b'_' as u32
            || c == b':' as u32
            || xml_is_combining(c)
            || xml_is_extender(c);
    }
    xml_is_name_start_char(c, false)
        || (c >= b'0' as u32 && c <= b'9' as u32)
        || c == b'-' as u32
        || c == b'.' as u32
        || c == 0xb7
        || (0x300..=0x36f).contains(&c)
        || (0x203f..=0x2040).contains(&c)
}
