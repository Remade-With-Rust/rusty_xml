//! xmlsave + xmlTextWriter matching libxml2 `xmlsave.h` / `xmlwriter.h` for M2.

#![forbid(unsafe_code)]

use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};

/// libxml2 `xmlSaveOption` bits.
pub const XML_SAVE_FORMAT: i32 = 1 << 0;
pub const XML_SAVE_NO_DECL: i32 = 1 << 1;
pub const XML_SAVE_NO_EMPTY: i32 = 1 << 2;
pub const XML_SAVE_NO_XHTML: i32 = 1 << 3;
pub const XML_SAVE_XHTML: i32 = 1 << 4;
pub const XML_SAVE_AS_XML: i32 = 1 << 5;
pub const XML_SAVE_AS_HTML: i32 = 1 << 6;
pub const XML_SAVE_WSNONSIG: i32 = 1 << 7;
pub const XML_SAVE_EMPTY: i32 = 1 << 8;
pub const XML_SAVE_NO_INDENT: i32 = 1 << 9;
pub const XML_SAVE_INDENT: i32 = 1 << 10;

fn hex_ref(c: u32) -> String {
    format!("&#x{c:X};")
}

fn escape_text(s: &str, attr: bool, non_ascii: bool) -> String {
    let mut out = String::new();
    for c in s.chars() {
        let u = c as u32;
        if non_ascii && u >= 0x80 {
            out.push_str(&hex_ref(u));
            continue;
        }
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            '\r' => out.push_str("&#13;"),
            '\t' if attr => out.push_str("&#9;"),
            '\n' if attr => out.push_str("&#10;"),
            c if u < 0x20 && c != '\t' && c != '\n' => out.push_str(&hex_ref(0xfffd)),
            c => out.push(c),
        }
    }
    out
}

fn qname(prefix: Option<&str>, local: &str) -> String {
    match prefix {
        Some(p) if !p.is_empty() => format!("{p}:{local}"),
        _ => local.to_string(),
    }
}

fn indent_unit() -> String {
    std::env::var("XMLLINT_INDENT").unwrap_or_else(|_| "  ".into())
}

fn write_indent(out: &mut String, level: i32) {
    let unit = indent_unit();
    for _ in 0..level {
        out.push_str(&unit);
    }
}

fn write_node(doc: &XmlDoc, id: NodeId, out: &mut String, opts: i32, level: i32, format: bool) {
    match doc.kind(id) {
        NodeKind::Element => {
            out.push('<');
            out.push_str(&qname(doc.prefix(id), doc.name(id)));
            for (pre, href) in doc.ns_defs(id) {
                out.push_str(" xmlns");
                if let Some(p) = pre {
                    out.push(':');
                    out.push_str(p);
                }
                let non_ascii = doc.encoding.is_none();
                out.push_str("=\"");
                out.push_str(&escape_text(href, true, non_ascii));
                out.push('"');
            }
            for a in doc.attrs(id) {
                out.push(' ');
                out.push_str(&qname(doc.prefix(a), doc.name(a)));
                out.push_str("=\"");
                out.push_str(&escape_text(
                    doc.content(a),
                    true,
                    doc.encoding.is_none(),
                ));
                out.push('"');
            }
            let has_kids = doc.first_child(id).is_some();
            if !has_kids {
                if (opts & XML_SAVE_NO_EMPTY) == 0 {
                    out.push_str("/>");
                } else {
                    out.push_str("></");
                    out.push_str(&qname(doc.prefix(id), doc.name(id)));
                    out.push('>');
                }
                return;
            }
            let mixed = {
                let mut c = doc.first_child(id);
                let mut m = false;
                while let Some(ch) = c {
                    if matches!(doc.kind(ch), NodeKind::Text | NodeKind::CData) {
                        m = true;
                        break;
                    }
                    c = doc.next_sibling(ch);
                }
                m
            };
            out.push('>');
            let child_format = format && !mixed;
            if child_format {
                out.push('\n');
            }
            let mut c = doc.first_child(id);
            while let Some(ch) = c {
                if child_format {
                    write_indent(out, level + 1);
                }
                write_node(doc, ch, out, opts, level + 1, child_format);
                if child_format {
                    out.push('\n');
                }
                c = doc.next_sibling(ch);
            }
            if child_format {
                write_indent(out, level);
            }
            out.push_str("</");
            out.push_str(&qname(doc.prefix(id), doc.name(id)));
            out.push('>');
        }
        NodeKind::Text => {
            out.push_str(&escape_text(
                doc.content(id),
                false,
                doc.encoding.is_none(),
            ));
        }
        NodeKind::CData => {
            let content = doc.content(id);
            if content.is_empty() {
                out.push_str("<![CDATA[]]>");
            } else {
                // Split on ]]> like C.
                let bytes = content.as_bytes();
                let mut start = 0usize;
                let mut i = 0usize;
                while i + 2 < bytes.len() {
                    if bytes[i] == b']' && bytes[i + 1] == b']' && bytes[i + 2] == b'>' {
                        out.push_str("<![CDATA[");
                        out.push_str(&content[start..=i + 1]);
                        out.push_str("]]>");
                        start = i + 2;
                        i += 2;
                    }
                    i += 1;
                }
                if start < content.len() {
                    out.push_str("<![CDATA[");
                    out.push_str(&content[start..]);
                    out.push_str("]]>");
                }
            }
        }
        NodeKind::Comment => {
            let _ = (format, level);
            out.push_str("<!--");
            out.push_str(doc.content(id));
            out.push_str("-->");
        }
        NodeKind::Pi => {
            let _ = (format, level);
            out.push_str("<?");
            out.push_str(doc.name(id));
            if !doc.content(id).is_empty() {
                out.push(' ');
                out.push_str(doc.content(id));
            }
            out.push_str("?>");
        }
        _ => {}
    }
}

/// `xmlSaveDoc` / `xmlDocDumpMemory` with `xmlSaveOption` bits.
#[doc(alias = "xmlSaveDoc")]
pub fn xml_save_doc(doc: &XmlDoc, options: i32) -> Vec<u8> {
    let mut out = String::new();
    if (options & XML_SAVE_NO_DECL) == 0 {
        out.push_str("<?xml version=\"");
        out.push_str(if doc.version.is_empty() {
            "1.0"
        } else {
            &doc.version
        });
        out.push('"');
        if let Some(enc) = &doc.encoding {
            out.push_str(" encoding=\"");
            out.push_str(enc);
            out.push('"');
        }
        match doc.standalone {
            Some(true) => out.push_str(" standalone=\"yes\""),
            Some(false) => out.push_str(" standalone=\"no\""),
            None => {}
        }
        out.push_str("?>\n");
    }
    let format = (options & XML_SAVE_FORMAT) != 0;
    let mut child = doc.first_child(rusty_xml_tree::NodeId::DOCUMENT);
    while let Some(id) = child {
        write_node(doc, id, &mut out, options, 0, format);
        out.push('\n');
        child = doc.next_sibling(id);
    }
    out.into_bytes()
}

/// `xmlDocDumpFormatMemory`.
#[doc(alias = "xmlDocDumpFormatMemory")]
pub fn xml_doc_dump_format_memory(doc: &XmlDoc, format: bool) -> Vec<u8> {
    xml_save_doc(doc, if format { XML_SAVE_FORMAT } else { 0 })
}

/// `xmlDocDumpMemory`.
#[doc(alias = "xmlDocDumpMemory")]
pub fn xml_doc_dump_memory(doc: &XmlDoc) -> Vec<u8> {
    xml_save_doc(doc, 0)
}

/// `xmlNodeDump` of a subtree (no XML declaration).
#[doc(alias = "xmlNodeDump")]
pub fn xml_node_dump(doc: &XmlDoc, node: NodeId, options: i32) -> Vec<u8> {
    let mut out = String::new();
    write_node(doc, node, &mut out, options, 0, (options & XML_SAVE_FORMAT) != 0);
    out.into_bytes()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    Document,
    Element,
}

struct Frame {
    kind: FrameKind,
    name: String,
    prefix: Option<String>,
    open: bool,
    has_content: bool,
}

/// `xmlTextWriter` writing into an in-memory buffer.
pub struct XmlTextWriter {
    buf: String,
    stack: Vec<Frame>,
    indent: bool,
    indent_unit: String,
    started: bool,
}

impl Default for XmlTextWriter {
    fn default() -> Self {
        Self::xml_new_text_writer_memory()
    }
}

impl XmlTextWriter {
    /// `xmlNewTextWriterMemory`.
    #[doc(alias = "xmlNewTextWriterMemory")]
    pub fn xml_new_text_writer_memory() -> Self {
        Self {
            buf: String::new(),
            stack: Vec::new(),
            indent: false,
            indent_unit: indent_unit(),
            started: false,
        }
    }

    pub fn set_indent(&mut self, indent: bool) {
        self.indent = indent;
    }

    fn close_start_tag(&mut self) {
        if let Some(f) = self.stack.last_mut() {
            if f.kind == FrameKind::Element && !f.open {
                self.buf.push('>');
                f.open = true;
                f.has_content = true;
            }
        }
    }

    /// `xmlTextWriterStartDocument`.
    #[doc(alias = "xmlTextWriterStartDocument")]
    pub fn start_document(
        &mut self,
        version: Option<&str>,
        encoding: Option<&str>,
        standalone: Option<&str>,
    ) -> Result<(), String> {
        self.buf.push_str("<?xml version=\"");
        self.buf.push_str(version.unwrap_or("1.0"));
        self.buf.push('"');
        if let Some(e) = encoding {
            self.buf.push_str(" encoding=\"");
            self.buf.push_str(e);
            self.buf.push('"');
        }
        if let Some(s) = standalone {
            self.buf.push_str(" standalone=\"");
            self.buf.push_str(s);
            self.buf.push('"');
        }
        self.buf.push_str("?>\n");
        self.stack.push(Frame {
            kind: FrameKind::Document,
            name: "#document".into(),
            prefix: None,
            open: true,
            has_content: true,
        });
        self.started = true;
        Ok(())
    }

    /// `xmlTextWriterStartElement`.
    #[doc(alias = "xmlTextWriterStartElement")]
    pub fn start_element(&mut self, name: &str) -> Result<(), String> {
        self.start_element_ns(None, name, None)
    }

    /// `xmlTextWriterStartElementNS`.
    #[doc(alias = "xmlTextWriterStartElementNS")]
    pub fn start_element_ns(
        &mut self,
        prefix: Option<&str>,
        name: &str,
        ns_uri: Option<&str>,
    ) -> Result<(), String> {
        self.close_start_tag();
        if self.indent && !self.buf.is_empty() && !self.buf.ends_with('\n') {
            self.buf.push('\n');
        }
        if self.indent {
            let depth = self.stack.iter().filter(|f| f.kind == FrameKind::Element).count();
            for _ in 0..depth {
                self.buf.push_str(&self.indent_unit);
            }
        }
        self.buf.push('<');
        self.buf.push_str(&qname(prefix, name));
        if let Some(uri) = ns_uri {
            if let Some(p) = prefix {
                self.buf.push_str(" xmlns:");
                self.buf.push_str(p);
            } else {
                self.buf.push_str(" xmlns");
            }
            self.buf.push_str("=\"");
            self.buf.push_str(&escape_text(uri, true, false));
            self.buf.push('"');
        }
        self.stack.push(Frame {
            kind: FrameKind::Element,
            name: name.to_string(),
            prefix: prefix.map(str::to_string),
            open: false,
            has_content: false,
        });
        Ok(())
    }

    /// `xmlTextWriterWriteAttribute`.
    #[doc(alias = "xmlTextWriterWriteAttribute")]
    pub fn write_attribute(&mut self, name: &str, value: &str) -> Result<(), String> {
        let top = self.stack.last().ok_or_else(|| "no open element".to_string())?;
        if top.kind != FrameKind::Element || top.open {
            return Err("attribute after element content".into());
        }
        self.buf.push(' ');
        self.buf.push_str(name);
        self.buf.push_str("=\"");
        self.buf.push_str(&escape_text(value, true, false));
        self.buf.push('"');
        Ok(())
    }

    /// `xmlTextWriterWriteAttributeNS`.
    #[doc(alias = "xmlTextWriterWriteAttributeNS")]
    pub fn write_attribute_ns(
        &mut self,
        prefix: Option<&str>,
        name: &str,
        _ns_uri: Option<&str>,
        value: &str,
    ) -> Result<(), String> {
        self.write_attribute(&qname(prefix, name), value)
    }

    /// `xmlTextWriterWriteString`.
    #[doc(alias = "xmlTextWriterWriteString")]
    pub fn write_string(&mut self, content: &str) -> Result<(), String> {
        self.close_start_tag();
        self.buf.push_str(&escape_text(content, false, false));
        Ok(())
    }

    /// `xmlTextWriterWriteComment`.
    #[doc(alias = "xmlTextWriterWriteComment")]
    pub fn write_comment(&mut self, content: &str) -> Result<(), String> {
        self.close_start_tag();
        self.buf.push_str("<!--");
        self.buf.push_str(content);
        self.buf.push_str("-->");
        Ok(())
    }

    /// `xmlTextWriterWritePI`.
    #[doc(alias = "xmlTextWriterWritePI")]
    pub fn write_pi(&mut self, target: &str, data: Option<&str>) -> Result<(), String> {
        self.close_start_tag();
        self.buf.push_str("<?");
        self.buf.push_str(target);
        if let Some(d) = data {
            self.buf.push(' ');
            self.buf.push_str(d);
        }
        self.buf.push_str("?>");
        Ok(())
    }

    /// `xmlTextWriterWriteCDATA`.
    #[doc(alias = "xmlTextWriterWriteCDATA")]
    pub fn write_cdata(&mut self, content: &str) -> Result<(), String> {
        self.close_start_tag();
        self.buf.push_str("<![CDATA[");
        self.buf.push_str(content);
        self.buf.push_str("]]>");
        Ok(())
    }

    /// `xmlTextWriterWriteRaw`.
    #[doc(alias = "xmlTextWriterWriteRaw")]
    pub fn write_raw(&mut self, content: &str) -> Result<(), String> {
        self.close_start_tag();
        self.buf.push_str(content);
        Ok(())
    }

    /// `xmlTextWriterEndElement`.
    #[doc(alias = "xmlTextWriterEndElement")]
    pub fn end_element(&mut self) -> Result<(), String> {
        let f = self.stack.pop().ok_or_else(|| "no open element".to_string())?;
        if f.kind != FrameKind::Element {
            return Err("end_element on document".into());
        }
        if !f.open {
            self.buf.push_str("/>");
        } else {
            if self.indent && f.has_content {
                // keep compact unless we wrote nested elements with indent
            }
            self.buf.push_str("</");
            self.buf.push_str(&qname(f.prefix.as_deref(), &f.name));
            self.buf.push('>');
        }
        Ok(())
    }

    /// `xmlTextWriterEndDocument`.
    #[doc(alias = "xmlTextWriterEndDocument")]
    pub fn end_document(&mut self) -> Result<(), String> {
        while self
            .stack
            .last()
            .map(|f| f.kind == FrameKind::Element)
            .unwrap_or(false)
        {
            self.end_element()?;
        }
        if !self.buf.ends_with('\n') {
            self.buf.push('\n');
        }
        self.stack.clear();
        Ok(())
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf.into_bytes()
    }

    pub fn as_str(&self) -> &str {
        &self.buf
    }
}

/// `xmlNewTextWriterMemory`.
#[doc(alias = "xmlNewTextWriterMemory")]
pub fn xml_new_text_writer_memory() -> XmlTextWriter {
    XmlTextWriter::xml_new_text_writer_memory()
}
