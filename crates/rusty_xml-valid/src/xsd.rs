//! XML Schema (XSD) subset: elements, attributes, sequence, choice, simple types.

use crate::interrupt::{NoPoll, Poll};
use crate::ValidError;
use rusty_xml_parser::{default_parse_options, xml_read_memory};
use rusty_xml_tree::{Interrupt, InterruptPoll, NodeId, NodeKind, XmlDoc};
use std::collections::HashMap;

/// How deeply the schema may nest. Compiling recurses once per level.
const MAX_SCHEMA_DEPTH: usize = 512;

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
    schema_validate(xsd, doc, &mut NoPoll).map_err(ValidError::into_message)
}

/// [`xml_schema_validate_doc`], stopping with [`ValidError::Interrupted`]
/// once `interrupt` fires. It is polled before and after the schema is
/// parsed, and every few hundred elements checked.
pub fn xml_schema_validate_doc_interruptible(
    xsd: &[u8],
    doc: &XmlDoc,
    interrupt: &dyn Interrupt,
) -> Result<(), ValidError> {
    schema_validate(xsd, doc, &mut InterruptPoll::new(interrupt))
}

fn schema_validate<P: Poll>(xsd: &[u8], doc: &XmlDoc, poll: &mut P) -> Result<(), ValidError> {
    poll.check()?;
    let sdoc = xml_read_memory(xsd, None, None, default_parse_options()).map_err(|e| e.to_string())?;
    poll.check()?;
    let schema = compile(&sdoc)?;
    let root = doc.xml_doc_get_root_element().ok_or("no root")?;
    let name = doc.name(root);
    let decl = schema
        .elements
        .get(name)
        .ok_or_else(|| format!("element {name} not declared"))?;
    check_element(doc, root, decl, &schema, poll)
}

fn is_xs(doc: &XmlDoc, id: NodeId, name: &str) -> bool {
    doc.kind(id) == NodeKind::Element
        && doc.name(id) == name
        && (doc.ns_uri(id) == Some(XS) || doc.ns_uri(id).is_none())
}

fn compile(doc: &XmlDoc) -> Result<Schema, String> {
    let root = doc.xml_doc_get_root_element().ok_or("empty schema")?;
    // compile_element recurses through nested complex types, once per level.
    let mut stack = vec![(root, 1usize)];
    while let Some((id, d)) = stack.pop() {
        if d > MAX_SCHEMA_DEPTH {
            return Err(format!("XML Schema nested deeper than {MAX_SCHEMA_DEPTH} levels"));
        }
        let mut c = doc.first_child(id);
        while let Some(x) = c {
            if doc.kind(x) == NodeKind::Element {
                stack.push((x, d + 1));
            }
            c = doc.next_sibling(x);
        }
    }
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

/// Check `id` and everything under it against `decl`.
///
/// Walked with an explicit stack of frames, one per open element, in the
/// order the recursive form visited them, so the first error is the same one.
/// The recursive form cost a stack frame per level, so a deep document (the
/// parser allows 5,000 levels, a million under `XML_PARSE_HUGE`) overflowed
/// the stack, which aborts the process.
fn check_element<'s, P: Poll>(
    doc: &XmlDoc,
    id: NodeId,
    decl: &'s ElementDecl,
    schema: &'s Schema,
    poll: &mut P,
) -> Result<(), ValidError> {
    enum Phase {
        Start,
        /// Inside a sequence: the part being matched, the next child, and how
        /// many children the part has taken so far.
        Seq { part: usize, i: usize, seen: u32 },
        Done,
    }
    struct Frame<'s> {
        id: NodeId,
        decl: &'s ElementDecl,
        kids: Vec<NodeId>,
        phase: Phase,
    }
    let mut stack = vec![Frame {
        id,
        decl,
        kids: Vec::new(),
        phase: Phase::Start,
    }];
    while let Some(f) = stack.last_mut() {
        match f.phase {
            Phase::Start => {
                poll.tick()?;
                let (id, decl) = (f.id, f.decl);
                for a in &decl.attrs {
                    if a.required && doc.xml_get_prop(id, &a.name).is_none() {
                        return Err(format!("required attribute {} missing", a.name).into());
                    }
                }
                let mut c = doc.first_child(id);
                while let Some(x) = c {
                    if doc.kind(x) == NodeKind::Element {
                        f.kids.push(x);
                    }
                    c = doc.next_sibling(x);
                }
                match &decl.content {
                    Content::Any | Content::Text => f.phase = Phase::Done,
                    Content::Empty => {
                        if !f.kids.is_empty() {
                            return Err(format!("{} must be empty", decl.name).into());
                        }
                        f.phase = Phase::Done;
                    }
                    Content::Sequence(_) => f.phase = Phase::Seq { part: 0, i: 0, seen: 0 },
                    Content::Choice(alts) => {
                        if f.kids.len() != 1 {
                            return Err("choice expects one child".into());
                        }
                        let kid = f.kids[0];
                        let n = doc.name(kid);
                        let part = alts.iter().find(|a| a.name == n).ok_or("choice mismatch")?;
                        let sub = schema.elements.get(&part.name).unwrap_or(part);
                        f.phase = Phase::Done;
                        stack.push(Frame {
                            id: kid,
                            decl: sub,
                            kids: Vec::new(),
                            phase: Phase::Start,
                        });
                    }
                }
            }
            Phase::Seq { part, i, seen } => {
                let Content::Sequence(seq) = &f.decl.content else {
                    f.phase = Phase::Done;
                    continue;
                };
                let Some(p) = seq.get(part) else {
                    if i != f.kids.len() {
                        return Err("extra children in sequence".into());
                    }
                    f.phase = Phase::Done;
                    continue;
                };
                // A part takes children while they carry its name, stopping
                // once it has taken maxOccurs of them.
                let full = seen > 0 && seen == p.max;
                if !full && i < f.kids.len() && doc.name(f.kids[i]) == p.name {
                    let kid = f.kids[i];
                    let sub = schema.elements.get(&p.name).unwrap_or(p);
                    f.phase = Phase::Seq { part, i: i + 1, seen: seen + 1 };
                    stack.push(Frame {
                        id: kid,
                        decl: sub,
                        kids: Vec::new(),
                        phase: Phase::Start,
                    });
                } else {
                    if seen < p.min {
                        return Err(format!("need {} of {}", p.min, p.name).into());
                    }
                    f.phase = Phase::Seq { part: part + 1, i, seen: 0 };
                }
            }
            Phase::Done => {
                stack.pop();
            }
        }
    }
    Ok(())
}
