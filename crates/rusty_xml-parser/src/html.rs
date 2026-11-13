//! HTML parser: libxml2 2.15 `HTMLparser.c` with default options (not
//! `HTML_PARSE_HTML5`), tokenizer and tree construction both.
//!
//! The tree is built the way libxml2 builds it: a stack of open elements on
//! which `html`, `head` and `body` are ordinary entries, `htmlCheckImplied`
//! inserting them, `htmlAutoClose` popping whatever the `htmlStartClose` table
//! says a new start tag ends, and `htmlAutoCloseOnClose` letting an end tag
//! close only elements of lower `htmlEndPriority`. Raw-text elements (`script`,
//! `style`, `title`, `textarea`, ...) take their content in libxml2's data
//! modes, and character references follow its legacy-name and attribute rules.
//!
//! Element names are interned to a small [`Tag`](crate::html_tags::Tag) when
//! read, so the per-tag table questions are a load and a bit test rather than
//! libxml2's `bsearch` + `strcmp`.
//!
//! Every formulation here was chosen on deterministic instruction counts
//! (callgrind Ir, parse only, two corpora: 25 stress fragments x20 and 197
//! real pages from three HTML generators). The numbers are in
//! `docs/html-ir-campaign.md`; the losing arms are recorded beside each choice.

use std::borrow::Cow;
use std::collections::HashSet;

use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};

use crate::error::XmlError;
use crate::html_entities::{decode_html, Nul};
use crate::html_tags::{self as tags, Tag, BODY, HEAD, HTML, P, UNKNOWN};

/// libxml2 `htmlParserOption` bits we honour.
pub const HTML_PARSE_NOIMPLIED: i32 = 1 << 13;
pub const HTML_PARSE_NONET: i32 = 1 << 11;

/// `htmlInsertMode`.
const INSERT_INITIAL: u8 = 1;
const INSERT_IN_HEAD: u8 = 3;
const INSERT_IN_BODY: u8 = 10;

/// libxml2's two script-escape states, past `DATA_SCRIPT`.
const DATA_SCRIPT_ESC1: u8 = 5;
const DATA_SCRIPT_ESC2: u8 = 6;

/// libxml2 truncates names to `HTML_PARSER_BUFFER_SIZE` bytes.
const MAX_NAME: usize = 100;
/// A start tag with more attributes than this deduplicates them through a
/// hash set. The pairwise scan is cheaper on ordinary tags, but quadratic.
const ATTR_SCAN_MAX: usize = 32;

/// A name holding an uppercase letter or a NUL: ASCII lowercased, NUL to
/// U+FFFD. Out of line on purpose. Inlined, it made every call of
/// `Parser::name` dearer, including the ones that borrow the name as it
/// stands, and whether the compiler inlined it changed with unrelated edits.
#[inline(never)]
fn fold_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\0' {
                '\u{fffd}'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

/// `IS_WS_HTML`.
#[inline]
fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0c | b'\r')
}

/// `htmlIsBooleanAttr`: written without a value, these take their own name.
/// Always inlined: with two callers the compiler left it out of line, which
/// cost the ordinary tag 0.05% parse Ir on real pages.
#[inline(always)]
fn is_boolean_attr(name: &str) -> bool {
    matches!(
        name,
        "checked"
            | "compact"
            | "declare"
            | "defer"
            | "disabled"
            | "ismap"
            | "multiple"
            | "nohref"
            | "noresize"
            | "noshade"
            | "nowrap"
            | "readonly"
            | "selected"
    )
}

/// `htmlReadMemory`.
#[doc(alias = "htmlReadMemory")]
pub fn html_read_memory(
    buffer: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError> {
    let _ = url;
    // Borrowed and validated once. Copying the input and re-validating it
    // lossily cost +6.5% / +21.9% parse Ir (fragments / pages).
    let (bytes, _) = crate::encoding::xml_convert_to_utf8_cow(buffer, encoding)?;
    let text = utf8_cow(bytes);
    let (has_cr, has_nul) = scan_cr_nul(text.as_bytes());
    let text = if has_cr {
        normalize_newlines(text)
    } else {
        text
    };
    Ok(html_parse_str(&text, options, has_nul))
}

/// Does the input contain a CR, a NUL? Fixed 64-byte blocks with no exit
/// inside, so the compare-and-or loop vectorizes. It replaced a memchr gate for
/// CR plus a NUL memchr in every text run and attribute value: those cost
/// +2.0% / +6.0% parse Ir, because memchr pays per byte value and per call.
/// Without a NUL in the input, no run needs a NUL policy at all.
fn scan_cr_nul(b: &[u8]) -> (bool, bool) {
    let (mut cr, mut nul) = (0u8, 0u8);
    let mut blocks = b.chunks_exact(64);
    for blk in &mut blocks {
        let (mut c, mut z) = (0u8, 0u8);
        for &x in blk {
            c |= u8::from(x == b'\r');
            z |= u8::from(x == 0);
        }
        cr |= c;
        nul |= z;
        if cr & nul != 0 {
            break;
        }
    }
    for &x in blocks.remainder() {
        cr |= u8::from(x == b'\r');
        nul |= u8::from(x == 0);
    }
    (cr != 0, nul != 0)
}

/// `htmlReadDoc`.
#[doc(alias = "htmlReadDoc")]
pub fn html_read_doc(
    cur: &str,
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError> {
    html_read_memory(cur.as_bytes(), url, encoding, options)
}

/// `htmlReadFile`.
#[doc(alias = "htmlReadFile")]
pub fn html_read_file(
    filename: &str,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError> {
    let b = std::fs::read(filename).map_err(|e| XmlError::new(4, e.to_string(), 0, 0))?;
    html_read_memory(&b, Some(filename), encoding, options)
}

/// Validate once, borrowing when the bytes are already UTF-8. The fast
/// validator only falls back to the lossy one when there is something to fix.
fn utf8_cow(bytes: Cow<'_, [u8]>) -> Cow<'_, str> {
    match bytes {
        Cow::Borrowed(b) => match std::str::from_utf8(b) {
            Ok(s) => Cow::Borrowed(s),
            Err(_) => Cow::Owned(String::from_utf8_lossy(b).into_owned()),
        },
        Cow::Owned(v) => match String::from_utf8(v) {
            Ok(s) => Cow::Owned(s),
            Err(e) => Cow::Owned(String::from_utf8_lossy(e.as_bytes()).into_owned()),
        },
    }
}

/// CRLF and lone CR to LF. libxml2 does this in every tokenizer state that
/// keeps text (character data, attribute values, comments, raw text); inside a
/// name a CR is whitespace either way. One pass up front is therefore the same
/// result. Called only when the input has a CR.
fn normalize_newlines(s: Cow<'_, str>) -> Cow<'_, str> {
    let b = s.as_bytes();
    let mut out = String::with_capacity(b.len());
    let mut last = 0;
    let mut i = 0;
    while let Some(k) = s[i..].find('\r') {
        let at = i + k;
        out.push_str(&s[last..at]);
        out.push('\n');
        i = if b.get(at + 1) == Some(&b'\n') {
            at + 2
        } else {
            at + 1
        };
        last = i;
    }
    out.push_str(&s[last..]);
    Cow::Owned(out)
}

fn html_parse_str(text: &str, options: i32, has_nul: bool) -> XmlDoc {
    let mut doc = XmlDoc::xml_new_doc(Some("1.0"));
    doc.encoding = Some("HTML".into());
    // Mark it as an HTML document so the serializer can tell: C emits the
    // doctype where XML would emit a declaration.
    doc.node_mut(NodeId::DOCUMENT).kind = NodeKind::HtmlDocument;
    let mut p = Parser {
        src: text,
        b: text.as_bytes(),
        pos: 0,
        doc,
        stack: Vec::new(),
        insert: INSERT_INITIAL,
        depth: 0,
        mode: 0,
        noimplied: (options & HTML_PARSE_NOIMPLIED) != 0,
        attrs: Vec::new(),
        has_nul,
    };
    p.document();
    p.doc
}

#[derive(Clone, Copy)]
struct Open {
    node: NodeId,
    tag: Tag,
}

struct Parser<'a> {
    src: &'a str,
    b: &'a [u8],
    pos: usize,
    doc: XmlDoc,
    /// libxml2's name stack and SAX2's node stack, which move together.
    stack: Vec<Open>,
    /// `ctxt->html`.
    insert: u8,
    /// Misplaced html/head/body start tags whose end tags are to be ignored.
    depth: u32,
    /// `ctxt->endCheckState`: the data mode the next character data is in.
    mode: u8,
    noimplied: bool,
    /// Reused across start tags.
    attrs: Vec<(Cow<'a, str>, Option<Cow<'a, str>>)>,
    /// The input has a NUL somewhere; without one, no run needs scanning.
    has_nul: bool,
}

impl<'a> Parser<'a> {
    /// The NUL policy for a run: none to apply when the input has no NUL.
    #[inline]
    fn nul(&self, want: Nul) -> Nul {
        if self.has_nul {
            want
        } else {
            Nul::Keep
        }
    }

    #[inline]
    fn peek(&self, k: usize) -> Option<u8> {
        self.b.get(self.pos + k).copied()
    }

    #[inline]
    fn skip_blanks(&mut self) {
        while self.pos < self.b.len() && is_ws(self.b[self.pos]) {
            self.pos += 1;
        }
    }

    fn is_doctype(&self) -> bool {
        self.b
            .get(self.pos + 2..self.pos + 9)
            .is_some_and(|k| k.eq_ignore_ascii_case(b"DOCTYPE"))
    }

    /// Find `c` (ASCII) at or after `from`; the slice start is always just past
    /// an ASCII byte, so it is a char boundary.
    #[inline]
    fn find_from(&self, from: usize, c: char) -> Option<usize> {
        self.src[from..].find(c).map(|k| from + k)
    }

    // ---- document and content (htmlParseDocument, htmlParseContent) ----

    fn document(&mut self) {
        self.skip_blanks();
        while self.peek(0) == Some(b'<') {
            match self.peek(1) {
                Some(b'!') => {
                    if self.b[self.pos..].starts_with(b"<!--") {
                        self.pos += 4;
                        self.comment(false);
                    } else if self.is_doctype() {
                        self.doctype(true);
                    } else {
                        self.pos += 2;
                        self.comment(true);
                    }
                }
                Some(b'?') => {
                    self.pos += 1;
                    self.comment(true);
                }
                _ => break,
            }
            self.skip_blanks();
        }
        self.content();
    }

    fn content(&mut self) {
        while self.pos < self.b.len() {
            if self.mode != 0 || self.b[self.pos] != b'<' {
                self.char_data();
                continue;
            }
            match self.peek(1) {
                Some(b'/') => self.end_tag(),
                Some(b'!') => {
                    if self.is_doctype() {
                        self.doctype(false);
                    } else if self.b[self.pos..].starts_with(b"<!--") {
                        self.pos += 4;
                        self.comment(false);
                    } else {
                        self.pos += 2;
                        self.comment(true);
                    }
                }
                Some(b'?') => {
                    self.pos += 1;
                    self.comment(true);
                }
                Some(c) if c.is_ascii_alphabetic() => self.element(),
                _ => {
                    self.pos += 1;
                    self.start_char_data();
                    self.append(Cow::Borrowed("<"), NodeKind::Text);
                }
            }
        }
    }

    // ---- the stack (htmlnamePush, htmlAutoClose, htmlCheckImplied) ----

    fn top(&self) -> Option<Open> {
        self.stack.last().copied()
    }

    fn top_tag(&self) -> Option<Tag> {
        self.stack.last().map(|o| o.tag)
    }

    /// Create an element under the current node (the document if none) and
    /// make it current.
    fn open(&mut self, name: &str, tag: Tag) -> NodeId {
        let el = self.doc.xml_new_node(None, name);
        let parent = self.top().map_or(NodeId::DOCUMENT, |o| o.node);
        self.doc.xml_add_child(parent, el);
        if tag == HEAD && self.insert < INSERT_IN_HEAD {
            self.insert = INSERT_IN_HEAD;
        }
        if tag == BODY && self.insert < INSERT_IN_BODY {
            self.insert = INSERT_IN_BODY;
        }
        self.stack.push(Open { node: el, tag });
        el
    }

    fn open_implied(&mut self, tag: Tag) {
        self.open(tags::tag_name(tag), tag);
    }

    /// `htmlAutoClose`. One bit test per open element popped. libxml2's own
    /// shape -- `bsearch` over (old, new) name pairs, a linear `strcmp` scan
    /// for end priority, `bsearch` for the element row -- measured +23.1% /
    /// +15.1% parse Ir against these tables.
    fn auto_close(&mut self, new: Tag) {
        while let Some(o) = self.top() {
            if !tags::start_closes(new, o.tag) {
                break;
            }
            self.stack.pop();
        }
    }

    /// `htmlCheckImplied`.
    fn check_implied(&mut self, new: Tag) {
        if self.noimplied || new == HTML {
            return;
        }
        if self.stack.is_empty() {
            self.open_implied(HTML);
        }
        if new == BODY || new == HEAD {
            return;
        }
        if self.stack.len() <= 1 && tags::is_headish(new) {
            if self.insert >= INSERT_IN_HEAD {
                return;
            }
            self.open_implied(HEAD);
        } else if !tags::is_frameish(new) {
            if self.insert >= INSERT_IN_BODY {
                return;
            }
            if self.stack.iter().any(|o| o.tag == BODY || o.tag == HEAD) {
                return;
            }
            self.open_implied(BODY);
        }
    }

    /// `htmlStartCharData`: what non-blank text implies.
    fn start_char_data(&mut self) {
        if self.noimplied {
            return;
        }
        if self.top_tag() == Some(HEAD) {
            self.auto_close(P);
        }
        self.check_implied(P);
    }

    // ---- text ----

    /// `xmlSAX2Characters` / `xmlSAX2CDataBlock`: append to the current node,
    /// merging with a preceding node of the same kind. Dropped with no node.
    fn append(&mut self, s: Cow<'_, str>, kind: NodeKind) {
        if s.is_empty() {
            return;
        }
        let Some(top) = self.top() else { return };
        if let Some(last) = self.doc.last_child(top.node) {
            if self.doc.kind(last) == kind {
                self.doc.node_mut(last).content.push_str(&s);
                return;
            }
        }
        let n = self.doc.alloc(kind, String::new());
        self.doc.node_mut(n).content = s.into_owned();
        self.doc.xml_add_child(top.node, n);
    }

    /// `htmlCharDataSAXCallback` for ordinary text and RCDATA: leading blanks
    /// directly in html/head stay there; anything else implies a body.
    fn text(&mut self, s: Cow<'_, str>) {
        let top = self.top_tag();
        if top.is_none() || top == Some(HTML) || top == Some(HEAD) {
            let ws = s.bytes().take_while(|&c| is_ws(c)).count();
            if ws == s.len() {
                self.append(s, NodeKind::Text);
                return;
            }
            let rest = if ws > 0 {
                let (blank, rest) = s.split_at(ws);
                let rest = rest.to_owned();
                self.append(Cow::Owned(blank.to_owned()), NodeKind::Text);
                Cow::Owned(rest)
            } else {
                s
            };
            self.start_char_data();
            self.append(rest, NodeKind::Text);
            return;
        }
        self.append(s, NodeKind::Text);
    }

    /// `htmlParseCharData`.
    fn char_data(&mut self) {
        let start = self.pos;
        match self.mode {
            0 => {
                let end = self.find_from(start, '<').unwrap_or(self.b.len());
                self.pos = end;
                let raw = &self.src[start..end];
                let t = decode_html(raw, false, self.nul(Nul::Strip));
                self.text(t);
            }
            tags::DATA_PLAINTEXT => {
                self.pos = self.b.len();
                let t = decode_none(&self.src[start..], self.has_nul);
                self.append(t, NodeKind::CData);
            }
            mode => {
                let name = self.top_tag().map_or("", tags::tag_name).as_bytes();
                let (end, complete) = scan_raw(self.src, self.b, start, name, mode);
                self.pos = end;
                if complete {
                    self.mode = 0;
                }
                let raw = &self.src[start..end];
                if mode == tags::DATA_RCDATA {
                    self.text(decode_html(raw, false, self.nul(Nul::Replace)));
                } else {
                    self.append(decode_none(raw, self.has_nul), NodeKind::CData);
                }
            }
        }
    }

    // ---- markup ----

    /// `htmlParseHTMLName`: the first byte unconditionally, then up to `/`,
    /// `>`, blank (or `=` for an attribute). ASCII lowercased, NUL to U+FFFD.
    fn name(&mut self, attr: bool) -> Cow<'a, str> {
        let b = self.b;
        let start = self.pos;
        if start >= b.len() {
            return Cow::Borrowed("");
        }
        let mut i = start + 1;
        let mut fold = b[start].is_ascii_uppercase() || b[start] == 0;
        while i < b.len() {
            let c = b[i];
            if c == b'/' || c == b'>' || is_ws(c) || (attr && c == b'=') {
                break;
            }
            fold |= c.is_ascii_uppercase() || c == 0;
            i += 1;
        }
        // The first byte may begin a multi-byte character; step to its end so
        // the slice stays on a boundary.
        while i < b.len() && !self.src.is_char_boundary(i) {
            i += 1;
        }
        self.pos = i;
        let mut end = i;
        if end - start > MAX_NAME {
            end = start + MAX_NAME;
            while !self.src.is_char_boundary(end) {
                end -= 1;
            }
        }
        let s = &self.src[start..end];
        if !fold {
            return Cow::Borrowed(s);
        }
        Cow::Owned(fold_name(s))
    }

    /// `htmlParseAttValue`'s extent: quoted to the matching quote, else to a
    /// blank or `>`.
    fn att_value_raw(&mut self) -> &'a str {
        let start = self.pos;
        match self.peek(0) {
            Some(q @ (b'"' | b'\'')) => {
                let from = start + 1;
                let end = self.find_from(from, q as char);
                let stop = end.unwrap_or(self.b.len());
                self.pos = end.map_or(stop, |e| e + 1);
                &self.src[from..stop]
            }
            _ => {
                let b = self.b;
                let mut i = start;
                while i < b.len() && !matches!(b[i], b' ' | b'\t' | b'\n' | 0x0c | b'\r' | b'>') {
                    i += 1;
                }
                self.pos = i;
                &self.src[start..i]
            }
        }
    }

    /// The attribute list of a start or end tag, up to `>`, `/>` or EOF.
    /// End tags parse and discard theirs.
    fn attributes(&mut self, keep: bool) {
        self.skip_blanks();
        while self.pos < self.b.len() {
            match self.b[self.pos] {
                b'>' => break,
                b'/' => {
                    if self.peek(1) == Some(b'>') {
                        break;
                    }
                    self.pos += 1;
                    self.skip_blanks();
                    continue;
                }
                _ => {}
            }
            let name = self.name(true);
            self.skip_blanks();
            let nul = self.nul(Nul::Replace);
            let value = if self.peek(0) == Some(b'=') {
                self.pos += 1;
                self.skip_blanks();
                let raw = self.att_value_raw();
                Some(raw)
            } else {
                None
            };
            if keep {
                let value = value.map(|raw| decode_html(raw, true, nul));
                self.attrs.push((name, value));
            }
            self.skip_blanks();
        }
    }

    /// `htmlParseElementInternal` + `htmlParseStartTag`.
    fn element(&mut self) {
        self.mode = 0;
        self.pos += 1;
        let name = self.name(false);
        let tag = tags::tag_of(name.as_bytes());
        self.auto_close(tag);
        self.check_implied(tag);
        let mut discard = false;
        if tag == HTML && !self.stack.is_empty() {
            discard = true;
        }
        if tag == HEAD && self.stack.len() != 1 {
            discard = true;
        }
        if tag == BODY && self.stack.iter().any(|o| o.tag == BODY) {
            discard = true;
        }
        if discard {
            self.depth += 1;
        }
        self.attrs.clear();
        self.attributes(true);
        if self.pos >= self.b.len() {
            // A tag cut off by the end of input is dropped.
            self.attrs.clear();
            return;
        }
        if !discard {
            let el = self.open(&name, tag);
            self.add_attributes(el);
        }
        let Some(top) = self.top() else { return };
        if self.b[self.pos..].starts_with(b"/>") {
            self.pos += 2;
            self.stack.pop();
            return;
        }
        if self.b[self.pos] != b'>' {
            return;
        }
        self.pos += 1;
        if tags::is_empty(top.tag) {
            self.stack.pop();
        } else {
            self.mode = tags::data_mode(top.tag);
        }
    }

    /// First occurrence of a name wins. A valueless attribute has no value
    /// (boolean ones take their own name). Deduplicating against the parsed
    /// list, then appending without a scan, beat probing the element's
    /// attributes in the arena by 0.3% / 0.1% parse Ir, and the old
    /// `xml_set_prop` (last wins, value = name) cost 1.0% / 0.9% more than both.
    /// Past `ATTR_SCAN_MAX` attributes `add_attributes_wide` takes over. The
    /// test comes before the buffer is taken, so the ordinary tag pays one
    /// comparison for it and nothing else.
    fn add_attributes(&mut self, el: NodeId) {
        if self.attrs.len() > ATTR_SCAN_MAX {
            return self.add_attributes_wide(el);
        }
        let attrs = std::mem::take(&mut self.attrs);
        for (i, (n, v)) in attrs.iter().enumerate() {
            if attrs[..i].iter().any(|(m, _)| m == n) {
                continue;
            }
            self.add_attribute(el, n, v);
        }
        // Hand the buffer back so its capacity is reused.
        self.attrs = attrs;
        self.attrs.clear();
    }

    /// `add_attributes` for a tag whose pairwise scan would be quadratic: a
    /// set finds the repeats, as `htmlAttrHashInsert` does. Names are
    /// attacker-chosen, so the set keeps the standard library's keyed hasher.
    #[cold]
    #[inline(never)]
    fn add_attributes_wide(&mut self, el: NodeId) {
        let attrs = std::mem::take(&mut self.attrs);
        let mut seen: HashSet<&str> = HashSet::with_capacity(attrs.len());
        for (n, v) in &attrs {
            if seen.insert(n) {
                self.add_attribute(el, n, v);
            }
        }
        drop(seen);
        self.attrs = attrs;
        self.attrs.clear();
    }

    #[inline(always)]
    fn add_attribute(&mut self, el: NodeId, n: &Cow<'a, str>, v: &Option<Cow<'a, str>>) {
        match v {
            Some(v) => {
                self.doc.add_attr(el, n, None, v);
            }
            None if is_boolean_attr(n) => {
                self.doc.add_attr(el, n, None, n);
            }
            None => {
                let id = self.doc.add_attr(el, n, None, "");
                self.doc.html_bare_attrs.push(id);
            }
        }
    }

    /// `htmlParseEndTag` + `htmlAutoCloseOnClose`.
    fn end_tag(&mut self) {
        self.mode = 0;
        self.pos += 2;
        let Some(c) = self.peek(0) else {
            self.start_char_data();
            self.append(Cow::Borrowed("</"), NodeKind::Text);
            return;
        };
        if c == b'>' {
            self.pos += 1;
            return;
        }
        if !c.is_ascii_alphabetic() {
            self.comment(true);
            return;
        }
        let name = self.name(false);
        self.attributes(false);
        match self.peek(0) {
            Some(b'>') => self.pos += 1,
            Some(b'/') if self.peek(1) == Some(b'>') => self.pos += 2,
            _ => return,
        }
        let tag = tags::tag_of(name.as_bytes());
        if self.depth > 0 && (tag == HTML || tag == BODY || tag == HEAD) {
            self.depth -= 1;
            return;
        }
        // Known elements match by tag; only unknown ones compare names.
        let same = |p: &Self, o: Open| {
            if tag == UNKNOWN {
                o.tag == tag && p.doc.name(o.node) == name.as_ref()
            } else {
                o.tag == tag
            }
        };
        let Some(at) = (0..self.stack.len())
            .rev()
            .find(|&i| same(self, self.stack[i]))
        else {
            return;
        };
        // A misplaced end tag only closes elements of lower or equal priority.
        // An index loop: `self.stack[at + 1..].iter().any(..)` measured +0.6%
        // parse Ir on pages, slice-iterator setup on every end tag.
        let pri = tags::end_priority(tag);
        for i in at + 1..self.stack.len() {
            if tags::end_priority(self.stack[i].tag) > pri {
                return;
            }
        }
        self.stack.truncate(at);
    }

    /// `htmlParseComment`. A real comment ends at `-->` or `--!>`; a bogus one
    /// (`<!x>`, `<?x>`, `</ x>`) at the next `>`. Either runs to the end of
    /// input if unterminated, where a real one drops the start of a closer the
    /// input cut off (`-`, `--`, `--!`): libxml2's `htmlParseData` consumes
    /// those as a short terminator rather than keeping them as text.
    fn comment(&mut self, bogus: bool) {
        let start = self.pos;
        let len = self.b.len();
        let body: &'a str = if bogus {
            let end = self.find_from(start, '>');
            self.pos = end.map_or(len, |e| e + 1);
            &self.src[start..end.unwrap_or(len)]
        } else if self.peek(0) == Some(b'>') {
            self.pos += 1;
            ""
        } else if self.b[start..].starts_with(b"->") {
            self.pos += 2;
            ""
        } else {
            let mut from = start;
            loop {
                match self.src[from..].find("--").map(|k| from + k) {
                    None => {
                        self.pos = len;
                        let rest = &self.src[start..];
                        break rest
                            .strip_suffix("--!")
                            .or_else(|| rest.strip_suffix("--"))
                            .or_else(|| rest.strip_suffix('-'))
                            .unwrap_or(rest);
                    }
                    Some(d) => match self.b.get(d + 2) {
                        Some(b'>') => {
                            self.pos = d + 3;
                            break &self.src[start..d];
                        }
                        Some(b'!') if self.b.get(d + 3) == Some(&b'>') => {
                            self.pos = d + 4;
                            break &self.src[start..d];
                        }
                        _ => from = d + 1,
                    },
                }
            }
        };
        let n = self.doc.alloc(NodeKind::Comment, String::new());
        self.doc.node_mut(n).content = decode_none(body, self.has_nul).into_owned();
        let parent = self.top().map_or(NodeId::DOCUMENT, |o| o.node);
        self.doc.xml_add_child(parent, n);
    }

    /// `htmlParseDocTypeDecl`. `xmlSAX2InternalSubset` takes an HTML doctype
    /// only in the prolog, where a later one replaces an earlier one; past the
    /// first element it is parsed and dropped.
    fn doctype(&mut self, prolog: bool) {
        let start = self.pos + 9;
        let end = self.find_from(start, '>');
        let body = &self.src[start..end.unwrap_or(self.b.len())];
        self.pos = end.map_or(self.b.len(), |e| e + 1);
        if prolog {
            self.doc.dtd = Some(parse_html_doctype(body));
        }
    }
}

/// Raw text keeps references as written; only NUL becomes U+FFFD.
fn decode_none(s: &str, has_nul: bool) -> Cow<'_, str> {
    if has_nul && s.as_bytes().contains(&0) {
        Cow::Owned(s.replace('\0', "\u{fffd}"))
    } else {
        Cow::Borrowed(s)
    }
}

/// The extent of RCDATA / RAWTEXT / script data starting at `from`: up to the
/// `<` of `</name` followed by a blank, `/` or `>` (case-insensitive), or the
/// end of input. Script data also tracks libxml2's `<!--` escape states, in
/// which `<script` and `</script` nest. Returns the end and whether the end
/// tag was found.
fn scan_raw(src: &str, b: &[u8], from: usize, name: &[u8], mode: u8) -> (usize, bool) {
    let mut m = mode;
    let mut i = from;
    loop {
        // Next candidate: '<' always; '-' too in the escape states. memchr
        // between '<'s beat a byte loop by 3.0% parse Ir on real pages (long
        // scripts) and lost 0.2% on short synthetic ones.
        let found = if m == DATA_SCRIPT_ESC1 || m == DATA_SCRIPT_ESC2 {
            b[i..]
                .iter()
                .position(|&c| c == b'<' || c == b'-')
                .map(|k| i + k)
        } else {
            src[i..].find('<').map(|k| i + k)
        };
        let Some(at) = found else {
            return (b.len(), false);
        };
        i = at;
        if b[i] == b'-' {
            if b.get(i + 1) == Some(&b'-') && b.get(i + 2) == Some(&b'>') {
                m = tags::DATA_SCRIPT;
            }
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if j < b.len() {
            if m == tags::DATA_SCRIPT && b[j] == b'!' {
                if b.get(j + 1) == Some(&b'-') && b.get(j + 2) == Some(&b'-') {
                    m = DATA_SCRIPT_ESC1;
                }
            } else {
                let solidus = b[j] == b'/';
                if solidus {
                    j += 1;
                }
                if solidus || m == DATA_SCRIPT_ESC1 {
                    let mut k = 0;
                    while j < b.len() && k < name.len() && name[k] == (b[j] | 0x20) {
                        k += 1;
                        j += 1;
                    }
                    if k == name.len() && j < b.len() {
                        let c = b[j];
                        if c == b'>' || c == b'/' || is_ws(c) {
                            if m == DATA_SCRIPT_ESC1 && !solidus {
                                m = DATA_SCRIPT_ESC2;
                            } else if m == DATA_SCRIPT_ESC2 {
                                m = DATA_SCRIPT_ESC1;
                            } else {
                                return (i, true);
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
}

/// Split the body of an HTML `<!DOCTYPE ...>` into name, public id, system id.
///
/// `<!DOCTYPE html>` and the HTML 4.01 form with PUBLIC/SYSTEM identifiers are
/// the two that occur; anything else degrades to a bare name, which is what C
/// does too.
fn parse_html_doctype(body: &str) -> rusty_xml_tree::XmlDtd {
    let mut dtd = rusty_xml_tree::XmlDtd::default();
    let mut rest = body.trim_start();
    let name_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    if name_end > 0 {
        dtd.name = Some(rest[..name_end].to_string());
    }
    rest = rest[name_end..].trim_start();

    // A quoted literal, either quoting style, as HTML permits both.
    fn literal(r: &mut &str) -> Option<String> {
        *r = r.trim_start();
        let q = r.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let after = &r[1..];
        let end = after.find(q)?;
        let v = after[..end].to_string();
        *r = &after[end + 1..];
        Some(v)
    }

    // .get(..6) rather than [..6]: a byte index that lands mid-character
    // panics, and a doctype is attacker-controlled text like anything else.
    if rest
        .get(..6)
        .is_some_and(|k| k.eq_ignore_ascii_case("PUBLIC"))
    {
        rest = &rest[6..];
        dtd.public_id = literal(&mut rest);
        dtd.system_id = literal(&mut rest);
    } else if rest
        .get(..6)
        .is_some_and(|k| k.eq_ignore_ascii_case("SYSTEM"))
    {
        rest = &rest[6..];
        dtd.system_id = literal(&mut rest);
    }
    dtd
}
