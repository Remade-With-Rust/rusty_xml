//! HTML parser matching libxml2 `HTMLparser.c` (a separate grammar, not XML recovery).

use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};

use crate::error::XmlError;
use crate::parse::default_parse_options;

/// libxml2 `htmlParserOption` bits we honour.
pub const HTML_PARSE_NOIMPLIED: i32 = 1 << 13;
pub const HTML_PARSE_NONET: i32 = 1 << 11;

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

fn is_void(name: &str) -> bool {
    VOID.contains(&name)
}

/// `htmlReadMemory`.
#[doc(alias = "htmlReadMemory")]
pub fn html_read_memory(
    buffer: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlDoc, XmlError> {
    let (utf8, _) = crate::encoding::xml_convert_to_utf8(buffer, encoding)?;
    html_parse_utf8(&utf8, url, options)
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
pub fn html_read_file(filename: &str, encoding: Option<&str>, options: i32) -> Result<XmlDoc, XmlError> {
    let b = std::fs::read(filename).map_err(|e| XmlError::new(4, e.to_string(), 0, 0))?;
    html_read_memory(&b, Some(filename), encoding, options)
}

fn html_parse_utf8(bytes: &[u8], _url: Option<&str>, options: i32) -> Result<XmlDoc, XmlError> {
    let text = String::from_utf8_lossy(bytes);
    let mut p = HtmlParser {
        src: text.as_ref(),
        pos: 0,
        doc: XmlDoc::xml_new_doc(Some("1.0")),
        stack: Vec::new(),
        noimplied: (options & HTML_PARSE_NOIMPLIED) != 0,
        html: None,
        head: None,
        body: None,
    };
    p.doc.encoding = Some("HTML".into());
    // Mark it as an HTML document so the serializer can tell. Without this the
    // writer had no way to know, and emitted an XML declaration where C emits
    // the doctype -- which, re-parsed as HTML, became a text node. An HTML
    // round trip was not stable.
    p.doc.node_mut(rusty_xml_tree::NodeId::DOCUMENT).kind = NodeKind::HtmlDocument;
    p.parse()?;
    while p.stack.len() > 1 {
        p.stack.pop();
    }
    let _ = options | HTML_PARSE_NONET | default_parse_options();
    Ok(p.doc)
}

struct HtmlParser<'a> {
    src: &'a str,
    pos: usize,
    doc: XmlDoc,
    stack: Vec<NodeId>,
    noimplied: bool,
    html: Option<NodeId>,
    head: Option<NodeId>,
    body: Option<NodeId>,
}

impl<'a> HtmlParser<'a> {
    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }
    fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }
    fn bump(&mut self, n: usize) {
        self.pos += n;
    }
    fn parent(&self) -> NodeId {
        *self.stack.last().unwrap_or(&NodeId::DOCUMENT)
    }
    fn ensure_html(&mut self) -> NodeId {
        if let Some(h) = self.html {
            return h;
        }
        let html = self.doc.xml_new_node(None, "html");
        self.doc.xml_doc_set_root_element(html);
        self.html = Some(html);
        html
    }
    fn ensure_head(&mut self) -> NodeId {
        if let Some(h) = self.head {
            return h;
        }
        let html = self.ensure_html();
        let head = self.doc.xml_new_node(None, "head");
        self.doc.xml_add_child(html, head);
        self.head = Some(head);
        head
    }
    fn ensure_body(&mut self) -> NodeId {
        if let Some(b) = self.body {
            return b;
        }
        let html = self.ensure_html();
        let body = self.doc.xml_new_node(None, "body");
        self.doc.xml_add_child(html, body);
        self.body = Some(body);
        body
    }
    fn ensure_html_body(&mut self) -> NodeId {
        if self.noimplied {
            return self.stack.last().copied().unwrap_or(NodeId::DOCUMENT);
        }
        self.ensure_body()
    }
    fn parse(&mut self) -> Result<(), XmlError> {
        while !self.eof() {
            if self.rest().starts_with("<!--") {
                self.parse_comment()?;
            } else if self.rest().starts_with("<!") {
                self.skip_decl();
            } else if self.rest().starts_with("</") {
                self.parse_end_tag();
            } else if self.rest().starts_with('<') {
                self.parse_start_tag()?;
            } else {
                self.parse_text();
            }
        }
        Ok(())
    }
    fn parse_comment(&mut self) -> Result<(), XmlError> {
        self.bump(4);
        if let Some(end) = self.rest().find("-->") {
            let body = self.rest()[..end].to_string();
            self.bump(end + 3);
            let n = self.doc.alloc(NodeKind::Comment, "#comment");
            self.doc.node_mut(n).content = body;
            self.doc.xml_add_child(self.parent(), n);
        } else {
            self.pos = self.src.len();
        }
        Ok(())
    }
    /// Markup declaration. Only DOCTYPE carries anything we keep.
    ///
    /// This used to discard the lot, so an HTML document's doctype was lost and
    /// the serializer had nothing to write. C round-trips it: `<!DOCTYPE html>`
    /// in, `<!DOCTYPE html>` out.
    fn skip_decl(&mut self) {
        let decl_is_doctype = self
            .rest()
            .get(2..9)
            .is_some_and(|k| k.eq_ignore_ascii_case("DOCTYPE"));
        let end = self.rest().find('>');
        let body = match end {
            Some(i) => self.rest()[..i].to_string(),
            None => self.rest().to_string(),
        };
        match end {
            Some(i) => self.bump(i + 1),
            None => self.pos = self.src.len(),
        }
        if decl_is_doctype && self.doc.dtd.is_none() {
            if let Some(tail) = body.get(9..) {
                self.doc.dtd = Some(parse_html_doctype(tail));
            }
        }
    }
    fn parse_text(&mut self) {
        let mut i = 0;
        let r = self.rest();
        for (off, c) in r.char_indices() {
            if c == '<' {
                i = off;
                break;
            }
            i = off + c.len_utf8();
        }
        if i == 0 {
            return;
        }
        // Entities were never decoded here, so `caf&eacute;` reached the tree
        // verbatim and came back out as `caf&amp;eacute;`.
        let t = crate::html_entities::decode_html_text(&r[..i]).into_owned();
        self.bump(i);
        if t.chars().all(|c| c.is_whitespace()) && self.stack.is_empty() {
            return;
        }
        let n = self.doc.alloc(NodeKind::Text, "#text");
        self.doc.node_mut(n).content = t;
        let parent = if self.stack.is_empty() {
            self.ensure_html_body()
        } else {
            self.parent()
        };
        self.doc.xml_add_child(parent, n);
    }
    fn parse_start_tag(&mut self) -> Result<(), XmlError> {
        self.bump(1);
        let name = self.read_name().to_ascii_lowercase();
        if name.is_empty() {
            return Ok(());
        }
        let mut attrs: Vec<(String, String)> = Vec::new();
        loop {
            self.skip_ws();
            if self.rest().starts_with('>') {
                self.bump(1);
                break;
            }
            if self.rest().starts_with("/>") {
                self.bump(2);
                break;
            }
            if self.eof() {
                break;
            }
            let an = self.read_name().to_ascii_lowercase();
            if an.is_empty() {
                // Skip one CHARACTER, not one byte. A multi-byte character that
                // cannot start an attribute name left self.pos mid-scalar, and
                // the next rest() slice panicked. `<r` + U+0777 + `/>` sufficed.
                let step = self.rest().chars().next().map_or(1, char::len_utf8);
                self.bump(step);
                continue;
            }
            self.skip_ws();
            let av = if self.rest().starts_with('=') {
                self.bump(1);
                self.skip_ws();
                self.read_attr_value()
            } else {
                an.clone()
            };
            attrs.push((an, av));
        }
        // autoclose p/li when another p/li starts
        if name == "p" || name == "li" || name == "tr" || name == "td" || name == "th" {
            while let Some(&top) = self.stack.last() {
                if self.doc.name(top) == name {
                    self.stack.pop();
                } else {
                    break;
                }
            }
        }
        // A block element also closes an open <p>: a paragraph cannot contain
        // one. Only same-name autoclose was implemented, so <p>two<div>three
        // nested the div INSIDE the paragraph where C makes them siblings --
        // which puts the text at the wrong depth for anything walking the tree
        // for structure.
        if is_block_element(&name) {
            while self.stack.last().is_some_and(|&t| self.doc.name(t) == "p") {
                self.stack.pop();
            }
        }
        let parent = if self.noimplied {
            self.stack.last().copied().unwrap_or(NodeId::DOCUMENT)
        } else if name == "html" {
            NodeId::DOCUMENT
        } else if name == "head" {
            self.ensure_html()
        } else if name == "body" || name == "frameset" {
            self.ensure_html()
        } else if matches!(name.as_str(), "title" | "meta" | "link" | "style" | "base") {
            self.ensure_head()
        } else if !self.stack.is_empty() {
            // The open element, not the body. This branch used to return
            // ensure_body() unconditionally, so NOTHING nested: fifty nested
            // <div> came out as fifty empty siblings, and structure-aware
            // consumers saw a flat document.
            self.parent()
        } else {
            self.ensure_body()
        };
        let elem = self.doc.xml_new_node(None, &name);
        for (k, v) in attrs {
            self.doc.xml_set_prop(elem, &k, &v);
        }
        if name == "html" {
            self.doc.xml_doc_set_root_element(elem);
            self.html = Some(elem);
        } else {
            self.doc.xml_add_child(parent, elem);
        }
        if name == "head" {
            self.head = Some(elem);
        }
        if name == "body" || name == "frameset" {
            self.body = Some(elem);
        }
        if !is_void(&name) {
            self.stack.push(elem);
        }
        Ok(())
    }
    fn parse_end_tag(&mut self) {
        self.bump(2);
        let name = self.read_name().to_ascii_lowercase();
        self.skip_ws();
        if self.rest().starts_with('>') {
            self.bump(1);
        }
        if let Some(idx) = self.stack.iter().rposition(|&id| self.doc.name(id) == name) {
            self.stack.truncate(idx);
        }
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.rest().chars().next() {
            if c.is_whitespace() {
                self.bump(c.len_utf8());
            } else {
                break;
            }
        }
    }
    fn read_name(&mut self) -> String {
        let r = self.rest();
        let mut n = 0;
        for (i, c) in r.char_indices() {
            if i == 0 {
                if !(c.is_ascii_alphabetic() || c == '_' || c == ':') {
                    return String::new();
                }
            } else if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.') {
                n = i;
                break;
            }
            n = i + c.len_utf8();
        }
        let s = r[..n].to_string();
        self.bump(n);
        s
    }
    fn read_attr_value(&mut self) -> String {
        let r = self.rest();
        if r.starts_with('"') || r.starts_with('\'') {
            let q = r.as_bytes()[0] as char;
            self.bump(1);
            if let Some(end) = self.rest().find(q) {
                let v = crate::html_entities::decode_html_text(&self.rest()[..end]).into_owned();
                self.bump(end + 1);
                return v;
            }
        }
        let mut n = 0;
        for (i, c) in self.rest().char_indices() {
            if c.is_whitespace() || c == '>' {
                n = i;
                break;
            }
            n = i + c.len_utf8();
        }
        let v = crate::html_entities::decode_html_text(&self.rest()[..n]).into_owned();
        self.bump(n);
        v
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
    if rest.get(..6).is_some_and(|k| k.eq_ignore_ascii_case("PUBLIC")) {
        rest = &rest[6..];
        dtd.public_id = literal(&mut rest);
        dtd.system_id = literal(&mut rest);
    } else if rest.get(..6).is_some_and(|k| k.eq_ignore_ascii_case("SYSTEM")) {
        rest = &rest[6..];
        dtd.system_id = literal(&mut rest);
    }
    dtd
}

/// Block-level elements, which cannot appear inside a paragraph and therefore
/// close an open one. This is libxml2's `htmlStartClose` table for `p`.
fn is_block_element(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "center"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "ul"
    )
}
