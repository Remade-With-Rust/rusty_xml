//! Character encodings matching libxml2 `encoding.h` without iconv.
//! 8-bit tables are byte-identical to v2.15.3 `codegen/charset.inc`.

use crate::encoding_tables::*;
use crate::error::{XmlError, XML_ERR_UNSUPPORTED_ENCODING};

/// libxml2 `xmlCharEncoding` discriminants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum XmlCharEncoding {
    Error = -1,
    None = 0,
    Utf8 = 1,
    Utf16Le = 2,
    Utf16Be = 3,
    Ucs4Le = 4,
    Ucs4Be = 5,
    Ebcdic = 6,
    Ucs4_2143 = 7,
    Ucs4_3412 = 8,
    Ucs2 = 9,
    Iso8859_1 = 10,
    Iso8859_2 = 11,
    Iso8859_3 = 12,
    Iso8859_4 = 13,
    Iso8859_5 = 14,
    Iso8859_6 = 15,
    Iso8859_7 = 16,
    Iso8859_8 = 17,
    Iso8859_9 = 18,
    Iso2022Jp = 19,
    ShiftJis = 20,
    EucJp = 21,
    Ascii = 22,
    Utf16 = 23,
    Html = 24,
    Iso8859_10 = 25,
    Iso8859_11 = 26,
    Iso8859_13 = 27,
    Iso8859_14 = 28,
    Iso8859_15 = 29,
    Iso8859_16 = 30,
    Windows1252 = 31,
}

const NAME_MAP: &[(&str, XmlCharEncoding)] = &[
    ("ascii", XmlCharEncoding::Ascii),
    ("csisolatin1", XmlCharEncoding::Iso8859_1),
    ("iso-8859-1", XmlCharEncoding::Iso8859_1),
    ("iso-8859-2", XmlCharEncoding::Iso8859_2),
    ("iso-8859-3", XmlCharEncoding::Iso8859_3),
    ("iso-8859-4", XmlCharEncoding::Iso8859_4),
    ("iso-8859-5", XmlCharEncoding::Iso8859_5),
    ("iso-8859-6", XmlCharEncoding::Iso8859_6),
    ("iso-8859-7", XmlCharEncoding::Iso8859_7),
    ("iso-8859-8", XmlCharEncoding::Iso8859_8),
    ("iso-8859-9", XmlCharEncoding::Iso8859_9),
    ("iso-8859-10", XmlCharEncoding::Iso8859_10),
    ("iso-8859-11", XmlCharEncoding::Iso8859_11),
    ("iso-8859-13", XmlCharEncoding::Iso8859_13),
    ("iso-8859-14", XmlCharEncoding::Iso8859_14),
    ("iso-8859-15", XmlCharEncoding::Iso8859_15),
    ("iso-8859-16", XmlCharEncoding::Iso8859_16),
    ("iso8859-1", XmlCharEncoding::Iso8859_1),
    ("iso_8859-1", XmlCharEncoding::Iso8859_1),
    ("latin1", XmlCharEncoding::Iso8859_1),
    ("us-ascii", XmlCharEncoding::Ascii),
    ("utf-16", XmlCharEncoding::Utf16),
    ("utf-16be", XmlCharEncoding::Utf16Be),
    ("utf-16le", XmlCharEncoding::Utf16Le),
    ("utf-8", XmlCharEncoding::Utf8),
    ("utf16", XmlCharEncoding::Utf16),
    ("utf8", XmlCharEncoding::Utf8),
    ("unicode", XmlCharEncoding::Utf16),
    ("ucs-2", XmlCharEncoding::Ucs2),
    ("ucs-4", XmlCharEncoding::Ucs4Le),
    ("ucs2", XmlCharEncoding::Ucs2),
    ("ucs4", XmlCharEncoding::Ucs4Le),
    ("windows-1252", XmlCharEncoding::Windows1252),
    ("x-cp1252", XmlCharEncoding::Windows1252),
    ("ibm037", XmlCharEncoding::Ebcdic),
    ("ebcdic", XmlCharEncoding::Ebcdic),
    ("iso-2022-jp", XmlCharEncoding::Iso2022Jp),
    ("shift_jis", XmlCharEncoding::ShiftJis),
    ("shift-jis", XmlCharEncoding::ShiftJis),
    ("sjis", XmlCharEncoding::ShiftJis),
    ("euc-jp", XmlCharEncoding::EucJp),
];

/// `xmlParseCharEncoding`.
#[doc(alias = "xmlParseCharEncoding")]
pub fn xml_parse_char_encoding(name: &str) -> XmlCharEncoding {
    let lower = name.trim().to_ascii_lowercase();
    for (n, enc) in NAME_MAP {
        if *n == lower {
            return if *enc == XmlCharEncoding::Utf16 {
                XmlCharEncoding::Utf16Le
            } else {
                *enc
            };
        }
    }
    XmlCharEncoding::Error
}

/// `xmlGetCharEncodingName`.
#[doc(alias = "xmlGetCharEncodingName")]
pub fn xml_get_char_encoding_name(enc: XmlCharEncoding) -> Option<&'static str> {
    Some(match enc {
        XmlCharEncoding::Utf8 => "UTF-8",
        XmlCharEncoding::Utf16Le | XmlCharEncoding::Utf16Be | XmlCharEncoding::Utf16 => "UTF-16",
        XmlCharEncoding::Ucs4Le | XmlCharEncoding::Ucs4Be => "UCS-4",
        XmlCharEncoding::Iso8859_1 => "ISO-8859-1",
        XmlCharEncoding::Iso8859_2 => "ISO-8859-2",
        XmlCharEncoding::Iso8859_3 => "ISO-8859-3",
        XmlCharEncoding::Iso8859_4 => "ISO-8859-4",
        XmlCharEncoding::Iso8859_5 => "ISO-8859-5",
        XmlCharEncoding::Iso8859_6 => "ISO-8859-6",
        XmlCharEncoding::Iso8859_7 => "ISO-8859-7",
        XmlCharEncoding::Iso8859_8 => "ISO-8859-8",
        XmlCharEncoding::Iso8859_9 => "ISO-8859-9",
        XmlCharEncoding::Iso8859_10 => "ISO-8859-10",
        XmlCharEncoding::Iso8859_11 => "ISO-8859-11",
        XmlCharEncoding::Iso8859_13 => "ISO-8859-13",
        XmlCharEncoding::Iso8859_14 => "ISO-8859-14",
        XmlCharEncoding::Iso8859_15 => "ISO-8859-15",
        XmlCharEncoding::Iso8859_16 => "ISO-8859-16",
        XmlCharEncoding::Ascii => "US-ASCII",
        XmlCharEncoding::Windows1252 => "windows-1252",
        XmlCharEncoding::Ebcdic => "IBM037",
        XmlCharEncoding::Ucs2 => "UCS-2",
        XmlCharEncoding::Iso2022Jp => "ISO-2022-JP",
        XmlCharEncoding::ShiftJis => "Shift_JIS",
        XmlCharEncoding::EucJp => "EUC-JP",
        XmlCharEncoding::Html => "HTML",
        XmlCharEncoding::None | XmlCharEncoding::Error | XmlCharEncoding::Ucs4_2143 | XmlCharEncoding::Ucs4_3412 => {
            return None
        }
    })
}

/// `xmlDetectCharEncoding` — XML 1.0 appendix F plus libxml2's UTF-16 extras.
#[doc(alias = "xmlDetectCharEncoding")]
pub fn xml_detect_char_encoding(input: &[u8]) -> XmlCharEncoding {
    if input.len() >= 4 {
        let b = &input[..4];
        if b == [0x00, 0x00, 0x00, 0x3C] {
            return XmlCharEncoding::Ucs4Be;
        }
        if b == [0x3C, 0x00, 0x00, 0x00] {
            return XmlCharEncoding::Ucs4Le;
        }
        if b == [0x00, 0x00, 0x3C, 0x00] {
            return XmlCharEncoding::Ucs4_2143;
        }
        if b == [0x00, 0x3C, 0x00, 0x00] {
            return XmlCharEncoding::Ucs4_3412;
        }
        if b == [0x4C, 0x6F, 0xA7, 0x94] {
            return XmlCharEncoding::Ebcdic;
        }
        if b == [0x3C, 0x3F, 0x78, 0x6D] {
            return XmlCharEncoding::Utf8;
        }
        if b == [0x3C, 0x00, 0x3F, 0x00] {
            return XmlCharEncoding::Utf16Le;
        }
        if b == [0x00, 0x3C, 0x00, 0x3F] {
            return XmlCharEncoding::Utf16Be;
        }
        if b == [0x00, 0x00, 0xFE, 0xFF] {
            return XmlCharEncoding::Ucs4Be;
        }
        if b == [0xFF, 0xFE, 0x00, 0x00] {
            return XmlCharEncoding::Ucs4Le;
        }
    }
    if input.len() >= 3 && input[..3] == [0xEF, 0xBB, 0xBF] {
        return XmlCharEncoding::Utf8;
    }
    if input.len() >= 2 {
        if input[0] == 0xFE && input[1] == 0xFF {
            return XmlCharEncoding::Utf16Be;
        }
        if input[0] == 0xFF && input[1] == 0xFE {
            return XmlCharEncoding::Utf16Le;
        }
    }
    XmlCharEncoding::None
}

fn eightbit_to_utf8(input: &[u8], table: &[u16; 128]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 2);
    for &b in input {
        let cp = if b < 0x80 {
            b as u32
        } else {
            table[(b - 0x80) as usize] as u32
        };
        if let Some(c) = char::from_u32(cp) {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    out
}

fn latin1_to_utf8(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 2);
    for &b in input {
        let c = b as char;
        let mut buf = [0u8; 4];
        out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    }
    out
}

fn utf16_to_utf8(input: &[u8], be: bool) -> Result<Vec<u8>, XmlError> {
    let mut i = 0;
    if input.len() >= 2 {
        let bom = if be {
            input[0] == 0xFE && input[1] == 0xFF
        } else {
            input[0] == 0xFF && input[1] == 0xFE
        };
        if bom {
            i = 2;
        }
    }
    let mut units = Vec::new();
    while i + 1 < input.len() {
        let u = if be {
            u16::from_be_bytes([input[i], input[i + 1]])
        } else {
            u16::from_le_bytes([input[i], input[i + 1]])
        };
        units.push(u);
        i += 2;
    }
    let s = String::from_utf16(&units).map_err(|_| {
        XmlError::new(XML_ERR_UNSUPPORTED_ENCODING, "Invalid UTF-16", 0, 0)
    })?;
    Ok(s.into_bytes())
}

fn ucs4_to_utf8(input: &[u8], order: [usize; 4]) -> Result<Vec<u8>, XmlError> {
    let mut i = 0;
    if input.len() >= 4 {
        let w = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);
        if w == 0xFEFF || w == 0xFFFE0000 {
            i = 4;
        }
    }
    let mut out = String::new();
    while i + 3 < input.len() {
        let b = [input[i], input[i + 1], input[i + 2], input[i + 3]];
        let cp = u32::from_be_bytes([b[order[0]], b[order[1]], b[order[2]], b[order[3]]]);
        match char::from_u32(cp) {
            Some(c) if c != '\u{feff}' || !out.is_empty() => out.push(c),
            Some(_) => {}
            None => {
                return Err(XmlError::new(
                    XML_ERR_UNSUPPORTED_ENCODING,
                    "Invalid UCS-4",
                    0,
                    0,
                ));
            }
        }
        i += 4;
    }
    Ok(out.into_bytes())
}

/// IBM037 (EBCDIC) — enough to convert XML documents detected as `4C 6F A7 94`.
static CP037: [u16; 256] = [
    0x00, 0x01, 0x02, 0x03, 0x9c, 0x09, 0x86, 0x7f, 0x97, 0x8d, 0x8e, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x9d, 0x85, 0x08, 0x87, 0x18, 0x19, 0x92, 0x8f, 0x1c, 0x1d, 0x1e, 0x1f,
    0x80, 0x81, 0x82, 0x83, 0x84, 0x0a, 0x17, 0x1b, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x05, 0x06, 0x07,
    0x90, 0x91, 0x16, 0x93, 0x94, 0x95, 0x96, 0x04, 0x98, 0x99, 0x9a, 0x9b, 0x14, 0x15, 0x9e, 0x1a,
    0x20, 0xa0, 0xe2, 0xe4, 0xe0, 0xe1, 0xe3, 0xe5, 0xe7, 0xf1, 0xa2, 0x2e, 0x3c, 0x28, 0x2b, 0x7c,
    0x26, 0xe9, 0xea, 0xeb, 0xe8, 0xed, 0xee, 0xef, 0xec, 0xdf, 0x21, 0x24, 0x2a, 0x29, 0x3b, 0xac,
    0x2d, 0x2f, 0xc2, 0xc4, 0xc0, 0xc1, 0xc3, 0xc5, 0xc7, 0xd1, 0xa6, 0x2c, 0x25, 0x5f, 0x3e, 0x3f,
    0xf8, 0xc9, 0xca, 0xcb, 0xc8, 0xcd, 0xce, 0xcf, 0xcc, 0x60, 0x3a, 0x23, 0x40, 0x27, 0x3d, 0x22,
    0xd8, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0xab, 0xbb, 0xf0, 0xfd, 0xfe, 0xb1,
    0xb0, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0xaa, 0xba, 0xe6, 0xb8, 0xc6, 0xa4,
    0xb5, 0x7e, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0xa1, 0xbf, 0xd0, 0xdd, 0xde, 0xae,
    0x5e, 0xa3, 0xa5, 0xb7, 0xa9, 0xa7, 0xb6, 0xbc, 0xbd, 0xbe, 0x5b, 0x5d, 0xaf, 0xa8, 0xb4, 0xd7,
    0x7b, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0xad, 0xf4, 0xf6, 0xf2, 0xf3, 0xf5,
    0x7d, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0xb9, 0xfb, 0xfc, 0xf9, 0xfa, 0xff,
    0x5c, 0xf7, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0xb2, 0xd4, 0xd6, 0xd2, 0xd3, 0xd5,
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0xb3, 0xdb, 0xdc, 0xd9, 0xda, 0x9f,
];

fn table_for(enc: XmlCharEncoding) -> Option<&'static [u16; 128]> {
    Some(match enc {
        XmlCharEncoding::Iso8859_2 => &XML_UNICODE_ISO8859_2,
        XmlCharEncoding::Iso8859_3 => &XML_UNICODE_ISO8859_3,
        XmlCharEncoding::Iso8859_4 => &XML_UNICODE_ISO8859_4,
        XmlCharEncoding::Iso8859_5 => &XML_UNICODE_ISO8859_5,
        XmlCharEncoding::Iso8859_6 => &XML_UNICODE_ISO8859_6,
        XmlCharEncoding::Iso8859_7 => &XML_UNICODE_ISO8859_7,
        XmlCharEncoding::Iso8859_8 => &XML_UNICODE_ISO8859_8,
        XmlCharEncoding::Iso8859_9 => &XML_UNICODE_ISO8859_9,
        XmlCharEncoding::Iso8859_10 => &XML_UNICODE_ISO8859_10,
        XmlCharEncoding::Iso8859_11 => &XML_UNICODE_ISO8859_11,
        XmlCharEncoding::Iso8859_13 => &XML_UNICODE_ISO8859_13,
        XmlCharEncoding::Iso8859_14 => &XML_UNICODE_ISO8859_14,
        XmlCharEncoding::Iso8859_15 => &XML_UNICODE_ISO8859_15,
        XmlCharEncoding::Iso8859_16 => &XML_UNICODE_ISO8859_16,
        XmlCharEncoding::Windows1252 => &XML_UNICODE_windows_1252,
        _ => return None,
    })
}

fn sniff_encoding_decl(bytes: &[u8]) -> Option<XmlCharEncoding> {
    // The declaration is ASCII. Scanning the bytes directly avoids building a
    // 1 KB String and a second lowercased copy of it on every single parse.
    let n = bytes.len().min(1024);
    let head = &bytes[..n];
    if head.len() < 8 {
        return None;
    }
    let idx = head
        .windows(8)
        .position(|w| w.eq_ignore_ascii_case(b"encoding"))?;
    let mut i = idx + 8;
    while i < n && matches!(head[i], b' ' | b'\t' | b'\r' | b'\n' | b'=') {
        i += 1;
    }
    if i >= n || (head[i] != b'"' && head[i] != b'\'') {
        return None;
    }
    let q = head[i];
    i += 1;
    let start = i;
    while i < n && head[i] != q {
        i += 1;
    }
    if i >= n {
        return None;
    }
    let name = std::str::from_utf8(&head[start..i]).ok()?;
    xml_parse_char_encoding(name).into_option()
}

impl XmlCharEncoding {
    fn into_option(self) -> Option<XmlCharEncoding> {
        match self {
            XmlCharEncoding::Error | XmlCharEncoding::None => None,
            e => Some(e),
        }
    }
}

/// Convert `input` to UTF-8. `hint` is the `encoding` argument to `xmlReadMemory`.
pub fn xml_convert_to_utf8(
    input: &[u8],
    hint: Option<&str>,
) -> Result<(Vec<u8>, Option<String>), XmlError> {
    let (c, n) = xml_convert_to_utf8_cow(input, hint)?;
    Ok((c.into_owned(), n.map(str::to_string)))
}

/// As [`xml_convert_to_utf8`], but borrows the input when it is already UTF-8.
/// The overwhelmingly common case is a UTF-8 document, and copying it whole
/// before parsing costs one allocation and one full memcpy per parse.
pub(crate) fn xml_convert_to_utf8_cow<'a>(
    input: &'a [u8],
    hint: Option<&str>,
) -> Result<(std::borrow::Cow<'a, [u8]>, Option<&'static str>), XmlError> {
    if input.len() >= 2 && input[0] == 0x1f && input[1] == 0x8b {
        return Err(XmlError::new(
            XML_ERR_UNSUPPORTED_ENCODING,
            "gzip input requires the unzip feature / XML_PARSE_UNZIP",
            0,
            0,
        ));
    }
    let mut enc = hint
        .and_then(|h| xml_parse_char_encoding(h).into_option())
        .unwrap_or_else(|| xml_detect_char_encoding(input));
    if enc == XmlCharEncoding::None || enc == XmlCharEncoding::Utf8 {
        if let Some(d) = sniff_encoding_decl(input) {
            if d != XmlCharEncoding::Utf8 && d != XmlCharEncoding::Ascii {
                enc = d;
            }
        }
    }
    if enc == XmlCharEncoding::None || enc == XmlCharEncoding::Utf8 || enc == XmlCharEncoding::Ascii
    {
        // Slice past a BOM rather than draining it, which memmoved the document.
        let body = if input.starts_with(&[0xEF, 0xBB, 0xBF]) { &input[3..] } else { input };
        return Ok((std::borrow::Cow::Borrowed(body), xml_get_char_encoding_name(enc)));
    }
    let converted = match enc {
        XmlCharEncoding::Iso8859_1 => latin1_to_utf8(input),
        XmlCharEncoding::Utf16Le | XmlCharEncoding::Utf16 => utf16_to_utf8(input, false)?,
        XmlCharEncoding::Utf16Be => utf16_to_utf8(input, true)?,
        XmlCharEncoding::Ucs4Be => ucs4_to_utf8(input, [0, 1, 2, 3])?,
        XmlCharEncoding::Ucs4Le => ucs4_to_utf8(input, [3, 2, 1, 0])?,
        XmlCharEncoding::Ucs4_2143 => ucs4_to_utf8(input, [1, 0, 3, 2])?,
        XmlCharEncoding::Ucs4_3412 => ucs4_to_utf8(input, [2, 3, 0, 1])?,
        XmlCharEncoding::Ucs2 => utf16_to_utf8(input, true)?,
        XmlCharEncoding::Ebcdic => {
            let mut out = Vec::new();
            for &b in input {
                let c = char::from_u32(CP037[b as usize] as u32).unwrap_or('\u{fffd}');
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            out
        }
        other => {
            if let Some(t) = table_for(other) {
                eightbit_to_utf8(input, t)
            } else {
                return Err(XmlError::new(
                    XML_ERR_UNSUPPORTED_ENCODING,
                    format!(
                        "Unsupported encoding {}",
                        xml_get_char_encoding_name(other).unwrap_or("?")
                    ),
                    0,
                    0,
                ));
            }
        }
    };
    Ok((
        std::borrow::Cow::Owned(converted),
        xml_get_char_encoding_name(enc),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin1_converts() {
        let (u, _) = xml_convert_to_utf8(&[0xE9], Some("ISO-8859-1")).unwrap();
        assert_eq!(u, "é".as_bytes());
    }

    #[test]
    fn utf16_le_bom() {
        // BOM + `<a/>` in UTF-16LE
        let mut b = vec![0xFF, 0xFE];
        for c in "<a/>".encode_utf16() {
            b.extend_from_slice(&c.to_le_bytes());
        }
        let (u, _) = xml_convert_to_utf8(&b, None).unwrap();
        assert_eq!(std::str::from_utf8(&u).unwrap().trim_start_matches('\u{feff}'), "<a/>");
    }

    #[test]
    fn iso8859_2_table() {
        // 0xA1 → U+0104 LATIN CAPITAL LETTER A WITH OGONEK
        let (u, _) = xml_convert_to_utf8(&[0xA1], Some("ISO-8859-2")).unwrap();
        assert_eq!(u, "Ą".as_bytes());
    }

    #[test]
    fn parse_encoding_names() {
        assert_eq!(xml_parse_char_encoding("utf-8"), XmlCharEncoding::Utf8);
        assert_eq!(xml_parse_char_encoding("latin1"), XmlCharEncoding::Iso8859_1);
        assert_eq!(xml_parse_char_encoding("windows-1252"), XmlCharEncoding::Windows1252);
    }
}

