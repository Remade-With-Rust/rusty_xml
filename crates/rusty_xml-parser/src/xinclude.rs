//! XInclude 1.0. Resource fetch is caller-supplied; the library never opens the network.

use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};
use crate::error::XmlError;
use crate::parse::{default_parse_options, xml_read_memory};

const XI: &str = "http://www.w3.org/2001/XInclude";

/// `xmlXIncludeProcess` with a caller resource loader.
#[doc(alias = "xmlXIncludeProcess")]
pub fn xml_xinclude_process<F>(doc: &mut XmlDoc, mut loader: F) -> Result<i32, XmlError>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    xml_xinclude_process_tree(doc, NodeId::DOCUMENT, &mut loader)
}

fn xml_xinclude_process_tree<F>(
    doc: &mut XmlDoc,
    start: NodeId,
    loader: &mut F,
) -> Result<i32, XmlError>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    let mut subs: Vec<NodeId> = Vec::new();
    collect_includes(doc, start, &mut subs);
    let mut n = 0i32;
    for id in subs {
        if replace_include(doc, id, loader)? {
            n += 1;
        }
    }
    Ok(n)
}

/// Every `xi:include` element under `start`, in document order, without
/// descending into one (its fallback stays nested).
///
/// Walks first-child / next-sibling / parent links rather than recursing:
/// under `XML_PARSE_HUGE` a document may nest 1,000,000 deep, and recursion
/// per level overflowed the stack at about 20,000 (#34).
fn collect_includes(doc: &XmlDoc, start: NodeId, out: &mut Vec<NodeId>) {
    let is_include = |id: NodeId| {
        doc.kind(id) == NodeKind::Element
            && doc.name(id) == "include"
            && (doc.ns_uri(id) == Some(XI) || doc.prefix(id) == Some("xi"))
    };
    let mut cur = start;
    loop {
        if is_include(cur) {
            out.push(cur);
        } else if let Some(c) = doc.first_child(cur) {
            cur = c;
            continue;
        }
        // Next in document order that is not inside `cur`, staying under start.
        loop {
            if cur == start {
                return;
            }
            if let Some(n) = doc.next_sibling(cur) {
                cur = n;
                break;
            }
            match doc.parent(cur) {
                Some(p) => cur = p,
                None => return,
            }
        }
    }
}

fn replace_include<F>(doc: &mut XmlDoc, id: NodeId, loader: &mut F) -> Result<bool, XmlError>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    let href = doc.xml_get_prop(id, "href");
    let parse = doc.xml_get_prop(id, "parse").unwrap_or_else(|| "xml".into());
    let parent = doc.parent(id).unwrap_or(NodeId::DOCUMENT);
    if let Some(h) = href {
        match loader(&h) {
            Ok(bytes) => {
                if parse == "text" {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let n = doc.alloc(NodeKind::Text, "#text");
                    doc.node_mut(n).content = text;
                    doc.xml_add_prev_sibling(id, n);
                    doc.xml_unlink_node(id);
                    return Ok(true);
                }
                let included = xml_read_memory(&bytes, Some(&h), None, default_parse_options())?;
                if let Some(root) = included.xml_doc_get_root_element() {
                    graft(doc, parent, id, &included, root);
                    doc.xml_unlink_node(id);
                    return Ok(true);
                }
            }
            Err(_) => {
                if let Some(fb) = find_fallback(doc, id) {
                    let mut kids = Vec::new();
                    let mut c = doc.first_child(fb);
                    while let Some(x) = c {
                        kids.push(x);
                        c = doc.next_sibling(x);
                    }
                    for k in kids {
                        doc.xml_unlink_node(k);
                        doc.xml_add_prev_sibling(id, k);
                    }
                    doc.xml_unlink_node(id);
                    return Ok(true);
                }
                return Err(XmlError::new(
                    crate::error::XML_ERR_DOCUMENT_EMPTY,
                    format!("XInclude failed to load {h}"),
                    0,
                    0,
                ));
            }
        }
    }
    Ok(false)
}

fn find_fallback(doc: &XmlDoc, include: NodeId) -> Option<NodeId> {
    let mut c = doc.first_child(include);
    while let Some(x) = c {
        if doc.kind(x) == NodeKind::Element && doc.name(x) == "fallback" {
            return Some(x);
        }
        c = doc.next_sibling(x);
    }
    None
}

fn graft(dst: &mut XmlDoc, _parent: NodeId, before: NodeId, src: &XmlDoc, src_id: NodeId) {
    let new = copy_node(dst, src, src_id);
    dst.xml_add_prev_sibling(before, new);
}

/// Copy `id` and its subtree from `src` into `dst`, returning the unlinked
/// copy. Iterative, for the same reason as [`collect_includes`]; nodes are
/// allocated in document order, as the recursive form allocated them.
fn copy_node(dst: &mut XmlDoc, src: &XmlDoc, id: NodeId) -> NodeId {
    let root = copy_one(dst, src, id);
    // (source child, destination parent), next to visit on top.
    let mut stack: Vec<(NodeId, NodeId)> = Vec::new();
    let mut kids: Vec<NodeId> = Vec::new();
    push_children(src, id, root, &mut stack, &mut kids);
    while let Some((s, parent)) = stack.pop() {
        let n = copy_one(dst, src, s);
        dst.xml_add_child(parent, n);
        push_children(src, s, n, &mut stack, &mut kids);
    }
    root
}

fn push_children(
    src: &XmlDoc,
    id: NodeId,
    copy: NodeId,
    stack: &mut Vec<(NodeId, NodeId)>,
    kids: &mut Vec<NodeId>,
) {
    kids.clear();
    let mut c = src.first_child(id);
    while let Some(x) = c {
        kids.push(x);
        c = src.next_sibling(x);
    }
    stack.extend(kids.iter().rev().map(|&k| (k, copy)));
}

/// One node with its namespace data and attributes, not its children.
fn copy_one(dst: &mut XmlDoc, src: &XmlDoc, id: NodeId) -> NodeId {
    let kind = src.kind(id);
    let n = dst.alloc(kind, src.name(id));
    // Prefixes and URIs are interned per document: re-intern through `dst`.
    dst.set_prefix(n, src.prefix(id));
    dst.set_ns_uri(n, src.ns_uri(id));
    dst.node_mut(n).content = src.content(id).to_string();
    for (p, u) in src.ns_defs(id) {
        dst.push_ns_def(n, p.clone(), u.clone());
    }
    // xml_set_prop builds the attribute node. This used to copy each
    // attribute first as well, leaving an unlinked node in the arena per
    // attribute.
    let mut a = src.first_attr(id);
    while let Some(x) = a {
        dst.xml_set_prop(n, src.name(x), src.content(x));
        a = src.next_sibling(x);
    }
    n
}
