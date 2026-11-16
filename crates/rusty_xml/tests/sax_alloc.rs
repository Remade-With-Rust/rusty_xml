//! #69, #70 and #71 regressions: the pooled end-tag QName, the pooled
//! attribute URI, the shared duplicate-attribute check, and the arena
//! policy that now ends every parse (push included) with a shrink.

use rusty_xml::{
    default_parse_options, html_read_memory, xml_create_push_parser_ctxt, xml_parse_chunk,
    xml_read_memory, xml_save_doc, xml_sax_parse_memory, SaxAttr, SaxEvent, SaxHandler,
    SaxRecorder, XML_PARSE_NO_TREE,
};

fn events(xml: &[u8], opts: i32) -> Vec<SaxEvent> {
    let mut rec = SaxRecorder::new();
    xml_sax_parse_memory(xml, opts, &mut rec).expect("parses");
    rec.events
}

/// Names around and past the 24-byte inline limit, nested so the pool holds
/// several buffers at once and hands them back in a different order.
#[test]
fn pooled_qnames_keep_every_end_tag_exact() {
    let mut xml = String::from("<r>");
    let names: Vec<String> = (0..300)
        .map(|i| format!("e{}", "n".repeat([1, 23, 24, 25, 40, 3][i % 6])))
        .collect();
    for n in &names {
        xml.push_str(&format!("<{n}>"));
    }
    for n in names.iter().rev() {
        xml.push_str(&format!("</{n}>"));
    }
    for n in &names {
        xml.push_str(&format!("<{n}>t</{n}>"));
    }
    xml.push_str("</r>");
    let opts = default_parse_options();
    assert_eq!(events(xml.as_bytes(), opts), events(xml.as_bytes(), opts | XML_PARSE_NO_TREE));
    // A mismatched long end tag is still caught, with the right names.
    let bad = format!("<r><{0}></{0}x></r>", "m".repeat(40));
    let e = xml_read_memory(bad.as_bytes(), None, None, opts).unwrap_err();
    assert!(e.message.contains("Opening and ending tag mismatch"), "{e}");
    // Push carries open elements across chunk buffers.
    let whole = xml_read_memory(xml.as_bytes(), None, None, opts).unwrap();
    for size in [7, 64, 1000] {
        let mut c = xml_create_push_parser_ctxt(&[], None, None, opts);
        for ch in xml.as_bytes().chunks(size) {
            xml_parse_chunk(&mut c, ch, 0).unwrap();
        }
        let got = xml_parse_chunk(&mut c, &[], 1).unwrap().unwrap();
        assert_eq!(xml_save_doc(&got, 0), xml_save_doc(&whole, 0), "chunk {size}");
    }
}

/// The URI buffer of a pooled attribute slot is reused for prefixed
/// attributes and must read `None` again for unprefixed ones.
#[test]
fn pooled_attribute_uris_follow_each_attribute() {
    #[derive(Default)]
    struct Uris(Vec<Vec<Option<String>>>);
    impl SaxHandler for Uris {
        fn start_element_ns(
            &mut self,
            _l: &str,
            _p: Option<&str>,
            _u: Option<&str>,
            _n: &[(Option<String>, String)],
            attrs: &[SaxAttr],
            _d: i32,
        ) {
            self.0.push(attrs.iter().map(|a| a.uri.clone()).collect());
        }
    }
    let xml = br#"<r xmlns:a="urn:a" xmlns:b="urn:b"><e a:x="1" y="2" b:z="3"/><e y="2" a:x="1"/><e q="1"/><e xml:lang="en" b:z="3"/></r>"#;
    for opts in [default_parse_options(), XML_PARSE_NO_TREE] {
        let mut h = Uris::default();
        xml_sax_parse_memory(xml, opts, &mut h).unwrap();
        let s = |x: &str| Some(x.to_string());
        assert_eq!(
            h.0,
            vec![
                vec![],
                vec![s("urn:a"), None, s("urn:b")],
                vec![None, s("urn:a")],
                vec![None],
                vec![s("http://www.w3.org/XML/1998/namespace"), s("urn:b")],
            ],
            "options {opts:#x}"
        );
    }
    // An unbound prefix is still a namespace error, not fatal.
    let doc = xml_read_memory(br#"<r q:x="1"/>"#, None, None, default_parse_options()).unwrap();
    assert!(doc.namespace_errors.iter().any(|e| e.contains("Namespace prefix q")));
}

/// Past 32 attributes both parsers use the shared keyed-hash check.
#[test]
fn wide_tags_find_duplicates_in_both_parsers() {
    let attrs: String = (0..40).map(|i| format!(" a{i}=\"{i}\"")).collect();
    let opts = default_parse_options();
    let dup = format!("<r{attrs} a0=\"x\"/>");
    let e = xml_read_memory(dup.as_bytes(), None, None, opts).unwrap_err();
    assert!(e.message.contains("Attribute redefined"), "{e}");
    let ok = format!("<r{attrs}/>");
    let doc = xml_read_memory(ok.as_bytes(), None, None, opts).unwrap();
    assert_eq!(doc.xml_get_prop(doc.xml_doc_get_root_element().unwrap(), "a39").as_deref(), Some("39"));
    // Two prefixes bound to one URI collide only after expansion: a
    // namespace error, and the parse goes on.
    let ns = format!(r#"<r xmlns:p="u" xmlns:q="u"{attrs} p:k="1" q:k="2"/>"#);
    let doc = xml_read_memory(ns.as_bytes(), None, None, opts).unwrap();
    assert!(doc
        .namespace_errors
        .iter()
        .any(|e| e.contains("Attribute redefined after namespace expansion")));
    // HTML keeps the first of a repeated name on the wide path too.
    let html = format!("<div{attrs} a5=\"late\" A6=\"late\">");
    let h = html_read_memory(html.as_bytes(), None, None, 0).unwrap();
    let out = String::from_utf8(xml_save_doc(&h, 0)).unwrap();
    assert!(out.contains("a5=\"5\"") && out.contains("a6=\"6\"") && !out.contains("late"), "{out}");
}

/// Every parse ends with the arena shrunk to the tree, the push parser too:
/// it used to keep whatever its doubling reached (#70).
#[test]
fn push_and_one_shot_trees_hold_no_spare_arena() {
    let xml: String = format!(
        "<mesh>{}</mesh>",
        (0..30_000).map(|i| format!("<v x=\"{i}\" y=\"1\"/>")).collect::<String>()
    );
    let opts = default_parse_options();
    let one = xml_read_memory(xml.as_bytes(), None, None, opts).unwrap();
    assert_eq!(one.node_capacity(), one.len());
    let mut c = xml_create_push_parser_ctxt(&[], None, None, opts);
    for ch in xml.as_bytes().chunks(4096) {
        xml_parse_chunk(&mut c, ch, 0).unwrap();
    }
    let pushed = xml_parse_chunk(&mut c, &[], 1).unwrap().unwrap();
    assert_eq!(pushed.len(), one.len());
    assert_eq!(pushed.node_capacity(), pushed.len());
}
