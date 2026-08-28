//! DTD validation, C14N, RelaxNG, XML Schema, Schematron.

#![forbid(unsafe_code)]

mod c14n;
mod rng;
mod xsd;
mod schematron;

use rusty_xml_tree::{AttrDefault, ElementDecl, NodeId, NodeKind, XmlDoc, XmlDtd};

pub use c14n::*;
pub use rng::*;
pub use xsd::*;
pub use schematron::*;

/// `xmlValidateDocument` against the document's attached DTD.
#[doc(alias = "xmlValidateDocument")]
pub fn xml_validate_document(doc: &XmlDoc) -> Result<(), String> {
    let dtd = doc.dtd.as_ref().ok_or("no DTD")?;
    xml_validate_dtd(doc, dtd)
}

/// `xmlValidateDtd`.
#[doc(alias = "xmlValidateDtd")]
pub fn xml_validate_dtd(doc: &XmlDoc, dtd: &XmlDtd) -> Result<(), String> {
    let root = doc
        .xml_doc_get_root_element()
        .ok_or("document has no root")?;
    if let Some(n) = &dtd.name {
        if doc.name(root) != n.as_str() {
            return Err(format!("root element {} does not match DOCTYPE {n}", doc.name(root)));
        }
    }
    // Constraints on the DECLARATIONS themselves, before any instance of them
    // is looked at. None of these was checked, so a declaration could promise
    // something no document could satisfy.
    // "Unique Element Type Declaration": no element type may be declared more
    // than once. A HashMap cannot say so on its own, so the parser records it.
    if let Some(dup) = dtd.duplicate_elements.first() {
        return Err(format!("Redefinition of element {dup}"));
    }
    for ((elem, aname), ad) in &dtd.attributes {
        // "ID Attribute Default": an ID attribute must be #IMPLIED or
        // #REQUIRED -- it cannot carry a default value, since two elements
        // taking the default would share an ID.
        if ad.att_type == "ID"
            && matches!(ad.default, AttrDefault::Value | AttrDefault::Fixed)
        {
            return Err(format!(
                "ID attribute {aname} of {elem} is not valid, must be #IMPLIED or #REQUIRED"
            ));
        }
        // "No Notation on Empty Element": an element declared EMPTY cannot
        // carry a NOTATION attribute, since it can never have content for the
        // notation to describe.
        if ad.att_type == "NOTATION" && matches!(dtd.elements.get(elem), Some(ElementDecl::Empty)) {
            return Err(format!(
                "NOTATION attribute type declared for EMPTY element {elem}"
            ));
        }
        // "Attribute Default Value Syntactically Correct": the default has to
        // satisfy the type it is declared with.
        let Some(def) = ad.default_value.as_deref() else {
            continue;
        };
        let ok = match ad.att_type.as_str() {
            "ID" | "IDREF" | "ENTITY" => is_name(def),
            "IDREFS" | "ENTITIES" => {
                def.split_ascii_whitespace().next().is_some()
                    && def.split_ascii_whitespace().all(is_name)
            }
            "NMTOKEN" => is_nmtoken(def),
            "NMTOKENS" => {
                def.split_ascii_whitespace().next().is_some()
                    && def.split_ascii_whitespace().all(is_nmtoken)
            }
            _ => true,
        };
        if !ok {
            return Err(format!("invalid default value for attribute {aname} of {elem}"));
        }
        if !ad.enumerated.is_empty() && !ad.enumerated.iter().any(|e| e == def) {
            return Err(format!("invalid default value for attribute {aname} of {elem}"));
        }
    }

    // Walk iteratively: validation used to recurse per element, which is the
    // same stack cliff the parser, the writer and C14N all had.
    let mut ids: std::collections::HashMap<String, ()> = Default::default();
    let mut idrefs: Vec<String> = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        validate_element(doc, id, dtd, &mut ids, &mut idrefs)?;
        let mut c = doc.last_child(id);
        while let Some(x) = c {
            if doc.kind(x) == NodeKind::Element {
                stack.push(x);
            }
            c = doc.prev_sibling(x);
        }
    }
    // IDREF VC: every referenced ID must be declared somewhere in the
    // document. This was never checked at all.
    for r in &idrefs {
        if !ids.contains_key(r) {
            return Err(format!("IDREF attribute references unknown ID \"{r}\""));
        }
    }
    Ok(())
}

/// A Name, as the ID / IDREF validity constraints require.
fn is_name(v: &str) -> bool {
    let mut cs = v.chars();
    match cs.next() {
        Some(c) if rusty_xml_parser::chvalid::xml_is_name_start_char(c as u32, false) => {}
        _ => return false,
    }
    cs.all(|c| rusty_xml_parser::chvalid::xml_is_name_char(c as u32, false))
}

/// An Nmtoken: like a Name but with no restriction on the first character.
fn is_nmtoken(v: &str) -> bool {
    !v.is_empty()
        && v.chars()
            .all(|c| rusty_xml_parser::chvalid::xml_is_name_char(c as u32, false))
}

fn validate_element(
    doc: &XmlDoc,
    id: NodeId,
    dtd: &XmlDtd,
    ids: &mut std::collections::HashMap<String, ()>,
    idrefs: &mut Vec<String>,
) -> Result<(), String> {
    if doc.kind(id) != NodeKind::Element {
        return Ok(());
    }
    let name = doc.name(id).to_string();
    // "Element Valid": an element with no declaration is invalid, and nothing
    // said so. A DTD that declares nothing at all is not a validating DTD, so
    // only complain when there are declarations to be missing from.
    if !dtd.elements.is_empty() && !dtd.elements.contains_key(&name) {
        return Err(format!("No declaration for element {name}"));
    }
    if let Some(decl) = dtd.elements.get(&name) {
        match decl {
            ElementDecl::Empty => {
                if doc.first_child(id).is_some() {
                    return Err(format!("element {name} must be EMPTY"));
                }
            }
            ElementDecl::Any => {}
            ElementDecl::Mixed(_) => {
                let mut c = doc.first_child(id);
                while let Some(x) = c {
                    match doc.kind(x) {
                        NodeKind::Element => {
                            if let ElementDecl::Mixed(allowed) = decl {
                                if !allowed.is_empty() && !allowed.iter().any(|n| n == doc.name(x)) {
                                    return Err(format!("element {} not allowed in mixed {name}", doc.name(x)));
                                }
                            }
                        }
                        NodeKind::Text | NodeKind::CData | NodeKind::Comment | NodeKind::Pi => {}
                        _ => {}
                    }
                    c = doc.next_sibling(x);
                }
            }
            ElementDecl::Children(spec) => {
                let kids: Vec<String> = {
                    let mut v = Vec::new();
                    let mut c = doc.first_child(id);
                    while let Some(x) = c {
                        if doc.kind(x) == NodeKind::Element {
                            v.push(doc.name(x).to_string());
                        } else if doc.kind(x) == NodeKind::Text && !doc.xml_is_blank_node(x) {
                            return Err(format!("character data not allowed in {name}"));
                        } else if doc.kind(x) == NodeKind::CData {
                            // A CDATA section is character data whatever is in
                            // it. Whitespace inside one is never the ignorable
                            // kind, so an empty `<![CDATA[]]>` still breaks an
                            // element-only content model -- and we were only
                            // looking at Text nodes.
                            return Err(format!("character data not allowed in {name}"));
                        }
                        c = doc.next_sibling(x);
                    }
                    v
                };
                if !match_children_spec(spec, &kids) {
                    return Err(format!("content of {name} does not match {spec}"));
                }
            }
        }
    }
    for ((elem, aname), ad) in &dtd.attributes {
        if elem != &name {
            continue;
        }
        // An ATTLIST declares a QName, so `xml:lang` is looked up as
        // `xml:lang`. xml_get_prop is xmlGetProp -- it matches unprefixed
        // attributes only -- so every prefixed declared attribute looked
        // absent, and a #REQUIRED one was reported missing on a document that
        // plainly had it.
        let have = {
            let mut found = None;
            let mut a = doc.first_attr(id);
            while let Some(x) = a {
                if doc.qname(x) == *aname {
                    found = Some(doc.content(x).to_string());
                    break;
                }
                a = doc.next_sibling(x);
            }
            found
        };
        match ad.default {
            AttrDefault::Required if have.is_none() => {
                return Err(format!("attribute {aname} of {name} is required"));
            }
            AttrDefault::Fixed => {
                if let (Some(v), Some(fix)) = (&have, &ad.default_value) {
                    if v != fix {
                        return Err(format!("attribute {aname} must be {fix}"));
                    }
                }
            }
            _ => {}
        }
        if let Some(v) = &have {
            if !ad.enumerated.is_empty() && !ad.enumerated.iter().any(|e| e == v) {
                return Err(format!("attribute {aname} value not in enumeration"));
            }
            // The tokenized types carry validity constraints on their VALUES,
            // and not one of them was enforced -- the ID branch said
            // "uniqueness checked loosely", which meant not at all.
            match ad.att_type.as_str() {
                "ID" | "IDREF" => {
                    if !is_name(v) {
                        return Err(format!(
                            "Syntax of value for attribute {aname} of {name} is not valid"
                        ));
                    }
                    if ad.att_type == "ID" {
                        if ids.insert(v.clone(), ()).is_some() {
                            return Err(format!("ID {v} already defined"));
                        }
                    } else {
                        idrefs.push(v.clone());
                    }
                }
                "IDREFS" => {
                    let mut any = false;
                    for part in v.split_ascii_whitespace() {
                        any = true;
                        if !is_name(part) {
                            return Err(format!(
                                "Syntax of value for attribute {aname} of {name} is not valid"
                            ));
                        }
                        idrefs.push(part.to_string());
                    }
                    if !any {
                        return Err(format!(
                            "Syntax of value for attribute {aname} of {name} is not valid"
                        ));
                    }
                }
                "NMTOKEN" => {
                    if !is_nmtoken(v) {
                        return Err(format!(
                            "Syntax of value for attribute {aname} of {name} is not valid"
                        ));
                    }
                }
                "NMTOKENS" => {
                    if v.split_ascii_whitespace().next().is_none()
                        || !v.split_ascii_whitespace().all(is_nmtoken)
                    {
                        return Err(format!(
                            "Syntax of value for attribute {aname} of {name} is not valid"
                        ));
                    }
                }
                "ENTITY" | "ENTITIES" => {
                    for part in v.split_ascii_whitespace() {
                        if !is_name(part) {
                            return Err(format!(
                                "Syntax of value for attribute {aname} of {name} is not valid"
                            ));
                        }
                        if !dtd.unparsed_entities.contains(part) {
                            return Err(format!(
                                "ENTITY attribute {aname} references an unknown entity \"{part}\""
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // "One ID per Element Type": an element type may carry at most one ID
    // attribute, however the declarations are spread across ATTLISTs.
    let id_attrs = dtd
        .attributes
        .iter()
        .filter(|((e, _), ad)| e == &name && ad.att_type == "ID")
        .count();
    if id_attrs > 1 {
        return Err(format!("Element {name} has {id_attrs} ID attributes"));
    }
    Ok(())
}

fn match_children_spec(spec: &str, kids: &[String]) -> bool {
    let toks = tokenize_content(spec);
    match_seq(&toks, kids, 0).contains(&kids.len())
}

#[derive(Clone, Debug)]
enum Tok {
    Name(String),
    Seq(Vec<Tok>),
    Choice(Vec<Tok>),
    Star,
    Plus,
    Q,
}

fn tokenize_content(spec: &str) -> Vec<Tok> {
    // Very small content-model parser: names, ',', '|', '*+?', parentheses.
    let p = spec.trim();
    fn parse_choice<'a>(p: &mut &'a str) -> Vec<Tok> {
        let mut alts = vec![Tok::Seq(parse_seq(p))];
        loop {
            skip(p);
            if p.starts_with('|') {
                *p = &p[1..];
                alts.push(Tok::Seq(parse_seq(p)));
            } else {
                break;
            }
        }
        alts
    }
    fn parse_seq<'a>(p: &mut &'a str) -> Vec<Tok> {
        let mut v = Vec::new();
        loop {
            skip(p);
            if p.is_empty() || p.starts_with('|') || p.starts_with(')') {
                break;
            }
            if p.starts_with(',') {
                *p = &p[1..];
                continue;
            }
            // A particle that consumes nothing is the end of what we can
            // read, not a reason to try again.
            //
            // `<!ELEMENT doc (a & b)?>` used SGML's "and" connector: `&` is
            // not a name character, so take_name returned "" and the position
            // never moved. This loop pushed an empty Name forever -- 32 bytes
            // of DTD grew a Vec until the process died asking for 32 GB. Any
            // document with a DTD could do it to anything that validates.
            let before = p.len();
            let particle = parse_particle(p);
            if p.len() == before {
                break;
            }
            v.push(particle);
        }
        v
    }
    fn parse_particle<'a>(p: &mut &'a str) -> Tok {
        skip(p);
        let mut inner = if p.starts_with('(') {
            *p = &p[1..];
            let c = parse_choice(p);
            skip(p);
            if p.starts_with(')') {
                *p = &p[1..];
            }
            if c.len() == 1 {
                Tok::Seq(match c.into_iter().next().unwrap() {
                    Tok::Seq(s) => s,
                    other => vec![other],
                })
            } else {
                Tok::Choice(c)
            }
        } else {
            let name = take_name(p);
            Tok::Name(name)
        };
        skip(p);
        inner = match p.chars().next() {
            Some('*') => {
                *p = &p[1..];
                Tok::Seq(vec![inner, Tok::Star])
            }
            Some('+') => {
                *p = &p[1..];
                Tok::Seq(vec![inner, Tok::Plus])
            }
            Some('?') => {
                *p = &p[1..];
                Tok::Seq(vec![inner, Tok::Q])
            }
            _ => inner,
        };
        inner
    }
    fn take_name<'a>(p: &mut &'a str) -> String {
        let bytes = p.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c.is_ascii_alphanumeric() || "-._:".contains(c) {
                i += 1;
            } else {
                break;
            }
        }
        let s = p[..i].to_string();
        *p = &p[i..];
        s
    }
    fn skip(p: &mut &str) {
        *p = p.trim_start();
    }
    let mut tmp = p;
    parse_choice(&mut tmp)
}

fn match_seq(toks: &[Tok], kids: &[String], i: usize) -> Vec<usize> {
    if toks.is_empty() {
        return vec![i];
    }
    match &toks[0] {
        Tok::Star => {
            let rest = &toks[1..];
            // Star applies to previous — encoded as Seq(inner, Star). Handle Seq instead.
            match_seq(rest, kids, i)
        }
        Tok::Plus | Tok::Q => match_seq(&toks[1..], kids, i),
        Tok::Name(n) => {
            if i < kids.len() && &kids[i] == n {
                match_seq(&toks[1..], kids, i + 1)
            } else {
                vec![]
            }
        }
        Tok::Seq(inner) => {
            let (body, quant) = split_quant(inner);
            apply_quant(body, quant, &toks[1..], kids, i)
        }
        Tok::Choice(alts) => {
            let mut out = Vec::new();
            for a in alts {
                let one = match_seq(&[a.clone()], kids, i);
                for pos in one {
                    out.extend(match_seq(&toks[1..], kids, pos));
                }
            }
            out.sort();
            out.dedup();
            out
        }
    }
}

enum Quant {
    One,
    Q,
    Star,
    Plus,
}

fn split_quant(inner: &[Tok]) -> (&[Tok], Quant) {
    if inner.len() >= 2 {
        match inner.last() {
            Some(Tok::Star) => return (&inner[..inner.len() - 1], Quant::Star),
            Some(Tok::Plus) => return (&inner[..inner.len() - 1], Quant::Plus),
            Some(Tok::Q) => return (&inner[..inner.len() - 1], Quant::Q),
            _ => {}
        }
    }
    (inner, Quant::One)
}

fn apply_quant(body: &[Tok], q: Quant, rest: &[Tok], kids: &[String], i: usize) -> Vec<usize> {
    match q {
        Quant::One => {
            let mut out = Vec::new();
            for p in match_seq(body, kids, i) {
                out.extend(match_seq(rest, kids, p));
            }
            out
        }
        Quant::Q => {
            let mut out = match_seq(rest, kids, i);
            for p in match_seq(body, kids, i) {
                out.extend(match_seq(rest, kids, p));
            }
            out.sort();
            out.dedup();
            out
        }
        Quant::Star => {
            let mut out = match_seq(rest, kids, i);
            let mut frontier = vec![i];
            while let Some(p) = frontier.pop() {
                for n in match_seq(body, kids, p) {
                    if n > p {
                        out.extend(match_seq(rest, kids, n));
                        frontier.push(n);
                    }
                }
            }
            out.sort();
            out.dedup();
            out
        }
        Quant::Plus => {
            let mut out = Vec::new();
            for p in match_seq(body, kids, i) {
                out.extend(apply_quant(body, Quant::Star, rest, kids, p));
            }
            out.sort();
            out.dedup();
            out
        }
    }
}
