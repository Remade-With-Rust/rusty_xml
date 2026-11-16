//! Numeric character references share one value parse (#71), and every XML
//! path applies the `Char` check to its result (#70): written in content, in
//! an attribute value, inside an entity's replacement text, in tree mode and
//! under `XML_PARSE_NO_TREE`. HTML keeps libxml2's HTML substitutions.
//!
//! Expected verdicts are `xmllint` 2.15.3's (`--noout`), run on each case.

use rusty_xml::{
    default_parse_options, html_read_memory, xml_read_memory, xml_sax_parse_memory, xml_save_doc,
    SaxRecorder, XML_PARSE_NO_TREE,
};

/// `(reference inside the entity, libxml2 accepts it)`.
const ENTITY_CASES: [(&str, bool); 17] = [
    ("#0", false),
    ("#1", false),
    ("#xFFFE", false),
    ("#xD800", false),
    ("#x41", true),
    ("#65", true),
    ("#x9", true),
    ("#X41", false),
    ("#+65", false),
    ("#x110000", false),
    ("#99999999999", false),
    ("#x", false),
    ("#", false),
    ("#xD7FF", true),
    ("#xE000", true),
    ("#x10FFFF", true),
    ("#xFFFF", false),
];

fn content_doc(r: &str) -> String {
    format!("<!DOCTYPE r [<!ENTITY e \"&#38;{r};\">]><r>&e;</r>")
}

fn attr_doc(r: &str) -> String {
    format!("<!DOCTYPE r [<!ENTITY e \"&#38;{r};\">]><r a=\"&e;\"/>")
}

#[test]
fn references_inside_entities_follow_libxml2_in_content_and_attributes() {
    let opts = default_parse_options();
    for (r, ok) in ENTITY_CASES {
        let c = xml_read_memory(content_doc(r).as_bytes(), None, None, opts);
        assert_eq!(c.is_ok(), ok, "content {r}: {c:?}", c = c.as_ref().err());
        let a = xml_read_memory(attr_doc(r).as_bytes(), None, None, opts);
        assert_eq!(a.is_ok(), ok, "attribute {r}: {a:?}", a = a.as_ref().err());
    }
}

/// The no-tree SAX path expands entities without re-parsing them; it took
/// U+0000, U+0001 and U+FFFE through `characters` (#70).
#[test]
fn references_inside_entities_follow_libxml2_under_no_tree() {
    for (r, ok) in ENTITY_CASES {
        for doc in [content_doc(r), attr_doc(r)] {
            let mut rec = SaxRecorder::new();
            let res = xml_sax_parse_memory(doc.as_bytes(), XML_PARSE_NO_TREE, &mut rec);
            assert_eq!(res.is_ok(), ok, "no-tree {doc}: {:?}", res.err());
        }
    }
}

/// The values that are accepted arrive as the right character.
#[test]
fn accepted_references_decode_to_their_character() {
    let opts = default_parse_options();
    for (r, want) in [("#x41", "A"), ("#65", "A"), ("#xE000", "\u{e000}"), ("#x10FFFF", "\u{10ffff}")] {
        let doc = xml_read_memory(attr_doc(r).as_bytes(), None, None, opts).unwrap();
        let root = doc.xml_doc_get_root_element().unwrap();
        assert_eq!(doc.xml_get_prop(root, "a").as_deref(), Some(want), "{r}");
    }
}

/// Written directly, a reference keeps its error codes: 7 or 6 for a value
/// past a u32 (decimal, hexadecimal), 8 for one that is not a `Char`.
#[test]
fn direct_references_keep_their_error_codes() {
    let opts = default_parse_options();
    let code = |s: &str| xml_read_memory(s.as_bytes(), None, None, opts).err().map(|e| e.code);
    assert_eq!(code("<r>&#0;</r>"), Some(8));
    assert_eq!(code("<r a='&#xFFFE;'/>"), Some(8));
    assert_eq!(code("<r>&#99999999999;</r>"), Some(7));
    assert_eq!(code("<r>&#x100000000;</r>"), Some(6));
    assert_eq!(code("<r>&#X41;</r>"), Some(7));
    assert_eq!(code("<r>&#65;&#x42;</r>"), None);
    let doc = xml_read_memory(b"<r a='&#65;&#x9;b'>&#x10FFFF;</r>", None, None, opts).unwrap();
    let root = doc.xml_doc_get_root_element().unwrap();
    assert_eq!(doc.xml_get_prop(root, "a").as_deref(), Some("A\tb"));
    let text = doc.first_child(root).unwrap();
    assert_eq!(doc.content(text), "\u{10ffff}");
}

/// DTD entity values keep their messages.
#[test]
fn dtd_entity_values_keep_their_messages() {
    let opts = default_parse_options();
    let msg = |s: &str| {
        xml_read_memory(s.as_bytes(), None, None, opts)
            .err()
            .map(|e| e.message)
            .unwrap_or_default()
    };
    assert!(msg("<!DOCTYPE r [<!ENTITY e \"&#0;\">]><r/>").contains("invalid XML character"));
    assert!(msg("<!DOCTYPE r [<!ENTITY e \"&#99999999999;\">]><r/>").contains("out of range"));
}

/// HTML substitutes as libxml2's HTML parser does, unchanged by the shared
/// value parse: C1 references remap to Windows-1252, 0 and anything past
/// U+10FFFF or a u32 become U+FFFD, U+0001 and U+FFFE pass through.
#[test]
fn html_references_keep_libxml2s_substitutions() {
    let html = "<p>&#128;|&#0;|&#x110000;|&#xD800;|&#99999999999999999999;|&#1;|&#xFFFE;|&#65|&#x41x</p>";
    let doc = html_read_memory(html.as_bytes(), None, Some("utf-8"), 0).unwrap();
    let out = String::from_utf8(xml_save_doc(&doc, 0)).unwrap();
    let p = &out[out.find("<p>").unwrap() + 3..out.find("</p>").unwrap()];
    let parts: Vec<&str> = p.split('|').collect();
    assert_eq!(parts[0], "\u{20ac}");
    for &part in &parts[1..5] {
        assert_eq!(part, "\u{fffd}", "{p}");
    }
    assert_eq!(parts[7], "A");
    assert_eq!(parts[8], "Ax");
}
