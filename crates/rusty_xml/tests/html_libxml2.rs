//! Issue #12: the HTML parser builds libxml2 2.15's trees.
//!
//! Serialized output is checked against the pinned oracle in
//! `html_fixtures.rs`. The tests here check what serialization cannot show:
//! node kinds, text content, attribute values and the root element.

use rusty_xml::{html_read_memory, NodeId, NodeKind, XmlDoc};

fn parse(src: &str) -> XmlDoc {
    html_read_memory(src.as_bytes(), None, None, 0).expect("parses")
}

#[test]
fn script_content_is_one_raw_text_node_in_head() {
    let doc = parse("<script>if (1<2) {}</script>");
    let head = doc
        .first_child(doc.xml_doc_get_root_element().expect("html"))
        .expect("head");
    let script = doc.first_child(head).expect("script");
    let raw = doc.first_child(script).expect("content");
    assert_eq!(
        doc.kind(raw),
        NodeKind::CData,
        "libxml2 keeps raw text as HTML_PRESERVE_NODE"
    );
    assert_eq!(doc.content(raw), "if (1<2) {}");
    assert!(doc.next_sibling(raw).is_none());
}

#[test]
fn only_legacy_names_may_omit_the_semicolon_in_text() {
    let doc = parse("<p>&lang x &copy y</p>");
    let mut texts = Vec::new();
    collect_text(&doc, NodeId::DOCUMENT, &mut texts);
    assert_eq!(texts, ["&lang x \u{a9} y"]);
}

#[test]
fn numeric_references_follow_libxml2() {
    let doc = parse("<p>&#128;&#0;&#x110000;&#xD800;&#65 &#x41x&#99999999999999;</p>");
    let mut texts = Vec::new();
    collect_text(&doc, NodeId::DOCUMENT, &mut texts);
    assert_eq!(texts, ["\u{20ac}\u{fffd}\u{fffd}\u{fffd}A Ax\u{fffd}"]);
}

fn collect_text(doc: &XmlDoc, id: NodeId, out: &mut Vec<String>) {
    let mut c = doc.first_child(id);
    while let Some(n) = c {
        if doc.kind(n) == NodeKind::Text {
            out.push(doc.content(n).to_string());
        }
        collect_text(doc, n, out);
        c = doc.next_sibling(n);
    }
}

#[test]
fn valueless_attributes_have_no_value() {
    let doc = parse("<input value checked>");
    let body = doc
        .first_child(doc.xml_doc_get_root_element().expect("html"))
        .expect("body");
    let input = doc.first_child(body).expect("input");
    assert_eq!(doc.xml_get_prop(input, "value").as_deref(), Some(""));
    // A boolean attribute takes its own name, as xmlSAX2HtmlAttribute does.
    assert_eq!(
        doc.xml_get_prop(input, "checked").as_deref(),
        Some("checked")
    );
}

#[test]
fn the_root_element_is_the_first_html_not_a_later_one() {
    let doc = parse("<html><body><p>a</p></body></html><div>after</div>");
    let root = doc.xml_doc_get_root_element().expect("root");
    let first = doc.first_child(NodeId::DOCUMENT).expect("first html");
    assert_eq!(
        root, first,
        "xmlDocGetRootElement returns the first element child"
    );
    assert!(
        doc.next_sibling(first).is_some(),
        "the second html is still there"
    );
}
