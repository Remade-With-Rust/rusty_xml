//! XInclude 1.0. Resource fetch is caller-supplied; the library never opens the network.

use rusty_xml_tree::{InterruptCheck, NodeId, NodeKind, XmlDoc};
use crate::checker::{stop_error, Checker, NoCheck};
use crate::error::XmlError;
use crate::parse::{default_parse_options, parse_memory_inner};

const XI: &str = "http://www.w3.org/2001/XInclude";

/// `xmlXIncludeProcess` with a caller resource loader.
#[doc(alias = "xmlXIncludeProcess")]
pub fn xml_xinclude_process<F>(doc: &mut XmlDoc, mut loader: F) -> Result<i32, XmlError>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    xml_xinclude_process_tree(doc, NodeId::DOCUMENT, &mut loader, &mut NoCheck)
}

/// Like [`xml_xinclude_process`], but fails with `XML_ERR_USER_STOP` once `check` reports an
/// interrupt. After any error the document is unspecified (includes already applied stay, and a
/// stop inside a copy can leave allocated, unlinked nodes): discard it.
pub fn xml_xinclude_process_with_interrupt<F>(
    doc: &mut XmlDoc,
    mut loader: F,
    check: &mut InterruptCheck<'_>,
) -> Result<i32, XmlError>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    xml_xinclude_process_tree(doc, NodeId::DOCUMENT, &mut loader, check)
}

fn xml_xinclude_process_tree<F, C: Checker>(
    doc: &mut XmlDoc,
    start: NodeId,
    loader: &mut F,
    check: &mut C,
) -> Result<i32, XmlError>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    let mut subs: Vec<NodeId> = Vec::new();
    collect_includes(doc, start, &mut subs);
    let mut n = 0i32;
    for id in subs {
        check.tick().map_err(|_| stop_error(0, 0))?;
        if replace_include(doc, id, loader, check)? {
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

fn replace_include<F, C: Checker>(
    doc: &mut XmlDoc,
    id: NodeId,
    loader: &mut F,
    check: &mut C,
) -> Result<bool, XmlError>
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
                let included =
                    parse_memory_inner(&bytes, Some(&h), None, default_parse_options(), check)?;
                if let Some(root) = included.xml_doc_get_root_element() {
                    graft(doc, parent, id, &included, root, check)?;
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

fn graft<C: Checker>(
    dst: &mut XmlDoc,
    _parent: NodeId,
    before: NodeId,
    src: &XmlDoc,
    src_id: NodeId,
    check: &mut C,
) -> Result<(), XmlError> {
    let new = copy_node(dst, src, src_id, check)?;
    dst.xml_add_prev_sibling(before, new);
    Ok(())
}

/// One check per copied node: each `xml_set_prop` scans the element's existing attributes.
fn copy_node<C: Checker>(
    dst: &mut XmlDoc,
    src: &XmlDoc,
    id: NodeId,
    check: &mut C,
) -> Result<NodeId, XmlError> {
    check.tick().map_err(|_| stop_error(0, 0))?;
    let kind = src.kind(id);
    let n = dst.alloc(kind, src.name(id));
    // Prefixes and URIs are interned per document: re-intern through `dst`.
    dst.set_prefix(n, src.prefix(id));
    dst.set_ns_uri(n, src.ns_uri(id));
    dst.node_mut(n).content = src.content(id).to_string();
    for (p, u) in src.ns_defs(id) {
        dst.push_ns_def(n, p.clone(), u.clone());
    }
    let mut a = src.first_attr(id);
    while let Some(x) = a {
        let an = copy_node(dst, src, x, check)?;
        dst.xml_set_prop(n, src.name(x), src.content(x));
        let _ = an;
        a = src.next_sibling(x);
    }
    let mut c = src.first_child(id);
    while let Some(x) = c {
        let ch = copy_node(dst, src, x, check)?;
        dst.xml_add_child(n, ch);
        c = src.next_sibling(x);
    }
    Ok(n)
}
