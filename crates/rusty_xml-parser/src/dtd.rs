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
    let mut dtd = parse_external_subset(&text, false)?;
    dtd.public_id = public_id.map(str::to_string);
    dtd.system_id = system_id.map(str::to_string);
    Ok(dtd)
}

/// Parse a DTD internal/external subset into declarations.
pub fn parse_dtd_subset(src: &str, old10: bool) -> Result<XmlDtd, XmlError> {
    parse_subset(src, true, old10, crate::parse::XML_MAX_AMPLIFICATION_DEFAULT)
}

/// [`parse_dtd_subset`] with the parse's entity amplification factor.
pub(crate) fn parse_dtd_subset_ampl(
    src: &str,
    old10: bool,
    max_ampl: u32,
) -> Result<XmlDtd, XmlError> {
    parse_subset(src, true, old10, max_ampl)
}

/// Parse an external subset, where conditional sections are legal and a
/// parameter entity may supply part of a declaration.
pub fn parse_external_subset(src: &str, old10: bool) -> Result<XmlDtd, XmlError> {
    parse_subset(src, false, old10, crate::parse::XML_MAX_AMPLIFICATION_DEFAULT)
}

fn parse_subset(
    src: &str,
    internal: bool,
    old10: bool,
    max_ampl: u32,
) -> Result<XmlDtd, XmlError> {
    // Conditional sections are resolved before parameter entities are
    // substituted, as libxml2 reads them: an IGNORE section is dropped whole,
    // so its text is never substituted and its declarations never bind, and an
    // INCLUDE section's declarations are kept (#58). They were all skipped.
    let resolved = if !internal && has_cond_markers(src) {
        std::borrow::Cow::Owned(resolve_conditional(src)?)
    } else {
        std::borrow::Cow::Borrowed(src)
    };
    let mut expanded = expand_pe(&resolved, src.len(), internal, max_ampl)?;
    // A parameter entity can carry a section of its own.
    if !internal && has_cond_markers(&expanded) {
        expanded = resolve_conditional(&expanded)?;
    }
    let mut dtd = XmlDtd::default();
    dtd.int_subset = Some(src.to_string());
    let mut p = DtdParser {
        src: expanded.as_str(),
        pos: 0,
        dtd: &mut dtd,
        internal,
        old10,
    };
    p.parse_markup()?;
    // A parameter entity reference in the subset means the declarations may be
    // incomplete, which changes "Entity Declared" from a well-formedness
    // constraint into a validity one.
    dtd.has_parameter_entity_refs = expanded != *resolved
        || src.chars().zip(src.chars().skip(1)).any(|(a, b)| {
            a == '%' && crate::chvalid::xml_is_name_start_char(b as u32, false)
        });
    check_entity_graph(&dtd)?;
    Ok(dtd)
}

/// Parameter entity amplification, charged as libxml2's
/// `xmlParserEntityCheck` charges a PE reference: the replacement's length
/// plus the fixed cost, failing past 1,000,000 bytes once the total exceeds
/// `max_ampl` times the subset's length.
struct PeAmpl {
    expanded: u64,
    consumed: u64,
    max_ampl: u64,
}

impl PeAmpl {
    fn charge(&mut self, len: usize) -> Result<(), XmlError> {
        self.expanded = self.expanded.saturating_add(len as u64).saturating_add(20);
        if self.expanded > 1_000_000
            && (self.expanded == u64::MAX || self.expanded / self.max_ampl > self.consumed)
        {
            return Err(XmlError::new(
                crate::error::XML_ERR_RESOURCE_LIMIT,
                "Maximum entity amplification factor exceeded, see xmlCtxtSetMaxAmplification.",
                0,
                0,
            ));
        }
        Ok(())
    }
}

fn expand_pe(
    src: &str,
    consumed: usize,
    internal: bool,
    max_ampl: u32,
) -> Result<String, XmlError> {
    // Multi-pass PE expansion so `%percent;` can invent new PE names.
    //
    // Every pass expands one more level, so ten references per entity grow
    // the subset tenfold per pass: 639 bytes became 17 MB at six levels, and
    // each further level is another factor of ten. The expansion is charged
    // as libxml2 charges it; xmllint refuses the five-level form.
    let mut ampl = PeAmpl {
        expanded: 0,
        consumed: consumed as u64,
        max_ampl: u64::from(max_ampl.max(1)),
    };
    let mut cur = src.to_string();
    for _ in 0..16 {
        let mut pes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let charged = harvest_pe(&cur, &mut pes, &mut ampl)?;
        let next = subst_pe(&cur, &pes, &charged, internal, &mut ampl)?;
        if next == cur {
            return Ok(cur);
        }
        cur = next;
    }
    Ok(cur)
}

/// Read every `<!ENTITY % name "value">` outside comments and PIs into
/// `pes`, the first declaration of a name binding (XML 1.0 section 4.2, and
/// libxml2; #58).
///
/// A reference in a value to an entity declared before it is expanded here,
/// once, and charged once, as libxml2 expands an entity value when it parses
/// the declaration (`xmlExpandPEsInEntityValue`). A use is then replaced by
/// text with no references left to charge again. Storing the raw value made
/// every copy of it carry its references into the next pass, where each copy
/// was substituted and charged anew: TEI P5 `tei_all.dtd` reached the
/// amplification limit on 1.9 MB of charges for text that grows to 1.2 MB.
///
/// Returns the byte offsets of the references charged here, so the
/// substitution pass does not charge them a second time.
fn harvest_pe(
    src: &str,
    pes: &mut std::collections::HashMap<String, String>,
    ampl: &mut PeAmpl,
) -> Result<std::collections::HashSet<usize>, XmlError> {
    let bytes = src.as_bytes();
    let mut charged = std::collections::HashSet::new();
    let mut i = 0;
    while i + 8 < bytes.len() {
        if bytes[i] == b'<' && bytes[i..].starts_with(b"<!--") {
            match src[i + 4..].find("-->") {
                Some(e) => {
                    i += 4 + e + 3;
                    continue;
                }
                None => break,
            }
        }
        if bytes[i] == b'<' && bytes[i..].starts_with(b"<?") {
            match src[i + 2..].find("?>") {
                Some(e) => {
                    i += 2 + e + 2;
                    continue;
                }
                None => break,
            }
        }
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
                    let ve = i;
                    // The harvest pass is a pre-scan; a malformed value is
                    // reported later, by the declaration parser proper.
                    if let Some(val) = expand_entity_value(src, vs, ve, pes, ampl, &mut charged)? {
                        pes.entry(name).or_insert(val);
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    Ok(charged)
}

/// The value of a parameter entity declared at `src[vs..ve]`: references to
/// entities already in `pes` expanded (and charged, their offsets recorded in
/// `charged`), others kept as written, and character references in the
/// literal text decoded. `None` if the literal text is malformed.
fn expand_entity_value(
    src: &str,
    vs: usize,
    ve: usize,
    pes: &std::collections::HashMap<String, String>,
    ampl: &mut PeAmpl,
    charged: &mut std::collections::HashSet<usize>,
) -> Result<Option<String>, XmlError> {
    let raw = &src[vs..ve];
    if !raw.contains('%') {
        return Ok(decode_charrefs(raw).ok());
    }
    let mut val = String::with_capacity(raw.len());
    let mut seg = 0;
    let mut k = 0;
    while let Some(off) = raw[k..].find('%') {
        let at = k + off;
        let Some((name, end)) = pe_ref_at(raw, at) else {
            k = at + 1;
            continue;
        };
        let Some(v) = pes.get(name) else {
            k = end;
            continue;
        };
        let Ok(lit) = decode_charrefs(&raw[seg..at]) else {
            return Ok(None);
        };
        val.push_str(&lit);
        ampl.charge(v.len())?;
        charged.insert(vs + at);
        val.push_str(v);
        seg = end;
        k = end;
    }
    let Ok(lit) = decode_charrefs(&raw[seg..]) else {
        return Ok(None);
    };
    val.push_str(&lit);
    Ok(Some(val))
}

/// A well-formed `%name;` starting at byte `at` of `s`: the name and the
/// offset just past the `;`.
fn pe_ref_at(s: &str, at: usize) -> Option<(&str, usize)> {
    let rest = &s[at + 1..];
    let mut end = None;
    for (i, c) in rest.char_indices() {
        let ok = if i == 0 {
            crate::chvalid::xml_is_name_start_char(c as u32, false)
        } else {
            crate::chvalid::xml_is_name_char(c as u32, false)
        };
        if c == ';' && i > 0 {
            end = Some(i);
            break;
        }
        if !ok {
            return None;
        }
    }
    let e = end?;
    Some((&rest[..e], at + 1 + e + 1))
}

fn has_cond_markers(s: &str) -> bool {
    s.contains("<![") || s.contains("]]>")
}

/// 1-based line and column of byte `at` in `s`, for errors in a pre-pass.
fn pos_of(s: &str, at: usize) -> (u32, u32) {
    let before = &s[..at.min(s.len())];
    let line = before.bytes().filter(|&b| b == b'\n').count() + 1;
    let col = at - before.rfind('\n').map_or(0, |n| n + 1) + 1;
    (line as u32, col as u32)
}

/// Resolve the conditional sections of an external subset (#58).
///
/// One left-to-right pass. Comments, processing instructions and markup
/// declarations are copied as units (quotes respected), so a `<![` or `]]>`
/// inside one is text. An INCLUDE section is replaced by its contents, which
/// are resolved in turn; an IGNORE section is dropped whole, nested sections
/// counted, as libxml2's `xmlParseConditionalSections` does. A keyword may be
/// a parameter entity reference, read from the declarations passed so far, the
/// first declaration of a name binding. Errors are libxml2's: an unknown
/// keyword (95), a missing `[` (83), an IGNORE section never closed (59), and
/// an INCLUDE section never closed or a `]]>` with no section open (60).
fn resolve_conditional(src: &str) -> Result<String, XmlError> {
    use crate::error::{
        XML_ERR_CONDSEC_INVALID, XML_ERR_CONDSEC_INVALID_KEYWORD, XML_ERR_CONDSEC_NOT_FINISHED,
        XML_ERR_EXT_SUBSET_NOT_FINISHED,
    };
    let err = |code: i32, msg: &str, at: usize| {
        let (l, c) = pos_of(src, at);
        XmlError::new(code, msg, l, c)
    };
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    // Raw (character references decoded) values of the parameter entities
    // declared so far; a keyword's value is expanded only when it is read.
    let mut pes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut open: Vec<usize> = Vec::new();
    let mut i = 0;
    let mut copied = 0;
    while i < b.len() {
        let rest = &b[i..];
        if rest.starts_with(b"<!--") {
            i = src[i + 4..].find("-->").map_or(b.len(), |e| i + 4 + e + 3);
            continue;
        }
        if rest.starts_with(b"<?") {
            i = src[i + 2..].find("?>").map_or(b.len(), |e| i + 2 + e + 2);
            continue;
        }
        if rest.starts_with(b"<![") {
            out.push_str(&src[copied..i]);
            let at = i;
            let mut j = i + 3;
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            let keyword: String = if b.get(j) == Some(&b'%') {
                match pe_ref_at(src, j) {
                    Some((name, end)) => {
                        j = end;
                        keyword_value(name, &pes)
                    }
                    None => String::new(),
                }
            } else if src[j..].starts_with("INCLUDE") {
                j += 7;
                "INCLUDE".into()
            } else if src[j..].starts_with("IGNORE") {
                j += 6;
                "IGNORE".into()
            } else {
                String::new()
            };
            if keyword != "INCLUDE" && keyword != "IGNORE" {
                return Err(err(
                    XML_ERR_CONDSEC_INVALID_KEYWORD,
                    "conditional section INCLUDE or IGNORE keyword expected",
                    at,
                ));
            }
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if b.get(j) != Some(&b'[') {
                return Err(err(XML_ERR_CONDSEC_INVALID, "XML conditional section '[' expected", at));
            }
            j += 1;
            if keyword == "INCLUDE" {
                open.push(at);
                i = j;
            } else {
                let mut depth = 0usize;
                loop {
                    if j >= b.len() {
                        return Err(err(XML_ERR_CONDSEC_NOT_FINISHED, "XML conditional section not closed", at));
                    }
                    if b[j..].starts_with(b"<![") {
                        depth += 1;
                        j += 3;
                    } else if b[j..].starts_with(b"]]>") {
                        j += 3;
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    } else {
                        j += 1;
                    }
                }
                i = j;
            }
            copied = i;
            continue;
        }
        if rest.starts_with(b"]]>") {
            if open.pop().is_none() {
                return Err(err(XML_ERR_EXT_SUBSET_NOT_FINISHED, "Content error in the external subset", i));
            }
            out.push_str(&src[copied..i]);
            i += 3;
            copied = i;
            continue;
        }
        if rest.starts_with(b"<!") {
            let start = i;
            let mut j = i + 2;
            let mut quote: Option<u8> = None;
            while j < b.len() {
                match (quote, b[j]) {
                    (Some(q), c) if c == q => quote = None,
                    (None, b'"' | b'\'') => quote = Some(b[j]),
                    (None, b'>') => break,
                    _ => {}
                }
                j += 1;
            }
            let end = (j + 1).min(b.len());
            record_pe_decl(&src[start..end], &mut pes);
            i = end;
            continue;
        }
        i += 1;
    }
    if let Some(&at) = open.last() {
        return Err(err(XML_ERR_EXT_SUBSET_NOT_FINISHED, "Content error in the external subset", at));
    }
    out.push_str(&src[copied..]);
    Ok(out)
}

/// Record `<!ENTITY % name "value">` for keyword lookups, first one binding.
fn record_pe_decl(decl: &str, pes: &mut std::collections::HashMap<String, String>) {
    let Some(rest) = decl.strip_prefix("<!ENTITY") else { return };
    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix('%') else { return };
    let rest = rest.trim_start();
    let name_end = rest
        .find(|c: char| c.is_ascii_whitespace() || c == '"' || c == '\'')
        .unwrap_or(rest.len());
    let (name, rest) = rest.split_at(name_end);
    let rest = rest.trim_start();
    let Some(q) = rest.chars().next().filter(|&c| c == '"' || c == '\'') else { return };
    let Some(close) = rest[1..].find(q) else { return };
    if let Ok(v) = decode_charrefs(&rest[1..1 + close]) {
        pes.entry(name.to_string()).or_insert(v);
    }
}

/// A keyword read through a parameter entity: its value with references to
/// other recorded entities expanded (a few levels, a short result: a keyword
/// is one word), then trimmed.
fn keyword_value(name: &str, pes: &std::collections::HashMap<String, String>) -> String {
    let Some(mut v) = pes.get(name).cloned() else { return String::new() };
    for _ in 0..8 {
        let Some(at) = v.find('%') else { break };
        let Some((n, end)) = pe_ref_at(&v, at) else { break };
        let Some(sub) = pes.get(n) else { break };
        if v.len() + sub.len() > 256 {
            break;
        }
        v = format!("{}{}{}", &v[..at], sub, &v[end..]);
    }
    v.trim().to_string()
}

fn subst_pe(
    src: &str,
    pes: &std::collections::HashMap<String, String>,
    charged: &std::collections::HashSet<usize>,
    internal: bool,
    ampl: &mut PeAmpl,
) -> Result<String, XmlError> {
    let mut out = String::new();
    let mut chars = Cur::new(src);
    let mut in_comment = false;
    // In the internal subset a parameter entity reference may only occur where
    // a markup declaration can occur -- never inside one. `<!ELEMENT %pe;` and
    // `<!ENTITY foo "%e;">` were both expanded happily.
    let mut in_decl = false;
    // A PE reference is recognized inside an EntityValue but NOT inside an
    // attribute default, where `%` is an ordinary character. `<!ATTLIST d a
    // CDATA "%e;">` is a valid document and must stay one.
    let mut decl_is_entity = false;
    let mut in_quote: Option<char> = None;
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
        if c == '>' && in_decl {
            in_decl = false;
            out.push(c);
            continue;
        }
        if in_decl {
            match in_quote {
                Some(q) if c == q => in_quote = None,
                None if c == '"' || c == '\'' => in_quote = Some(c),
                _ => {}
            }
        }
        // A processing instruction is markup too, and a parameter entity may
        // not supply part of one in the internal subset. `<?music %pe;` where
        // the entity carries the `?>` was going through.
        if c == '<' && chars.peek() == Some(&'?') {
            in_decl = true;
            decl_is_entity = false;
            in_quote = None;
            out.push(c);
            out.push(chars.next().unwrap());
            continue;
        }
        if c == '?' && in_decl && chars.peek() == Some(&'>') {
            in_decl = false;
            out.push(c);
            out.push(chars.next().unwrap());
            continue;
        }
        if c == '<' && chars.peek() == Some(&'!') {
            in_decl = true;
            decl_is_entity = false;
            in_quote = None;
            out.push(c);
            out.push(chars.next().unwrap());
            if chars.peek() == Some(&'-') {
                out.push(chars.next().unwrap());
                if chars.peek() == Some(&'-') {
                    out.push(chars.next().unwrap());
                    in_comment = true;
                    in_decl = false;
                }
            } else {
                // Which declaration this is decides whether a `%` inside its
                // literals is a reference at all.
                let rest: String = chars.clone().take(6).collect();
                decl_is_entity = rest.starts_with("ENTITY");
            }
            continue;
        }
        if c == '%' {
            // Where this reference starts, to know whether the harvest pass
            // already charged it.
            let at = chars.pos - 1;
            // `%` followed by anything that cannot start a Name is the PE
            // marker of an `<!ENTITY % name ...>` declaration, not a reference.
            let is_ref = chars
                .peek()
                .is_some_and(|n| crate::chvalid::xml_is_name_start_char(*n as u32, false));
            if !is_ref {
                // PEReference ::= '%' Name ';' -- `%;` has no name at all.
                if chars.peek() == Some(&';') {
                    return Err(XmlError::new(
                        crate::error::XML_ERR_ENTITYREF_NO_NAME,
                        "PEReference: no name",
                        0,
                        0,
                    ));
                }
                out.push('%');
                continue;
            }
            // Inside an attribute default the `%` is literal; leave it alone.
            if in_decl && in_quote.is_some() && !decl_is_entity {
                out.push('%');
                continue;
            }
            if internal && in_decl {
                return Err(XmlError::new(
                    crate::error::XML_ERR_ENTITYREF_NO_NAME,
                    "PEReferences forbidden in internal subset",
                    0,
                    0,
                ));
            }
            let mut name = String::new();
            let mut terminated = false;
            while let Some(&n) = chars.peek() {
                if n == ';' {
                    chars.next();
                    terminated = true;
                    break;
                }
                if !crate::chvalid::xml_is_name_char(n as u32, false) {
                    break;
                }
                name.push(n);
                chars.next();
            }
            // `%paaa` and `%paaa ;` were both accepted. The semicolon is not
            // optional, and no whitespace may come before it.
            if !terminated {
                return Err(XmlError::new(
                    crate::error::XML_ERR_ENTITYREF_SEMICOL_MISSING,
                    "PEReference: expecting ';'",
                    0,
                    0,
                ));
            }
            if let Some(v) = pes.get(&name) {
                if !charged.contains(&at) {
                    ampl.charge(v.len())?;
                }
                out.push_str(v);
            } else {
                out.push('%');
                out.push_str(&name);
                out.push(';');
            }
            continue;
        }
        out.push(c);
    }
    Ok(out)
}

/// A character cursor that knows its byte offset, for [`subst_pe`].
#[derive(Clone)]
struct Cur<'s> {
    s: &'s str,
    /// Byte offset of the next character.
    pos: usize,
    next: Option<char>,
}

impl<'s> Cur<'s> {
    fn new(s: &'s str) -> Self {
        Cur { s, pos: 0, next: s.chars().next() }
    }

    fn peek(&self) -> Option<&char> {
        self.next.as_ref()
    }
}

impl Iterator for Cur<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        let c = self.next?;
        self.pos += c.len_utf8();
        self.next = self.s[self.pos..].chars().next();
        Some(c)
    }
}

/// Decode character references in an entity value or attribute default.
///
/// EntityValue forbids a bare `&`: it must begin a character or entity
/// reference. Nothing checked that, so `&49;` was kept as literal text and
/// `&#002f;` -- digits followed by a non-digit -- was silently left alone
/// instead of being reported as an invalid decimal value.
///
/// General entity references are kept verbatim; they are expanded at the point
/// of use, not here.
fn decode_charrefs(s: &str) -> Result<String, &'static str> {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        if it.peek() == Some(&'#') {
            it.next();
            let hex = matches!(it.peek(), Some('x') | Some('X'));
            let upper_x = it.peek() == Some(&'X');
            if hex {
                it.next();
            }
            let mut digits = String::new();
            while let Some(&d) = it.peek() {
                if hex && d.is_ascii_hexdigit() || !hex && d.is_ascii_digit() {
                    digits.push(d);
                    it.next();
                } else {
                    break;
                }
            }
            // `&#X41;` -- the production spells the marker lowercase only.
            if upper_x || digits.is_empty() || it.next() != Some(';') {
                return Err(if hex {
                    "CharRef: invalid hexadecimal value"
                } else {
                    "CharRef: invalid decimal value"
                });
            }
            let radix = if hex { 16 } else { 10 };
            let v = crate::charref::value(digits.as_bytes(), radix)
                .map_err(|_| "CharRef: value out of range")?;
            match char::from_u32(v).filter(|ch| crate::chvalid::xml_is_char(*ch as u32)) {
                Some(ch) => out.push(ch),
                None => return Err("CharRef: invalid XML character"),
            }
            continue;
        }
        // A general entity reference: keep it, but it must be well formed.
        let mut name = String::new();
        while let Some(&d) = it.peek() {
            if crate::chvalid::xml_is_name_char(d as u32, false) {
                name.push(d);
                it.next();
            } else {
                break;
            }
        }
        if name.is_empty() || it.next() != Some(';') {
            return Err("EntityValue: '&' forbidden except for entities references");
        }
        out.push('&');
        out.push_str(&name);
        out.push(';');
    }
    Ok(out)
}

struct DtdParser<'a> {
    src: &'a str,
    pos: usize,
    dtd: &'a mut XmlDtd,
    /// The internal subset carries rules the external one does not: no
    /// conditional sections, and a parameter entity may not supply part of a
    /// declaration.
    internal: bool,
    /// XML 1.0 before the 5th edition: the narrower name character classes.
    /// The DTD parser had no idea this option existed, so a name illegal
    /// under the old rules sailed through in a declaration -- and a PI
    /// target inside the subset was never checked at all.
    old10: bool,
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

    fn skip_ws_and_comments(&mut self) -> Result<(), XmlError> {
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
                // An XML declaration is only legal at the very start of the
                // document. Inside the internal subset it is a PI whose target
                // is reserved, and this loop skipped every PI without looking.
                let after = &self.rest()[2..];
                // .get(..3), not [..3]: a byte index that lands inside a
                // multi-byte character panics, and a PI target is arbitrary
                // text. The suite hit this on the first run.
                let is_xml_decl = after
                    .get(..3)
                    .is_some_and(|k| k.eq_ignore_ascii_case("xml"))
                    && after[3..]
                        .chars()
                        .next()
                        .is_none_or(|c| c.is_whitespace() || c == '?');
                if is_xml_decl {
                    return Err(self.err(
                        "XML declaration allowed only at the start of the document",
                    ));
                }
                // A PI inside the subset was skipped without a glance at its
                // target. That is where the suite puts its illegal-name cases
                // -- roughly three hundred of them.
                let after = &self.rest()[2..];
                let mut target_len = 0usize;
                for (i, ch) in after.char_indices() {
                    let ok = if i == 0 {
                        crate::chvalid::xml_is_name_start_char(ch as u32, self.old10)
                    } else {
                        crate::chvalid::xml_is_name_char(ch as u32, self.old10)
                    };
                    if !ok {
                        break;
                    }
                    target_len = i + ch.len_utf8();
                }
                if target_len == 0 {
                    return Err(self.err("xmlParsePI : no target name"));
                }
                // The target has to END there too. `<?_` followed by a
                // character that is not a name character is not a PI with the
                // target `_`; it is a PI with an illegal character in its
                // target, which is what the suite is testing.
                let tail = &after[target_len..];
                let ends_cleanly = tail.is_empty()
                    || tail.starts_with("?>")
                    || tail.chars().next().is_some_and(char::is_whitespace);
                if !ends_cleanly {
                    return Err(self.err("xmlParsePI : invalid character in target name"));
                }
                if let Some(e) = self.rest().find("?>") {
                    self.pos += e + 2;
                    continue;
                }
            }
            break;
        }
        Ok(())
    }
    fn parse_markup(&mut self) -> Result<(), XmlError> {
        loop {
            self.skip_ws_and_comments()?;
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
                self.parse_notation()?;
            } else if self.rest().starts_with("<![") {
                // INCLUDE and IGNORE sections are external-subset only.
                if self.internal {
                    return Err(self.err("Content error in the internal subset"));
                }
                self.skip_cond()?;
            } else if self.rest().starts_with('<') {
                // Anything else beginning with '<' is not a markup
                // declaration, and skipping it accepted `<ELEMENT ...>` with
                // the bang missing, `<!Attlist ...>` and `<!notation ...>`
                // with the keyword miscased, and every other near-miss.
                return Err(self.err("Content error in the internal subset"));
            } else if self.rest().starts_with('%') {
                // A well-formed PE reference was already substituted, so a '%'
                // still sitting at markup level is not one -- `% foo;` with a
                // space is not a reference, it is garbage between declarations.
                return Err(self.err("PEReference: expecting ';'"));
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
        // A misplaced XML declaration is reported by the markup loop; here we
        // only need the position advanced.
        let _ = self.skip_ws_and_comments();
        let r = self.rest();
        let mut n = 0;
        // Names here were ASCII-only, so `<!ELEMENT เจมส์ (#PCDATA)>` -- a
        // perfectly valid declaration -- came back empty and the document was
        // rejected. The document body accepted the same name happily; only the
        // DTD disagreed.
        for (i, c) in r.char_indices() {
            let ok = if i == 0 {
                crate::chvalid::xml_is_name_start_char(c as u32, self.old10)
            } else {
                crate::chvalid::xml_is_name_char(c as u32, self.old10)
            };
            if !ok {
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
        self.skip_ws_and_comments()?;
        let r = self.rest();
        if r.starts_with('"') || r.starts_with('\'') {
            let q = r.as_bytes()[0] as char;
            self.bump(1);
            if let Some(e) = self.rest().find(q) {
                let s = decode_charrefs(&self.rest()[..e]).map_err(|m| self.err(m))?;
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
        if !self.require_ws() {
            return Err(self.err("Space required after '<!ELEMENT'"));
        }
        let name = self.parse_name();
        if name.is_empty() {
            return Err(self.err("Element name expected"));
        }
        if !self.require_ws() {
            return Err(self.err("Space required after the element name"));
        }
        let decl = if self.rest().starts_with("EMPTY") {
            self.bump(5);
            ElementDecl::Empty
        } else if self.rest().starts_with("ANY") {
            self.bump(3);
            ElementDecl::Any
        } else if self.rest().starts_with('(') {
            let mut spec = self.take_until_gt_paren();
            // take_until_gt_paren stops at the closing paren, so a trailing
            // occurrence indicator is still in the stream. It is part of the
            // content spec and Mixed content is not valid without it.
            if let Some(q @ ('?' | '*' | '+')) = self.rest().chars().next() {
                self.bump(1);
                spec.push(q);
            }
            // The content model was never checked, only scanned for '#PCDATA'
            // and split on '|'. Everything else was accepted: `(a & b)`,
            // `(a b)`, `(a|b,c)` mixing connectors, `(doc*?)`, `()`. That is
            // 73 conformance cases, and the unchecked loop behind it was the
            // 32 GB allocation.
            let mixed = validate_contentspec(&spec).map_err(|e| self.err(e))?;
            if mixed {
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
            return Err(self.err("xmlParseElementDecl: 'EMPTY', 'ANY' or '(' expected"));
        };
        if self.dtd.elements.contains_key(&name) {
            self.dtd.duplicate_elements.push(name.clone());
        }
        self.dtd.elements.insert(name, decl);
        self.expect_decl_end("Element")
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
        if !self.require_ws() {
            return Err(self.err("Space required after '<!ATTLIST'"));
        }
        let elem = self.parse_name();
        if elem.is_empty() {
            return Err(self.err("Element name expected in ATTLIST"));
        }
        loop {
            // AttDef ::= S Name S AttType S DefaultDecl -- every one of those
            // S is required, and none of them was checked.
            let had_ws = self.require_ws();
            self.skip_ws_and_comments()?;
            if self.rest().starts_with('>') {
                self.bump(1);
                break;
            }
            if self.pos >= self.src.len() {
                return Err(self.err("xmlParseAttributeListDecl: not terminated"));
            }
            if !had_ws {
                return Err(self.err("Space required after the attribute name"));
            }
            let aname = self.parse_name();
            if aname.is_empty() {
                return Err(self.err("Attribute name expected"));
            }
            if !self.require_ws() {
                return Err(self.err("Space required after the attribute name"));
            }
            self.skip_ws_and_comments()?;
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
                    if !self.require_ws() {
                        return Err(self.err("Space required after 'NOTATION'"));
                    }
                    if !self.rest().starts_with('(') {
                        return Err(self.err("'(' required to start ATTLIST enumeration"));
                    }
                    let spec = self.take_until_gt_paren();
                    for part in spec.trim().trim_matches(['(', ')']).split('|') {
                        let n = part.trim();
                        // NotationType ::= 'NOTATION' S '(' S? Name (S? '|' S?
                        // Name)* S? ')' -- every entry is a Name, and an empty
                        // or malformed one was quietly dropped.
                        let ok = !n.is_empty()
                            && n.chars().enumerate().all(|(i, c)| {
                                if i == 0 {
                                    crate::chvalid::xml_is_name_start_char(c as u32, self.old10)
                                } else {
                                    crate::chvalid::xml_is_name_char(c as u32, self.old10)
                                }
                            });
                        if !ok {
                            return Err(self.err("Name expected in NOTATION declaration"));
                        }
                        enumerated.push(n.to_string());
                    }
                }
                t
            };
            if !self.require_ws() {
                return Err(self.err("Space required after the attribute type"));
            }
            self.skip_ws_and_comments()?;
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
            // "When more than one definition is provided for the same
            // attribute of a given element type, the FIRST declaration is
            // binding and later declarations are ignored." We were inserting
            // into a map, so the last one won and a later #REQUIRED overrode
            // an earlier default.
            // Attribute-value normalization applies to a default too, and to
            // the LITERAL whitespace only: a tab written out becomes a space,
            // a character-referenced one stays a tab. Entity values and system
            // identifiers get no such treatment, so this belongs here and not
            // in the shared literal reader.
            let default_value = default_value.map(|v: String| {
                v.chars()
                    .map(|c| if matches!(c, '\t' | '\n' | '\r') { ' ' } else { c })
                    .collect::<String>()
            });
            // In an attribute default the references ARE expanded, so the
            // entity has to be declared already -- unlike an EntityValue,
            // where they are bypassed and a forward reference is legal. The
            // graph check runs after the whole subset and so could not tell
            // the two apart; this runs in declaration order and can.
            if let Some(v) = default_value.as_deref() {
                const PREDEFINED: &[&str] = &["lt", "gt", "amp", "apos", "quot"];
                for r in entity_refs_in(v) {
                    if !PREDEFINED.contains(&r.as_str()) && !self.dtd.entities.contains_key(&r) {
                        return Err(self.err(&format!("Entity '{r}' not defined")));
                    }
                }
            }
            self.dtd.attributes.entry((elem.clone(), aname)).or_insert(
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
    /// `NotationDecl ::= '<!NOTATION' S Name S (ExternalID | PublicID) S? '>'`
    ///
    /// This went to skip_decl, which took everything up to the next '>' and
    /// asked no questions: a missing space, a missing name, a public
    /// identifier holding characters the production forbids, all accepted.
    fn parse_notation(&mut self) -> Result<(), XmlError> {
        self.bump("<!NOTATION".len());
        if !self.require_ws() {
            return Err(self.err("Space required after '<!NOTATION'"));
        }
        let name = self.parse_name();
        if name.is_empty() {
            return Err(self.err("Notation name expected"));
        }
        if !self.require_ws() {
            return Err(self.err("Space required after the notation name"));
        }
        if name.contains(':') {
            self.dtd
                .namespace_errors
                .push(format!("colons are forbidden from notation names '{name}'"));
        }
        self.dtd.notations.insert(name.clone());
        let public = if self.rest().starts_with("PUBLIC") {
            true
        } else if self.rest().starts_with("SYSTEM") {
            false
        } else {
            return Err(self.err("'PUBLIC' or 'SYSTEM' expected in NOTATION"));
        };
        self.bump(6);
        if !self.require_ws() {
            return Err(self.err("Space required after the external ID keyword"));
        }
        if !self.at_quote() {
            return Err(self.err("Unfinished System or Public ID \" or ' expected"));
        }
        let first = self.parse_quoted()?;
        if public {
            if let Some(bad) = first.chars().find(|c| !is_pubid_char(*c)) {
                return Err(self.err(&format!(
                    "Invalid character 0x{:X} in public identifier",
                    bad as u32
                )));
            }
            // PublicID (notation only) may stop after the public identifier;
            // ExternalID continues with a system literal.
            let before = self.pos;
            self.skip_ws();
            if self.at_quote() {
                if self.pos == before {
                    return Err(self.err("Space required after the Public Identifier"));
                }
                self.parse_quoted()?;
            }
        }
        self.expect_decl_end("Notation")
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
        // An entity name is an NCName -- the colon belongs to QNames. Not
        // fatal, as C is not: it is a namespace error and gets reported.
        if name.contains(':') {
            self.dtd
                .namespace_errors
                .push(format!("colons are forbidden from entities names '{name}'"));
        }
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
                let pid = self.parse_quoted()?;
                if let Some(bad) = pid.chars().find(|c| !is_pubid_char(*c)) {
                    return Err(self.err(&format!(
                        "Invalid character 0x{:X} in public identifier",
                        bad as u32
                    )));
                }
                if !self.require_ws() {
                    return Err(self.err("Space required after the Public Identifier"));
                }
                if !self.at_quote() {
                    return Err(self.err("SystemLiteral expected"));
                }
            }
            // parse_quoted returns an empty string rather than an error when
            // it is not looking at a quote, so `<!ENTITY p SYSTEM >` with no
            // literal at all went through as an entity with no system id.
            if !self.at_quote() {
                return Err(self.err("SystemLiteral \" or ' expected"));
            }
            self.parse_quoted()?;
            // NDataDecl is the only thing allowed to follow, and it needs the
            // space before it. Measure BEFORE skipping, or the skip eats the
            // very thing being checked for.
            let ws_before_ndata = {
                let before = self.pos;
                self.skip_ws();
                self.pos > before
            };
            self.skip_ws_and_comments()?;
            if self.rest().starts_with("NDATA") {
                // A parameter entity is always parsed; NDATA is for unparsed
                // general entities only.
                if pe {
                    return Err(self.err("xmlParseEntityDecl: entity not terminated"));
                }
                if !ws_before_ndata {
                    return Err(self.err("Space required before 'NDATA'"));
                }
                self.bump(5);
                if !self.require_ws() {
                    return Err(self.err("Space required after 'NDATA'"));
                }
                let notation = self.parse_name();
                if notation.is_empty() {
                    return Err(self.err("Notation name expected after 'NDATA'"));
                }
                self.dtd.ndata_notations.push(notation);
                // An NDATA entity is unparsed, and only an unparsed entity may
                // be the value of an ENTITY attribute.
                self.dtd.unparsed_entities.insert(name.clone());
                self.skip_ws();
            }
            return self.expect_decl_end("entity");
        }
        if !self.at_quote() {
            return Err(self.err("Entity value expected"));
        }
        let val = self.parse_quoted()?;
        // "If the same entity is declared more than once, the first
        // declaration encountered is binding." We inserted into a map, so the
        // last won -- and a document whose second declaration is deliberately
        // junk was rejected on the strength of a declaration it never uses.
        if pe {
            self.dtd.parameter_entities.entry(name).or_insert(val);
        } else {
            self.dtd.entities.entry(name).or_insert(val);
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
    dst.unparsed_entities.extend(src.unparsed_entities);
    dst.duplicate_elements.extend(src.duplicate_elements);
    dst.notations.extend(src.notations);
    dst.namespace_errors.extend(src.namespace_errors);
    dst.ndata_notations.extend(src.ndata_notations);
    dst.has_parameter_entity_refs |= src.has_parameter_entity_refs;
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

/// Validate a content specification against XML 1.0 productions 46-51.
///
/// Returns `Ok(true)` for Mixed content, `Ok(false)` for a children model.
///
/// ```text
/// Mixed    ::= '(' S? '#PCDATA' (S? '|' S? Name)* S? ')*'
///            | '(' S? '#PCDATA' S? ')'
/// children ::= (choice | seq) ('?' | '*' | '+')?
/// cp       ::= (Name | choice | seq) ('?' | '*' | '+')?
/// choice   ::= '(' S? cp ( S? '|' S? cp )+ S? ')'
/// seq      ::= '(' S? cp ( S? ',' S? cp )* S? ')'
/// ```
///
/// The two rules that catch most malformed models: a group may not mix `,` and
/// `|` at the same level, and Mixed content that names elements must close
/// with `)*`.
fn validate_contentspec(spec: &str) -> Result<bool, &'static str> {
    let mut p = SpecParser {
        b: spec.as_bytes(),
        i: 0,
        depth: 0,
    };
    p.ws();
    if !p.eat(b'(') {
        return Err("ContentDecl : '(' expected");
    }
    p.ws();
    if p.b[p.i..].starts_with(b"#PCDATA") {
        p.i += 7;
        let mut named = false;
        loop {
            p.ws();
            if p.eat(b')') {
                break;
            }
            if !p.eat(b'|') {
                return Err("ContentDecl : ',' '|' or ')' expected");
            }
            p.ws();
            p.name()?;
            named = true;
        }
        // `(#PCDATA|a)` without the star is not a legal Mixed model.
        let star = p.eat(b'*');
        if named && !star {
            return Err("Element content model is not finished with ')*'");
        }
        p.ws();
        return if p.i == p.b.len() {
            Ok(true)
        } else {
            Err("trailing content after the content model")
        };
    }
    // A children model: rewind to the '(' and read it as a group.
    p.i = 0;
    p.ws();
    p.group()?;
    p.quant();
    p.ws();
    if p.i != p.b.len() {
        return Err("ContentDecl : garbage after the content model");
    }
    Ok(false)
}

struct SpecParser<'a> {
    b: &'a [u8],
    i: usize,
    depth: u32,
}

impl SpecParser<'_> {
    fn ws(&mut self) {
        while matches!(self.b.get(self.i), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn quant(&mut self) {
        if matches!(self.peek(), Some(b'?' | b'*' | b'+')) {
            self.i += 1;
        }
    }
    fn name(&mut self) -> Result<(), &'static str> {
        let start = self.i;
        // Names here are ASCII in practice, but a UTF-8 name must not be cut
        // mid-character, so decode properly.
        let rest = match std::str::from_utf8(&self.b[self.i..]) {
            Ok(r) => r,
            Err(_) => return Err("invalid UTF-8 in the content model"),
        };
        let mut chars = rest.char_indices();
        match chars.next() {
            Some((_, c)) if crate::chvalid::xml_is_name_start_char(c as u32, false) => {
                self.i += c.len_utf8();
            }
            _ => return Err("Name expected in the content model"),
        }
        for (off, c) in chars {
            if !crate::chvalid::xml_is_name_char(c as u32, false) {
                self.i = start + off;
                return Ok(());
            }
            self.i = start + off + c.len_utf8();
        }
        Ok(())
    }
    /// choice or seq. Which one is decided by the first separator, and the
    /// group must then use only that one.
    fn group(&mut self) -> Result<(), &'static str> {
        // `((((((...` must not recurse the stack away.
        self.depth += 1;
        if self.depth > 256 {
            return Err("content model nested too deeply");
        }
        if !self.eat(b'(') {
            return Err("ContentDecl : '(' expected");
        }
        self.ws();
        self.cp()?;
        self.ws();
        let sep = match self.peek() {
            Some(b')') => {
                self.i += 1;
                self.depth -= 1;
                return Ok(());
            }
            Some(c @ (b'|' | b',')) => c,
            _ => return Err("ContentDecl : ',' '|' or ')' expected"),
        };
        loop {
            if !self.eat(sep) {
                // A different connector at the same level: `(a|b,c)`.
                return Err("ContentDecl : ',' '|' or ')' expected");
            }
            self.ws();
            self.cp()?;
            self.ws();
            match self.peek() {
                Some(b')') => {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(());
                }
                Some(c) if c == sep => continue,
                _ => return Err("ContentDecl : ',' '|' or ')' expected"),
            }
        }
    }
    fn cp(&mut self) -> Result<(), &'static str> {
        if self.peek() == Some(b'(') {
            self.group()?;
        } else {
            self.name()?;
        }
        self.quant();
        Ok(())
    }
}

/// PubidChar ::= #x20 | #xD | #xA | [a-zA-Z0-9] | [-'()+,./:=?;!*#@$_%]
///
/// A public identifier is a restricted character set, not free text. Nothing
/// checked it, so `<!NOTATION n PUBLIC "a^b">` was accepted.
pub fn is_pubid_char(c: char) -> bool {
    matches!(c, ' ' | '\r' | '\n')
        || c.is_ascii_alphanumeric()
        || matches!(
            c,
            '-' | '\'' | '(' | ')' | '+' | ',' | '.' | '/' | ':'
                | '=' | '?' | ';' | '!' | '*' | '#' | '@' | '$' | '_' | '%'
        )
}

/// Well-formedness constraints on the entity graph, checked once the whole
/// subset is parsed.
///
/// `decode_charrefs` keeps general entity references verbatim, because they are
/// expanded at the point of use. Nothing then looked at them, so an entity
/// value or an ATTLIST default could reference an entity that was never
/// declared, or one declared NDATA (which may not be referenced at all), or
/// itself by way of a cycle. All three were accepted silently, and the cycle
/// only surfaced later as a depth-limit error pointing at the wrong entity.
fn check_entity_graph(dtd: &XmlDtd) -> Result<(), XmlError> {
    const PREDEFINED: &[&str] = &["lt", "gt", "amp", "apos", "quot"];
    let err = |m: String| XmlError::new(crate::error::XML_ERR_UNDECLARED_ENTITY, m, 0, 0);

    // Every reference in a literal must name a declared, parsed entity.
    // Only entity values form the reference graph: an ATTLIST default cannot
    // be referenced, so it cannot close a cycle. Keying both by bare name made
    // an attribute `a` and an entity `a` share an entry, which reported a loop
    // that does not exist.
    let mut refs: std::collections::HashMap<&str, Vec<String>> = Default::default();
    let literals = dtd
        .entities
        .iter()
        .map(|(k, v)| (Some(k.as_str()), v.as_str()))
        .chain(
            dtd.attributes
                .values()
                .filter_map(|d| d.default_value.as_deref().map(|v| (None, v))),
        );
    for (owner, text) in literals {
        for name in entity_refs_in(text) {
            if PREDEFINED.contains(&name.as_str()) {
                continue;
            }
            if dtd.unparsed_entities.contains(&name) {
                return Err(err(format!("Entity reference to unparsed entity {name}")));
            }
            if !dtd.entities.contains_key(&name) {
                return Err(err(format!("Entity '{name}' not defined")));
            }
            if let Some(owner) = owner {
                refs.entry(owner).or_default().push(name);
            }
        }
    }

    // A cycle in the reference graph is a well-formedness error, not something
    // to discover by running out of depth. One depth-first pass with three
    // colours visits each entity and reference once. The previous form
    // walked the graph afresh from every entity, which is quadratic: a chain
    // of 20,000 entities took 38 s to load.
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Active,
        Done,
    }
    let mut mark: std::collections::HashMap<&str, Mark> =
        std::collections::HashMap::with_capacity(dtd.entities.len());
    let mut names: Vec<&str> = dtd.entities.keys().map(String::as_str).collect();
    // HashMap order is random; sorting makes the reported error repeatable.
    names.sort_unstable();
    for start in names {
        if mark.contains_key(start) {
            continue;
        }
        // (entity, index of the next reference to follow)
        let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
        mark.insert(start, Mark::Active);
        while let Some(top) = stack.last_mut() {
            let (cur, idx) = *top;
            match refs.get(cur).and_then(|v| v.get(idx)) {
                Some(n) => {
                    top.1 += 1;
                    match mark.get(n.as_str()) {
                        Some(Mark::Active) => {
                            return Err(XmlError::new(
                                crate::error::XML_ERR_ENTITY_LOOP,
                                "Detected an entity reference loop",
                                0,
                                0,
                            ));
                        }
                        Some(Mark::Done) => {}
                        None => {
                            mark.insert(n.as_str(), Mark::Active);
                            stack.push((n.as_str(), 0));
                        }
                    }
                }
                None => {
                    mark.insert(cur, Mark::Done);
                    stack.pop();
                }
            }
        }
    }
    Ok(())
}

/// The general entity references in a literal, as `&name;` occurrences.
fn entity_refs_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Inside a CDATA section an ampersand is an ampersand. Scanning straight
    // through one reported `<!ENTITY e "<![CDATA[&foo;]]>">` as referencing an
    // undeclared entity that it does not reference at all.
    let mut rest = text;
    let mut scan = String::new();
    while let Some(i) = rest.find("<![CDATA[") {
        scan.push_str(&rest[..i]);
        rest = &rest[i + 9..];
        match rest.find("]]>") {
            Some(e) => rest = &rest[e + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    scan.push_str(rest);
    let text = scan.as_str();
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        if c != '&' || it.peek() == Some(&'#') {
            continue;
        }
        let mut name = String::new();
        while let Some(&d) = it.peek() {
            if crate::chvalid::xml_is_name_char(d as u32, false) {
                name.push(d);
                it.next();
            } else {
                break;
            }
        }
        if !name.is_empty() && it.peek() == Some(&';') {
            it.next();
            out.push(name);
        }
    }
    out
}
