//! DTD subset parser (internal + caller-supplied external). No network.

use rusty_xml_tree::{AttrDecl, AttrDefault, ElementDecl, XmlDtd};
use crate::error::XmlError;

/// `xmlParseDTD` — parse a DTD from memory (caller already loaded the bytes).
#[doc(alias = "xmlParseDTD")]
pub fn xml_parse_dtd(
    buffer: &[u8],
    public_id: Option<&str>,
    system_id: Option<&str>,
) -> Result<XmlDtd, XmlError> {
    let text = String::from_utf8_lossy(buffer);
    let mut dtd = parse_dtd_subset(&text)?;
    dtd.public_id = public_id.map(str::to_string);
    dtd.system_id = system_id.map(str::to_string);
    Ok(dtd)
}

/// Parse a DTD internal/external subset into declarations.
pub fn parse_dtd_subset(src: &str) -> Result<XmlDtd, XmlError> {
    let expanded = expand_pe(src);
    let mut dtd = XmlDtd::default();
    dtd.int_subset = Some(src.to_string());
    let mut p = DtdParser {
        src: expanded.as_str(),
        pos: 0,
        dtd: &mut dtd,
    };
    p.parse_markup()?;
    Ok(dtd)
}

fn expand_pe(src: &str) -> String {
    // Multi-pass PE expansion so `%percent;` can invent new PE names.
    let mut cur = src.to_string();
    for _ in 0..16 {
        let mut pes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        harvest_pe(&cur, &mut pes);
        let next = subst_pe(&cur, &pes);
        if next == cur {
            return cur;
        }
        cur = next;
    }
    cur
}

fn harvest_pe(src: &str, pes: &mut std::collections::HashMap<String, String>) {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + 8 < bytes.len() {
        if bytes[i] == b'<' && bytes.get(i..i + 9) == Some(b"<!ENTITY ") {
            i += 9;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'%' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'"' && bytes[i] != b'\'' {
                    i += 1;
                }
                let name = src[start..i].to_string();
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                    let q = bytes[i];
                    i += 1;
                    let vs = i;
                    while i < bytes.len() && bytes[i] != q {
                        i += 1;
                    }
                    let val = decode_charrefs(&src[vs..i]);
                    pes.insert(name, val);
                }
            }
        } else {
            i += 1;
        }
    }
}

fn subst_pe(src: &str, pes: &std::collections::HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut chars = src.chars().peekable();
    let mut in_comment = false;
    while let Some(c) = chars.next() {
        if in_comment {
            out.push(c);
            if c == '-' && chars.peek() == Some(&'-') {
                out.push(chars.next().unwrap());
                if chars.peek() == Some(&'>') {
                    out.push(chars.next().unwrap());
                    in_comment = false;
                }
            }
            continue;
        }
        if c == '<' && chars.peek() == Some(&'!') {
            out.push(c);
            out.push(chars.next().unwrap());
            if chars.peek() == Some(&'-') {
                out.push(chars.next().unwrap());
                if chars.peek() == Some(&'-') {
                    out.push(chars.next().unwrap());
                    in_comment = true;
                }
            }
            continue;
        }
        if c == '%' {
            let mut name = String::new();
            while let Some(&n) = chars.peek() {
                if n == ';' {
                    chars.next();
                    break;
                }
                if n.is_ascii_whitespace() || n == '"' || n == '\'' {
                    break;
                }
                name.push(n);
                chars.next();
            }
            if let Some(v) = pes.get(&name) {
                out.push_str(v);
            } else {
                out.push('%');
                out.push_str(&name);
                if !name.is_empty() {
                    out.push(';');
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn decode_charrefs(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("&#") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 2..];
        if let Some(hex) = after.strip_prefix('x').or_else(|| after.strip_prefix('X')) {
            if let Some(end) = hex.find(';') {
                if let Ok(v) = u32::from_str_radix(&hex[..end], 16) {
                    if let Some(ch) = char::from_u32(v) {
                        out.push(ch);
                        rest = &hex[end + 1..];
                        continue;
                    }
                }
            }
        } else if let Some(end) = after.find(';') {
            if let Ok(v) = after[..end].parse::<u32>() {
                if let Some(ch) = char::from_u32(v) {
                    out.push(ch);
                    rest = &after[end + 1..];
                    continue;
                }
            }
        }
        out.push_str("&#");
        rest = after;
    }
    out.push_str(rest);
    out
}

struct DtdParser<'a> {
    src: &'a str,
    pos: usize,
    dtd: &'a mut XmlDtd,
}

impl<'a> DtdParser<'a> {
    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }
    fn skip_ws_and_comments(&mut self) {
        loop {
            let r = self.rest();
            let trimmed = r.trim_start();
            let n = r.len() - trimmed.len();
            self.pos += n;
            if self.rest().starts_with("<!--") {
                if let Some(e) = self.rest().find("-->") {
                    self.pos += e + 3;
                    continue;
                }
            }
            if self.rest().starts_with("<?") {
                if let Some(e) = self.rest().find("?>") {
                    self.pos += e + 2;
                    continue;
                }
            }
            break;
        }
    }
    fn parse_markup(&mut self) -> Result<(), XmlError> {
        loop {
            self.skip_ws_and_comments();
            if self.pos >= self.src.len() {
                break;
            }
            if self.rest().starts_with("<!ELEMENT") {
                self.parse_element()?;
            } else if self.rest().starts_with("<!ATTLIST") {
                self.parse_attlist()?;
            } else if self.rest().starts_with("<!ENTITY") {
                self.parse_entity()?;
            } else if self.rest().starts_with("<!NOTATION") {
                self.skip_decl()?;
            } else if self.rest().starts_with("<![") {
                self.skip_cond()?;
            } else if self.rest().starts_with('<') {
                self.skip_decl()?;
            } else {
                self.pos += self.rest().chars().next().unwrap().len_utf8();
            }
        }
        Ok(())
    }
    fn skip_decl(&mut self) -> Result<(), XmlError> {
        if let Some(i) = self.rest().find('>') {
            self.pos += i + 1;
            Ok(())
        } else {
            self.pos = self.src.len();
            Ok(())
        }
    }
    fn skip_cond(&mut self) -> Result<(), XmlError> {
        let mut depth = 0i32;
        let bytes = self.rest().as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'<' && bytes.get(i..i + 3) == Some(b"<![") {
                depth += 1;
                i += 3;
                continue;
            }
            if bytes[i] == b']' && bytes.get(i..i + 3) == Some(b"]]>") {
                depth -= 1;
                i += 3;
                if depth == 0 {
                    self.pos += i;
                    return Ok(());
                }
                continue;
            }
            i += 1;
        }
        self.pos = self.src.len();
        Ok(())
    }
    fn bump(&mut self, n: usize) {
        self.pos += n;
    }
    fn parse_name(&mut self) -> String {
        self.skip_ws_and_comments();
        let r = self.rest();
        let mut n = 0;
        for (i, c) in r.char_indices() {
            if i == 0 {
                if !(c.is_ascii_alphabetic() || c == '_' || c == ':') {
                    break;
                }
            } else if !(c.is_ascii_alphanumeric() || "-._:".contains(c)) {
                n = i;
                break;
            }
            n = i + c.len_utf8();
        }
        let s = r[..n].to_string();
        self.bump(n);
        s
    }
    fn parse_quoted(&mut self) -> String {
        self.skip_ws_and_comments();
        let r = self.rest();
        if r.starts_with('"') || r.starts_with('\'') {
            let q = r.as_bytes()[0] as char;
            self.bump(1);
            if let Some(e) = self.rest().find(q) {
                let s = decode_charrefs(&self.rest()[..e]);
                self.bump(e + 1);
                return s;
            }
        }
        String::new()
    }
    fn parse_element(&mut self) -> Result<(), XmlError> {
        self.bump("<!ELEMENT".len());
        let name = self.parse_name();
        self.skip_ws_and_comments();
        let decl = if self.rest().starts_with("EMPTY") {
            self.bump(5);
            ElementDecl::Empty
        } else if self.rest().starts_with("ANY") {
            self.bump(3);
            ElementDecl::Any
        } else if self.rest().starts_with('(') {
            let spec = self.take_until_gt_paren();
            if spec.contains("#PCDATA") {
                let mut names = Vec::new();
                for part in spec.split('|') {
                    let t = part.trim().trim_matches(|c: char| c == '(' || c == ')' || c == '*');
                    if t != "#PCDATA" && !t.is_empty() {
                        names.push(t.to_string());
                    }
                }
                ElementDecl::Mixed(names)
            } else {
                ElementDecl::Children(spec)
            }
        } else {
            self.skip_decl()?;
            return Ok(());
        };
        self.dtd.elements.insert(name, decl);
        self.skip_ws_and_comments();
        if self.rest().starts_with('>') {
            self.bump(1);
        } else {
            self.skip_decl()?;
        }
        Ok(())
    }
    fn take_until_gt_paren(&mut self) -> String {
        let r = self.rest();
        let mut depth = 0i32;
        let mut i = 0;
        for (off, c) in r.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        i = off + 1;
                        break;
                    }
                }
                '>' if depth == 0 => {
                    i = off;
                    break;
                }
                _ => {}
            }
            i = off + c.len_utf8();
        }
        let s = r[..i].to_string();
        self.bump(i);
        s
    }
    fn parse_attlist(&mut self) -> Result<(), XmlError> {
        self.bump("<!ATTLIST".len());
        let elem = self.parse_name();
        loop {
            self.skip_ws_and_comments();
            if self.rest().starts_with('>') {
                self.bump(1);
                break;
            }
            if self.pos >= self.src.len() {
                break;
            }
            let aname = self.parse_name();
            if aname.is_empty() {
                self.skip_decl()?;
                break;
            }
            self.skip_ws_and_comments();
            let mut enumerated = Vec::new();
            let att_type = if self.rest().starts_with('(') {
                let spec = self.take_until_gt_paren();
                for part in spec.split('|') {
                    let t = part.trim().trim_matches(|c: char| "()".contains(c));
                    if !t.is_empty() {
                        enumerated.push(t.to_string());
                    }
                }
                "ENUMERATION".into()
            } else {
                self.parse_name()
            };
            self.skip_ws_and_comments();
            let (default, default_value) = if self.rest().starts_with("#REQUIRED") {
                self.bump(9);
                (AttrDefault::Required, None)
            } else if self.rest().starts_with("#IMPLIED") {
                self.bump(8);
                (AttrDefault::Implied, None)
            } else if self.rest().starts_with("#FIXED") {
                self.bump(6);
                (AttrDefault::Fixed, Some(self.parse_quoted()))
            } else {
                (AttrDefault::Value, Some(self.parse_quoted()))
            };
            self.dtd.attributes.insert(
                (elem.clone(), aname),
                AttrDecl {
                    att_type,
                    default,
                    default_value,
                    enumerated,
                },
            );
        }
        Ok(())
    }
    fn parse_entity(&mut self) -> Result<(), XmlError> {
        self.bump("<!ENTITY".len());
        self.skip_ws_and_comments();
        let pe = self.rest().starts_with('%');
        if pe {
            self.bump(1);
            self.skip_ws_and_comments();
        }
        let name = self.parse_name();
        self.skip_ws_and_comments();
        if self.rest().starts_with("SYSTEM") || self.rest().starts_with("PUBLIC") {
            self.skip_decl()?;
            return Ok(());
        }
        let val = self.parse_quoted();
        if pe {
            self.dtd.parameter_entities.insert(name, val);
        } else {
            self.dtd.entities.insert(name, val);
        }
        self.skip_ws_and_comments();
        if self.rest().starts_with('>') {
            self.bump(1);
        } else {
            self.skip_decl()?;
        }
        Ok(())
    }
}

/// Merge `src` into `dst` (external subset onto internal).
pub fn merge_dtd(dst: &mut XmlDtd, src: XmlDtd) {
    dst.entities.extend(src.entities);
    dst.parameter_entities.extend(src.parameter_entities);
    dst.elements.extend(src.elements);
    dst.attributes.extend(src.attributes);
    if dst.public_id.is_none() {
        dst.public_id = src.public_id;
    }
    if dst.system_id.is_none() {
        dst.system_id = src.system_id;
    }
}
