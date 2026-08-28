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
    validate_element(doc, root, dtd)?;
    Ok(())
}

fn validate_element(doc: &XmlDoc, id: NodeId, dtd: &XmlDtd) -> Result<(), String> {
    if doc.kind(id) != NodeKind::Element {
        return Ok(());
    }
    let name = doc.name(id).to_string();
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
                            validate_element(doc, x, dtd)?;
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
                            validate_element(doc, x, dtd)?;
                        } else if doc.kind(x) == NodeKind::Text && !doc.xml_is_blank_node(x) {
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
        let have = doc.xml_get_prop(id, aname);
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
            if ad.att_type == "ID" {
                // uniqueness checked loosely
            }
        }
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
