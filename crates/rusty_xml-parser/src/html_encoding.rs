//! The input encoding of an HTML document with no BOM and no `encoding`
//! argument: libxml2 2.15.3's decision, made before parsing (#27).
//!
//! libxml2 decides while it parses. Until it has decided:
//!
//! - a `<meta>` start tag with a usable `charset`, or `http-equiv=Content-Type`
//!   and a `charset` in `content`, switches to that encoding (`htmlCheckMeta`).
//!   The label is read in HTML mode, where `iso-8859-1` and `us-ascii` mean
//!   windows-1252. Labels it cannot decode, and ones that are not
//!   ASCII-compatible, are ignored and the search goes on.
//! - the first non-ASCII byte in text, raw text, an attribute value, a comment
//!   or a doctype decides instead (`htmlParseCharData`, `htmlParseData`): a text
//!   search of the rest of the input for `HTTP-EQUIV`, `CONTENT`, `CHARSET=`
//!   (`htmlFindEncoding`), read without the HTML mapping. A label it cannot
//!   decode leaves the input as UTF-8. With no label it falls back to
//!   windows-1252.
//! - a non-ASCII byte in a tag or attribute name decides nothing; libxml2
//!   reads names as UTF-8.
//!
//! [`decide`] walks the tokens the parser would see, without building
//! anything, up to that decision. Decoding the bytes before the decision as
//! UTF-8 and the rest with the chosen encoding then gives libxml2's tree.
//!
//! One deliberate difference: undeclared input that is valid UTF-8 stays
//! UTF-8. libxml2 reads it as windows-1252 (its source marks the HTML5 prescan
//! as unfinished), which turns `café` into `cafÃ©`; UTF-8 is what such pages
//! almost always are.

use std::borrow::Cow;

use crate::encoding::{
    is_single_byte, single_byte_to_utf8, xml_parse_char_encoding, XmlCharEncoding,
};
use crate::html_entities::{decode_html, Nul};
use crate::html_tags as tags;

/// Where and how the input switches away from UTF-8.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// UTF-8 throughout, invalid sequences becoming U+FFFD.
    Utf8,
    /// UTF-8 before `at`, then `enc` (an ASCII-compatible single-byte encoding).
    Switch { at: usize, enc: XmlCharEncoding },
}

/// Decode `b` as [`decide`] says. The input is validated as UTF-8 once, here,
/// and the scanner and the result both reuse that.
pub(crate) fn decode(b: &[u8]) -> Cow<'_, str> {
    let valid = std::str::from_utf8(b).ok();
    match decide(b, valid) {
        Decision::Utf8 => match valid {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned(String::from_utf8_lossy(b).into_owned()),
        },
        Decision::Switch { at, enc } => {
            let mut out = String::from_utf8_lossy(&b[..at]).into_owned();
            let rest =
                single_byte_to_utf8(&b[at..], enc).expect("decide returns decodable encodings");
            out.push_str(&String::from_utf8_lossy(&rest));
            Cow::Owned(out)
        }
    }
}

/// The index of the first byte in `b[from..limit]` that is not ASCII, or
/// `limit`. Sixteen bytes at a time: `is_ascii` on a fixed block is a few word
/// tests, where a byte loop with an exit inside does not vectorize.
fn next_non_ascii(b: &[u8], from: usize, limit: usize) -> usize {
    let limit = limit.min(b.len());
    let mut i = from;
    while i + 16 <= limit && b[i..i + 16].is_ascii() {
        i += 16;
    }
    while i < limit && b[i] < 0x80 {
        i += 1;
    }
    i
}

#[inline]
fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0c | b'\r')
}

/// The first window. A page that declares `<meta charset>` near its top is
/// decided inside it, so the rest of the input is never scanned.
const WINDOW: usize = 4096;

/// The scanner. `ascii` is `b[lo..hi]`, a run of ASCII bytes; searches run on
/// it. When a search reaches `hi` the window grows, until `hi` is the end of
/// input or a non-ASCII byte, where a decision may be due. `lo` moves only past
/// a name holding non-ASCII bytes (which may not be valid UTF-8), so the window
/// never contains one.
struct Scan<'a> {
    b: &'a [u8],
    /// The whole input, when it is valid UTF-8: windows are sliced from it
    /// instead of validated again.
    src: Option<&'a str>,
    lo: usize,
    ascii: &'a str,
    pos: usize,
}

/// What a step of the scan found.
enum Step {
    Continue,
    Done(Decision),
}

impl<'a> Scan<'a> {
    fn new(b: &'a [u8], src: Option<&'a str>, at: usize) -> Self {
        let hi = next_non_ascii(b, at, at + WINDOW);
        Scan {
            b,
            src,
            lo: at,
            ascii: Self::slice(b, src, at, hi),
            pos: at,
        }
    }

    /// `b[lo..hi]`, known to be ASCII.
    fn slice(b: &'a [u8], src: Option<&'a str>, lo: usize, hi: usize) -> &'a str {
        match src {
            Some(s) => &s[lo..hi],
            None => std::str::from_utf8(&b[lo..hi]).expect("ASCII is UTF-8"),
        }
    }

    fn hi(&self) -> usize {
        self.lo + self.ascii.len()
    }

    /// Grow the window to the next non-ASCII byte or the end of input, in one
    /// flat pass. False when it already ends there.
    fn extend(&mut self) -> bool {
        let hi = self.hi();
        if hi == self.b.len() || self.b[hi] >= 0x80 {
            return false;
        }
        let new_hi = next_non_ascii(self.b, hi, self.b.len());
        self.ascii = Self::slice(self.b, self.src, self.lo, new_hi);
        true
    }

    /// Whether everything from `pos` on is ASCII. No decision can then change
    /// a byte of the decoded text.
    fn rest_is_ascii(&self) -> bool {
        self.hi() == self.b.len() && self.pos >= self.lo
    }

    /// Re-anchor the window after a name ran past it.
    fn rescan(&mut self) {
        if self.pos > self.hi() {
            *self = Scan::new(self.b, self.src, self.pos);
        }
    }

    /// The decision for a non-ASCII byte at `at` in a deciding position, or
    /// for the end of input.
    fn decided_at(&self, at: usize) -> Step {
        if at >= self.b.len() {
            Step::Done(Decision::Utf8)
        } else {
            Step::Done(by_text_search(self.b, at, self.src.is_some()))
        }
    }

    /// The decision when a search ran out of window: the window then ends at
    /// the end of input or at a non-ASCII byte.
    fn ran_out(&self) -> Step {
        self.decided_at(self.hi())
    }

    /// `pat` at or after `from`, growing the window as needed. `None` means it
    /// is not there before the window's end, which is then a real one.
    fn find(&mut self, from: usize, pat: &str) -> Option<usize> {
        let mut start = from;
        loop {
            let hay = self.ascii.get(start.checked_sub(self.lo)?..)?;
            // A one-byte pattern is a memchr; `str::find(&str)` would build a
            // substring searcher for it on every call.
            let hit = match pat.as_bytes() {
                [c] => hay.find(char::from(*c)),
                _ => hay.find(pat),
            };
            if let Some(k) = hit {
                return Some(start + k);
            }
            let hi = self.hi();
            if !self.extend() {
                return None;
            }
            start = hi.saturating_sub(pat.len() - 1).max(from);
        }
    }

    fn skip_blanks(&mut self) {
        while self.pos < self.b.len() && is_ws(self.b[self.pos]) {
            self.pos += 1;
        }
    }

    /// A tag or attribute name, as `Parser::name` reads it: the first byte
    /// unconditionally, then up to a blank, `/`, `>` (or `=`). Non-ASCII bytes
    /// are part of it and decide nothing.
    fn name(&mut self, attr: bool) -> (usize, usize) {
        let start = self.pos;
        let mut i = start + 1;
        while i < self.b.len() {
            let c = self.b[i];
            if c == b'/' || c == b'>' || is_ws(c) || (attr && c == b'=') {
                break;
            }
            i += 1;
        }
        self.pos = i.min(self.b.len());
        self.rescan();
        (start, self.pos)
    }

    /// A comment (bogus or real) from `start`, as `Parser::comment` ends it.
    fn comment(&mut self, start: usize, bogus: bool) -> Step {
        if bogus {
            return match self.find(start, ">") {
                Some(e) => {
                    self.pos = e + 1;
                    Step::Continue
                }
                None => self.ran_out(),
            };
        }
        let b = self.b;
        if b.get(start) == Some(&b'>') {
            self.pos = start + 1;
            return Step::Continue;
        }
        if b[start..].starts_with(b"->") {
            self.pos = start + 2;
            return Step::Continue;
        }
        let mut from = start;
        loop {
            let Some(d) = self.find(from, "--") else {
                return self.ran_out();
            };
            match b.get(d + 2) {
                Some(b'>') => {
                    self.pos = d + 3;
                    return Step::Continue;
                }
                Some(b'!') if b.get(d + 3) == Some(&b'>') => {
                    self.pos = d + 4;
                    return Step::Continue;
                }
                _ => from = d + 1,
            }
        }
    }

    /// The attribute list of a tag, up to `>`, `/>` or the end of input, as
    /// `Parser::attributes` reads it. Collects name/value pairs when `keep`.
    /// `Err` is a decision: a non-ASCII byte in a value.
    fn attributes(&mut self, keep: bool, out: &mut Vec<(String, String)>) -> Result<(), Step> {
        let b = self.b;
        self.skip_blanks();
        while self.pos < b.len() {
            match b[self.pos] {
                b'>' => break,
                b'/' => {
                    if b.get(self.pos + 1) == Some(&b'>') {
                        break;
                    }
                    self.pos += 1;
                    self.skip_blanks();
                    continue;
                }
                _ => {}
            }
            let (ns, ne) = self.name(true);
            self.skip_blanks();
            let mut value = None;
            if b.get(self.pos) == Some(&b'=') {
                self.pos += 1;
                self.skip_blanks();
                let (vs, ve) = match b.get(self.pos) {
                    Some(&q @ (b'"' | b'\'')) => {
                        let from = self.pos + 1;
                        let quote = if q == b'"' { "\"" } else { "'" };
                        match self.find(from, quote) {
                            Some(e) => {
                                self.pos = e + 1;
                                (from, e)
                            }
                            None => return Err(self.ran_out()),
                        }
                    }
                    _ => {
                        let mut i = self.pos;
                        while i < b.len() && !is_ws(b[i]) && b[i] != b'>' {
                            if b[i] >= 0x80 {
                                return Err(self.decided_at(i));
                            }
                            i += 1;
                        }
                        let s = self.pos;
                        self.pos = i;
                        (s, i)
                    }
                };
                value = Some((vs, ve));
            }
            if keep {
                if let Some((vs, ve)) = value {
                    let name = String::from_utf8_lossy(&b[ns..ne]).to_ascii_lowercase();
                    let raw = std::str::from_utf8(&b[vs..ve]).expect("ASCII is UTF-8");
                    out.push((name, decode_html(raw, true, Nul::Replace).into_owned()));
                }
            }
            self.skip_blanks();
        }
        Ok(())
    }

    /// A start tag at `<`. Returns the decision a `<meta>` makes, if any.
    fn start_tag(&mut self, attrs: &mut Vec<(String, String)>) -> Step {
        self.pos += 1;
        let (ns, ne) = self.name(false);
        let mut lower = [0u8; 16];
        let tag = if ne - ns < lower.len() {
            for (d, s) in lower.iter_mut().zip(&self.b[ns..ne]) {
                *d = s.to_ascii_lowercase();
            }
            tags::tag_of(&lower[..ne - ns])
        } else {
            tags::UNKNOWN
        };
        let is_meta = self.b[ns..ne].eq_ignore_ascii_case(b"meta");
        attrs.clear();
        if let Err(step) = self.attributes(is_meta, attrs) {
            return step;
        }
        if self.pos >= self.b.len() {
            // A tag cut off by the end of input is dropped.
            return Step::Done(Decision::Utf8);
        }
        if is_meta {
            if let Some(d) = check_meta(attrs, self.pos) {
                return Step::Done(d);
            }
        }
        if self.b[self.pos..].starts_with(b"/>") {
            self.pos += 2;
            return Step::Continue;
        }
        self.pos += 1;
        if tags::is_empty(tag) {
            return Step::Continue;
        }
        match tags::data_mode(tag) {
            0 => Step::Continue,
            tags::DATA_PLAINTEXT => {
                // The rest of the input is text: its first non-ASCII byte decides.
                while self.extend() {}
                let at = next_non_ascii(self.b, self.pos.max(self.lo), self.b.len());
                self.decided_at(at)
            }
            mode => {
                // Known tags only, so the name fits `lower`.
                let name = &lower[..ne - ns];
                loop {
                    self.rescan();
                    let lo = self.lo;
                    let (end, found) = crate::html::scan_raw(
                        self.ascii,
                        self.ascii.as_bytes(),
                        self.pos - lo,
                        name,
                        mode,
                    );
                    if found {
                        self.pos = lo + end;
                        return Step::Continue;
                    }
                    if !self.extend() {
                        return self.ran_out();
                    }
                }
            }
        }
    }

    /// An end tag at `<`, as `Parser::end_tag` reads it.
    fn end_tag(&mut self, attrs: &mut Vec<(String, String)>) -> Step {
        self.pos += 2;
        let Some(&c) = self.b.get(self.pos) else {
            return Step::Done(Decision::Utf8);
        };
        if c == b'>' {
            self.pos += 1;
            return Step::Continue;
        }
        if !c.is_ascii_alphabetic() {
            return self.comment(self.pos, true);
        }
        self.name(false);
        if let Err(step) = self.attributes(false, attrs) {
            return step;
        }
        match self.b.get(self.pos) {
            Some(b'>') => self.pos += 1,
            Some(b'/') => self.pos += 2,
            _ => return Step::Done(Decision::Utf8),
        }
        Step::Continue
    }
}

/// libxml2's decision for input with no BOM and no `encoding` argument.
/// `src` is the input as text when it is valid UTF-8.
pub(crate) fn decide<'a>(b: &'a [u8], src: Option<&'a str>) -> Decision {
    let mut s = Scan::new(b, src, 0);
    if s.rest_is_ascii() {
        return Decision::Utf8;
    }
    // Every way to another encoding names `charset`: a `charset` attribute,
    // `charset` in a Content-Type `content`, or the `CHARSET=` text search.
    // Valid UTF-8 without it stays UTF-8 whatever the tokens are, so the walk
    // is skipped. (A `content` that spells the word with character references
    // is not seen.)
    if src.is_some() && !contains_charset(b) {
        return Decision::Utf8;
    }
    let mut attrs = Vec::new();
    loop {
        if s.rest_is_ascii() {
            return Decision::Utf8;
        }
        let Some(lt) = s.find(s.pos, "<") else {
            // Text runs into a non-ASCII byte, or to the end.
            return match s.ran_out() {
                Step::Done(d) => d,
                Step::Continue => unreachable!(),
            };
        };
        s.pos = lt;
        let step = match b.get(lt + 1) {
            Some(b'/') => s.end_tag(&mut attrs),
            Some(b'!') => {
                if b.get(lt + 2..lt + 9)
                    .is_some_and(|k| k.eq_ignore_ascii_case(b"DOCTYPE"))
                {
                    s.comment(lt + 9, true)
                } else if b[lt..].starts_with(b"<!--") {
                    s.comment(lt + 4, false)
                } else {
                    s.comment(lt + 2, true)
                }
            }
            Some(b'?') => s.comment(lt + 1, true),
            Some(c) if c.is_ascii_alphabetic() => s.start_tag(&mut attrs),
            _ => {
                s.pos = lt + 1;
                Step::Continue
            }
        };
        if let Step::Done(d) = step {
            return d;
        }
    }
}

/// `htmlCheckMeta`: the encoding a `<meta>` names, read in HTML mode, if
/// libxml2 could switch to it. `at` is where the switch takes effect.
fn check_meta(attrs: &[(String, String)], at: usize) -> Option<Decision> {
    let mut seen: Vec<&str> = Vec::new();
    let (mut content_type, mut content, mut label) = (false, None, None);
    for (name, value) in attrs {
        // libxml2 drops a repeated attribute before this check sees it.
        if seen.contains(&name.as_str()) {
            continue;
        }
        seen.push(name);
        if name == "http-equiv" && value.eq_ignore_ascii_case("content-type") {
            content_type = true;
        } else if name == "charset" {
            label = Some(value.as_str());
            break;
        } else if name == "content" {
            content = Some(value.as_str());
        }
    }
    if label.is_none() && content_type {
        label = content.and_then(parse_content_type);
    }
    let enc = match xml_parse_char_encoding(label?) {
        XmlCharEncoding::Ascii | XmlCharEncoding::Iso8859_1 => XmlCharEncoding::Windows1252,
        e => e,
    };
    match enc {
        XmlCharEncoding::Utf8 => Some(Decision::Utf8),
        e if is_single_byte(e) => Some(Decision::Switch { at, enc: e }),
        // Unsupported, or not ASCII-compatible (UTF-16): ignored.
        _ => None,
    }
}

/// `htmlParseContentType`: the `charset` parameter of a Content-Type value.
fn parse_content_type(val: &str) -> Option<&str> {
    let b = val.as_bytes();
    let mut p = 0;
    loop {
        while p < b.len() && b[p] != b'c' && b[p] != b'C' {
            p += 1;
        }
        if p >= b.len() {
            return None;
        }
        p += 1;
        if !b
            .get(p..p + 6)
            .is_some_and(|k| k.eq_ignore_ascii_case(b"harset"))
        {
            continue;
        }
        p += 6;
        while p < b.len() && is_ws(b[p]) {
            p += 1;
        }
        if b.get(p) != Some(&b'=') {
            continue;
        }
        p += 1;
        while p < b.len() && is_ws(b[p]) {
            p += 1;
        }
        let &q = b.get(p)?;
        if q == b'"' || q == b'\'' {
            p += 1;
            while p < b.len() && is_ws(b[p]) {
                p += 1;
            }
            let start = p;
            let mut end = start;
            while *b.get(p)? != q {
                if !is_ws(b[p]) {
                    end = p + 1;
                }
                p += 1;
            }
            return Some(&val[start..end]);
        }
        let start = p;
        while p < b.len() && b[p] != b';' && !is_ws(b[p]) {
            p += 1;
        }
        return Some(&val[start..p]);
    }
}

/// `htmlFindEncoding`, then the fallback, for a decision at `at`.
fn by_text_search(b: &[u8], at: usize, valid: bool) -> Decision {
    let found = find_ci(b, at, b"http-equiv")
        .and_then(|i| find_ci(b, i, b"content"))
        .and_then(|i| find_ci(b, i, b"charset="))
        .and_then(|i| {
            let start = i + 8;
            let mut end = start;
            while end < b.len()
                && (b[end].is_ascii_alphanumeric() || matches!(b[end], b'-' | b'_' | b':' | b'/'))
            {
                end += 1;
            }
            (end > start).then(|| &b[start..end])
        });
    match found {
        Some(label) => {
            // No HTML mapping here: `iso-8859-1` is Latin-1.
            let enc = xml_parse_char_encoding(std::str::from_utf8(label).unwrap_or(""));
            if is_single_byte(enc) {
                Decision::Switch { at, enc }
            } else {
                // UTF-8, or a label libxml2 cannot use: it stays on UTF-8.
                Decision::Utf8
            }
        }
        // libxml2 falls back to windows-1252 here; valid UTF-8 is kept.
        None if valid => Decision::Utf8,
        None => Decision::Switch {
            at,
            enc: XmlCharEncoding::Windows1252,
        },
    }
}

/// Whether `b` holds `charset` in any ASCII case. Thirty-two positions at a
/// time are tested for `ch` with no exit inside, which vectorizes; only a block
/// with a candidate is checked byte by byte.
fn contains_charset(b: &[u8]) -> bool {
    const W: &[u8] = b"charset";
    let at = |i: usize| {
        b.get(i..i + W.len())
            .is_some_and(|w| w.eq_ignore_ascii_case(W))
    };
    let mut i = 0;
    while i + 33 <= b.len() {
        // A fixed-size view, so the index loop carries no bounds checks and
        // compiles to vector compares.
        let w: &[u8; 33] = b[i..i + 33].try_into().expect("33 bytes");
        let mut hit = 0u8;
        for k in 0..32 {
            hit |= u8::from(w[k] | 0x20 == b'c') & u8::from(w[k + 1] | 0x20 == b'h');
        }
        if hit != 0 && (i..i + 32).any(at) {
            return true;
        }
        i += 32;
    }
    (i..b.len()).any(at)
}

/// ASCII case-insensitive search for lowercase `needle` in `b[from..]`.
fn find_ci(b: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    let first = needle[0];
    let mut i = from;
    while i + needle.len() <= b.len() {
        let k = b[i..].iter().position(|&c| c | 0x20 == first)? + i;
        if b.get(k..k + needle.len())
            .is_some_and(|w| w.eq_ignore_ascii_case(needle))
        {
            return Some(k);
        }
        i = k + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use XmlCharEncoding::{Iso8859_1, Iso8859_2, Windows1252};

    fn d(b: &[u8]) -> Decision {
        decide(b, std::str::from_utf8(b).ok())
    }

    #[test]
    fn ascii_and_valid_utf8_stay_utf8() {
        assert_eq!(d(b"<p>plain"), Decision::Utf8);
        assert_eq!(d("<p>café".as_bytes()), Decision::Utf8);
    }

    #[test]
    fn meta_charset_decides_in_html_mode() {
        let b = b"<meta charset=\"iso-8859-1\"><p>caf\xE9";
        assert_eq!(
            d(b),
            Decision::Switch {
                at: 26,
                enc: Windows1252
            }
        );
        assert!(matches!(
            d(b"<meta charset=latin2><p>\xB1"),
            Decision::Switch { enc: Iso8859_2, .. }
        ));
        assert!(matches!(
            d(b"<meta charset=us-ascii><p>\x93"),
            Decision::Switch {
                enc: Windows1252,
                ..
            }
        ));
    }

    #[test]
    fn content_type_meta_decides() {
        let b = b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=iso-8859-1\"><p>caf\xE9";
        assert!(matches!(
            d(b),
            Decision::Switch {
                enc: Windows1252,
                ..
            }
        ));
        let b = b"<meta http-equiv=content-type content=\"text/html;charset = 'latin2'\"><p>\xB1";
        assert!(matches!(d(b), Decision::Switch { enc: Iso8859_2, .. }));
    }

    #[test]
    fn unusable_meta_is_skipped() {
        assert!(matches!(
            d(b"<meta charset=bogus><meta charset=latin2><p>\xB1"),
            Decision::Switch { enc: Iso8859_2, .. }
        ));
        // UTF-16 is not ASCII-compatible: ignored, then the fallback.
        assert!(matches!(
            d(b"<meta charset=utf-16><p>caf\xE9"),
            Decision::Switch {
                enc: Windows1252,
                ..
            }
        ));
    }

    #[test]
    fn meta_text_outside_a_tag_does_not_count() {
        for b in [
            &b"<!--<meta charset=\"latin2\">--><p>\xB1"[..],
            b"<script><meta charset=\"latin2\"></script><p>\xB1",
            b"<title><meta charset=\"latin2\"></title><p>\xB1",
            b"<p title=\"<meta charset=latin2>\">\xB1",
        ] {
            assert!(
                matches!(
                    d(b),
                    Decision::Switch {
                        enc: Windows1252,
                        ..
                    }
                ),
                "{:?}",
                String::from_utf8_lossy(b)
            );
        }
    }

    #[test]
    fn non_ascii_before_meta_decides_first() {
        for b in [
            &b"<p>\xE9</p><meta charset=\"latin2\"><p>\xB1"[..],
            b"<p title=\"\xB1\"></p><meta charset=\"latin2\">",
            b"<!DOCTYPE html SYSTEM \"\xB1\"><meta charset=\"latin2\">",
            b"<!-- \xB1 --><meta charset=\"latin2\">",
        ] {
            assert!(
                matches!(
                    d(b),
                    Decision::Switch {
                        enc: Windows1252,
                        ..
                    }
                ),
                "{:?}",
                String::from_utf8_lossy(b)
            );
        }
    }

    #[test]
    fn non_ascii_name_decides_nothing() {
        let b = b"<x\xC3\xA9><meta charset=\"latin2\"><p>\xB1";
        // The switch lands on the meta's `>`.
        assert_eq!(
            d(b),
            Decision::Switch {
                at: 27,
                enc: Iso8859_2
            }
        );
    }

    #[test]
    fn invalid_utf8_in_a_name_is_skipped_without_panicking() {
        // The fuzzer's case: the window must restart past the name, whose bytes
        // are not UTF-8, rather than reach back over them.
        assert!(matches!(
            d(b"<x\xFF\xFE a=1><p>\xE9"),
            Decision::Switch {
                enc: Windows1252,
                ..
            }
        ));
        assert!(matches!(
            d(b"<x\xFF><y\xC3><meta charset=latin2><p>\xB1"),
            Decision::Switch { enc: Iso8859_2, .. }
        ));
        assert!(matches!(
            d(b"<p \xFF=1 title='\xB1'>"),
            Decision::Switch {
                enc: Windows1252,
                ..
            }
        ));
    }

    #[test]
    fn text_search_reads_without_the_html_mapping() {
        let b = b"<p>\x93</p><meta http-equiv=\"Content-Type\" content=\"text/html; charset=iso-8859-1\">";
        assert_eq!(
            d(b),
            Decision::Switch {
                at: 3,
                enc: Iso8859_1
            }
        );
        // A label libxml2 cannot use leaves UTF-8.
        let b = b"<p>caf\xE9</p><meta http-equiv=\"Content-Type\" content=\"text/html; charset=bogus\">";
        assert_eq!(d(b), Decision::Utf8);
    }

    #[test]
    fn charset_word_is_found_in_any_case_and_position() {
        assert!(contains_charset(b"<meta charset=x>"));
        assert!(contains_charset(b"CHARSET"));
        assert!(contains_charset(
            b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxCharSet"
        ));
        assert!(contains_charset(
            b"xxxxxxxxxxxxxxxc
hxxxxxxxxxxxxxxcharsetxxxxx"
        ));
        // Straddling a 16-byte block boundary.
        assert!(contains_charset(b"xxxxxxxxxxxxxxxcharset"));
        assert!(!contains_charset(b"chars set charse char-set"));
        assert!(!contains_charset(b""));
    }

    #[test]
    fn content_type_parameter() {
        assert_eq!(
            parse_content_type("text/html; charset=latin2"),
            Some("latin2")
        );
        assert_eq!(
            parse_content_type("text/html; Charset = \" utf-8 \""),
            Some("utf-8")
        );
        assert_eq!(parse_content_type("text/html"), None);
        assert_eq!(parse_content_type("text/html; charset=\"open"), None);
    }
}
