//! `Node::name` and `Node::content` are `CompactString` (#62): up to 24 bytes
//! inline, on the heap beyond. Untrusted text flows straight into them, so the
//! boundary is tested exhaustively rather than sampled: every length from 0 to
//! 64 bytes, with 1- to 4-byte UTF-8 characters at every offset, through each
//! path that stores a string, and back out through every reader.

use rusty_xml::{
    default_parse_options, html_read_memory, xml_read_memory, xml_save_doc, CompactString,
    NodeId, NodeKind, XmlDoc,
};

/// Strings of `len` bytes made of ASCII plus one character of `width` bytes
/// at every possible position, so every split of a multi-byte character
/// around the 24-byte inline limit is covered.
fn samples(len: usize) -> Vec<String> {
    let wide = ['\u{e9}', '\u{20ac}', '\u{1f600}']; // 2, 3 and 4 bytes
    let mut out = vec!["a".repeat(len)];
    for c in wide {
        let w = c.len_utf8();
        if w > len {
            continue;
        }
        for at in 0..=len - w {
            let mut s = "b".repeat(at);
            s.push(c);
            s.push_str(&"c".repeat(len - w - at));
            debug_assert_eq!(s.len(), len);
            out.push(s);
        }
    }
    out
}

fn find(doc: &XmlDoc, kind: NodeKind) -> Vec<NodeId> {
    (0..doc.len() as u32)
        .map(NodeId)
        .filter(|&i| doc.kind(i) == kind && doc.parent(i).is_some())
        .collect()
}

#[test]
fn every_length_and_split_round_trips_through_parse_tree_and_save() {
    let opts = default_parse_options();
    for len in 0..=64 {
        for s in samples(len) {
            // Text, attribute value, comment and CDATA content.
            let esc = s.clone(); // no markup characters in the samples
            let xml = format!(
                "<r a=\"{esc}\"><t>{esc}</t><!--{esc}--><c><![CDATA[{esc}]]></c></r>"
            );
            let doc = xml_read_memory(xml.as_bytes(), None, None, opts)
                .unwrap_or_else(|e| panic!("len {len} {s:?}: {e}"));
            let r = doc.xml_doc_get_root_element().unwrap();
            assert_eq!(doc.xml_get_prop(r, "a").as_deref(), Some(s.as_str()), "attr {len}");
            if len > 0 {
                let texts = find(&doc, NodeKind::Text);
                assert!(texts.iter().any(|&t| doc.content(t) == s), "text {len} {s:?}");
                let comments = find(&doc, NodeKind::Comment);
                assert_eq!(doc.content(comments[0]), s, "comment {len}");
                let cdata = find(&doc, NodeKind::CData);
                assert_eq!(doc.content(cdata[0]), s, "cdata {len}");
            }
            // Serialized and parsed again: a fixed point.
            let out = xml_save_doc(&doc, 0);
            let again = xml_read_memory(&out, None, None, opts).unwrap();
            assert_eq!(xml_save_doc(&again, 0), out, "round trip {len} {s:?}");

            // Element and attribute names (names must start with a letter).
            if len > 0 {
                let name = format!("n{s}");
                let xml = format!("<{name} {name}=\"v\">x</{name}>");
                let doc = xml_read_memory(xml.as_bytes(), None, None, opts)
                    .unwrap_or_else(|e| panic!("name len {}: {e}", name.len()));
                let r = doc.xml_doc_get_root_element().unwrap();
                assert_eq!(doc.name(r), name);
                assert_eq!(&*doc.node(r).name, name.as_str());
                assert_eq!(doc.xml_get_prop(r, &name).as_deref(), Some("v"));
            }

            // HTML text and attribute values take the Cow conversion path.
            let html = format!("<p title=\"{esc}\">{esc}</p>");
            let h = html_read_memory(html.as_bytes(), None, Some("utf-8"), 0).unwrap();
            let out = String::from_utf8(xml_save_doc(&h, 0)).unwrap();
            if len > 0 {
                assert!(out.contains(&s), "html {len} {s:?}: {out}");
            }
        }
    }
}

#[test]
fn mutation_apis_store_every_length_exactly() {
    for len in 0..=64 {
        for s in samples(len) {
            let mut doc = XmlDoc::xml_new_doc(Some("1.0"));
            let r = doc.xml_new_node(None, "r");
            doc.xml_add_child(NodeId::DOCUMENT, r);
            doc.xml_set_prop(r, "a", &s);
            assert_eq!(doc.xml_get_prop(r, "a").as_deref(), Some(s.as_str()));
            // Overwriting crosses the boundary in both directions.
            for other in ["", "short", "x".repeat(25).as_str(), s.as_str()] {
                doc.xml_set_prop(r, "a", other);
                assert_eq!(doc.xml_get_prop(r, "a").as_deref(), Some(other), "len {len}");
            }
            let t = doc.alloc(NodeKind::Text, "#text");
            doc.xml_add_child(r, t);
            doc.xml_node_set_content(t, &s);
            assert_eq!(doc.content(t), s);
            // The field is public: direct assignment and in-place growth.
            doc.node_mut(t).content = CompactString::from(s.as_str());
            doc.node_mut(t).content.push_str(&s);
            assert_eq!(doc.content(t), format!("{s}{s}"));
            let n = doc.alloc(NodeKind::Element, s.as_str());
            assert_eq!(doc.name(n), s.as_str());
        }
    }
}

/// `CompactString` is the same size as `String`, so `Node` keeps its layout.
#[test]
fn node_layout_is_unchanged() {
    assert_eq!(std::mem::size_of::<CompactString>(), std::mem::size_of::<String>());
    assert_eq!(std::mem::size_of::<rusty_xml::Node>(), 88);
}
