//! Canonical XML 1.0 / exclusive C14N. Byte-identity vs `xmllint --c14n`.

use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};
use std::cmp::Ordering;

/// libxml2 `xmlC14NMode`.
pub const XML_C14N_1_0: i32 = 0;
pub const XML_C14N_EXCLUSIVE_1_0: i32 = 1;
pub const XML_C14N_1_1: i32 = 2;

const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// `xmlC14NDocDumpMemory`.
#[doc(alias = "xmlC14NDocDumpMemory")]
pub fn xml_c14n_doc_dump_memory(
    doc: &XmlDoc,
    exclusive: bool,
    with_comments: bool,
) -> Result<Vec<u8>, String> {
    let mut out = String::new();
    emit_node(doc, NodeId::DOCUMENT, exclusive, with_comments, &[], &[], &mut out, 0)?;
    Ok(out.into_bytes())
}

/// Inclusive C14N 1.0 without comments.
pub fn xml_c14n_1_0(doc: &XmlDoc) -> Result<Vec<u8>, String> {
    xml_c14n_doc_dump_memory(doc, false, false)
}

/// Exclusive C14N 1.0 without comments.
pub fn xml_exc_c14n_1_0(doc: &XmlDoc) -> Result<Vec<u8>, String> {
    xml_c14n_doc_dump_memory(doc, true, false)
}

/// Nesting limit for canonicalization.
///
/// `emit_node` recurses into children, carrying the inherited namespace
/// rendering with it, so depth costs stack: about 500 bytes a level, and a
/// 2000-deep document ABORTED THE PROCESS while being canonicalized even though
/// the parser accepts 5000. A signature path must not be a crash path.
///
/// 400 is far beyond signed XML, which is shallow in practice. The real fix is
/// to drive this from an explicit stack the way the parser and the writer now
/// are; a constant is a bound, not a substitute, and this one is recorded as
/// such.
const MAX_C14N_DEPTH: u32 = 400;

fn emit_node(
    doc: &XmlDoc,
    id: NodeId,
    exclusive: bool,
    with_comments: bool,
    vis_prefixes: &[&str],
    rendered: &[(String, String)],
    out: &mut String,
    depth: u32,
) -> Result<(), String> {
    if depth > MAX_C14N_DEPTH {
        return Err(format!(
            "document nested deeper than {MAX_C14N_DEPTH} for canonicalization"
        ));
    }
    match doc.kind(id) {
        NodeKind::Document | NodeKind::HtmlDocument => {
            let mut kids = Vec::new();
            let mut c = doc.first_child(id);
            while let Some(x) = c {
                kids.push(x);
                c = doc.next_sibling(x);
            }
            let mut after_element = false;
            let mut before_element = true;
            let mut seen_pi_or_comment = false;
            for kid in &kids {
                match doc.kind(*kid) {
                    NodeKind::Element => {
                        before_element = false;
                        emit_node(doc, *kid, exclusive, with_comments, vis_prefixes, rendered, out, depth + 1)?;
                        after_element = true;
                    }
                    NodeKind::Pi => {
                        if after_element || seen_pi_or_comment {
                            out.push('\n');
                        }
                        emit_pi(doc, *kid, out);
                        if before_element {
                            out.push('\n');
                        }
                        seen_pi_or_comment = true;
                    }
                    NodeKind::Comment if with_comments => {
                        if after_element || seen_pi_or_comment {
                            out.push('\n');
                        }
                        emit_comment(doc, *kid, out);
                        if before_element {
                            out.push('\n');
                        }
                        seen_pi_or_comment = true;
                    }
                    _ => {}
                }
            }
        }
        NodeKind::Element => {
            emit_element(doc, id, exclusive, with_comments, vis_prefixes, rendered, out, depth)?
        }
        NodeKind::Text => out.push_str(&escape_text(doc.content(id))),
        NodeKind::CData => out.push_str(&escape_text(doc.content(id))),
        NodeKind::Pi => emit_pi(doc, id, out),
        NodeKind::Comment if with_comments => emit_comment(doc, id, out),
        _ => {}
    }
    Ok(())
}

fn emit_pi(doc: &XmlDoc, id: NodeId, out: &mut String) {
    out.push_str("<?");
    out.push_str(doc.name(id));
    let data = doc.content(id);
    if !data.is_empty() {
        out.push(' ');
        out.push_str(data);
    }
    out.push_str("?>");
}

fn emit_comment(doc: &XmlDoc, id: NodeId, out: &mut String) {
    out.push_str("<!--");
    out.push_str(doc.content(id));
    out.push_str("-->");
}

fn emit_element(
    doc: &XmlDoc,
    id: NodeId,
    exclusive: bool,
    with_comments: bool,
    vis_prefixes: &[&str],
    rendered: &[(String, String)],
    out: &mut String,
    depth: u32,
) -> Result<(), String> {
    let qn = qname(doc.prefix(id), doc.name(id));
    out.push('<');
    out.push_str(&qn);

    let mut ns_attrs = namespaces_to_emit(doc, id, exclusive, vis_prefixes, rendered);
    ns_attrs.sort_by(|a, b| a.0.cmp(&b.0));
    for (pre, href) in &ns_attrs {
        out.push(' ');
        if pre.is_empty() {
            out.push_str("xmlns");
        } else {
            out.push_str("xmlns:");
            out.push_str(pre);
        }
        out.push_str("=\"");
        out.push_str(&escape_attr(href));
        out.push('"');
    }

    let mut child_rendered: Vec<(String, String)> = rendered.to_vec();
    for (pre, href) in &ns_attrs {
        child_rendered.retain(|(p, _)| p != pre);
        child_rendered.push((pre.clone(), href.clone()));
    }

    let mut attrs: Vec<(String, String, String)> = Vec::new();
    let mut a = doc.first_attr(id);
    while let Some(x) = a {
        let ns = doc.ns_uri(x).unwrap_or("").to_string();
        let local = doc.name(x).to_string();
        let qn = doc.qname(x);
        attrs.push((ns, qn, doc.content(x).to_string()));
        let _ = local;
        a = doc.next_sibling(x);
    }
    attrs.sort_by(|a, b| cmp_attr(&a.0, &a.1, &b.0, &b.1));
    for (_ns, name, val) in attrs {
        out.push(' ');
        out.push_str(&name);
        out.push_str("=\"");
        out.push_str(&escape_attr(&val));
        out.push('"');
    }

    out.push('>');
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        emit_node(doc, x, exclusive, with_comments, vis_prefixes, &child_rendered, out, depth + 1)?;
        c = doc.next_sibling(x);
    }
    out.push_str("</");
    out.push_str(&qn);
    out.push('>');
    Ok(())
}

fn qname(prefix: Option<&str>, local: &str) -> String {
    match prefix {
        Some(p) if !p.is_empty() => format!("{p}:{local}"),
        _ => local.to_string(),
    }
}

fn in_scope_ns(doc: &XmlDoc, id: NodeId) -> Vec<(String, String)> {
    let mut map: Vec<(String, String)> = Vec::new();
    let mut cur = Some(id);
    while let Some(n) = cur {
        for (pre, href) in doc.ns_defs(n).iter().rev() {
            let key = pre.clone().unwrap_or_default();
            if !map.iter().any(|(k, _)| k == &key) {
                map.push((key, href.clone()));
            }
        }
        cur = doc.parent(n);
        if cur == Some(NodeId::DOCUMENT) {
            break;
        }
    }
    map
}

fn namespaces_to_emit(
    doc: &XmlDoc,
    id: NodeId,
    exclusive: bool,
    vis_prefixes: &[&str],
    rendered: &[(String, String)],
) -> Vec<(String, String)> {
    let mut map = in_scope_ns(doc, id);
    map.retain(|(pre, href)| {
        if pre == "xml" && href == XML_NS {
            return false;
        }
        if exclusive {
            if !visibly_used(doc, id, pre, vis_prefixes) {
                return false;
            }
            if pre.is_empty() && href.is_empty() {
                return rendered.iter().any(|(p, h)| p.is_empty() && !h.is_empty());
            }
            return !rendered.iter().any(|(p, h)| p == pre && h == href);
        }
        if rendered.iter().any(|(p, h)| p == pre && h == href) {
            return false;
        }
        if pre.is_empty() && href.is_empty() {
            return rendered.iter().any(|(p, h)| p.is_empty() && !h.is_empty());
        }
        true
    });
    map
}

fn visibly_used(doc: &XmlDoc, id: NodeId, pre: &str, extra: &[&str]) -> bool {
    if extra.contains(&pre) {
        return true;
    }
    if pre.is_empty() {
        return doc.prefix(id).is_none();
    }
    if doc.prefix(id) == Some(pre) {
        return true;
    }
    let mut a = doc.first_attr(id);
    while let Some(x) = a {
        if doc.prefix(x) == Some(pre) {
            return true;
        }
        a = doc.next_sibling(x);
    }
    false
}

fn cmp_attr(ans: &str, an: &str, bns: &str, bn: &str) -> Ordering {
    let au = ans;
    let bu = bns;
    match (au.is_empty(), bu.is_empty()) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => au.cmp(bu).then_with(|| {
            let al = an.rsplit(':').next().unwrap_or(an);
            let bl = bn.rsplit(':').next().unwrap_or(bn);
            al.cmp(bl)
        }),
    }
}

fn escape_text(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            c => out.push(c),
        }
    }
    out
}

fn escape_attr(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            c => out.push(c),
        }
    }
    out
}
