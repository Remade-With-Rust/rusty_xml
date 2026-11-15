//! SAX streaming with `XML_PARSE_NO_TREE` (#52), and the in-place name and
//! attribute handling that came with it.
//!
//! A no-tree SAX parse used to allocate, link and keep an element node per
//! element, so memory grew with the document. It now builds no element node
//! at all, and the event stream must stay identical to a tree parse's.

use rusty_xml::{
    default_parse_options, xml_create_push_parser_ctxt, xml_parse_chunk, xml_read_memory,
    xml_sax_parse_memory, xml_save_doc, NodeKind, SaxAttr, SaxEvent, SaxHandler, SaxRecorder,
    XML_PARSE_HUGE, XML_PARSE_NO_TREE,
};

/// A 3MF-shaped document: a default namespace and many empty elements with
/// attributes.
fn mesh(n: usize) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\"?>\n<model unit=\"millimeter\" \
         xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\"><mesh><vertices>\n",
    );
    for i in 0..n {
        s.push_str(&format!("<vertex x=\"{i}.125\" y=\"{}.5\" z=\"0.75\"/>\n", i % 7));
    }
    s.push_str("</vertices></mesh></model>\n");
    s
}

fn events(xml: &[u8], opts: i32) -> Vec<SaxEvent> {
    let mut rec = SaxRecorder::new();
    xml_sax_parse_memory(xml, opts, &mut rec).expect("parses");
    rec.events
}

#[test]
fn no_tree_sax_builds_no_element_nodes() {
    let xml = mesh(20_000);
    let mut rec = SaxRecorder::new();
    let doc = xml_sax_parse_memory(xml.as_bytes(), XML_PARSE_NO_TREE, &mut rec).unwrap();
    let elements = (0..doc.len() as u32)
        .filter(|&i| doc.kind(rusty_xml::NodeId(i)) == NodeKind::Element)
        .count();
    assert_eq!(elements, 0, "a no-tree parse kept element nodes");
    assert!(doc.len() < 8, "{} nodes", doc.len());
    assert!(
        doc.node_capacity() < 64,
        "a no-tree parse reserved an arena of {} nodes",
        doc.node_capacity()
    );
    let starts = rec
        .events
        .iter()
        .filter(|e| matches!(e, SaxEvent::StartElementNs { .. }))
        .count();
    assert_eq!(starts, 20_003);

    // The tree parse of the same input still builds every element.
    let tree = xml_read_memory(xml.as_bytes(), None, None, default_parse_options()).unwrap();
    let tree_elements = (0..tree.len() as u32)
        .filter(|&i| tree.kind(rusty_xml::NodeId(i)) == NodeKind::Element)
        .count();
    assert_eq!(tree_elements, 20_003);
}

/// Namespaces are where the no-tree path differs most: the end tag's URI no
/// longer comes from an element node but from the scope still open.
#[test]
fn no_tree_events_match_tree_events_with_namespaces() {
    let docs: [&str; 6] = [
        r#"<r xmlns="d" xmlns:p="u1"><p:a p:x="1" y="2"><b xmlns="d2"><p:c/></b><c xmlns:p="u2"><p:d p:z="3"/></c></p:a><e/></r>"#,
        // Undeclared prefixes are namespace errors, not fatal.
        r#"<r><q:a q:x="1"/><b/></r>"#,
        // Colons that do not form a QName are ordinary name characters.
        r#"<r xmlns:a="u"><a:b:c d:e:f="1"/><x: y:="2"/><_z :w="3"/></r>"#,
        // The reserved prefixes.
        r#"<r xml:lang="en" xmlns:xml="http://www.w3.org/XML/1998/namespace"><xml:a/></r>"#,
        // Non-ASCII names take the decoding path.
        "<r xmlns:\u{e9}=\"u\"><\u{e9}:\u{fc}l \u{e9}:\u{df}=\"v\">t</\u{e9}:\u{fc}l></r>",
        // Entity and character references in attribute values.
        r#"<!DOCTYPE r [<!ENTITY e "ent">]><r a="&e;&#65;&lt;" b='x&#10;y'><s c="&amp;"/></r>"#,
    ];
    for xml in docs {
        let opts = default_parse_options();
        assert_eq!(
            events(xml.as_bytes(), opts),
            events(xml.as_bytes(), opts | XML_PARSE_NO_TREE),
            "NO_TREE changed the events of {xml}"
        );
    }
}

/// Attribute buffers are pooled across elements; the handler must see exactly
/// each element's own attributes, never a stale one from a longer element.
#[test]
fn pooled_attributes_are_exact_per_element() {
    #[derive(Default)]
    struct Attrs(Vec<(String, Vec<(Option<String>, String, Option<String>, String)>)>);
    impl SaxHandler for Attrs {
        fn start_element_ns(
            &mut self,
            local: &str,
            _p: Option<&str>,
            _u: Option<&str>,
            _ns: &[(Option<String>, String)],
            attrs: &[SaxAttr],
            _d: i32,
        ) {
            let a = attrs
                .iter()
                .map(|a| (a.prefix.clone(), a.local.clone(), a.uri.clone(), a.value.clone()))
                .collect();
            self.0.push((local.to_string(), a));
        }
    }
    let xml = br#"<r xmlns:p="u"><a x="1" p:y="two" z="3"/><b w="4"/><c/><d p:v="5"/></r>"#;
    for opts in [default_parse_options(), XML_PARSE_NO_TREE] {
        let mut h = Attrs::default();
        xml_sax_parse_memory(xml, opts, &mut h).unwrap();
        let s = |x: &str| x.to_string();
        assert_eq!(
            h.0,
            vec![
                (s("r"), vec![]),
                (
                    s("a"),
                    vec![
                        (None, s("x"), None, s("1")),
                        (Some(s("p")), s("y"), Some(s("u")), s("two")),
                        (None, s("z"), None, s("3")),
                    ]
                ),
                (s("b"), vec![(None, s("w"), None, s("4"))]),
                (s("c"), vec![]),
                (s("d"), vec![(Some(s("p")), s("v"), Some(s("u")), s("5"))]),
            ],
            "options {opts:#x}"
        );
    }
}

/// The table-driven name scan keeps the name-length limit where it was.
#[test]
fn name_length_limit_is_unchanged() {
    let opts = default_parse_options();
    let ok = format!("<{0}/>", "n".repeat(50_000));
    assert!(xml_read_memory(ok.as_bytes(), None, None, opts).is_ok());
    let long = format!("<{0}/>", "n".repeat(50_001));
    let e = xml_read_memory(long.as_bytes(), None, None, opts).unwrap_err();
    assert!(e.message.contains("Name too long"), "{e}");
    assert!(xml_read_memory(long.as_bytes(), None, None, opts | XML_PARSE_HUGE).is_ok());
    let attr = format!("<r {0}=\"v\"/>", "a".repeat(50_001));
    assert!(xml_read_memory(attr.as_bytes(), None, None, opts).is_err());
}

/// Push chunks are not validated up front, so names and runs there take the
/// per-slice path; a chunked parse must still equal the whole-buffer one,
/// with chunks that split multi-byte names.
#[test]
fn push_chunks_take_the_per_slice_path_and_agree() {
    let xml = "<r xmlns=\"d\" xmlns:\u{e9}=\"u\"><\u{e9}:\u{fc}l \u{e9}:\u{df}=\"v w\" a=\"1\">t\u{e9}xt</\u{e9}:\u{fc}l><b c=\"&amp;\"/></r>";
    let opts = default_parse_options();
    let whole = xml_read_memory(xml.as_bytes(), None, None, opts).unwrap();
    for size in [1, 2, 3, 5, 7] {
        let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, opts);
        for ch in xml.as_bytes().chunks(size) {
            xml_parse_chunk(&mut ctxt, ch, 0).unwrap();
        }
        let got = xml_parse_chunk(&mut ctxt, &[], 1).unwrap().unwrap();
        assert_eq!(xml_save_doc(&got, 0), xml_save_doc(&whole, 0), "chunk size {size}");
    }
    // A bad byte later in a whole buffer turns the up-front view off; the
    // document still fails.
    let mut bad = xml.as_bytes().to_vec();
    let at = bad.len() - 4;
    bad.insert(at, 0xff);
    assert!(xml_read_memory(&bad, None, None, opts).is_err());
}
