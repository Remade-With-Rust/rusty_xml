//! XML Schema (XSD) subset: elements, attributes, sequence, choice, simple types.

use rusty_xml_parser::{default_parse_options, xml_read_memory};
use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};
use std::collections::HashMap;

const XS: &str = "http://www.w3.org/2001/XMLSchema";

#[derive(Clone, Debug)]
struct ElementDecl {
    name: String,
    min: u32,
    max: u32,
    attrs: Vec<AttrDecl>,
    content: Content,
}

#[derive(Clone, Debug)]
struct AttrDecl {
    name: String,
    required: bool,
}

#[derive(Clone, Debug)]
enum Content {
    Any,
    Empty,
    Text,
    Sequence(Vec<ElementDecl>),
    Choice(Vec<ElementDecl>),
}

struct Schema {
    elements: HashMap<String, ElementDecl>,
}

/// `xmlSchemaValidateDoc`.
#[doc(alias = "xmlSchemaValidateDoc")]
pub fn xml_schema_validate_doc(xsd: &[u8], doc: &XmlDoc) -> Result<(), String> {
    let sdoc = xml_read_memory(xsd, None, None, default_parse_options()).map_err(|e| e.to_string())?;
    let schema = compile(&sdoc)?;
    let root = doc.xml_doc_get_root_element().ok_or("no root")?;
    let name = doc.name(root);
    let decl = schema
        .elements
        .get(name)
        .ok_or_else(|| format!("element {name} not declared"))?;
    check_element(doc, root, decl, &schema)
}

fn is_xs(doc: &XmlDoc, id: NodeId, name: &str) -> bool {
    doc.kind(id) == NodeKind::Element
        && doc.name(id) == name
        && (doc.ns_uri(id) == Some(XS) || doc.ns_uri(id).is_none())
}

fn compile(doc: &XmlDoc) -> Result<Schema, String> {
    let root = doc.xml_doc_get_root_element().ok_or("empty schema")?;
    let mut elements = HashMap::new();
    let mut c = doc.first_child(root);
    while let Some(x) = c {
        if is_xs(doc, x, "element") {
            if let Some(d) = compile_element(doc, x) {
                elements.insert(d.name.clone(), d);
            }
        }
        c = doc.next_sibling(x);
    }
    Ok(Schema { elements })
}

fn compile_element(doc: &XmlDoc, id: NodeId) -> Option<ElementDecl> {
    let name = doc.xml_get_prop(id, "name")?;
    let min = doc
        .xml_get_prop(id, "minOccurs")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let max = match doc.xml_get_prop(id, "maxOccurs").as_deref() {
        Some("unbounded") => u32::MAX,
        Some(s) => s.parse().unwrap_or(1),
        None => 1,
    };
    let mut attrs = Vec::new();
    let mut content = Content::Text;
    if let Some(ct) = find(doc, id, "complexType") {
        content = Content::Empty;
        let mut ch = doc.first_child(ct);
        while let Some(x) = ch {
            if is_xs(doc, x, "sequence") {
                content = Content::Sequence(child_elements(doc, x));
            } else if is_xs(doc, x, "choice") {
                content = Content::Choice(child_elements(doc, x));
            } else if is_xs(doc, x, "all") {
                content = Content::Sequence(child_elements(doc, x));
            } else if is_xs(doc, x, "attribute") {
                if let Some(n) = doc.xml_get_prop(x, "name") {
                    let required = doc.xml_get_prop(x, "use").as_deref() == Some("required");
                    attrs.push(AttrDecl { name: n, required });
                }
            } else if is_xs(doc, x, "simpleContent") || is_xs(doc, x, "complexContent") {
                content = Content::Any;
            }
            ch = doc.next_sibling(x);
        }
    } else if find(doc, id, "simpleType").is_some() {
        content = Content::Text;
    }
    Some(ElementDecl {
        name,
        min,
        max,
        attrs,
        content,
    })
}

fn child_elements(doc: &XmlDoc, id: NodeId) -> Vec<ElementDecl> {
    let mut v = Vec::new();
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        if is_xs(doc, x, "element") {
            if let Some(d) = compile_element(doc, x) {
                v.push(d);
            } else if let Some(r) = doc.xml_get_prop(x, "ref") {
                let local = r.rsplit(':').next().unwrap_or(&r).to_string();
                let min = doc
                    .xml_get_prop(x, "minOccurs")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
                let max = match doc.xml_get_prop(x, "maxOccurs").as_deref() {
                    Some("unbounded") => u32::MAX,
                    Some(s) => s.parse().unwrap_or(1),
                    None => 1,
                };
                v.push(ElementDecl {
                    name: local,
                    min,
                    max,
                    attrs: vec![],
                    content: Content::Any,
                });
            }
        }
        c = doc.next_sibling(x);
    }
    v
}

fn find(doc: &XmlDoc, id: NodeId, name: &str) -> Option<NodeId> {
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        if is_xs(doc, x, name) {
            return Some(x);
        }
        c = doc.next_sibling(x);
    }
    None
}

fn check_element(doc: &XmlDoc, id: NodeId, decl: &ElementDecl, schema: &Schema) -> Result<(), String> {
    for a in &decl.attrs {
        if a.required && doc.xml_get_prop(id, &a.name).is_none() {
            return Err(format!("required attribute {} missing", a.name));
        }
    }
    let kids: Vec<NodeId> = {
        let mut v = Vec::new();
        let mut c = doc.first_child(id);
        while let Some(x) = c {
            if doc.kind(x) == NodeKind::Element {
                v.push(x);
            }
            c = doc.next_sibling(x);
        }
        v
    };
    match &decl.content {
        Content::Any | Content::Text => {}
        Content::Empty => {
            if !kids.is_empty() {
                return Err(format!("{} must be empty", decl.name));
            }
        }
        Content::Sequence(seq) => {
            let mut i = 0;
            for part in seq {
                let mut seen = 0u32;
                while i < kids.len() && doc.name(kids[i]) == part.name {
                    let sub = schema.elements.get(&part.name).unwrap_or(part);
                    check_element(doc, kids[i], sub, schema)?;
                    i += 1;
                    seen += 1;
                    if seen == part.max {
                        break;
                    }
                }
                if seen < part.min {
                    return Err(format!("need {} of {}", part.min, part.name));
                }
            }
            if i != kids.len() {
                return Err("extra children in sequence".into());
            }
        }
        Content::Choice(alts) => {
            if kids.len() != 1 {
                return Err("choice expects one child".into());
            }
            let n = doc.name(kids[0]);
            let part = alts.iter().find(|a| a.name == n).ok_or("choice mismatch")?;
            let sub = schema.elements.get(&part.name).unwrap_or(part);
            check_element(doc, kids[0], sub, schema)?;
        }
    }
    Ok(())
}
