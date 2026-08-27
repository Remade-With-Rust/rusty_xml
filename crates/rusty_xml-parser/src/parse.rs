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
pub const XML_PARSE_NO_SYS_CATALOG: i32 = 1 << 25;
pub const XML_PARSE_CATALOG_PI: i32 = 1 << 26;
pub const XML_PARSE_SKIP_IDS: i32 = 1 << 27;

const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";

const MAX_DEPTH: u32 = 256;
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
    let mut rec = rusty_xml_sax::SaxRecorder::new();
    parse_doc(buffer, url, encoding, options, &mut rec)
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
    doc: Option<XmlDoc>,
    last_error: Option<XmlError>,
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
        doc: None,
        last_error: None,
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
    if terminate == 0 {
        return Ok(None);
    }
    match xml_read_memory(
        &ctxt.buf,
        ctxt.url.as_deref(),
        ctxt.encoding.as_deref(),
        ctxt.options,
    ) {
        Ok(doc) => {
            ctxt.doc = Some(doc.clone());
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
    started: bool,
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
        if self.eof() {
            return Ok(None);
        }
        let rest = &self.input[self.pos..];
        if rest[0] == b'\r' {
            return Ok(Some('\n'));
        }
        if rest[0] < 0x80 {
            return Ok(Some(rest[0] as char));
        }
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
        let c = match self.peek_char()? {
            None => return Ok(None),
            Some(c) => c,
        };
        if self.peek_byte() == Some(b'\r') {
            self.bump_byte();
            if self.peek_byte() == Some(b'\n') {
                self.bump_byte();
            }
            return Ok(Some('\n'));
        }
        let n = c.len_utf8();
        for _ in 0..n {
            self.bump_byte();
        }
        Ok(Some(c))
    }

    fn skip_s(&mut self) -> Result<(), XmlError> {
        while let Some(c) = self.peek_char()? {
            if crate::chvalid::xml_is_blank(c as u32) {
                self.bump_char()?;
            } else {
                break;
            }
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

    fn parse_name(&mut self) -> Result<String, XmlError> {
        let c = self.peek_char()?.ok_or_else(|| self.err(XML_ERR_NAME_REQUIRED, "Name expected"))?;
        if !xml_is_name_start_char(c as u32, self.old10) {
            return Err(self.err(XML_ERR_NAME_REQUIRED, "Name expected"));
        }
        let mut s = String::new();
        s.push(self.bump_char()?.unwrap());
        while let Some(c) = self.peek_char()? {
            if xml_is_name_char(c as u32, self.old10) {
                if s.len() >= MAX_NAME && (self.options & XML_PARSE_HUGE) == 0 {
                    return Err(self.err(XML_ERR_NAME_REQUIRED, "Name too long"));
                }
                s.push(self.bump_char()?.unwrap());
            } else {
                break;
            }
        }
        Ok(s)
    }

    fn split_qname(name: &str) -> Result<(Option<String>, String), XmlError> {
        let mut parts = name.split(':');
        let a = parts.next().unwrap();
        match parts.next() {
            None => Ok((None, a.to_string())),
            Some(b) => {
                if parts.next().is_some() || a.is_empty() || b.is_empty() {
                    return Err(XmlError::new(
                        XML_NS_ERR_QNAME,
                        format!("Invalid QName {name}"),
                        0,
                        0,
                    ));
                }
                Ok((Some(a.to_string()), b.to_string()))
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
            if let Some(p) = parent {
                let t = self.doc.alloc(NodeKind::Text, "#text");
                self.doc.node_mut(t).content = self.char_buf.clone();
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
        if let Some(p) = parent {
            let n = self.doc.alloc(NodeKind::Comment, "#comment");
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
        let data = if matches!(self.peek_char()?, Some(c) if crate::chvalid::xml_is_blank(c as u32)) {
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
                d.push(self.bump_char()?.unwrap());
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
        if let Some(p) = parent {
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
            body.push(self.bump_char()?.unwrap());
        }
        if (self.options & XML_PARSE_NOCDATA) != 0 {
            self.sax.characters(&body);
            if let Some(p) = parent {
                let t = self.doc.alloc(NodeKind::Text, "#text");
                self.doc.node_mut(t).content = body;
                self.doc.xml_add_child(p, t);
            }
        } else {
            self.sax.cdata_block(&body);
            if let Some(p) = parent {
                let t = self.doc.alloc(NodeKind::CData, "#cdata-section");
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
            let hex = self.peek_byte() == Some(b'x') || self.peek_byte() == Some(b'X');
            if hex {
                self.bump_byte();
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
                if let Some(dtd) = &self.doc.dtd {
                    if let Some(repl) = dtd.entities.get(&name) {
                        return Ok(repl.clone());
                    }
                }
                Err(self.err(
                    XML_ERR_UNDECLARED_ENTITY,
                    format!("Entity '{name}' not defined"),
                ))
            }
        }
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
            public_id = Some(self.parse_quoted()?);
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
            crate::dtd::parse_dtd_subset(subset).unwrap_or_default()
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

    fn parse_element(&mut self, parent: NodeId) -> Result<(), XmlError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH && (self.options & XML_PARSE_HUGE) == 0 {
            return Err(self.err(XML_ERR_INTERNAL_ERROR, "Excessive element nesting"));
        }
        self.expect_byte(b'<', XML_ERR_LT_REQUIRED, "'<' required")?;
        let qname = self.parse_name()?;
        let (prefix, local) = Self::split_qname(&qname).map_err(|mut e| {
            e.line = self.line;
            e.col = self.col;
            e
        })?;

        struct RawAttr {
            qname: String,
            value: String,
            value_off: usize,
        }
        let mut raw_attrs: Vec<RawAttr> = Vec::new();
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
            raw_attrs.push(RawAttr {
                qname: an,
                value,
                value_off,
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
            let (ap, al) = Self::split_qname(&a.qname).map_err(|mut e| {
                e.line = self.line;
                e.col = self.col;
                e
            })?;
            if ap.is_none() && al == "xmlns" {
                if !a.value.is_empty() && !Self::uri_has_scheme(&a.value) {
                    let msg = format!("xmlns: URI {} is not absolute\n", a.value);
                    self.sax.warning(&msg);
                }
                ns_frame.push((None, a.value.clone()));
            } else if ap.as_deref() == Some("xmlns") {
                if !a.value.is_empty()
                    && !Self::uri_has_scheme(&a.value)
                    && (self.options & XML_PARSE_PEDANTIC) != 0
                {
                    let msg = format!("xmlns:{}: URI {} is not absolute\n", al, a.value);
                    self.sax.warning(&msg);
                }
                ns_frame.push((Some(al), a.value.clone()));
            }
        }
        self.ns_stack.push(ns_frame.clone());

        let elem_uri = self.lookup_ns(prefix.as_deref());
        if prefix.is_some() && elem_uri.is_none() {
            return Err(self.err(
                XML_NS_ERR_UNDEFINED_NAMESPACE,
                format!("Undefined namespace prefix {}", prefix.as_deref().unwrap()),
            ));
        }

        let mut sax_attrs: Vec<SaxAttr> = Vec::new();
        let mut seen: Vec<(Option<String>, String)> = Vec::new();
        for a in &raw_attrs {
            let (ap, al) = Self::split_qname(&a.qname).unwrap();
            if (ap.is_none() && al == "xmlns") || ap.as_deref() == Some("xmlns") {
                continue;
            }
            let uri = if ap.is_some() {
                let u = self.lookup_ns(ap.as_deref());
                if u.is_none() {
                    return Err(self.err(
                        XML_NS_ERR_UNDEFINED_NAMESPACE,
                        format!("Undefined namespace prefix {}", ap.as_deref().unwrap()),
                    ));
                }
                u
            } else {
                None
            };
            let key = (uri.clone(), al.clone());
            if seen.iter().any(|s| s == &key) {
                return Err(self.err(XML_ERR_ATTRIBUTE_REDEFINED, "Attribute redefined"));
            }
            seen.push(key);
            sax_attrs.push(SaxAttr {
                local: al,
                prefix: ap,
                uri,
                value: a.value.clone(),
                value_input_off: Some(a.value_off),
            });
        }

        self.sax.start_element_ns(
            &local,
            prefix.as_deref(),
            elem_uri.as_deref(),
            &ns_frame,
            &sax_attrs,
            0,
        );

        let elem = self.doc.alloc(NodeKind::Element, local.clone());
        self.doc.node_mut(elem).prefix = prefix.clone();
        self.doc.node_mut(elem).ns_uri = elem_uri.clone();
        for (p, u) in &ns_frame {
            self.doc.push_ns_def(elem, p.clone(), u.clone());
        }
        for a in &sax_attrs {
            let aid = self.doc.add_attr(elem, &a.local, a.prefix.as_deref(), &a.value);
            self.doc.node_mut(aid).ns_uri = a.uri.clone();
        }
        self.doc.xml_add_child(parent, elem);

        if empty {
            self.sax.end_element_ns(&local, prefix.as_deref(), elem_uri.as_deref());
            self.ns_stack.pop();
            self.depth -= 1;
            return Ok(());
        }

        self.stack.push(elem);
        self.parse_content(elem)?;
        if !self.starts_with(b"</") {
            return Err(self.err(
                XML_ERR_TAG_NOT_FINISHED,
                format!("Premature end of data in tag {local}"),
            ));
        }
        self.pos += 2;
        self.col += 2;
        let end_name = self.parse_name()?;
        self.skip_s()?;
        self.expect_byte(b'>', XML_ERR_GT_REQUIRED, "'>' required")?;
        if end_name != qname {
            return Err(self.err(
                XML_ERR_TAG_NAME_MISMATCH,
                format!("Opening and ending tag mismatch: {qname} and {end_name}"),
            ));
        }
        self.sax.end_element_ns(&local, prefix.as_deref(), elem_uri.as_deref());
        self.ns_stack.pop();
        self.stack.pop();
        self.depth -= 1;
        Ok(())
    }

    fn parse_content(&mut self, parent: NodeId) -> Result<(), XmlError> {
        loop {
            if self.eof() {
                self.flush_chars(Some(parent))?;
                return Ok(());
            }
            if self.starts_with(b"</") {
                self.flush_chars(Some(parent))?;
                return Ok(());
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
            if self.peek_byte() == Some(b'<') {
                self.flush_chars(Some(parent))?;
                self.parse_element(parent)?;
                continue;
            }
            if self.peek_byte() == Some(b'&') {
                self.flush_chars(Some(parent))?;
                let repl = self.parse_reference()?;
                self.char_buf.push_str(&repl);
                self.flush_chars(Some(parent))?;
                continue;
            }
            if self.starts_with(b"]]>") {
                return Err(self.err(XML_ERR_MISPLACED_CDATA_END, "Misplaced CDATA end"));
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

    fn parse_document(&mut self) -> Result<(), XmlError> {
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
            match self.peek_char()? {
                Some(c) if crate::chvalid::xml_is_blank(c as u32) => {
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
        self.parse_element(NodeId::DOCUMENT)?;
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
    let (converted, enc_name) = crate::encoding::xml_convert_to_utf8(buffer, encoding)?;
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
        doc: XmlDoc::xml_new_doc(Some("1.0")),
        stack: Vec::new(),
        char_buf: String::new(),
        started: false,
    };
    match p.parse_document() {
        Ok(()) => {
            apply_dtd_defaults(&mut p.doc);
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
            Err(e)
        }
    }
}

fn apply_dtd_defaults(doc: &mut XmlDoc) {
    let Some(dtd) = doc.dtd.clone() else { return };
    let n = doc.len();
    for i in 0..n {
        let id = NodeId(i as u32);
        if doc.kind(id) != NodeKind::Element {
            continue;
        }
        let name = doc.name(id).to_string();
        for ((elem, aname), ad) in &dtd.attributes {
            if elem != &name {
                continue;
            }
            if doc.xml_get_prop(id, aname).is_some() {
                continue;
            }
            if let Some(v) = &ad.default_value {
                doc.xml_set_prop(id, aname, v);
            }
        }
    }
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
