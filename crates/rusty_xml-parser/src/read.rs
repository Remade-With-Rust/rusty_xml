//! UTF-8 well-formed reader that builds a [`rusty_xml_tree::XmlDoc`].
//! DTD / external entities are refused (NO_XXE).

use crate::error::{XmlError, XML_ERR_DOCUMENT_EMPTY, XML_ERR_LT_REQUIRED, XML_ERR_TAG_NAME_MISMATCH};
use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};

/// Refuse network I/O during parse (no-op here: we never fetch).
pub const XML_PARSE_NONET: u32 = 1 << 11;
/// Refuse DTD / entity expansion (always on in this reader).
pub const XML_PARSE_NO_XXE: u32 = 1 << 19;

pub fn xml_read_memory(
    bytes: &[u8],
    _url: Option<&str>,
    _encoding: Option<&str>,
    _options: u32,
) -> Result<XmlDoc, XmlError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        XmlError::new(crate::error::XML_ERR_INVALID_CHAR, "input is not UTF-8", 1, 1)
    })?;
    let mut p = Reader {
        src: text,
        pos: 0,
        line: 1,
        col: 1,
        doc: XmlDoc::xml_new_doc(Some("1.0")),
    };
    p.skip_bom();
    p.skip_misc()?;
    if p.eof() {
        return Err(p.err(XML_ERR_DOCUMENT_EMPTY, "empty document"));
    }
    let root = p.parse_element()?;
    p.doc.xml_doc_set_root_element(root);
    p.skip_misc()?;
    Ok(p.doc)
}

struct Reader<'a> {
    src: &'a str,
    pos: usize,
    line: u32,
    col: u32,
    doc: XmlDoc,
}

impl<'a> Reader<'a> {
    fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn err(&self, code: i32, msg: &str) -> XmlError {
        XmlError::new(code, msg, self.line, self.col)
    }

    fn bump(&mut self, n: usize) {
        for c in self.src[self.pos..self.pos + n].chars() {
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        self.pos += n;
    }

    fn skip_bom(&mut self) {
        if self.rest().starts_with('\u{feff}') {
            self.bump('\u{feff}'.len_utf8());
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

    fn skip_misc(&mut self) -> Result<(), XmlError> {
        loop {
            self.skip_ws();
            if self.rest().starts_with("<?") {
                self.skip_pi()?;
            } else if self.rest().starts_with("<!--") {
                self.skip_comment()?;
            } else if self.rest().starts_with("<!DOCTYPE") {
                return Err(self.err(
                    crate::error::XML_ERR_INVALID_CHAR,
                    "DTD refused (XML_PARSE_NO_XXE)",
                ));
            } else {
                break;
            }
        }
        Ok(())
    }

    fn skip_pi(&mut self) -> Result<(), XmlError> {
        let Some(end) = self.rest().find("?>") else {
            return Err(self.err(crate::error::XML_ERR_PI_NOT_FINISHED, "unterminated PI"));
        };
        self.bump(end + 2);
        Ok(())
    }

    fn skip_comment(&mut self) -> Result<(), XmlError> {
        let Some(end) = self.rest().find("-->") else {
            return Err(self.err(
                crate::error::XML_ERR_COMMENT_NOT_FINISHED,
                "unterminated comment",
            ));
        };
        self.bump(end + 3);
        Ok(())
    }

    fn parse_element(&mut self) -> Result<NodeId, XmlError> {
        if !self.rest().starts_with('<') {
            return Err(self.err(XML_ERR_LT_REQUIRED, "expected '<'"));
        }
        self.bump(1);
        if self.rest().starts_with('/') || self.rest().starts_with('!') || self.rest().starts_with('?')
        {
            return Err(self.err(XML_ERR_LT_REQUIRED, "expected start tag"));
        }
        let qname = self.parse_name()?;
        let (prefix, local) = split_qname(&qname);
        let elem = self.doc.xml_new_node(None, local);
        if let Some(p) = prefix {
            self.doc.node_mut(elem).prefix = Some(p.to_string());
        }
        loop {
            self.skip_ws();
            if self.rest().starts_with("/>") {
                self.bump(2);
                return Ok(elem);
            }
            if self.rest().starts_with('>') {
                self.bump(1);
                break;
            }
            let aname = self.parse_name()?;
            self.skip_ws();
            if !self.rest().starts_with('=') {
                return Err(self.err(crate::error::XML_ERR_EQUAL_REQUIRED, "expected '='"));
            }
            self.bump(1);
            self.skip_ws();
            let aval = self.parse_quoted()?;
            let (aprefix, alocal) = split_qname(&aname);
            if aname == "xmlns" {
                self.doc.push_ns_def(elem, None, aval);
            } else if let Some(rest) = aname.strip_prefix("xmlns:") {
                self.doc.push_ns_def(elem, Some(rest.to_string()), aval);
            } else {
                let pref = aprefix.map(str::to_string);
                self.doc
                    .add_attr(elem, alocal, pref.as_deref(), &aval);
            }
        }
        loop {
            if self.rest().starts_with("</") {
                self.bump(2);
                let end_name = self.parse_name()?;
                self.skip_ws();
                if !self.rest().starts_with('>') {
                    return Err(self.err(
                        crate::error::XML_ERR_GT_REQUIRED,
                        "expected '>' on end tag",
                    ));
                }
                self.bump(1);
                if end_name != qname {
                    return Err(self.err(XML_ERR_TAG_NAME_MISMATCH, "end tag mismatch"));
                }
                return Ok(elem);
            }
            if self.rest().starts_with("<!--") {
                self.skip_comment()?;
                continue;
            }
            if self.rest().starts_with("<![CDATA[") {
                self.parse_cdata(elem)?;
                continue;
            }
            if self.rest().starts_with('<') {
                let child = self.parse_element()?;
                self.doc.xml_add_child(elem, child);
                continue;
            }
            if self.eof() {
                return Err(self.err(
                    crate::error::XML_ERR_TAG_NOT_FINISHED,
                    "unterminated element",
                ));
            }
            self.parse_text(elem)?;
        }
    }

    fn parse_name(&mut self) -> Result<String, XmlError> {
        let rest = self.rest();
        let mut n = 0;
        for (i, c) in rest.char_indices() {
            if i == 0 {
                if !(c.is_ascii_alphabetic() || c == '_' || c == ':') {
                    return Err(self.err(crate::error::XML_ERR_NAME_REQUIRED, "expected name"));
                }
                n = c.len_utf8();
            } else if c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '-' | '.') {
                n = i + c.len_utf8();
            } else {
                break;
            }
        }
        if n == 0 {
            return Err(self.err(crate::error::XML_ERR_NAME_REQUIRED, "expected name"));
        }
        let name = rest[..n].to_string();
        self.bump(n);
        Ok(name)
    }

    fn parse_quoted(&mut self) -> Result<String, XmlError> {
        let quote = self.rest().chars().next();
        if quote != Some('"') && quote != Some('\'') {
            return Err(self.err(
                crate::error::XML_ERR_ATTRIBUTE_WITHOUT_VALUE,
                "expected quoted attribute",
            ));
        }
        let q = quote.unwrap();
        self.bump(1);
        let rest = self.rest();
        let Some(end) = rest.find(q) else {
            return Err(self.err(
                crate::error::XML_ERR_LITERAL_NOT_FINISHED,
                "unterminated attribute",
            ));
        };
        let raw = rest[..end].to_string();
        self.bump(end + 1);
        unescape(&raw)
            .map_err(|m| self.err(crate::error::XML_ERR_INVALID_CHAR, m))
    }

    fn parse_text(&mut self, parent: NodeId) -> Result<(), XmlError> {
        let rest = self.rest();
        let end = rest.find('<').unwrap_or(rest.len());
        if end == 0 {
            return Ok(());
        }
        let raw = rest[..end].to_string();
        self.bump(end);
        let text = unescape(&raw)
            .map_err(|m| self.err(crate::error::XML_ERR_INVALID_CHAR, m))?;
        if text.is_empty() {
            return Ok(());
        }
        let t = self.doc.alloc(NodeKind::Text, "#text");
        self.doc.node_mut(t).content = text;
        self.doc.xml_add_child(parent, t);
        Ok(())
    }

    fn parse_cdata(&mut self, parent: NodeId) -> Result<(), XmlError> {
        const OPEN: &str = "<![CDATA[";
        self.bump(OPEN.len());
        let Some(end) = self.rest().find("]]>") else {
            return Err(self.err(
                crate::error::XML_ERR_CDATA_NOT_FINISHED,
                "unterminated CDATA",
            ));
        };
        let text = self.rest()[..end].to_string();
        self.bump(end + 3);
        let t = self.doc.alloc(NodeKind::CData, "#cdata-section");
        self.doc.node_mut(t).content = text;
        self.doc.xml_add_child(parent, t);
        Ok(())
    }
}

fn split_qname(name: &str) -> (Option<&str>, &str) {
    match name.split_once(':') {
        Some((p, l)) if !p.is_empty() && !l.is_empty() => (Some(p), l),
        _ => (None, name),
    }
}

fn unescape(s: &str) -> Result<String, &'static str> {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i + 1..];
        let Some(end) = rest.find(';') else {
            return Err("unterminated entity");
        };
        let ent = &rest[..end];
        rest = &rest[end + 1..];
        match ent {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "apos" => out.push('\''),
            "quot" => out.push('"'),
            other if other.starts_with('#') => {
                let n = if let Some(hex) = other.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).map_err(|_| "bad charref")?
                } else {
                    other[1..].parse::<u32>().map_err(|_| "bad charref")?
                };
                out.push(char::from_u32(n).ok_or("bad charref")?);
            }
            _ => return Err("undeclared entity refused (NO_XXE)"),
        }
    }
    out.push_str(rest);
    Ok(out)
}
