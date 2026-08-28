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
    /// Whitespace only. Where the grammar says S it means S, not "whatever
    /// happens to be in the way" -- skip_ws_and_comments swallows the SGML
    /// `-- comment --` form and PIs, which is exactly how a malformed
    /// declaration slipped past.
    fn skip_ws(&mut self) {
        let r = self.rest();
        let trimmed = r.trim_start_matches([' ', '\t', '\r', '\n']);
        self.pos += r.len() - trimmed.len();
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
    /// Read a quoted literal from the internal subset.
    ///
    /// This returned a bare String and so could not report anything. An
    /// ATTLIST default or entity value holding a C0 control byte was
    /// therefore accepted, copied into every element that took the default,
    /// and written back out as U+FFFD -- a value the document never
    /// contained. C stops at the declaration with "invalid character in
    /// entity value". Found by the fuzz round-trip check, which saw the
    /// first save escape the character and the second not.
    fn parse_quoted(&mut self) -> Result<String, XmlError> {
        self.skip_ws_and_comments();
        let r = self.rest();
        if r.starts_with('"') || r.starts_with('\'') {
            let q = r.as_bytes()[0] as char;
            self.bump(1);
            if let Some(e) = self.rest().find(q) {
                let s = decode_charrefs(&self.rest()[..e]);
                self.bump(e + 1);
                if let Some(bad) =
                    s.chars().find(|c| !crate::chvalid::xml_is_char(*c as u32))
                {
                    return Err(XmlError::new(
                        crate::error::XML_ERR_INVALID_CHAR,
                        format!("invalid character 0x{:X} in entity value", bad as u32),
                        0,
                        0,
                    ));
                }
                return Ok(s);
            }
        }
        Ok(String::new())
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
                // Enumeration ::= '(' S? Nmtoken (S? '|' S? Nmtoken)* S? ')'
                // Only '|' separates. `(foo,bar)` used to be accepted because
                // this split on '|' and shrugged at whatever else was inside.
                let body = spec.trim();
                if !body.starts_with('(') || !body.ends_with(')') {
                    return Err(self.err("')' required to finish ATTLIST enumeration"));
                }
                for part in body[1..body.len() - 1].split('|') {
                    let t = part.trim();
                    if t.is_empty() || !t.chars().all(|c| crate::chvalid::xml_is_name_char(c as u32, false)) {
                        return Err(self.err("')' required to finish ATTLIST enumeration"));
                    }
                    enumerated.push(t.to_string());
                }
                "ENUMERATION".into()
            } else {
                let t = self.parse_name();
                // AttType is a closed set. `NAME` is not in it, and was taken
                // as a perfectly good type.
                const TYPES: &[&str] = &[
                    "CDATA", "ID", "IDREF", "IDREFS", "ENTITY", "ENTITIES", "NMTOKEN",
                    "NMTOKENS", "NOTATION",
                ];
                if !TYPES.contains(&t.as_str()) {
                    return Err(self.err("'(' required to start ATTLIST enumeration"));
                }
                if t == "NOTATION" {
                    self.skip_ws();
                    if !self.rest().starts_with('(') {
                        return Err(self.err("'(' required to start ATTLIST enumeration"));
                    }
                    let spec = self.take_until_gt_paren();
                    for part in spec.trim().trim_matches(['(', ')']).split('|') {
                        let n = part.trim();
                        if !n.is_empty() {
                            enumerated.push(n.to_string());
                        }
                    }
                }
                t
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
                if !self.require_ws() {
                    return Err(self.err("Space required after '#FIXED'"));
                }
                if !self.at_quote() {
                    return Err(self.err("AttValue: \" or ' expected"));
                }
                (AttrDefault::Fixed, Some(self.parse_quoted()?))
            } else {
                // A default value is an AttValue, which is quoted. `v1` bare
                // was accepted and silently became an empty string.
                if !self.at_quote() {
                    return Err(self.err("AttValue: \" or ' expected"));
                }
                (AttrDefault::Value, Some(self.parse_quoted()?))
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
        if !self.require_ws() {
            return Err(self.err("Space required after '<!ENTITY'"));
        }
        let pe = self.rest().starts_with('%');
        if pe {
            self.bump(1);
            if !self.require_ws() {
                return Err(self.err("Space required after '%'"));
            }
        }
        let name = self.parse_name();
        if name.is_empty() {
            return Err(self.err("Entity name expected"));
        }
        // EntityDecl requires S between the name and the definition. Without
        // this, `<!ENTITY foo"some text">` was accepted.
        if !self.require_ws() {
            return Err(self.err("Space required after the entity name"));
        }
        if self.rest().starts_with("SYSTEM") || self.rest().starts_with("PUBLIC") {
            let public = self.rest().starts_with("PUBLIC");
            self.bump(6);
            if !self.require_ws() {
                return Err(self.err("Space required after the external ID keyword"));
            }
            if public {
                // ExternalID ::= 'PUBLIC' S PubidLiteral S SystemLiteral --
                // two literals, with space between them. One was accepted, and
                // so was `"whatever""e.ent"` with no space.
                self.parse_quoted()?;
                if !self.require_ws() {
                    return Err(self.err("Space required after the Public Identifier"));
                }
                if !self.at_quote() {
                    return Err(self.err("SystemLiteral expected"));
                }
            }
            self.parse_quoted()?;
            self.skip_ws_and_comments();
            // NDataDecl is the only thing allowed to follow.
            if self.rest().starts_with("NDATA") {
                self.bump(5);
                if !self.require_ws() {
                    return Err(self.err("Space required after 'NDATA'"));
                }
                if self.parse_name().is_empty() {
                    return Err(self.err("Notation name expected after 'NDATA'"));
                }
                self.skip_ws();
            }
            return self.expect_decl_end("entity");
        }
        if !self.at_quote() {
            return Err(self.err("Entity value expected"));
        }
        let val = self.parse_quoted()?;
        if pe {
            self.dtd.parameter_entities.insert(name, val);
        } else {
            self.dtd.entities.insert(name, val);
        }
        self.skip_ws();
        self.expect_decl_end("entity")
    }

    /// Position of the parser as a line and column, so an error points at the
    /// declaration rather than at 0:0.
    fn line_col(&self) -> (u32, u32) {
        let mut line = 1u32;
        let mut col = 1u32;
        for c in self.src[..self.pos.min(self.src.len())].chars() {
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn err(&self, msg: &str) -> XmlError {
        let (line, col) = self.line_col();
        XmlError::new(crate::error::XML_ERR_SPACE_REQUIRED, msg, line, col)
    }

    /// Consume required whitespace, reporting whether any was there.
    fn require_ws(&mut self) -> bool {
        let before = self.pos;
        self.skip_ws();
        self.pos > before || self.pos >= self.src.len()
    }

    fn at_quote(&self) -> bool {
        self.rest().starts_with('"') || self.rest().starts_with('\'')
    }

    /// A declaration ends at '>' and nothing else. It used to fall through to
    /// skip_decl(), which swallowed whatever was in the way -- including the
    /// SGML `-- comment --` form that XML does not have.
    fn expect_decl_end(&mut self, what: &str) -> Result<(), XmlError> {
        self.skip_ws();
        if self.rest().starts_with('>') {
            self.bump(1);
            Ok(())
        } else {
            Err(self.err(&format!("xmlParse{what}Decl: not terminated")))
        }
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
