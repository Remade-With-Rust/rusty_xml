//! UTF-8 well-formed document parser. No DTD / HTML / XInclude / recovery (M1).

use rusty_xml_sax::{SaxAttr, SaxHandler};
use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};

use crate::chvalid::{xml_is_char, xml_is_name_char, xml_is_name_start_char};
use crate::error::*;

/// libxml2 `xmlParserOption` bits (numeric identity).
pub const XML_PARSE_RECOVER: i32 = 1 << 0;
pub const XML_PARSE_NOENT: i32 = 1 << 1;
pub const XML_PARSE_DTDLOAD: i32 = 1 << 2;
pub const XML_PARSE_DTDATTR: i32 = 1 << 3;
pub const XML_PARSE_DTDVALID: i32 = 1 << 4;
pub const XML_PARSE_NOERROR: i32 = 1 << 5;
pub const XML_PARSE_NOWARNING: i32 = 1 << 6;
pub const XML_PARSE_PEDANTIC: i32 = 1 << 7;
pub const XML_PARSE_NOBLANKS: i32 = 1 << 8;
pub const XML_PARSE_SAX1: i32 = 1 << 9;
pub const XML_PARSE_XINCLUDE: i32 = 1 << 10;
pub const XML_PARSE_NONET: i32 = 1 << 11;
pub const XML_PARSE_NODICT: i32 = 1 << 12;
pub const XML_PARSE_NSCLEAN: i32 = 1 << 13;
pub const XML_PARSE_NOCDATA: i32 = 1 << 14;
pub const XML_PARSE_NOXINCNODE: i32 = 1 << 15;
pub const XML_PARSE_COMPACT: i32 = 1 << 16;
pub const XML_PARSE_OLD10: i32 = 1 << 17;
pub const XML_PARSE_NOBASEFIX: i32 = 1 << 18;
pub const XML_PARSE_HUGE: i32 = 1 << 19;
pub const XML_PARSE_OLDSAX: i32 = 1 << 20;
pub const XML_PARSE_IGNORE_ENC: i32 = 1 << 21;
pub const XML_PARSE_BIG_LINES: i32 = 1 << 22;
pub const XML_PARSE_NO_XXE: i32 = 1 << 23;
pub const XML_PARSE_UNZIP: i32 = 1 << 24;

/// Deliver SAX events without building a document tree.
/// **A rusty_xml extension, not a libxml2 flag.**
///
/// Every entry point here materialises the whole document, including the ones
/// whose job is streaming: `xml_sax_parse_memory` built a full tree and then
/// discarded it, and `xml_reader_for_memory` builds a tree and walks it with a
/// cursor. A consumer that only wants text -- an indexer, a document converter
/// -- paid for a DOM it never touched.
///
/// With this set, character data, CDATA, comments, processing instructions and
/// attributes create no nodes. The SAX event stream is unchanged and complete;
/// the returned [`XmlDoc`] holds only the element skeleton and should be
/// ignored.
pub const XML_PARSE_NO_TREE: i32 = 1 << 30;
pub const XML_PARSE_NO_SYS_CATALOG: i32 = 1 << 25;
pub const XML_PARSE_CATALOG_PI: i32 = 1 << 26;
pub const XML_PARSE_SKIP_IDS: i32 = 1 << 27;

const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";

/// Element nesting limit.
///
/// This was 64 because the parser recursed and the cap had to sit below the
/// stack limit -- about 1.4 KB per level in release and 22 KB in debug, so a
/// deeper document aborted the process instead of returning an error. The
/// content loop is iterative now and document depth costs no stack, so the cap
/// is a POLICY limit again rather than a crash guard.
///
/// 5000 matches what libxml2 permits by default and is beyond any real markup;
/// `XML_PARSE_HUGE` lifts it, which it can now do safely.
const MAX_DEPTH: u32 = 5_000;

/// The ceiling `XML_PARSE_HUGE` raises the nesting limit to. Bounded rather
/// than unlimited so a hostile document cannot make the arena grow without end.
const MAX_DEPTH_HUGE: u32 = 1_000_000;

/// `XML_PARSE_HUGE` deliberately does **not** lift the nesting limit.
///
/// A depth cap only protects you when the cap is BELOW the stack limit. The
/// element parser is recursive descent, and a debug build overflows around 95
/// levels, so any cap above that is not a limit at all -- the process aborts
/// before the check fires, and a stack overflow cannot be caught. Raising the
/// cap to 256 or 512 therefore did not give callers deeper documents, it gave
/// them a crash instead of an error.
///
/// HUGE still lifts the other limits it guards (name length, text length).
/// Genuinely deeper nesting requires an iterative parser; that is the fix, and
/// a larger constant is not a substitute for it.

const MAX_NAME: usize = 50_000;
const MAX_TEXT: usize = 10_000_000;

/// Safe defaults: no network, no XXE.
pub fn default_parse_options() -> i32 {
    XML_PARSE_NONET | XML_PARSE_NO_XXE
}

/// `xmlInitParser` — no process-global ctor in Rust.
#[doc(alias = "xmlInitParser")]
pub fn xml_init_parser() {}

/// `xmlCleanupParser` — no-op.
#[doc(alias = "xmlCleanupParser")]
pub fn xml_cleanup_parser() {}

/// Parser context (`xmlParserCtxt`).
#[derive(Debug, Default)]
pub struct XmlParserCtxt {
    pub options: i32,
    pub last_error: Option<XmlError>,
    pub doc: Option<XmlDoc>,
}

/// `xmlNewParserCtxt`.
#[doc(alias = "xmlNewParserCtxt")]
pub fn xml_new_parser_ctxt() -> XmlParserCtxt {
    XmlParserCtxt {
        options: default_parse_options(),
        last_error: None,
        doc: None,
    }
}

/// `xmlCtxtUseOptions`.
#[doc(alias = "xmlCtxtUseOptions")]
pub fn xml_ctxt_use_options(ctxt: &mut XmlParserCtxt, options: i32) -> i32 {
    ctxt.options = options | XML_PARSE_NONET | XML_PARSE_NO_XXE;
    0
}

/// `xmlCtxtSetOptions`.
#[doc(alias = "xmlCtxtSetOptions")]
pub fn xml_ctxt_set_options(ctxt: &mut XmlParserCtxt, options: i32) -> i32 {
    xml_ctxt_use_options(ctxt, options)
}

/// `xmlCtxtGetOptions`.
#[doc(alias = "xmlCtxtGetOptions")]
pub fn xml_ctxt_get_options(ctxt: &XmlParserCtxt) -> i32 {
    ctxt.options
}

/// `xmlCtxtGetLastError`.
#[doc(alias = "xmlCtxtGetLastError")]
pub fn xml_ctxt_get_last_error(ctxt: &XmlParserCtxt) -> Option<&XmlError> {
    ctxt.last_error.as_ref()
}

/// `xmlCtxtGetDocument`.
#[doc(alias = "xmlCtxtGetDocument")]
pub fn xml_ctxt_get_document(ctxt: &XmlParserCtxt) -> Option<&XmlDoc> {
    ctxt.doc.as_ref()
}

/// `xmlReadMemory`.
#[doc(alias = "xmlReadMemory")]
pub fn xml_read_memory(
    buffer: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError> {
    let mut sink = rusty_xml_sax::NullSax;
    parse_doc(buffer, url, encoding, options, &mut sink)
}

/// `xmlReadDoc`.
#[doc(alias = "xmlReadDoc")]
pub fn xml_read_doc(
    cur: &str,
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError> {
    xml_read_memory(cur.as_bytes(), url, encoding, options)
}

/// `xmlReadFile`.
#[doc(alias = "xmlReadFile")]
pub fn xml_read_file(
    filename: &str,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError> {
    let bytes = std::fs::read(filename).map_err(|e| {
        XmlError::new(XML_ERR_DOCUMENT_START, e.to_string(), 0, 0)
    })?;
    xml_read_memory(&bytes, Some(filename), encoding, options)
}

/// `xmlCtxtReadMemory`.
#[doc(alias = "xmlCtxtReadMemory")]
pub fn xml_ctxt_read_memory(
    ctxt: &mut XmlParserCtxt,
    buffer: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError> {
    let opts = if options != 0 { options } else { ctxt.options };
    match xml_read_memory(buffer, url, encoding, opts) {
        Ok(doc) => {
            ctxt.doc = Some(doc.clone());
            ctxt.last_error = None;
            Ok(doc)
        }
        Err(e) => {
            ctxt.last_error = Some(e.clone());
            Err(e)
        }
    }
}

/// Parse and record SAX events (for the event-exact gate).
pub fn xml_sax_parse_memory(
    buffer: &[u8],
    options: i32,
    sax: &mut dyn SaxHandler,
) -> Result<XmlDoc, XmlError> {
    parse_doc(buffer, None, None, options, sax)
}

/// Push parser context (`xmlCreatePushParserCtxt`).
pub struct XmlPushParserCtxt {
    buf: Vec<u8>,
    options: i32,
    url: Option<String>,
    encoding: Option<String>,
    last_error: Option<XmlError>,
    /// Parser state between chunks. `None` until the prolog and the root's
    /// start tag have been seen, because until then there is nothing to resume.
    state: Option<PushState>,
    consumed: usize,
    /// Set when the document needs an encoding conversion or has a BOM. The
    /// streaming path works on raw bytes and cannot convert as it goes, so
    /// those documents are buffered whole, as they were before streaming.
    no_stream: bool,
}

impl XmlPushParserCtxt {
    /// The last error, if the most recent chunk failed to parse.
    pub fn last_error(&self) -> Option<&XmlError> {
        self.last_error.as_ref()
    }

    /// Bytes buffered but not yet parsed.
    ///
    /// Once streaming starts this is only the unparsed tail, not the document
    /// seen so far -- that is the whole point of the push parser.
    pub fn buffered(&self) -> usize {
        self.buf.len()
    }

    /// Total bytes parsed and released so far.
    pub fn consumed(&self) -> usize {
        self.consumed
    }
}

/// `xmlCreatePushParserCtxt`.
#[doc(alias = "xmlCreatePushParserCtxt")]
pub fn xml_create_push_parser_ctxt(
    chunk: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> XmlPushParserCtxt {
    XmlPushParserCtxt {
        buf: chunk.to_vec(),
        options: options | XML_PARSE_NONET | XML_PARSE_NO_XXE,
        url: url.map(str::to_string),
        encoding: encoding.map(str::to_string),
        last_error: None,
        state: None,
        consumed: 0,
        no_stream: false,
    }
}

/// `xmlParseChunk`. `terminate != 0` finishes the document.
#[doc(alias = "xmlParseChunk")]
pub fn xml_parse_chunk(
    ctxt: &mut XmlPushParserCtxt,
    chunk: &[u8],
    terminate: i32,
) -> Result<Option<XmlDoc>, XmlError> {
    ctxt.buf.extend_from_slice(chunk);
    let terminate = terminate != 0;
    let opts = ctxt.options;

    // Phase 1 -- prolog. Nothing can stream until the root's start tag is in
    // hand, so buffer until it parses. The prolog is small, and re-parsing it
    // per chunk is cheap; the body, which is not small, is never re-parsed.
    if ctxt.state.is_none() && !ctxt.no_stream {
        // Conversion is stateful and the streaming path hands raw bytes to the
        // parser, so anything that is not already plain UTF-8 is buffered.
        match crate::encoding::xml_convert_to_utf8_cow(&ctxt.buf, ctxt.encoding.as_deref()) {
            Ok((std::borrow::Cow::Borrowed(b), _)) if b.len() == ctxt.buf.len() => {}
            _ => {
                ctxt.no_stream = true;
            }
        }
    }
    if ctxt.state.is_none() && !ctxt.no_stream {
        let mut sink = rusty_xml_sax::NullSax;
        let started = {
            let mut p = fresh_parser(&ctxt.buf, opts, &mut sink);
            match p.parse_prolog().and_then(|()| p.open_element(NodeId::DOCUMENT)) {
                Ok(Some(root)) => {
                    let at = p.pos;
                    Some((p.suspend(vec![root], false), at))
                }
                // `<root/>`, or not enough input yet, or a real error. All three
                // are handled by parsing the buffer whole -- which for the empty
                // root is correct and for an error reports it at the right time.
                _ => None,
            }
        };
        match started {
            Some((st, at)) => {
                ctxt.state = Some(st);
                ctxt.buf.drain(..at);
                ctxt.consumed += at;
            }
            None => {
                if !terminate {
                    return Ok(None);
                }
                return finish_whole(ctxt);
            }
        }
    }

    if ctxt.no_stream {
        if !terminate {
            return Ok(None);
        }
        return finish_whole(ctxt);
    }

    // Phase 2 -- content, streamed. Parse as far as the buffer allows, then
    // release what was consumed: peak memory becomes the tree plus the
    // unparsed tail rather than the tree plus the whole document.
    let mut sink = rusty_xml_sax::NullSax;
    let mut st = ctxt.state.take().expect("state is present past the prolog");
    let mut open = std::mem::take(&mut st.open);
    let was_closed = st.root_closed;
    let mut p = Parser::resume(&ctxt.buf, opts, &mut sink, st);

    // Once the root has closed, everything left is epilogue; re-entering the
    // content loop would parse trailing whitespace as document content.
    let safe = if was_closed {
        0
    } else {
        match p.parse_content_inner(NodeId::DOCUMENT, &mut open, !terminate, true) {
            Ok(at) => at,
            Err(e) => {
                ctxt.last_error = Some(e.clone());
                return Err(e);
            }
        }
    };
    let root_closed = was_closed || open.is_empty();

    if !terminate {
        let at = safe.min(ctxt.buf.len());
        ctxt.state = Some(p.suspend(open, root_closed));
        ctxt.buf.drain(..at);
        ctxt.buf.shrink_to_fit();
        ctxt.consumed += at;
        return Ok(None);
    }

    if let Some(o) = open.last() {
        let (_, local) = Parser::split_qname(&o.qname).unwrap_or((None, &o.qname));
        let e = p.err(
            XML_ERR_TAG_NOT_FINISHED,
            format!("Premature end of data in tag {local}"),
        );
        ctxt.last_error = Some(e.clone());
        return Err(e);
    }

    if let Err(e) = p.parse_epilog() {
        ctxt.last_error = Some(e.clone());
        return Err(e);
    }
    let total = ctxt.consumed + ctxt.buf.len();
    let mut doc = p.suspend(open, true).doc;
    apply_dtd_defaults(&mut doc, total, ctxt.options)?;
    ctxt.buf = Vec::new();
    ctxt.buf.shrink_to_fit();
    ctxt.last_error = None;
    Ok(Some(doc))
}

/// A parser over a whole buffer, configured exactly as `parse_utf8` does.
fn fresh_parser<'a>(
    input: &'a [u8],
    options: i32,
    sax: &'a mut dyn SaxHandler,
) -> Parser<'a> {
    Parser {
        input,
        pos: 0,
        line: 1,
        col: 1,
        options,
        old10: (options & XML_PARSE_OLD10) != 0,
        depth: 0,
        ns_stack: Vec::new(),
        sax,
        doc: XmlDoc::with_node_capacity(
            Some("1.0"),
            if (options & XML_PARSE_NO_TREE) != 0 {
                input.len() / 32
            } else {
                input.len() / 10
            },
        ),
        stack: Vec::new(),
        char_buf: String::new(),
        no_tree: (options & XML_PARSE_NO_TREE) != 0,
        recover: (options & XML_PARSE_RECOVER) != 0,
        // libxml2 bounds entity amplification at a small multiple of the input
        // for the same reason; without a bound, nesting is a bomb.
        entity_budget: input.len().saturating_mul(10).max(1 << 16),
        scratch_raw: Vec::new(),
        scratch_sax: Vec::new(),
        started: false,
    }
}

/// Parse the accumulated buffer as one whole document.
fn finish_whole(ctxt: &mut XmlPushParserCtxt) -> Result<Option<XmlDoc>, XmlError> {
    match xml_read_memory(
        &ctxt.buf,
        ctxt.url.as_deref(),
        ctxt.encoding.as_deref(),
        ctxt.options,
    ) {
        Ok(doc) => {
            ctxt.buf = Vec::new();
            ctxt.buf.shrink_to_fit();
            ctxt.last_error = None;
            Ok(Some(doc))
        }
        Err(e) => {
            ctxt.last_error = Some(e.clone());
            Err(e)
        }
    }
}

/// `xmlReadIO` — caller-supplied read callback, no network.
#[doc(alias = "xmlReadIO")]
pub fn xml_read_io<F>(
    mut read: F,
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError>
where
    F: FnMut(&mut [u8]) -> Result<usize, std::io::Error>,
{
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = read(&mut tmp).map_err(|e| XmlError::new(XML_ERR_DOCUMENT_START, e.to_string(), 0, 0))?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    xml_read_memory(&buf, url, encoding, options)
}

/// `xmlCtxtReset`.
#[doc(alias = "xmlCtxtReset")]
pub fn xml_ctxt_reset(ctxt: &mut XmlParserCtxt) {
    ctxt.doc = None;
    ctxt.last_error = None;
}

/// Parser state that survives between push chunks.
///
/// Everything the parser needs to carry across a chunk boundary is owned data,
/// which is why streaming is possible at all: the descent lives in `open`, not
/// on the call stack.
struct PushState {
    doc: XmlDoc,
    ns_stack: Vec<Vec<(Option<String>, String)>>,
    stack: Vec<NodeId>,
    open: Vec<OpenElem>,
    char_buf: String,
    line: u32,
    col: u32,
    depth: u32,
    /// The root's end tag has been consumed. Without this, a later chunk would
    /// re-enter the content loop with an empty stack and parse the document's
    /// trailing whitespace as content, adding a stray text node.
    root_closed: bool,
}

/// An element whose start tag has been consumed and whose end tag has not.
struct OpenElem {
    /// The QName exactly as written, for the end-tag comparison.
    qname: String,
    elem: NodeId,
}

/// One attribute exactly as it was scanned, before namespaces are resolved.
struct RawAttr {
    qname: String,
    value: String,
    value_off: usize,
    /// Byte index of the QName's colon, resolved once at scan time.
    colon: Option<usize>,
}

impl RawAttr {
    fn parts(&self) -> (Option<&str>, &str) {
        match self.colon {
            None => (None, self.qname.as_str()),
            Some(i) => (Some(&self.qname[..i]), &self.qname[i + 1..]),
        }
    }
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    line: u32,
    col: u32,
    options: i32,
    old10: bool,
    depth: u32,
    ns_stack: Vec<Vec<(Option<String>, String)>>,
    sax: &'a mut dyn SaxHandler,
    doc: XmlDoc,
    stack: Vec<NodeId>,
    char_buf: String,
    scratch_raw: Vec<RawAttr>,
    scratch_sax: Vec<SaxAttr>,
    started: bool,
    no_tree: bool,
    recover: bool,
    /// Bytes of entity expansion still permitted. Expanding nested entities
    /// creates the billion-laughs vector, so it is bounded from the start.
    entity_budget: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, code: i32, msg: impl Into<String>) -> XmlError {
        XmlError::new(code, msg, self.line, self.col)
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn starts_with(&self, s: &[u8]) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn bump_byte(&mut self) -> Option<u8> {
        let b = self.peek_byte()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    /// Next Unicode scalar with XML 1.0 §2.11 EOL: `\r\n` / `\r` → `\n`.
    fn peek_char(&self) -> Result<Option<char>, XmlError> {
        // One bounds-checked load covers the end test and the byte fetch; the
        // previous form did eof(), then re-sliced, then indexed.
        let Some(&b0) = self.input.get(self.pos) else {
            return Ok(None);
        };
        if b0 == b'\r' {
            return Ok(Some('\n'));
        }
        if b0 < 0x80 {
            return Ok(Some(b0 as char));
        }
        let rest = &self.input[self.pos..];
        // A UTF-8 scalar is at most 4 bytes, so the leading one is always complete
        // within the first 4. Validating only those keeps this O(1); validating the
        // whole tail made a parse O(n^2) in the document length.
        let head = &rest[..rest.len().min(4)];
        match std::str::from_utf8(head) {
            Ok(s) => Ok(s.chars().next()),
            // The leading scalar decoded; the error belongs to a later one, which
            // this call is not responsible for reporting.
            Err(e) if e.valid_up_to() > 0 => Ok(std::str::from_utf8(&head[..e.valid_up_to()])
                .ok()
                .and_then(|s| s.chars().next())),
            Err(_) => Err(XmlError::new(
                XML_ERR_INVALID_CHAR,
                "Invalid UTF-8",
                self.line,
                self.col,
            )),
        }
    }

    fn bump_char(&mut self) -> Result<Option<char>, XmlError> {
        // ASCII and CR are handled without a decode and without the second
        // peek_byte the CR test used to cost on every character.
        match self.input.get(self.pos) {
            None => return Ok(None),
            Some(&b) if b == b'\r' => {
                self.pos += 1;
                self.col += 1;
                if self.input.get(self.pos) == Some(&b'\n') {
                    self.pos += 1;
                    self.line += 1;
                    self.col = 1;
                }
                return Ok(Some('\n'));
            }
            Some(&b) if b < 0x80 => {
                self.pos += 1;
                if b == b'\n' {
                    self.line += 1;
                    self.col = 1;
                } else {
                    self.col += 1;
                }
                return Ok(Some(b as char));
            }
            _ => {}
        }
        let c = match self.peek_char()? {
            None => return Ok(None),
            Some(c) => c,
        };
        // Advance the whole scalar at once. The byte-at-a-time loop re-ran a
        // bounds-checked load and a newline test for every continuation byte,
        // none of which can be a newline.
        let n = c.len_utf8();
        self.pos += n;
        if c as u32 == 0x0A {
            self.line += 1;
            self.col = 1;
        } else {
            // The byte-at-a-time loop this replaces advanced col once per byte,
            // so keep col in bytes or error positions shift on non-ASCII lines.
            self.col += n as u32;
        }
        Ok(Some(c))
    }

    fn skip_s(&mut self) -> Result<(), XmlError> {
        // Every XML whitespace character is ASCII, so this never needs a decode.
        // The previous form decoded each one twice (peek, then bump).
        while let Some(b) = self.peek_byte() {
            if b >= 0x80 || !crate::chvalid::xml_is_blank(b as u32) {
                break;
            }
            self.bump_byte();
        }
        Ok(())
    }

    fn expect_byte(&mut self, b: u8, code: i32, msg: &str) -> Result<(), XmlError> {
        if self.peek_byte() != Some(b) {
            return Err(self.err(code, msg));
        }
        self.bump_byte();
        Ok(())
    }

    fn parse_name_span(&mut self) -> Result<(usize, usize), XmlError> {
        // The first character went through peek_char AND bump_char -- two
        // decodes -- for what is almost always one ASCII byte.
        match self.input.get(self.pos) {
            Some(&b) if b < 0x80 && b != b'\r' => {
                if !xml_is_name_start_char(b as u32, self.old10) {
                    return Err(self.err(XML_ERR_NAME_REQUIRED, "Name expected"));
                }
            }
            _ => {
                let c = self
                    .peek_char()?
                    .ok_or_else(|| self.err(XML_ERR_NAME_REQUIRED, "Name expected"))?;
                if !xml_is_name_start_char(c as u32, self.old10) {
                    return Err(self.err(XML_ERR_NAME_REQUIRED, "Name expected"));
                }
            }
        }
        // Scan the name in place and copy it out once. Name characters are
        // overwhelmingly ASCII, and an ASCII byte needs no decode at all -- the
        // char-at-a-time form decoded every character twice (peek, then bump)
        // and grew the String one push at a time.
        let start = self.pos;
        self.bump_char()?;
        loop {
            let Some(b) = self.peek_byte() else { break };
            if b < 0x80 {
                if !xml_is_name_char(b as u32, self.old10) {
                    break;
                }
                if self.pos - start >= MAX_NAME && (self.options & XML_PARSE_HUGE) == 0 {
                    return Err(self.err(XML_ERR_NAME_REQUIRED, "Name too long"));
                }
                self.bump_byte();
            } else {
                let Some(c) = self.peek_char()? else { break };
                if !xml_is_name_char(c as u32, self.old10) {
                    break;
                }
                if self.pos - start >= MAX_NAME && (self.options & XML_PARSE_HUGE) == 0 {
                    return Err(self.err(XML_ERR_NAME_REQUIRED, "Name too long"));
                }
                self.bump_char()?;
            }
        }
        // Every byte in the span was accepted as part of a decoded character,
        // so this is valid UTF-8; validate anyway rather than reach for unsafe.
        Ok((start, self.pos))
    }

    /// The owning form. Prefer [`Parser::parse_name_span`] where the name is
    /// only compared -- an end tag allocated a String purely to discard it.
    fn parse_name(&mut self) -> Result<String, XmlError> {
        let (a, b) = self.parse_name_span()?;
        match std::str::from_utf8(&self.input[a..b]) {
            Ok(name) => Ok(name.to_string()),
            Err(_) => Err(self.err(XML_ERR_INVALID_CHAR, "Invalid UTF-8")),
        }
    }

    fn split_qname(name: &str) -> Result<(Option<&str>, &str), XmlError> {
        let mut parts = name.split(':');
        let a = parts.next().unwrap();
        match parts.next() {
            None => Ok((None, a)),
            Some(b) => {
                if parts.next().is_some() || a.is_empty() || b.is_empty() {
                    return Err(XmlError::new(
                        XML_NS_ERR_QNAME,
                        format!("Invalid QName {name}"),
                        0,
                        0,
                    ));
                }
                Ok((Some(a), b))
            }
        }
    }

    fn lookup_ns(&self, prefix: Option<&str>) -> Option<String> {
        if prefix == Some("xml") {
            return Some(XML_NS.into());
        }
        if prefix == Some("xmlns") {
            return Some(XMLNS_NS.into());
        }
        for frame in self.ns_stack.iter().rev() {
            for (p, uri) in frame.iter().rev() {
                if p.as_deref() == prefix {
                    return Some(uri.clone());
                }
            }
        }
        None
    }

    fn uri_has_scheme(uri: &str) -> bool {
        let bytes = uri.as_bytes();
        if bytes.is_empty() {
            return false;
        }
        if !bytes[0].is_ascii_alphabetic() {
            return false;
        }
        let mut i = 1;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b':' {
                return true;
            }
            if b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.' {
                i += 1;
            } else {
                return false;
            }
        }
        false
    }

    fn flush_chars(&mut self, parent: Option<NodeId>) -> Result<(), XmlError> {
        if self.char_buf.is_empty() {
            return Ok(());
        }
        if self.char_buf.len() > MAX_TEXT && (self.options & XML_PARSE_HUGE) == 0 {
            return Err(self.err(XML_ERR_INVALID_CHAR, "Text too long"));
        }
        let skip_blank = (self.options & XML_PARSE_NOBLANKS) != 0
            && self.char_buf.chars().all(|c| crate::chvalid::xml_is_blank(c as u32));
        if !skip_blank {
            self.sax.characters(&self.char_buf);
            if let Some(p) = parent.filter(|_| !self.no_tree) {
                let t = self.doc.alloc_unnamed(NodeKind::Text);
                // Moved, not copied: the buffer is cleared immediately after,
                // so the clone was a pure allocation plus memcpy per text node.
                self.doc.node_mut(t).content = std::mem::take(&mut self.char_buf);
                self.doc.xml_add_child(p, t);
            }
        }
        self.char_buf.clear();
        Ok(())
    }

    fn parse_comment(&mut self, parent: Option<NodeId>) -> Result<(), XmlError> {
        // called after seeing "<!--"
        let mut body = String::new();
        loop {
            if self.starts_with(b"-->") {
                self.pos += 3;
                self.col += 3;
                break;
            }
            if self.eof() {
                return Err(self.err(XML_ERR_COMMENT_NOT_FINISHED, "Comment not finished"));
            }
            if self.starts_with(b"--") {
                return Err(self.err(XML_ERR_HYPHEN_IN_COMMENT, "Double hyphen in comment"));
            }
            let c = self.bump_char()?.unwrap();
            if !xml_is_char(c as u32) {
                return Err(self.err(XML_ERR_INVALID_CHAR, "Invalid character"));
            }
            body.push(c);
        }
        self.sax.comment(&body);
        if let Some(p) = parent.filter(|_| !self.no_tree) {
            let n = self.doc.alloc_unnamed(NodeKind::Comment);
            self.doc.node_mut(n).content = body;
            self.doc.xml_add_child(p, n);
        }
        Ok(())
    }

    fn parse_pi(&mut self, parent: Option<NodeId>, xml_decl_ok: bool) -> Result<bool, XmlError> {
        // called after seeing "<?"
        let target = self.parse_name()?;
        if target.eq_ignore_ascii_case("xml") {
            if xml_decl_ok {
                return self.parse_xml_decl_rest().map(|_| true);
            }
            return Err(self.err(XML_ERR_RESERVED_XML_NAME, "Reserved PI target xml"));
        }
        // Namespaces in XML reserves the colon for QNames, so a PI target
        // should be an NCName -- but C reports this and carries on, and
        // rejecting a document libxml2 accepts is a worse trade than the three
        // conformance cases it would win.
        if target.contains(':') {
            self.sax
                .warning(&format!("colons are forbidden from PI names '{target}'
"));
        }
        let data = if matches!(self.peek_byte(), Some(b) if b < 0x80 && crate::chvalid::xml_is_blank(b as u32)) {
            self.skip_s()?;
            let mut d = String::new();
            loop {
                if self.starts_with(b"?>") {
                    self.pos += 2;
                    self.col += 2;
                    break;
                }
                if self.eof() {
                    return Err(self.err(XML_ERR_PI_NOT_FINISHED, "PI not finished"));
                }
                let c = self.bump_char()?.unwrap();
                // The character rule applies inside a PI too. A form feed in
                // one was accepted; C stops at it.
                if !xml_is_char(c as u32) {
                    return Err(self.err(XML_ERR_INVALID_CHAR, "Invalid character in PI"));
                }
                d.push(c);
            }
            Some(d)
        } else {
            if !self.starts_with(b"?>") {
                return Err(self.err(XML_ERR_PI_NOT_FINISHED, "PI not finished"));
            }
            self.pos += 2;
            self.col += 2;
            None
        };
        self.sax.processing_instruction(&target, data.as_deref());
        if let Some(p) = parent.filter(|_| !self.no_tree) {
            let n = self.doc.alloc(NodeKind::Pi, target);
            self.doc.node_mut(n).content = data.unwrap_or_default();
            self.doc.xml_add_child(p, n);
        }
        Ok(false)
    }

    fn parse_xml_decl_rest(&mut self) -> Result<(), XmlError> {
        self.skip_s()?;
        // version
        if !self.starts_with(b"version") {
            return Err(self.err(XML_ERR_XMLDECL_NOT_FINISHED, "XML declaration version required"));
        }
        self.pos += 7;
        self.col += 7;
        self.skip_s()?;
        self.expect_byte(b'=', XML_ERR_EQUAL_REQUIRED, "'=' required")?;
        self.skip_s()?;
        let ver = self.parse_quoted()?;
        self.doc.version = ver;
        self.skip_s()?;
        if self.starts_with(b"encoding") {
            self.pos += 8;
            self.col += 8;
            self.skip_s()?;
            self.expect_byte(b'=', XML_ERR_EQUAL_REQUIRED, "'=' required")?;
            self.skip_s()?;
            let enc = self.parse_quoted()?;
            // EncName ::= [A-Za-z] ([A-Za-z0-9._] | '-')*
            // Any string at all was accepted, including "_UTF-8" and "".
            let mut cs = enc.chars();
            let ok = cs.next().is_some_and(|c| c.is_ascii_alphabetic())
                && cs.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
            if !ok {
                return Err(self.err(XML_ERR_ENCODING_NAME, "Invalid XML encoding name"));
            }
            self.doc.encoding = Some(enc);
            self.skip_s()?;
        }
        if self.starts_with(b"standalone") {
            self.pos += 10;
            self.col += 10;
            self.skip_s()?;
            self.expect_byte(b'=', XML_ERR_EQUAL_REQUIRED, "'=' required")?;
            self.skip_s()?;
            let st = self.parse_quoted()?;
            self.doc.standalone = match st.as_str() {
                "yes" => Some(true),
                "no" => Some(false),
                _ => return Err(self.err(XML_ERR_XMLDECL_NOT_FINISHED, "standalone must be yes or no")),
            };
            self.skip_s()?;
        }
        if !self.starts_with(b"?>") {
            return Err(self.err(XML_ERR_XMLDECL_NOT_FINISHED, "XML declaration not finished"));
        }
        self.pos += 2;
        self.col += 2;
        Ok(())
    }

    fn parse_quoted(&mut self) -> Result<String, XmlError> {
        let q = self.peek_byte().ok_or_else(|| self.err(XML_ERR_LITERAL_NOT_FINISHED, "Quote expected"))?;
        if q != b'\'' && q != b'"' {
            return Err(self.err(XML_ERR_LITERAL_NOT_FINISHED, "Quote expected"));
        }
        self.bump_byte();
        let mut s = String::new();
        loop {
            let c = self.bump_char()?.ok_or_else(|| self.err(XML_ERR_LITERAL_NOT_FINISHED, "Unterminated literal"))?;
            if c as u8 == q && c.is_ascii() {
                break;
            }
            // The shared literal reader: ATTLIST defaults, entity values,
            // system and public identifiers, and the XML declaration all come
            // through here, and none of them validated. A control byte in an
            // ATTLIST default was injected into every element that took the
            // default and written back as U+FFFD; C says "invalid character in
            // entity value" and stops.
            if !xml_is_char(c as u32) {
                return Err(self.err(XML_ERR_INVALID_CHAR, "invalid character in literal"));
            }
            s.push(c);
        }
        Ok(s)
    }

    fn parse_cdata(&mut self, parent: Option<NodeId>) -> Result<(), XmlError> {
        // after "<![CDATA["
        let mut body = String::new();
        loop {
            if self.starts_with(b"]]>") {
                self.pos += 3;
                self.col += 3;
                break;
            }
            if self.eof() {
                return Err(self.err(XML_ERR_CDATA_NOT_FINISHED, "CDATA not finished"));
            }
            let c = self.bump_char()?.unwrap();
            // CDATA is unparsed, not unchecked: the character rule still
            // applies inside it.
            if !xml_is_char(c as u32) {
                return Err(self.err(XML_ERR_INVALID_CHAR, "invalid character in CDATA"));
            }
            body.push(c);
        }
        if (self.options & XML_PARSE_NOCDATA) != 0 {
            self.sax.characters(&body);
            if let Some(p) = parent.filter(|_| !self.no_tree) {
                let t = self.doc.alloc_unnamed(NodeKind::Text);
                self.doc.node_mut(t).content = body;
                self.doc.xml_add_child(p, t);
            }
        } else {
            self.sax.cdata_block(&body);
            if let Some(p) = parent.filter(|_| !self.no_tree) {
                let t = self.doc.alloc_unnamed(NodeKind::CData);
                self.doc.node_mut(t).content = body;
                self.doc.xml_add_child(p, t);
            }
        }
        Ok(())
    }

    fn parse_reference(&mut self) -> Result<String, XmlError> {
        self.expect_byte(b'&', XML_ERR_ENTITYREF_NO_NAME, "& expected")?;
        if self.peek_byte() == Some(b'#') {
            self.bump_byte();
            // CharRef ::= '&#' [0-9]+ ';' | '&#x' [0-9a-fA-F]+ ';'
            // The marker is lowercase only; `&#X58;` is not a character
            // reference, and we were accepting it.
            let hex = self.peek_byte() == Some(b'x');
            if hex {
                self.bump_byte();
            } else if self.peek_byte() == Some(b'X') {
                return Err(self.err(
                    XML_ERR_INVALID_DEC_CHARREF,
                    "CharRef: invalid decimal value",
                ));
            }
            let mut digits = String::new();
            while let Some(b) = self.peek_byte() {
                let ok = if hex {
                    b.is_ascii_hexdigit()
                } else {
                    b.is_ascii_digit()
                };
                if !ok {
                    break;
                }
                digits.push(b as char);
                self.bump_byte();
            }
            if digits.is_empty() {
                return Err(self.err(
                    if hex { XML_ERR_INVALID_HEX_CHARREF } else { XML_ERR_INVALID_DEC_CHARREF },
                    "Invalid character reference",
                ));
            }
            self.expect_byte(b';', XML_ERR_ENTITYREF_SEMICOL_MISSING, "';' required")?;
            let val = if hex {
                u32::from_str_radix(&digits, 16).map_err(|_| {
                    self.err(XML_ERR_INVALID_HEX_CHARREF, "Invalid hex charref")
                })?
            } else {
                digits.parse::<u32>().map_err(|_| {
                    self.err(XML_ERR_INVALID_DEC_CHARREF, "Invalid decimal charref")
                })?
            };
            if !xml_is_char(val) {
                return Err(self.err(XML_ERR_INVALID_CHARREF, "Invalid character reference"));
            }
            return Ok(char::from_u32(val).unwrap().to_string());
        }
        let name = self.parse_name()?;
        self.expect_byte(b';', XML_ERR_ENTITYREF_SEMICOL_MISSING, "';' required")?;
        match name.as_str() {
            "lt" => Ok("<".into()),
            "gt" => Ok(">".into()),
            "amp" => Ok("&".into()),
            "apos" => Ok("'".into()),
            "quot" => Ok("\"".into()),
            _ => {
                let raw = self
                    .doc
                    .dtd
                    .as_ref()
                    .and_then(|d| d.entities.get(&name))
                    .cloned();
                if let Some(raw) = raw {
                    // The replacement was returned VERBATIM, so a nested
                    // reference landed in the tree as literal text and came
                    // back out escaped: `&b;&b;` became `&amp;b;&amp;b;`.
                    return self.expand_entity(&name, &raw, 0);
                }
                if self.recover {
                    // Recovering: keep the reference as written rather than
                    // losing the whole document over one unknown entity.
                    self.sax
                        .error(&format!("Entity '{name}' not defined"));
                    return Ok(format!("&{name};"));
                }
                Err(self.err(
                    XML_ERR_UNDECLARED_ENTITY,
                    format!("Entity '{name}' not defined"),
                ))
            }
        }
    }

    /// Expand an entity's replacement text, resolving references inside it.
    ///
    /// Bounded twice, because recursion here IS the billion-laughs vector: by
    /// nesting depth, and by a byte budget proportional to the document.
    fn expand_entity(&mut self, name: &str, raw: &str, depth: u32) -> Result<String, XmlError> {
        const MAX_ENTITY_DEPTH: u32 = 40;
        if depth > MAX_ENTITY_DEPTH {
            return Err(self.err(
                XML_ERR_UNDECLARED_ENTITY,
                format!("Entity '{name}' nested too deeply"),
            ));
        }
        let b = raw.as_bytes();
        let mut out = String::with_capacity(raw.len());
        let mut i = 0usize;
        while i < b.len() {
            if b[i] != b'&' {
                let start = i;
                while i < b.len() && b[i] != b'&' {
                    i += 1;
                }
                out.push_str(&raw[start..i]);
                continue;
            }
            let Some(semi) = raw[i..].find(';').map(|k| i + k) else {
                out.push('&');
                i += 1;
                continue;
            };
            let inner = raw[i + 1..semi].to_string();
            if let Some(rest) = inner.strip_prefix('#') {
                let (radix, digits) = match rest.strip_prefix(['x', 'X']) {
                    Some(h) => (16u32, h),
                    None => (10u32, rest),
                };
                match u32::from_str_radix(digits, radix).ok().and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    None => {
                        return Err(self.err(
                            XML_ERR_INVALID_CHAR,
                            format!("Invalid character reference in entity '{name}'"),
                        ))
                    }
                }
                i = semi + 1;
                continue;
            }
            let replacement: Option<String> = match inner.as_str() {
                "lt" => Some("<".into()),
                "gt" => Some(">".into()),
                "amp" => Some("&".into()),
                "apos" => Some("'".into()),
                "quot" => Some('"'.to_string()),
                other => {
                    let nested = self
                        .doc
                        .dtd
                        .as_ref()
                        .and_then(|d| d.entities.get(other))
                        .cloned();
                    match nested {
                        Some(r) => Some(self.expand_entity(other, &r, depth + 1)?),
                        None => None,
                    }
                }
            };
            match replacement {
                Some(r) => {
                    if r.len() > self.entity_budget {
                        return Err(self.err(
                            XML_ERR_INTERNAL_ERROR,
                            "Maximum entity amplification exceeded",
                        ));
                    }
                    self.entity_budget -= r.len();
                    out.push_str(&r);
                }
                None if self.recover => out.push_str(&raw[i..=semi]),
                None => {
                    return Err(self.err(
                        XML_ERR_UNDECLARED_ENTITY,
                        format!("Entity '{inner}' not defined"),
                    ))
                }
            }
            i = semi + 1;
        }
        Ok(out)
    }

    fn parse_att_value(&mut self) -> Result<(String, usize), XmlError> {
        let q = self.peek_byte().ok_or_else(|| {
            self.err(XML_ERR_ATTRIBUTE_WITHOUT_VALUE, "Attribute value expected")
        })?;
        if q != b'\'' && q != b'"' {
            return Err(self.err(XML_ERR_ATTRIBUTE_WITHOUT_VALUE, "Attribute value expected"));
        }
        self.bump_byte();
        let start = self.pos;
        let mut val = String::new();
        loop {
            // Same run trick as character data: most attribute values are plain
            // ASCII with no reference and no whitespace needing normalisation.
            {
                let rs = self.pos;
                let mut i = rs;
                while i < self.input.len() {
                    let b = self.input[i];
                    if b == q || b == b'<' || b == b'&' || b < 0x20 || b >= 0x80 {
                        break;
                    }
                    i += 1;
                }
                if i > rs {
                    if let Ok(run) = std::str::from_utf8(&self.input[rs..i]) {
                        // Almost every value is a single run, so this is the
                        // exact size and the String never grows.
                        if val.is_empty() {
                            val.reserve_exact(i - rs);
                        }
                        val.push_str(run);
                        self.col += (i - rs) as u32;
                        self.pos = i;
                        continue;
                    }
                }
            }
            if self.peek_byte() == Some(q) {
                self.bump_byte();
                break;
            }
            if self.eof() {
                return Err(self.err(XML_ERR_LITERAL_NOT_FINISHED, "Unterminated attribute"));
            }
            if self.peek_byte() == Some(b'<') {
                return Err(self.err(XML_ERR_LT_IN_ATTRIBUTE, "'<' in attribute value"));
            }
            if self.peek_byte() == Some(b'&') {
                val.push_str(&self.parse_reference()?);
                continue;
            }
            let c = self.bump_char()?.unwrap();
            // Character data is validated; attribute values were not, so a
            // stray C0 control byte sailed straight through and the writer
            // quietly substituted U+FFFD for it on the way out -- a silently
            // corrupted value where C reports "invalid character in attribute
            // value". Found by the round-trip check: escaping it on the first
            // save and not the second made serialization non-idempotent.
            if !xml_is_char(c as u32) {
                return Err(self.err(
                    XML_ERR_INVALID_CHAR,
                    "invalid character in attribute value",
                ));
            }
            // AttValue: physical whitespace → space
            if c == '\n' || c == '\t' {
                val.push(' ');
            } else {
                val.push(c);
            }
        }
        Ok((val, start))
    }

    fn skip_doctype(&mut self) -> Result<(), XmlError> {
        // after "<!DOCTYPE"
        self.skip_s()?;
        let name = self.parse_name()?;
        self.skip_s()?;
        let mut public_id = None;
        let mut system_id = None;
        if self.starts_with(b"SYSTEM") {
            self.pos += 6;
            self.col += 6;
            self.skip_s()?;
            system_id = Some(self.parse_quoted()?);
        } else if self.starts_with(b"PUBLIC") {
            self.pos += 6;
            self.col += 6;
            self.skip_s()?;
            let pid = self.parse_quoted()?;
            // PubidLiteral is a restricted character set, not free text.
            if let Some(bad) = pid.chars().find(|c| !crate::dtd::is_pubid_char(*c)) {
                return Err(self.err(
                    XML_ERR_INVALID_CHAR,
                    format!("Invalid character 0x{:X} in public identifier", bad as u32),
                ));
            }
            public_id = Some(pid);
            self.skip_s()?;
            system_id = Some(self.parse_quoted()?);
        }
        self.skip_s()?;
        let mut int_subset = None;
        if self.peek_byte() == Some(b'[') {
            self.bump_byte();
            let start = self.pos;
            let mut depth = 1i32;
            let mut in_quote: Option<u8> = None;
            while depth > 0 {
                let b = self.bump_byte().ok_or_else(|| {
                    self.err(XML_ERR_DOCUMENT_END, "Unterminated DOCTYPE")
                })?;
                if let Some(q) = in_quote {
                    if b == q {
                        in_quote = None;
                    }
                    continue;
                }
                match b {
                    b'\'' | b'"' => in_quote = Some(b),
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    _ => {}
                }
            }
            // exclude the closing ']'
            int_subset = Some(String::from_utf8_lossy(&self.input[start..self.pos.saturating_sub(1)]).into_owned());
        }
        self.skip_s()?;
        self.expect_byte(b'>', XML_ERR_GT_REQUIRED, "'>' required")?;
        let mut dtd = if let Some(ref subset) = int_subset {
            // unwrap_or_default() here discarded EVERY internal-subset
            // error: a malformed DTD silently became an empty one, so the
            // entities and ATTLIST defaults it declared just vanished and the
            // failure surfaced later as a bogus "entity not defined". Recovery
            // mode still tolerates it, because that is what recovery is for.
            match crate::dtd::parse_dtd_subset(subset) {
                Ok(d) => d,
                Err(_) if self.recover => rusty_xml_tree::XmlDtd::default(),
                Err(e) => return Err(e),
            }
        } else {
            rusty_xml_tree::XmlDtd::default()
        };
        dtd.name = Some(name);
        dtd.public_id = public_id;
        dtd.system_id = system_id;
        dtd.int_subset = int_subset;
        self.doc.dtd = Some(dtd);
        Ok(())
    }

    /// Parse a start tag and everything that belongs to it: attributes,
    /// namespace frame, the SAX start event and the element node.
    ///
    /// Returns the open element, or `None` if it was `<x/>` and is already
    /// closed. Split out of `parse_element` so the content loop can be driven
    /// by an explicit stack instead of by recursion.
    fn open_element(&mut self, parent: NodeId) -> Result<Option<OpenElem>, XmlError> {
        self.depth += 1;
        let cap = if (self.options & XML_PARSE_HUGE) != 0 {
            MAX_DEPTH_HUGE
        } else {
            MAX_DEPTH
        };
        if self.depth > cap {
            return Err(self.err(XML_ERR_INTERNAL_ERROR, "Excessive element nesting"));
        }
        self.expect_byte(b'<', XML_ERR_LT_REQUIRED, "'<' required")?;
        let qname = self.parse_name()?;
        let (prefix, local) = Self::split_qname(&qname).map_err(|mut e| {
            e.line = self.line;
            e.col = self.col;
            e
        })?;

        // Reused across elements: a fresh Vec per element allocated once and
        // then grew 1-2-4-8 as the attributes were pushed.
        let mut raw_attrs: Vec<RawAttr> = std::mem::take(&mut self.scratch_raw);
        raw_attrs.clear();
        loop {
            self.skip_s()?;
            if self.starts_with(b"/>") || self.peek_byte() == Some(b'>') {
                break;
            }
            let an = self.parse_name()?;
            self.skip_s()?;
            self.expect_byte(b'=', XML_ERR_EQUAL_REQUIRED, "'=' required")?;
            self.skip_s()?;
            let (value, value_off) = self.parse_att_value()?;
            let colon = match Self::split_qname(&an).map_err(|mut e| {
                e.line = self.line;
                e.col = self.col;
                e
            })? {
                (None, _) => None,
                (Some(pfx), _) => Some(pfx.len()),
            };
            raw_attrs.push(RawAttr {
                qname: an,
                value,
                value_off,
                colon,
            });
        }
        let empty = if self.starts_with(b"/>") {
            self.pos += 2;
            self.col += 2;
            true
        } else {
            self.expect_byte(b'>', XML_ERR_GT_REQUIRED, "'>' required")?;
            false
        };

        let mut ns_frame: Vec<(Option<String>, String)> = Vec::new();
        for a in &raw_attrs {
            let (ap, al) = a.parts();
            if ap.is_none() && al == "xmlns" {
                if !a.value.is_empty() && !Self::uri_has_scheme(&a.value) {
                    let msg = format!("xmlns: URI {} is not absolute\n", a.value);
                    self.sax.warning(&msg);
                }
                ns_frame.push((None, a.value.clone()));
            } else if ap == Some("xmlns") {
                if !a.value.is_empty()
                    && !Self::uri_has_scheme(&a.value)
                    && (self.options & XML_PARSE_PEDANTIC) != 0
                {
                    let msg = format!("xmlns:{}: URI {} is not absolute\n", al, a.value);
                    self.sax.warning(&msg);
                }
                // Namespaces in XML 1.0 reserves `xml` and `xmlns` and forbids
                // undeclaring a prefix. None of this was checked.
                if al == "xml" {
                    if a.value != XML_NS {
                        return Err(self.err(
                            XML_NS_ERR_UNDEFINED_NAMESPACE,
                            "xml namespace prefix mapped to wrong URI",
                        ));
                    }
                } else if a.value == XML_NS {
                    return Err(self.err(
                        XML_NS_ERR_UNDEFINED_NAMESPACE,
                        "xml namespace URI mapped to wrong prefix",
                    ));
                }
                if al == "xmlns" {
                    return Err(self.err(
                        XML_NS_ERR_UNDEFINED_NAMESPACE,
                        "redefinition of the xmlns prefix is forbidden",
                    ));
                }
                if a.value == XMLNS_NS {
                    return Err(self.err(
                        XML_NS_ERR_UNDEFINED_NAMESPACE,
                        "reuse of the xmlns namespace name is forbidden",
                    ));
                }
                // Prefix undeclaring (`xmlns:p=""`) is XML 1.1 only.
                if a.value.is_empty() {
                    return Err(self.err(
                        XML_NS_ERR_UNDEFINED_NAMESPACE,
                        "Empty XML namespace is not allowed",
                    ));
                }
                ns_frame.push((Some(al.to_string()), a.value.clone()));
            }
        }
        // The frame is pushed, not copied; the stack owns it and both later
        // readers borrow it back from there.
        self.ns_stack.push(ns_frame);

        let elem_uri = self.lookup_ns(prefix);
        if prefix.is_some() && elem_uri.is_none() {
            // Scraped markup is full of prefixes nobody declared. libxml2
            // reports this and carries on; refusing the document loses all of
            // its text over a namespace nicety.
            if !self.recover {
                return Err(self.err(
                    XML_NS_ERR_UNDEFINED_NAMESPACE,
                    format!("Undefined namespace prefix {}", prefix.unwrap()),
                ));
            }
            self.sax.error(&format!(
                "Undefined namespace prefix {}",
                prefix.unwrap_or_default()
            ));
        }

        let mut seen_keys: std::collections::HashSet<(Option<String>, String)> =
            std::collections::HashSet::new();
        let mut sax_attrs: Vec<SaxAttr> = std::mem::take(&mut self.scratch_sax);
        sax_attrs.clear();
        for idx in 0..raw_attrs.len() {
            // Own the parts first; SaxAttr needs them owned anyway, so this
            // costs nothing extra and releases the borrow on raw_attrs.
            let (ap_owned, al_owned, is_ns, value_off) = {
                let a = &mut raw_attrs[idx];
                let voff = a.value_off;
                match a.colon {
                    // Unprefixed: the local name IS the whole QName, so move it
                    // instead of allocating a second copy of the same bytes.
                    None => {
                        let is_ns = a.qname == "xmlns";
                        (None, std::mem::take(&mut a.qname), is_ns, voff)
                    }
                    Some(i) => {
                        let is_ns = &a.qname[..i] == "xmlns";
                        (
                            Some(a.qname[..i].to_string()),
                            a.qname[i + 1..].to_string(),
                            is_ns,
                            voff,
                        )
                    }
                }
            };
            if is_ns {
                continue;
            }
            let uri = if ap_owned.is_some() {
                let u = self.lookup_ns(ap_owned.as_deref());
                if u.is_none() && !self.recover {
                    return Err(self.err(
                        XML_NS_ERR_UNDEFINED_NAMESPACE,
                        format!("Undefined namespace prefix {}", ap_owned.clone().unwrap()),
                    ));
                }
                u
            } else {
                None
            };
            // The attributes already accepted ARE the "seen" set -- a separate
            // vector of copies was allocated per element to hold the same thing.
            // Linear over the accepted attributes is fine for the handful a real
            // element carries, but it is O(n^2) and an element with 16,000
            // attributes took 185 ms. Switch to a set once it could matter.
            if sax_attrs.len() < 32 {
                if sax_attrs
                    .iter()
                    .any(|s| s.uri.as_deref() == uri.as_deref() && s.local == al_owned)
                {
                    return Err(self.err(XML_ERR_ATTRIBUTE_REDEFINED, "Attribute redefined"));
                }
            } else {
                if seen_keys.is_empty() {
                    for a in sax_attrs.iter() {
                        seen_keys.insert((a.uri.clone(), a.local.clone()));
                    }
                }
                if !seen_keys.insert((uri.clone(), al_owned.clone())) {
                    return Err(self.err(XML_ERR_ATTRIBUTE_REDEFINED, "Attribute redefined"));
                }
            }
            sax_attrs.push(SaxAttr {
                local: al_owned,
                prefix: ap_owned,
                uri,
                // Moved out of raw_attrs rather than copied: one String clone
                // per attribute in the document.
                value: std::mem::take(&mut raw_attrs[idx].value),
                value_input_off: Some(value_off),
            });
        }

        let frame: &[(Option<String>, String)] =
            self.ns_stack.last().map(Vec::as_slice).unwrap_or(&[]);
        self.sax.start_element_ns(
            local,
            prefix,
            elem_uri.as_deref(),
            frame,
            &sax_attrs,
            0,
        );

        let elem = self.doc.alloc(NodeKind::Element, local);
        self.doc.node_mut(elem).prefix = prefix.map(str::to_string);
        self.doc.node_mut(elem).ns_uri = elem_uri;
        for i in 0..self.ns_stack.last().map_or(0, Vec::len) {
            let (p, u) = {
                let f = self.ns_stack.last().unwrap();
                (f[i].0.clone(), f[i].1.clone())
            };
            self.doc.push_ns_def(elem, p, u);
        }
        if self.no_tree {
            sax_attrs.clear();
        } else {
            for a in sax_attrs.drain(..) {
                let uri = a.uri;
                let aid = self.doc.add_attr_owned(elem, a.local, a.prefix, a.value);
                self.doc.node_mut(aid).ns_uri = uri;
            }
        }
        self.doc.xml_add_child(parent, elem);

        raw_attrs.clear();
        sax_attrs.clear();
        self.scratch_raw = raw_attrs;
        self.scratch_sax = sax_attrs;

        if empty {
            let uri = self.doc.node(elem).ns_uri.as_deref();
            self.sax.end_element_ns(local, prefix, uri);
            self.ns_stack.pop();
            self.depth -= 1;
            return Ok(None);
        }

        self.stack.push(elem);
        Ok(Some(OpenElem { qname, elem }))
    }

    /// Consume the end tag of an open element and emit its SAX end event.
    ///
    /// `local` and `prefix` are re-derived from the stored QName rather than
    /// carried across the call: `split_qname` borrows, so this allocates
    /// nothing.
    fn close_element(&mut self, open: &OpenElem) -> Result<(), XmlError> {
        let (prefix, local) = Self::split_qname(&open.qname).map_err(|mut e| {
            e.line = self.line;
            e.col = self.col;
            e
        })?;
        if !self.starts_with(b"</") {
            return Err(self.err(
                XML_ERR_TAG_NOT_FINISHED,
                format!("Premature end of data in tag {local}"),
            ));
        }
        self.pos += 2;
        self.col += 2;
        let (ea, eb) = self.parse_name_span()?;
        self.skip_s()?;
        self.expect_byte(b'>', XML_ERR_GT_REQUIRED, "'>' required")?;
        if &self.input[ea..eb] != open.qname.as_bytes() {
            let end_name = String::from_utf8_lossy(&self.input[ea..eb]).into_owned();
            let qname = &open.qname;
            return Err(self.err(
                XML_ERR_TAG_NAME_MISMATCH,
                format!("Opening and ending tag mismatch: {qname} and {end_name}"),
            ));
        }
        let uri = self.doc.node(open.elem).ns_uri.as_deref();
        self.sax.end_element_ns(local, prefix, uri);
        self.ns_stack.pop();
        self.stack.pop();
        self.depth -= 1;
        Ok(())
    }

    /// Parse one complete element. Calls into the iterative content loop, so
    /// this is the only frame a document of any depth costs.
    fn parse_element(&mut self, parent: NodeId) -> Result<(), XmlError> {
        let Some(open) = self.open_element(parent)? else {
            return Ok(());
        };
        self.parse_content(open.elem)?;
        self.close_element(&open)
    }

    /// Parse the content of `parent` and of every element nested inside it.
    ///
    /// This used to recurse into `parse_element`, which recursed back here, so
    /// document nesting consumed the call stack -- about 1.4 KB per level in
    /// release and 22 KB in debug, and a stack overflow aborts the process
    /// rather than returning an error. The element context was already heap
    /// state (`stack`, `ns_stack`); only the call frames were not. Now the
    /// descent is an explicit stack and the depth of a document costs no stack
    /// at all.
    fn parse_content(&mut self, parent: NodeId) -> Result<(), XmlError> {
        let mut open: Vec<OpenElem> = Vec::new();
        self.parse_content_inner(parent, &mut open, false, false)?;
        Ok(())
    }

    fn parse_document(&mut self) -> Result<(), XmlError> {
        self.parse_prolog()?;
        self.parse_element(NodeId::DOCUMENT)?;
        self.parse_epilog()
    }

    /// Rebuild a parser over a fresh buffer from saved state.
    fn resume(
        input: &'a [u8],
        options: i32,
        sax: &'a mut dyn SaxHandler,
        st: PushState,
    ) -> Self {
        let _ = st.root_closed;
        Parser {
            input,
            pos: 0,
            line: st.line,
            col: st.col,
            options,
            old10: (options & XML_PARSE_OLD10) != 0,
            depth: st.depth,
            ns_stack: st.ns_stack,
            sax,
            doc: st.doc,
            stack: st.stack,
            char_buf: st.char_buf,
            no_tree: (options & XML_PARSE_NO_TREE) != 0,
            recover: (options & XML_PARSE_RECOVER) != 0,
        // libxml2 bounds entity amplification at a small multiple of the input
        // for the same reason; without a bound, nesting is a bomb.
        entity_budget: input.len().saturating_mul(10).max(1 << 16),
            scratch_raw: Vec::new(),
            scratch_sax: Vec::new(),
            started: true,
        }
    }

    fn suspend(self, open: Vec<OpenElem>, root_closed: bool) -> PushState {
        PushState {
            root_closed,
            doc: self.doc,
            ns_stack: self.ns_stack,
            stack: self.stack,
            open,
            char_buf: self.char_buf,
            line: self.line,
            col: self.col,
            depth: self.depth,
        }
    }

    /// True when the remaining bytes are a proper prefix of a construct and we
    /// cannot tell what it is without more input.
    ///
    /// Only consulted while streaming. Character data is never "incomplete":
    /// the run scanner stops at `<`, `&` and `]`, and pending text is kept in
    /// `char_buf` rather than flushed, so more of it can simply be appended.
    fn incomplete_construct(&self) -> bool {
        let r = &self.input[self.pos..];
        fn has(h: &[u8], n: &[u8]) -> bool {
            h.len() >= n.len() && h.windows(n.len()).any(|w| w == n)
        }
        // A tag ends at the first '>' that is not inside an attribute value.
        fn tag_complete(r: &[u8]) -> bool {
            let mut quote: Option<u8> = None;
            for &b in &r[1..] {
                match quote {
                    Some(q) if b == q => quote = None,
                    Some(_) => {}
                    None => match b {
                        b'"' | 0x27 => quote = Some(b),
                        b'>' => return true,
                        _ => {}
                    },
                }
            }
            false
        }
        match r.first() {
            Some(b'<') => {
                if r.len() < 2 {
                    return true;
                }
                if r.starts_with(b"<!--") {
                    return !has(&r[4..], b"-->");
                }
                if r.starts_with(b"<![CDATA[") {
                    return !has(&r[9..], b"]]>");
                }
                if r.starts_with(b"<?") {
                    return !has(&r[2..], b"?>");
                }
                // `<!` could still become a comment, CDATA or a doctype.
                if r[1] == b'!' && r.len() < 9 {
                    return true;
                }
                !tag_complete(r)
            }
            Some(b'&') => !r.contains(&b';'),
            // `]` might yet become `]]>`.
            Some(b']') => r.len() < 3,
            // XML 1.0 2.11 folds CRLF to a single LF. A trailing CR gives no
            // way to know whether the LF follows, and guessing turned every
            // CRLF that landed on a chunk boundary into two newlines.
            Some(0x0D) => r.len() < 2,
            // A multi-byte character split across chunks: the lead byte says
            // how many continuation bytes belong to it, and without them the
            // scalar cannot be decoded.
            Some(&b0) => {
                let need = if b0 < 0x80 {
                    1
                } else if b0 >> 5 == 0b110 {
                    2
                } else if b0 >> 4 == 0b1110 {
                    3
                } else if b0 >> 3 == 0b11110 {
                    4
                } else {
                    1
                };
                r.len() < need
            }
            None => false,
        }
    }

    /// The content loop, with the open-element stack supplied by the caller so
    /// it can survive between chunks.
    ///
    /// With `stop_at_eof`, running out of input is not an error: parsing stops
    /// at the last SAFE BOUNDARY -- the top of the loop, where we sit between
    /// content items rather than half way through a tag -- and returns that
    /// position. Pending character data stays in `char_buf` rather than being
    /// flushed, so a text run split across two chunks still produces one event
    /// and the push parser matches a whole-document parse exactly.
    fn parse_content_inner(
        &mut self,
        parent: NodeId,
        open: &mut Vec<OpenElem>,
        stop_at_eof: bool,
        stop_when_empty: bool,
    ) -> Result<usize, XmlError> {
        // The element the caller asked us to fill. When the innermost element
        // closes and nothing else is open, content belongs to THIS again --
        // falling back to the mutable `parent` would name the element that had
        // just been closed.
        let outer = parent;
        let mut parent = open.last().map(|f| f.elem).unwrap_or(parent);
        loop {
            let safe = self.pos;
            if self.eof() {
                if stop_at_eof {
                    return Ok(safe);
                }
                self.flush_chars(Some(parent))?;
                if let Some(o) = open.last() {
                    let (_, local) = Self::split_qname(&o.qname).unwrap_or((None, &o.qname));
                    return Err(self.err(
                        XML_ERR_TAG_NOT_FINISHED,
                        format!("Premature end of data in tag {local}"),
                    ));
                }
                return Ok(safe);
            }
            // Without the whole of a construct in hand we cannot tell what it
            // is, so stop here and wait for more input.
            if stop_at_eof && self.incomplete_construct() {
                return Ok(safe);
            }
            if self.starts_with(b"</") {
                self.flush_chars(Some(parent))?;
                // Our own end tag closes the innermost open element; when
                // nothing is open it belongs to the caller.
                match open.pop() {
                    Some(o) => {
                        self.close_element(&o)?;
                        // Streaming starts with the root already open, so an
                        // empty stack means the root just closed and the
                        // epilogue is the driver's job.
                        if stop_when_empty && open.is_empty() {
                            return Ok(self.pos);
                        }
                        parent = open.last().map(|f| f.elem).unwrap_or(outer);
                        continue;
                    }
                    None => return Ok(safe),
                }
            }
            if self.starts_with(b"<!--") {
                self.flush_chars(Some(parent))?;
                self.pos += 4;
                self.col += 4;
                self.parse_comment(Some(parent))?;
                continue;
            }
            if self.starts_with(b"<![CDATA[") {
                self.flush_chars(Some(parent))?;
                self.pos += 9;
                self.col += 9;
                self.parse_cdata(Some(parent))?;
                continue;
            }
            if self.starts_with(b"<?") {
                self.flush_chars(Some(parent))?;
                self.pos += 2;
                self.col += 2;
                self.parse_pi(Some(parent), false)?;
                continue;
            }
            let lead = self.peek_byte();
            if lead == Some(b'<') {
                self.flush_chars(Some(parent))?;
                if let Some(o) = self.open_element(parent)? {
                    parent = o.elem;
                    open.push(o);
                }
                continue;
            }
            if lead == Some(b'&') {
                // A CHARACTER reference is character data by definition -- it
                // cannot introduce markup -- so it belongs in the run it sits
                // in, not in a text node of its own.
                //
                // Flushing around it split `&#65; &#66;` into three nodes, and
                // the middle one was whitespace-only, so XML_PARSE_NOBLANKS
                // deleted it: `A B` came back as `AB`. Losing a space between
                // two character references is silent text corruption. It also
                // costs a node and an allocation per reference.
                //
                // A general entity still gets its own node: its replacement can
                // contain markup and is not ours to inline here.
                let is_charref = self.input.get(self.pos + 1) == Some(&b'#');
                if is_charref {
                    let repl = self.parse_reference()?;
                    self.char_buf.push_str(&repl);
                } else {
                    self.flush_chars(Some(parent))?;
                    let repl = self.parse_reference()?;
                    self.char_buf.push_str(&repl);
                    self.flush_chars(Some(parent))?;
                }
                continue;
            }
            if self.starts_with(b"]]>") {
                return Err(self.err(XML_ERR_MISPLACED_CDATA_END, "Misplaced CDATA end"));
            }
            // Character data is the bulk of most documents and is almost all
            // ordinary ASCII. Take it in one run: one bounds test and one
            // push_str instead of a decode, two peeks and a push per character.
            {
                let start = self.pos;
                let mut i = start;
                while i < self.input.len() {
                    let b = self.input[i];
                    let plain = b == 0x09 || (0x20..0x80).contains(&b);
                    if !plain || b == b'<' || b == b'&' || b == b']' {
                        break;
                    }
                    i += 1;
                }
                if i > start {
                    // Every byte in the run is ASCII and a legal XML character.
                    match std::str::from_utf8(&self.input[start..i]) {
                        Ok(run) => {
                            self.char_buf.push_str(run);
                            self.col += (i - start) as u32;
                            self.pos = i;
                            continue;
                        }
                        Err(_) => {}
                    }
                }
            }
            let c = self.bump_char()?.unwrap();
            if !xml_is_char(c as u32) {
                return Err(self.err(XML_ERR_INVALID_CHAR, "Invalid character"));
            }
            self.char_buf.push(c);
        }
    }

    fn parse_misc(&mut self, parent: NodeId) -> Result<(), XmlError> {
        loop {
            self.skip_s()?;
            if self.starts_with(b"<!--") {
                self.pos += 4;
                self.col += 4;
                self.parse_comment(Some(parent))?;
                continue;
            }
            if self.starts_with(b"<?") {
                self.pos += 2;
                self.col += 2;
                self.parse_pi(Some(parent), false)?;
                continue;
            }
            break;
        }
        Ok(())
    }

    /// Everything before the root element's start tag: BOM, XML declaration,
    /// misc, doctype. Split out so the push parser can reach the root without
    /// committing to parse the whole document in one go.
    fn parse_prolog(&mut self) -> Result<(), XmlError> {
        if self.starts_with(&[0xef, 0xbb, 0xbf]) {
            self.pos += 3;
        }
        self.sax.set_document_locator();
        self.sax.start_document();
        self.started = true;

        // XMLDecl must be at the start (after BOM). `<?xml-stylesheet` is a PI.
        if self.starts_with(b"<?xml") {
            let save_pos = self.pos;
            let save_col = self.col;
            let save_line = self.line;
            self.pos += 5;
            self.col += 5;
            match self.peek_byte() {
                Some(b) if b < 0x80 && crate::chvalid::xml_is_blank(b as u32) => {
                    self.parse_xml_decl_rest()?;
                }
                _ => {
                    self.pos = save_pos;
                    self.col = save_col;
                    self.line = save_line;
                    self.pos += 2;
                    self.col += 2;
                    self.parse_pi(Some(NodeId::DOCUMENT), false)?;
                }
            }
        }

        self.parse_misc(NodeId::DOCUMENT)?;
        if self.starts_with(b"<!DOCTYPE") {
            self.pos += 9;
            self.col += 9;
            self.skip_doctype()?;
            self.parse_misc(NodeId::DOCUMENT)?;
        }

        if self.peek_byte() != Some(b'<') {
            return Err(self.err(XML_ERR_DOCUMENT_EMPTY, "Document is empty"));
        }
        Ok(())
    }

    /// Everything after the root element: trailing misc, then end-of-document.
    fn parse_epilog(&mut self) -> Result<(), XmlError> {
        self.parse_misc(NodeId::DOCUMENT)?;
        self.skip_s()?;
        if !self.eof() {
            return Err(self.err(XML_ERR_EXTRA_CONTENT, "Extra content at the end of the document"));
        }
        self.sax.end_document();
        Ok(())
    }
}

fn parse_doc(
    buffer: &[u8],
    _url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
    sax: &mut dyn SaxHandler,
) -> Result<XmlDoc, XmlError> {
    let (converted, enc_name) = crate::encoding::xml_convert_to_utf8_cow(buffer, encoding)?;
    parse_utf8(&converted, enc_name.as_deref(), options, sax)
}

fn parse_utf8(
    buffer: &[u8],
    enc_name: Option<&str>,
    options: i32,
    sax: &mut dyn SaxHandler,
) -> Result<XmlDoc, XmlError> {
    let options = options | XML_PARSE_NONET | XML_PARSE_NO_XXE;
    let mut p = Parser {
        input: buffer,
        pos: 0,
        line: 1,
        col: 1,
        options,
        old10: (options & XML_PARSE_OLD10) != 0,
        depth: 0,
        ns_stack: Vec::new(),
        sax,
        // Reserving a full arena is the dominant cost of a no-tree parse --
        // pre-allocating a tree only to leave it empty.
        doc: XmlDoc::with_node_capacity(
            Some("1.0"),
            if (options & XML_PARSE_NO_TREE) != 0 {
                // Only element nodes are created in this mode, which measure
                // about one per 36 input bytes. Reserving for a full tree
                // wasted the arena; reserving nothing made it double instead.
                buffer.len() / 32
            } else {
                buffer.len() / 10
            },
        ),
        stack: Vec::new(),
        char_buf: String::new(),
        no_tree: (options & XML_PARSE_NO_TREE) != 0,
        recover: (options & XML_PARSE_RECOVER) != 0,
        // libxml2 bounds entity amplification at a small multiple of the input
        // for the same reason; without a bound, nesting is a bomb.
        entity_budget: buffer.len().saturating_mul(10).max(1 << 16),
        scratch_raw: Vec::new(),
        scratch_sax: Vec::new(),
        started: false,
    };
    match p.parse_document() {
        Ok(()) => {
            apply_dtd_defaults(&mut p.doc, buffer.len(), options)?;
            if p.doc.encoding.is_none() {
                if let Some(n) = enc_name {
                    if !n.eq_ignore_ascii_case("UTF-8") && !n.eq_ignore_ascii_case("US-ASCII") {
                        p.doc.encoding = Some(n.to_string());
                    }
                }
            }
            Ok(p.doc)
        }
        Err(e) => {
            if p.started {
                p.sax.end_document();
            }
            if (options & XML_PARSE_RECOVER) != 0 {
                // Hand back everything parsed before the failure. One bad byte
                // in a large document used to cost the caller all of it.
                p.sax.error(&e.message);
                return Ok(p.doc);
            }
            Err(e)
        }
    }
}

fn apply_dtd_defaults(
    doc: &mut XmlDoc,
    input_len: usize,
    options: i32,
) -> Result<(), XmlError> {
    // Completing attributes from ATTLIST defaults is opt-in, as it is in C:
    // libxml2 does it for XML_PARSE_DTDATTR (xmllint --dtdattr) and not
    // otherwise -- not even for --valid. We did it unconditionally, so every
    // document with an ATTLIST default came back with attributes libxml2 would
    // not have added, which is a visible difference in the serialized output.
    if (options & XML_PARSE_DTDATTR) == 0 {
        return Ok(());
    }
    // The common cases -- no DTD, or a DTD carrying no ATTLIST default -- cost
    // nothing now. Testing before the clone matters: cloning the DTD copies
    // every entity and declaration in it.
    match &doc.dtd {
        None => return Ok(()),
        Some(d) => {
            if !d.attributes.values().any(|a| a.default_value.is_some()) {
                return Ok(());
            }
        }
    }
    // Defaulted attributes are an amplification vector: 13 KB with 200 ATTLIST
    // defaults expanded to 402,002 nodes (~74 MB) before this bound existed.
    // libxml2 caps entity amplification for the same reason. The budget is
    // generous enough that a real DTD never reaches it.
    // Sized from measurement, not taste: a real DTD-heavy document runs about
    // 0.18 defaulted attributes per input byte, while the amplification cases
    // run 12-30 per byte -- two orders of magnitude apart. One per input byte
    // sits in the gap, with a floor so small documents are never penalised and
    // a ceiling so a huge one cannot walk past it.
    let mut budget = input_len.max(65_536).min(5_000_000);
    let dtd = match doc.dtd.clone() {
        Some(d) => d,
        None => return Ok(()),
    };
    // Group the defaults by element name once. The previous form rescanned
    // every declaration for every element in the document, and allocated the
    // element's name each time round.
    let mut by_elem: std::collections::HashMap<&str, Vec<(&str, &str)>> =
        std::collections::HashMap::new();
    for ((elem, aname), ad) in &dtd.attributes {
        if let Some(v) = &ad.default_value {
            by_elem
                .entry(elem.as_str())
                .or_default()
                .push((aname.as_str(), v.as_str()));
        }
    }
    // dtd.attributes is a HashMap with a randomly seeded hasher, so without
    // this the defaulted attributes serialised in a DIFFERENT ORDER ON EVERY
    // RUN of the same binary. Any signature or digest over the saved tree --
    // C14N included -- has to be reproducible.
    for list in by_elem.values_mut() {
        list.sort_unstable_by(|a, b| a.0.cmp(b.0));
    }
    let n = doc.len();
    for i in 0..n {
        let id = NodeId(i as u32);
        if doc.kind(id) != NodeKind::Element {
            continue;
        }
        let Some(list) = by_elem.get(doc.name(id)) else {
            continue;
        };
        for (aname, v) in list.iter() {
            if doc.xml_get_prop(id, aname).is_none() {
                if budget == 0 {
                    return Err(XmlError::new(
                        XML_ERR_INTERNAL_ERROR,
                        "Maximum attribute-default amplification exceeded",
                        0,
                        0,
                    ));
                }
                budget -= 1;
                doc.xml_set_prop(id, aname, v);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod chvalid_tests {
    use crate::xml_is_char;
    use std::path::PathBuf;

    #[test]
    fn xml_is_char_matches_c_bmp_dump() {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop();
        p.pop();
        p.push("corpora");
        p.push("xmlIsChar-bmp.bin");
        if !p.exists() {
            return;
        }
        let dump = std::fs::read(&p).expect("corpora/xmlIsChar-bmp.bin");
        assert_eq!(dump.len(), 65536);
        for i in 0u32..=0xffff {
            let want = dump[i as usize] != 0;
            let got = xml_is_char(i);
            assert_eq!(got, want, "xml_is_char({i:#x}) = {got}, C dump = {want}");
        }
    }
}
