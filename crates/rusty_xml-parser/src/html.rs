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

use rusty_xml_tree::{Interrupt, NodeId, NodeKind, XmlDoc};

use crate::error::XmlError;
use crate::html_entities::{decode_html, Nul};
use crate::html_tags::{self as tags, Tag, BODY, HEAD, HTML, P, UNKNOWN};

/// libxml2 `htmlParserOption` bits we honour.
pub const HTML_PARSE_NOIMPLIED: i32 = 1 << 13;
pub const HTML_PARSE_NONET: i32 = 1 << 11;

/// libxml2's HTML nesting cap (`nodePush`): an element that would sit
/// deeper is refused and the parse stops there (#48).
pub const HTML_MAX_DEPTH_DEFAULT: u32 = 256;
/// The cap under `XML_PARSE_HUGE` (libxml2's `HTML_PARSE_HUGE`, the same bit).
pub const HTML_MAX_DEPTH_HUGE: u32 = 2048;

/// Settings for an HTML parse beyond the option bits.
///
/// Pass one to [`html_read_memory_with`]. Build it from [`Default`] (or
/// [`HtmlParseSettings::new`]) and the `with_*` methods; the struct is
/// `#[non_exhaustive]` so later settings can be added without a break.
#[derive(Clone, Copy, Default)]
#[non_exhaustive]
pub struct HtmlParseSettings<'a> {
    /// The nesting cap, counting `html` and `body`.
    ///
    /// - `0` (the default): libxml2's, [`HTML_MAX_DEPTH_DEFAULT`] (256), or
    ///   [`HTML_MAX_DEPTH_HUGE`] (2048) when the options carry
    ///   `XML_PARSE_HUGE`.
    /// - A positive value: that cap.
    /// - A negative value (`-1`): no cap.
    ///
    /// An element that would sit deeper than the cap is dropped and the parse
    /// stops, keeping the tree built so far, as libxml2 does. The error
    /// (`XML_ERR_RESOURCE_LIMIT`, 114, libxml2's "Excessive depth in
    /// document" message) is recorded in [`XmlDoc::warnings`].
    pub max_depth: i32,
    /// Cooperative cancellation, as [`html_read_memory_interruptible`].
    pub interrupt: Option<&'a dyn Interrupt>,
}

impl std::fmt::Debug for HtmlParseSettings<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HtmlParseSettings")
            .field("max_depth", &self.max_depth)
            .field("interrupt", &self.interrupt.is_some())
            .finish()
    }
}

impl<'a> HtmlParseSettings<'a> {
    /// The defaults: libxml2's nesting cap, no interrupt.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the nesting cap: `0` for libxml2's, a positive cap, or `-1` for
    /// none.
    pub fn with_max_depth(mut self, max_depth: i32) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Sets the interrupt the parse polls.
    pub fn with_interrupt(mut self, interrupt: &'a dyn Interrupt) -> Self {
        self.interrupt = Some(interrupt);
        self
    }

    /// The cap in stack entries for these options.
    fn depth_cap(&self, options: i32) -> usize {
        match self.max_depth {
            0 if (options & crate::parse::XML_PARSE_HUGE) != 0 => HTML_MAX_DEPTH_HUGE as usize,
            0 => HTML_MAX_DEPTH_DEFAULT as usize,
            d if d < 0 => usize::MAX,
            d => d as usize,
        }
    }
}

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
    html_read(buffer, url, encoding, options, &HtmlParseSettings::default())
}

/// [`html_read_memory`] with [`HtmlParseSettings`]: the nesting cap and an
/// optional interrupt.
///
/// ```
/// use rusty_xml_parser::{html_read_memory_with, HtmlParseSettings};
///
/// let deep = format!("{}x", "<div>".repeat(300));
/// // libxml2's cap: 254 divs under html and body, then the parse stops.
/// let doc = html_read_memory_with(deep.as_bytes(), None, None, 0, &HtmlParseSettings::new()).unwrap();
/// assert!(doc.warnings[0].contains("Excessive depth in document: 256"));
/// // No cap.
/// let all = HtmlParseSettings::new().with_max_depth(-1);
/// let doc = html_read_memory_with(deep.as_bytes(), None, None, 0, &all).unwrap();
/// assert!(doc.warnings.is_empty());
/// ```
pub fn html_read_memory_with(
    buffer: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
    settings: &HtmlParseSettings<'_>,
) -> Result<XmlDoc, XmlError> {
    html_read(buffer, url, encoding, options, settings)
}

/// [`html_read_memory`] that polls `interrupt` at token boundaries and fails
/// with `XML_ERR_USER_STOP` (111) once it fires. Not interrupted, it returns
/// exactly what [`html_read_memory`] returns.
pub fn html_read_memory_interruptible(
    buffer: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
    interrupt: &dyn Interrupt,
) -> Result<XmlDoc, XmlError> {
    html_read(buffer, url, encoding, options, &HtmlParseSettings::new().with_interrupt(interrupt))
}

fn html_read(
    buffer: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
    settings: &HtmlParseSettings<'_>,
) -> Result<XmlDoc, XmlError> {
    let _ = url;
    let interrupt = settings.interrupt;
    // Decoding is one linear pass and is not polled; check either side.
    if interrupt.is_some_and(|i| i.is_interrupted()) {
        return Err(interrupted());
    }
    // A BOM, the `encoding` argument or a leading `<?xm` decide the encoding
    // up front. Otherwise libxml2 decides as it parses: a `<meta>` charset, or
    // a guess at the first non-ASCII byte (`html_encoding`, #27).
    let undeclared = encoding.is_none()
        && crate::encoding::xml_detect_char_encoding(buffer) == crate::encoding::XmlCharEncoding::None
        && !buffer.starts_with(&[0x1f, 0x8b]);
    // Borrowed and validated once. Copying the input and re-validating it
    // lossily cost +6.5% / +21.9% parse Ir (fragments / pages).
    let text = if undeclared {
        crate::html_encoding::decode(buffer)
    } else {
        let (bytes, _) = crate::encoding::xml_convert_to_utf8_cow(buffer, encoding)?;
        utf8_cow(bytes)
    };
    let (has_cr, has_nul, lt) = scan_cr_nul(text.as_bytes());
    let text = if has_cr {
        normalize_newlines(text)
    } else {
        text
    };
    html_parse_str(&text, options, has_nul, interrupt, settings.depth_cap(options), lt)
}

fn interrupted() -> XmlError {
    XmlError::new(crate::error::XML_ERR_USER_STOP, "Parser interrupted", 0, 0)
}

/// Does the input contain a CR, a NUL, and how many `<`? Fixed 64-byte blocks
/// with no exit inside, so the compare-and-or loop vectorizes. It replaced a
/// memchr gate for CR plus a NUL memchr in every text run and attribute value:
/// those cost +2.0% / +6.0% parse Ir, because memchr pays per byte value and
/// per call. Without a NUL in the input, no run needs a NUL policy at all.
///
/// The `<` count sizes the node arena (#49). It rides the same loads for
/// 0.22 Ir per byte; a separate counting pass cost +13.4% pages parse Ir.
fn scan_cr_nul(b: &[u8]) -> (bool, bool, usize) {
    let (mut cr, mut nul) = (0u8, 0u8);
    let mut lt = 0usize;
    let mut blocks = b.chunks_exact(64);
    for blk in &mut blocks {
        let (mut c, mut z, mut l) = (0u8, 0u8, 0u8);
        for &x in blk {
            c |= u8::from(x == b'\r');
            z |= u8::from(x == 0);
            l += u8::from(x == b'<');
        }
        cr |= c;
        nul |= z;
        lt += usize::from(l);
    }
    for &x in blocks.remainder() {
        cr |= u8::from(x == b'\r');
        nul |= u8::from(x == 0);
        lt += usize::from(x == b'<');
    }
    (cr != 0, nul != 0, lt)
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

fn html_parse_str(
    text: &str,
    options: i32,
    has_nul: bool,
    interrupt: Option<&dyn Interrupt>,
    max_depth: usize,
    lt: usize,
) -> Result<XmlDoc, XmlError> {
    // Sized from the markup, two nodes per `<` (#49): an element, its
    // attributes and the text after it, against end tags that make none.
    // Real pages run 1.3-2.3 nodes per `<` while their bytes per node vary
    // 26x (scripts, inline SVG), so this reserves about 1.2x what a page
    // uses where one node per 10 bytes reserved 2-3x on documentation and
    // up to 25x on script-heavy pages, committed up front, and the shrink
    // below then copied the tree out of it: parse Ir -0.5% / -1.7%. Never
    // more than one node per 2 bytes, the densest markup gets (`<i>x<i>x`,
    // `<a b c>`), so input made of `<` reserves no more than a real tree of
    // its size could fill; capping at the old guess instead cost the dense
    // fragments +5.4%. The slack is released once the tree is built (#22).
    let mut doc = XmlDoc::with_node_capacity(Some("1.0"), (2 * lt).min(text.len() / 2));
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
        interrupt,
        limit: if interrupt.is_some() { POLL_BYTES.min(text.len()) } else { text.len() },
        stopped: false,
        deep: None,
        max_depth,
        deep_at: DEEP.min(max_depth.saturating_add(1)),
        halted: false,
    };
    p.document();
    if p.stopped {
        return Err(interrupted());
    }
    p.doc.shrink_nodes();
    Ok(p.doc)
}

#[derive(Clone, Copy)]
struct Open {
    node: NodeId,
    tag: Tag,
}

/// Input bytes parsed between two polls of an interruptible parse.
const POLL_BYTES: usize = 16 * 1024;

/// Stack depth at which the open-element stack gets an index.
///
/// libxml2 caps HTML nesting at 256 (2048 with `XML_PARSE_HUGE`), so its
/// linear scans of the stack stay cheap. This parser keeps that cap by
/// default (#48) but lets a caller raise or remove it, and with no cap the
/// same scans made an end tag that matches nothing, or a stray `<body>`, cost
/// the whole depth: 80,000 unmatched end tags under 80,000 open elements took
/// 3.8 s (#41). Below this depth the scans are faster than any index.
const DEEP: usize = 256;

/// End priorities in use (`tags::end_priority`), one slot each.
const PRI_SLOTS: [u8; 8] = [100, 150, 160, 170, 180, 190, 200, 220];

fn pri_slot(p: u8) -> usize {
    PRI_SLOTS.iter().position(|&x| x == p).unwrap_or(0)
}

/// Where each tag, unknown name and end priority sits on a deep stack: the
/// positions of its open elements, the top one last.
struct DeepIndex {
    by_tag: Vec<Vec<u32>>,
    by_name: std::collections::HashMap<String, Vec<u32>>,
    by_pri: [Vec<u32>; 8],
}

impl DeepIndex {
    fn build(stack: &[Open], doc: &XmlDoc) -> Self {
        let mut d = DeepIndex {
            by_tag: vec![Vec::new(); tags::NT],
            by_name: std::collections::HashMap::new(),
            by_pri: Default::default(),
        };
        for (i, o) in stack.iter().enumerate() {
            d.push(i, *o, doc);
        }
        d
    }

    fn push(&mut self, i: usize, o: Open, doc: &XmlDoc) {
        let i = i as u32;
        if o.tag == UNKNOWN {
            self.by_name.entry(doc.name(o.node).to_string()).or_default().push(i);
        } else {
            self.by_tag[o.tag as usize].push(i);
        }
        self.by_pri[pri_slot(tags::end_priority(o.tag))].push(i);
    }

    fn pop(&mut self, o: Open, doc: &XmlDoc) {
        if o.tag == UNKNOWN {
            if let Some(v) = self.by_name.get_mut(doc.name(o.node)) {
                v.pop();
            }
        } else {
            self.by_tag[o.tag as usize].pop();
        }
        self.by_pri[pri_slot(tags::end_priority(o.tag))].pop();
    }

    fn top_of(&self, tag: Tag, name: &str) -> Option<usize> {
        let v = if tag == UNKNOWN {
            self.by_name.get(name)?
        } else {
            &self.by_tag[tag as usize]
        };
        v.last().map(|&i| i as usize)
    }

    fn has(&self, tag: Tag) -> bool {
        !self.by_tag[tag as usize].is_empty()
    }

    /// Is there an element above `at` whose end priority exceeds `pri`?
    fn higher_above(&self, at: usize, pri: u8) -> bool {
        PRI_SLOTS
            .iter()
            .zip(&self.by_pri)
            .any(|(&p, v)| p > pri && v.last().is_some_and(|&i| i as usize > at))
    }
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
    interrupt: Option<&'a dyn Interrupt>,
    /// Where the content loop stops to poll the interrupt: the end of the
    /// input without one, so the plain parse pays nothing per token and the
    /// loop is compiled once for both paths.
    limit: usize,
    /// The interrupt fired.
    stopped: bool,
    /// Present only while the stack is deep; see [`DEEP`].
    deep: Option<Box<DeepIndex>>,
    /// The nesting cap in stack entries; `usize::MAX` for none (#48).
    max_depth: usize,
    /// Stack length at which a push takes the cold path: [`DEEP`] for the
    /// index, lower for a cap below it, 0 once halted. One compare against a
    /// field keeps the cap off the common path.
    deep_at: usize,
    /// The cap was hit: libxml2's `xmlHaltParser`. Nothing more is added.
    halted: bool,
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

    /// Poll the interrupt and move `limit` on by [`POLL_BYTES`]; true once
    /// it has fired.
    #[cold]
    #[inline(never)]
    fn poll(&mut self) -> bool {
        if self.halted {
            return true;
        }
        if let Some(i) = self.interrupt {
            if i.is_interrupted() {
                self.stopped = true;
                return true;
            }
        }
        self.limit = if self.interrupt.is_some() {
            self.pos.saturating_add(POLL_BYTES).min(self.b.len())
        } else {
            self.b.len()
        };
        false
    }

    fn document(&mut self) {
        self.skip_blanks();
        while self.peek(0) == Some(b'<') {
            if self.pos >= self.limit && self.poll() {
                return;
            }
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
        loop {
            self.content_until_limit();
            if self.pos >= self.b.len() || self.poll() {
                return;
            }
        }
    }

    #[inline(always)]
    fn content_until_limit(&mut self) {
        while self.pos < self.limit {
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
                    self.append_unless_halted(Cow::Borrowed("<"));
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
    #[inline]
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
        self.push_open(Open { node: el, tag });
        el
    }

    #[inline]
    fn push_open(&mut self, o: Open) {
        self.stack.push(o);
        // One compare on the common path: the index and the cap both start
        // at or past `deep_at`, so a shallower stack can skip every arm.
        if self.stack.len() >= self.deep_at {
            self.push_deep(o);
        }
    }

    #[cold]
    #[inline(never)]
    fn push_deep(&mut self, o: Open) {
        if self.halted || self.stack.len() > self.max_depth {
            return self.refuse_open(o);
        }
        if self.stack.len() < DEEP {
            return;
        }
        if let Some(d) = self.deep.as_mut() {
            d.push(self.stack.len() - 1, o, &self.doc);
        } else {
            self.deep = Some(Box::new(DeepIndex::build(&self.stack, &self.doc)));
        }
    }

    /// libxml2's `nodePush` past `maxDepth`: the element is unlinked and
    /// freed, the error is raised once, and the parser halts. The element was
    /// never indexed, so it leaves the stack directly.
    #[cold]
    #[inline(never)]
    fn refuse_open(&mut self, o: Open) {
        self.stack.pop();
        self.doc.xml_unlink_node(o.node);
        if self.halted {
            return;
        }
        self.halted = true;
        self.deep_at = 0;
        self.limit = 0;
        let before = &self.b[..self.pos];
        let line = 1 + before.iter().filter(|&&c| c == b'\n').count();
        let col = self.pos - before.iter().rposition(|&c| c == b'\n').map_or(0, |i| i + 1) + 1;
        let e = XmlError::new(
            crate::error::XML_ERR_RESOURCE_LIMIT,
            format!(
                "Excessive depth in document: {}, use XML_PARSE_HUGE option",
                self.stack.len()
            ),
            u32::try_from(line).unwrap_or(u32::MAX),
            u32::try_from(col).unwrap_or(u32::MAX),
        );
        self.doc.warnings.push(e.to_string());
    }

    #[inline]
    fn pop_open(&mut self) {
        if let Some(o) = self.stack.pop() {
            if self.deep.is_some() {
                self.pop_deep(o);
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn pop_deep(&mut self, o: Open) {
        if let Some(d) = self.deep.as_mut() {
            d.pop(o, &self.doc);
            // Hysteresis: hovering around DEEP does not rebuild per tag.
            if self.stack.len() < DEEP / 2 {
                self.deep = None;
            }
        }
    }

    fn truncate_open(&mut self, at: usize) {
        if self.deep.is_none() {
            self.stack.truncate(at);
            return;
        }
        while self.stack.len() > at {
            self.pop_open();
        }
    }

    /// Is an element with this (known) tag open?
    fn is_open(&self, tag: Tag) -> bool {
        match &self.deep {
            Some(d) => d.has(tag),
            None => self.stack.iter().any(|o| o.tag == tag),
        }
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
            self.pop_open();
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
            if self.is_open(BODY) || self.is_open(HEAD) {
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

    /// Text after `start_char_data`, whose implied element may have hit the
    /// nesting cap: a halted parser adds nothing (#48).
    fn append_unless_halted(&mut self, s: Cow<'_, str>) {
        if !self.halted {
            self.append(s, NodeKind::Text);
        }
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
            self.append_unless_halted(rest);
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
        if tag == BODY && self.is_open(BODY) {
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
            self.pop_open();
            return;
        }
        if self.b[self.pos] != b'>' {
            return;
        }
        self.pos += 1;
        if tags::is_empty(top.tag) {
            self.pop_open();
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
            self.append_unless_halted(Cow::Borrowed("</"));
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
        if let Some(d) = &self.deep {
            let Some(at) = d.top_of(tag, name.as_ref()) else {
                return;
            };
            if d.higher_above(at, tags::end_priority(tag)) {
                return;
            }
            self.truncate_open(at);
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
pub(crate) fn scan_raw(src: &str, b: &[u8], from: usize, name: &[u8], mode: u8) -> (usize, bool) {
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

#[cfg(test)]
mod tests {
    use super::scan_cr_nul;

    #[test]
    fn scan_counts_lt_across_blocks_and_the_remainder() {
        for len in [0, 1, 63, 64, 65, 127, 128, 200, 1000] {
            let b: Vec<u8> = (0..len).map(|i| if i % 3 == 0 { b'<' } else { b'a' }).collect();
            let want = b.iter().filter(|&&c| c == b'<').count();
            assert_eq!(scan_cr_nul(&b), (false, false, want), "len {len}");
        }
        let all = vec![b'<'; 64 * 5 + 7];
        assert_eq!(scan_cr_nul(&all).2, all.len());
        let mixed = b"<a>\r\n<b>\0</b>".repeat(20);
        assert_eq!(scan_cr_nul(&mixed), (true, true, 60));
    }
}
