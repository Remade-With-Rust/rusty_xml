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

fn collect_includes(doc: &XmlDoc, id: NodeId, out: &mut Vec<NodeId>) {
    if doc.kind(id) == NodeKind::Element
        && doc.name(id) == "include"
        && (doc.ns_uri(id) == Some(XI) || doc.prefix(id) == Some("xi"))
    {
        out.push(id);
        return; // do not recurse into include (fallback stays nested)
    }
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        collect_includes(doc, x, out);
        c = doc.next_sibling(x);
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

fn copy_node(dst: &mut XmlDoc, src: &XmlDoc, id: NodeId) -> NodeId {
    let kind = src.kind(id);
    let n = dst.alloc(kind, src.name(id));
    {
        let node = dst.node_mut(n);
        node.prefix = src.prefix(id).map(str::to_string);
        node.ns_uri = src.ns_uri(id).map(str::to_string);
        node.content = src.content(id).to_string();
        node.ns_defs = src.ns_defs(id).to_vec();
    }
    let mut a = src.first_attr(id);
    while let Some(x) = a {
        let an = copy_node(dst, src, x);
        dst.xml_set_prop(n, src.name(x), src.content(x));
        let _ = an;
        a = src.next_sibling(x);
    }
    let mut c = src.first_child(id);
    while let Some(x) = c {
        let ch = copy_node(dst, src, x);
        dst.xml_add_child(n, ch);
        c = src.next_sibling(x);
    }
    n
}
