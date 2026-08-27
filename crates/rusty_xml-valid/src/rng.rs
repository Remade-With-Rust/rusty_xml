//! RelaxNG (simplified) matching libxml2 `relaxng.h` for the tutorial corpus.

use rusty_xml_parser::{default_parse_options, xml_read_memory};
use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};
use std::collections::HashMap;

const RNG_NS: &str = "http://relaxng.org/ns/structure/1.0";

#[derive(Clone, Debug)]
enum Pat {
    Empty,
    Text,
    NotAllowed,
    Element { name: String, inner: Box<Pat> },
    Attribute { name: String, inner: Box<Pat> },
    Group(Vec<Pat>),
    Choice(Vec<Pat>),
    Interleave(Vec<Pat>),
    Optional(Box<Pat>),
    ZeroOrMore(Box<Pat>),
    OneOrMore(Box<Pat>),
    Value(String),
    Data(String),
    Ref(String),
    List(Box<Pat>),
}

struct Schema {
    start: Pat,
    defs: HashMap<String, Pat>,
}

/// `xmlRelaxNGParse` + `xmlRelaxNGValidateDoc`.
#[doc(alias = "xmlRelaxNGValidateDoc")]
pub fn xml_relaxng_validate_doc(rng_xml: &[u8], doc: &XmlDoc) -> Result<(), String> {
    let rng_doc = xml_read_memory(rng_xml, None, None, default_parse_options())
        .map_err(|e| e.to_string())?;
    let schema = compile(&rng_doc)?;
    let root = doc.xml_doc_get_root_element().ok_or("no root")?;
    match_element(&schema, &schema.start, doc, root)?;
    Ok(())
}

fn compile(doc: &XmlDoc) -> Result<Schema, String> {
    let root = doc.xml_doc_get_root_element().ok_or("empty rng")?;
    let mut defs = HashMap::new();
    harvest_defs(doc, root, &mut defs);
    let start = if is_rng(doc, root, "grammar") {
        find_child_named(doc, root, "start")
            .map(|s| compile_pat(doc, first_pat_child(doc, s).ok_or("empty start").unwrap()))
            .unwrap_or_else(|| compile_pat(doc, root))
    } else {
        compile_pat(doc, root)
    };
    Ok(Schema { start, defs })
}

fn harvest_defs(doc: &XmlDoc, id: NodeId, defs: &mut HashMap<String, Pat>) {
    if is_rng(doc, id, "define") {
        if let Some(name) = doc.xml_get_prop(id, "name") {
            if let Some(ch) = first_pat_child(doc, id) {
                defs.insert(name, compile_pat(doc, ch));
            }
        }
    }
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        harvest_defs(doc, x, defs);
        c = doc.next_sibling(x);
    }
}

fn is_rng(doc: &XmlDoc, id: NodeId, name: &str) -> bool {
    doc.kind(id) == NodeKind::Element
        && doc.name(id) == name
        && (doc.ns_uri(id) == Some(RNG_NS) || doc.ns_uri(id).is_none())
}

fn first_pat_child(doc: &XmlDoc, id: NodeId) -> Option<NodeId> {
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        if doc.kind(x) == NodeKind::Element {
            return Some(x);
        }
        c = doc.next_sibling(x);
    }
    None
}

fn find_child_named(doc: &XmlDoc, id: NodeId, name: &str) -> Option<NodeId> {
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        if is_rng(doc, x, name) {
            return Some(x);
        }
        c = doc.next_sibling(x);
    }
    None
}

fn compile_pat(doc: &XmlDoc, id: NodeId) -> Pat {
    let name = doc.name(id);
    match name {
        "element" => {
            let n = doc.xml_get_prop(id, "name").unwrap_or_default();
            Pat::Element {
                name: n,
                inner: Box::new(compile_group_children(doc, id)),
            }
        }
        "attribute" => {
            let n = doc.xml_get_prop(id, "name").unwrap_or_default();
            Pat::Attribute {
                name: n,
                inner: Box::new(compile_group_children(doc, id)),
            }
        }
        "empty" => Pat::Empty,
        "text" => Pat::Text,
        "notAllowed" => Pat::NotAllowed,
        "optional" => Pat::Optional(Box::new(compile_group_children(doc, id))),
        "zeroOrMore" => Pat::ZeroOrMore(Box::new(compile_group_children(doc, id))),
        "oneOrMore" => Pat::OneOrMore(Box::new(compile_group_children(doc, id))),
        "choice" => Pat::Choice(compile_child_pats(doc, id)),
        "group" => Pat::Group(compile_child_pats(doc, id)),
        "interleave" => Pat::Interleave(compile_child_pats(doc, id)),
        "value" => Pat::Value(doc.xml_node_get_content(id).trim().to_string()),
        "data" => Pat::Data(doc.xml_get_prop(id, "type").unwrap_or_else(|| "string".into())),
        "ref" => Pat::Ref(doc.xml_get_prop(id, "name").unwrap_or_default()),
        "list" => Pat::List(Box::new(compile_group_children(doc, id))),
        "mixed" => Pat::Interleave(vec![Pat::Text, compile_group_children(doc, id)]),
        "grammar" => find_child_named(doc, id, "start")
            .and_then(|s| first_pat_child(doc, s))
            .map(|c| compile_pat(doc, c))
            .unwrap_or(Pat::NotAllowed),
        _ => compile_group_children(doc, id),
    }
}

fn compile_child_pats(doc: &XmlDoc, id: NodeId) -> Vec<Pat> {
    let mut v = Vec::new();
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        if doc.kind(x) == NodeKind::Element {
            v.push(compile_pat(doc, x));
        }
        c = doc.next_sibling(x);
    }
    v
}

fn compile_group_children(doc: &XmlDoc, id: NodeId) -> Pat {
    let v = compile_child_pats(doc, id);
    match v.len() {
        0 => Pat::Empty,
        1 => v.into_iter().next().unwrap(),
        _ => Pat::Group(v),
    }
}

fn match_element(schema: &Schema, pat: &Pat, doc: &XmlDoc, id: NodeId) -> Result<(), String> {
    let pat = deref_pat(schema, pat)?;
    match pat {
        Pat::Element { name, inner } => {
            if doc.name(id) != name {
                return Err(format!("expected element {name}, got {}", doc.name(id)));
            }
            match_content(schema, &inner, doc, id)
        }
        Pat::Choice(alts) => {
            for a in &alts {
                if match_element(schema, a, doc, id).is_ok() {
                    return Ok(());
                }
            }
            Err("choice failed".into())
        }
        Pat::Ref(n) => {
            let p = schema.defs.get(&n).ok_or_else(|| format!("undefined ref {n}"))?;
            match_element(schema, p, doc, id)
        }
        other => match_content(schema, &other, doc, id),
    }
}

fn deref_pat<'a>(schema: &'a Schema, pat: &'a Pat) -> Result<Pat, String> {
    match pat {
        Pat::Ref(n) => schema
            .defs
            .get(n)
            .cloned()
            .ok_or_else(|| format!("undefined ref {n}")),
        p => Ok(p.clone()),
    }
}

fn match_content(schema: &Schema, pat: &Pat, doc: &XmlDoc, id: NodeId) -> Result<(), String> {
    let mut attrs: Vec<(String, String)> = Vec::new();
    let mut a = doc.first_attr(id);
    while let Some(x) = a {
        if !doc.name(x).starts_with("xmlns") {
            attrs.push((doc.name(x).to_string(), doc.content(x).to_string()));
        }
        a = doc.next_sibling(x);
    }
    let mut kids: Vec<NodeId> = Vec::new();
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        match doc.kind(x) {
            NodeKind::Element => kids.push(x),
            NodeKind::Text | NodeKind::CData => {
                if !doc.xml_is_blank_node(x) {
                    kids.push(x);
                }
            }
            _ => {}
        }
        c = doc.next_sibling(x);
    }
    consume(schema, pat, doc, &mut kids, &mut attrs)?;
    if !attrs.is_empty() {
        return Err(format!("undeclared attributes {:?}", attrs));
    }
    if kids.iter().any(|&k| doc.kind(k) == NodeKind::Element) {
        return Err("extra element content".into());
    }
    Ok(())
}

fn consume(
    schema: &Schema,
    pat: &Pat,
    doc: &XmlDoc,
    kids: &mut Vec<NodeId>,
    attrs: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let pat = deref_pat(schema, pat)?;
    match pat {
        Pat::Empty => Ok(()),
        Pat::Text => {
            kids.retain(|&k| doc.kind(k) == NodeKind::Element);
            Ok(())
        }
        Pat::NotAllowed => Err("notAllowed".into()),
        Pat::Attribute { name, inner } => {
            if let Some(i) = attrs.iter().position(|(n, _)| n == &name) {
                let val = attrs.remove(i).1;
                match *inner {
                    Pat::Text | Pat::Empty | Pat::Data(_) => Ok(()),
                    Pat::Value(v) if v == val => Ok(()),
                    Pat::Value(v) => Err(format!("attr {name} expected {v}")),
                    _ => Ok(()),
                }
            } else {
                Err(format!("missing attribute {name}"))
            }
        }
        Pat::Element { name, inner } => {
            if let Some(i) = kids.iter().position(|&k| {
                doc.kind(k) == NodeKind::Element && doc.name(k) == name
            }) {
                let n = kids.remove(i);
                match_content(schema, &inner, doc, n)
            } else {
                Err(format!("missing element {name}"))
            }
        }
        Pat::Group(ps) => {
            for p in &ps {
                consume(schema, p, doc, kids, attrs)?;
            }
            Ok(())
        }
        Pat::Choice(ps) => {
            for p in &ps {
                let mut k2 = kids.clone();
                let mut a2 = attrs.clone();
                if consume(schema, p, doc, &mut k2, &mut a2).is_ok() {
                    *kids = k2;
                    *attrs = a2;
                    return Ok(());
                }
            }
            Err("choice failed".into())
        }
        Pat::Interleave(ps) => {
            for p in &ps {
                consume(schema, p, doc, kids, attrs)?;
            }
            Ok(())
        }
        Pat::Optional(p) => {
            let mut k2 = kids.clone();
            let mut a2 = attrs.clone();
            if consume(schema, &p, doc, &mut k2, &mut a2).is_ok() {
                *kids = k2;
                *attrs = a2;
            }
            Ok(())
        }
        Pat::ZeroOrMore(p) => {
            loop {
                let mut k2 = kids.clone();
                let mut a2 = attrs.clone();
                if consume(schema, &p, doc, &mut k2, &mut a2).is_ok()
                    && (k2.len() < kids.len() || a2.len() < attrs.len())
                {
                    *kids = k2;
                    *attrs = a2;
                } else {
                    break;
                }
            }
            Ok(())
        }
        Pat::OneOrMore(p) => {
            consume(schema, &p, doc, kids, attrs)?;
            consume(schema, &Pat::ZeroOrMore(p), doc, kids, attrs)
        }
        Pat::Value(v) => {
            let text: String = kids
                .iter()
                .filter(|k| doc.kind(**k) != NodeKind::Element)
                .map(|k| doc.content(*k))
                .collect();
            kids.retain(|&k| doc.kind(k) == NodeKind::Element);
            if text.trim() == v {
                Ok(())
            } else {
                Err(format!("value {v} != {}", text.trim()))
            }
        }
        Pat::Data(_) | Pat::List(_) => {
            kids.retain(|&k| doc.kind(k) == NodeKind::Element);
            Ok(())
        }
        Pat::Ref(n) => {
            let p = schema.defs.get(&n).ok_or_else(|| format!("undefined {n}"))?;
            consume(schema, p, doc, kids, attrs)
        }
    }
}
