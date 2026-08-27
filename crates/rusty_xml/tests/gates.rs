//! M1 SAX-exact + M2 save/writer/reader/round-trip gates.

use rusty_xml::{
    default_parse_options, xml_is_char, xml_new_text_writer_memory, xml_read_memory,
    xml_reader_for_memory, xml_sax_parse_memory, xml_save_doc, ReaderType, SaxRecorder, XmlDoc,
    XML_ERR_DOCUMENT_EMPTY, XML_ERR_TAG_NOT_FINISHED, XML_ERR_UNDECLARED_ENTITY, XML_SAVE_NO_DECL,
    XML_SAVE_NO_EMPTY,
};
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

fn oracle_bin() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RUSTY_XML_ORACLE") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let mut p = workspace_root();
    p.push("oracle");
    p.push("bin");
    if cfg!(windows) {
        p.push("xmllint.exe");
    } else {
        p.push("xmllint");
    }
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn our_sax(xml: &[u8]) -> (SaxRecorder, Result<XmlDoc, rusty_xml::XmlError>) {
    let mut rec = SaxRecorder::new();
    let doc = xml_sax_parse_memory(xml, default_parse_options(), &mut rec);
    (rec, doc)
}

fn c_sax(xml: &[u8]) -> Option<String> {
    let oracle = oracle_bin()?;
    let dir = tempfile_dir();
    let f = dir.join(format!(
        "in-{:?}-{}.xml",
        std::thread::current().id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&f, xml).ok()?;
    let out = Command::new(&oracle)
        .args(["--sax", f.to_str()?])
        .output()
        .ok()?;
    let _ = std::fs::remove_file(&f);
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn tempfile_dir() -> PathBuf {
    let p = std::env::temp_dir().join("rusty_xml-gates");
    let _ = std::fs::create_dir_all(&p);
    p
}

fn assert_sax_matches_oracle(xml: &[u8]) {
    let (rec, result) = our_sax(xml);
    assert!(result.is_ok(), "parse failed for {:?}: {:?}", std::str::from_utf8(xml), result.err());
    let ours = rec.to_xmllint_debug(xml);
    if let Some(c) = c_sax(xml) {
        assert_eq!(ours, c, "SAX dump mismatch for {:?}", std::str::from_utf8(xml));
    }
}

#[test]
fn chvalid_bmp_via_facade() {
    let dump = std::fs::read(workspace_root().join("corpora/xmlIsChar-bmp.bin")).unwrap();
    assert_eq!(dump.len(), 65536);
    for i in 0u32..=0xffff {
        assert_eq!(xml_is_char(i), dump[i as usize] != 0, "{i:#x}");
    }
}

#[test]
fn sax_empty_element() {
    assert_sax_matches_oracle(b"<a/>");
}

#[test]
fn sax_with_xml_decl() {
    assert_sax_matches_oracle(br#"<?xml version="1.0"?><a/>"#);
}

#[test]
fn sax_comment() {
    assert_sax_matches_oracle(b"<a><!--c--></a>");
}

#[test]
fn sax_pi() {
    assert_sax_matches_oracle(b"<a><?pi data?></a>");
}

#[test]
fn sax_cdata() {
    assert_sax_matches_oracle(b"<a><![CDATA[x]]></a>");
}

#[test]
fn sax_predefined_entities_not_coalesced() {
    assert_sax_matches_oracle(b"<a>&lt;&amp;</a>");
}

#[test]
fn sax_prefixed_ns() {
    assert_sax_matches_oracle(br#"<n:e xmlns:n="u"/>"#);
}

#[test]
fn sax_default_ns_relative_uri_warning() {
    assert_sax_matches_oracle(br#"<e xmlns="u"/>"#);
}

#[test]
fn sax_whitespace_text() {
    assert_sax_matches_oracle(b"<a>\n  <b/>\n</a>");
}

#[test]
fn sax_attr() {
    assert_sax_matches_oracle(br#"<a b="c"/>"#);
}

#[test]
fn not_wf_unclosed() {
    let err = xml_read_memory(b"<a>", None, None, default_parse_options()).unwrap_err();
    assert!(
        err.code == XML_ERR_TAG_NOT_FINISHED || err.code == XML_ERR_DOCUMENT_EMPTY,
        "code {}",
        err.code
    );
}

#[test]
fn undeclared_entity() {
    let err = xml_read_memory(b"<a>&foo;</a>", None, None, default_parse_options()).unwrap_err();
    assert_eq!(err.code, XML_ERR_UNDECLARED_ENTITY);
}

#[test]
fn xml_read_memory_root() {
    let doc = xml_read_memory(b"<a/>", None, None, default_parse_options()).unwrap();
    let root = doc.xml_doc_get_root_element().unwrap();
    assert_eq!(doc.name(root), "a");
}

#[test]
fn save_empty_element() {
    let doc = xml_read_memory(b"<a/>", None, None, default_parse_options()).unwrap();
    let bytes = xml_save_doc(&doc, 0);
    let s = String::from_utf8(bytes).unwrap();
    assert_eq!(s, "<?xml version=\"1.0\"?>\n<a/>\n");
}

#[test]
fn save_no_empty() {
    let doc = xml_read_memory(b"<a/>", None, None, default_parse_options()).unwrap();
    let s = String::from_utf8(xml_save_doc(&doc, XML_SAVE_NO_EMPTY)).unwrap();
    assert_eq!(s, "<?xml version=\"1.0\"?>\n<a></a>\n");
}

#[test]
fn save_no_decl() {
    let doc = xml_read_memory(b"<a/>", None, None, default_parse_options()).unwrap();
    let s = String::from_utf8(xml_save_doc(&doc, XML_SAVE_NO_DECL)).unwrap();
    assert_eq!(s, "<a/>\n");
}

#[test]
fn tree_mutation() {
    let mut doc = XmlDoc::xml_new_doc(Some("1.0"));
    let root = doc.xml_new_node(None, "root");
    doc.xml_doc_set_root_element(root);
    doc.xml_set_prop(root, "id", "1");
    doc.xml_new_child(root, None, "child", Some("x"));
    assert_eq!(doc.xml_get_prop(root, "id").as_deref(), Some("1"));
    let s = String::from_utf8(xml_save_doc(&doc, XML_SAVE_NO_DECL)).unwrap();
    assert!(s.contains("<root id=\"1\">"));
    assert!(s.contains("<child>x</child>"));
}

#[test]
fn writer_document() {
    let mut w = xml_new_text_writer_memory();
    w.start_document(Some("1.0"), None, None).unwrap();
    w.start_element("a").unwrap();
    w.write_attribute("b", "c").unwrap();
    w.write_string("x").unwrap();
    w.end_element().unwrap();
    w.end_document().unwrap();
    let xml = w.into_bytes();
    let doc = xml_read_memory(&xml, None, None, default_parse_options()).unwrap();
    let root = doc.xml_doc_get_root_element().unwrap();
    assert_eq!(doc.name(root), "a");
    assert_eq!(doc.xml_get_prop(root, "b").as_deref(), Some("c"));
    assert_eq!(doc.xml_node_get_content(root), "x");
}

#[test]
fn reader_empty_and_nested() {
    let mut r = xml_reader_for_memory(b"<a><b/></a>", None, None, default_parse_options()).unwrap();
    assert_eq!(r.read(), 1);
    assert_eq!(r.node_type(), ReaderType::Element);
    assert_eq!(r.local_name(), Some("a"));
    assert!(!r.is_empty_element());
    assert_eq!(r.read(), 1);
    assert_eq!(r.local_name(), Some("b"));
    assert!(r.is_empty_element());
    assert_eq!(r.read(), 1);
    assert_eq!(r.node_type(), ReaderType::EndElement);
    assert_eq!(r.local_name(), Some("a"));
    assert_eq!(r.read(), 0);
}

fn sax_events_of(xml: &[u8]) -> SaxRecorder {
    let (rec, r) = our_sax(xml);
    r.expect("well-formed");
    rec
}

fn core_events(rec: &SaxRecorder) -> Vec<rusty_xml::SaxEvent> {
    rec.events
        .iter()
        .filter(|e| {
            !matches!(
                e,
                rusty_xml::SaxEvent::SetDocumentLocator
                    | rusty_xml::SaxEvent::StartDocument
                    | rusty_xml::SaxEvent::EndDocument
            )
        })
        .cloned()
        .collect()
}

#[test]
fn roundtrip_parse_write_parse_event_exact() {
    let fixtures: &[&[u8]] = &[
        b"<a/>",
        b"<a>x</a>",
        b"<a><!--c--></a>",
        b"<a><?pi data?></a>",
        b"<a><![CDATA[x]]></a>",
        b"<a>&lt;&amp;</a>",
        br#"<n:e xmlns:n="http://example.com/n"/>"#,
        b"<a><b/></a>",
        b"<a b=\"c\">z</a>",
    ];
    for xml in fixtures {
        let first = sax_events_of(xml);
        let doc = xml_read_memory(xml, None, None, default_parse_options()).unwrap();
        let written = xml_save_doc(&doc, 0);
        let second = sax_events_of(&written);
        assert_eq!(
            core_events(&first),
            core_events(&second),
            "round-trip events for {:?}",
            std::str::from_utf8(xml)
        );
    }
}

#[test]
fn corpora_parse() {
    for name in ["title.xml", "slashdot.xml", "android-lite.xml", "svg-lite.xml", "atom-lite.xml"] {
        let p = workspace_root().join("corpora").join(name);
        let bytes = std::fs::read(&p).unwrap_or_else(|_| panic!("missing {}", p.display()));
        xml_read_memory(&bytes, Some(name), None, default_parse_options())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        if oracle_bin().is_some() {
            assert_sax_matches_oracle(&bytes);
        }
    }
}

#[test]
fn title_xml_exists() {
    assert!(workspace_root().join("corpora/title.xml").exists());
}

#[test]
fn m3_latin1_document() {
    let mut xml = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a>".to_vec();
    xml.push(0xE9);
    xml.extend_from_slice(b"</a>");
    let doc = xml_read_memory(&xml, None, None, default_parse_options()).unwrap();
    let root = doc.xml_doc_get_root_element().unwrap();
    assert_eq!(doc.xml_node_get_content(root), "é");
}

#[test]
fn m3_push_parser() {
    let mut ctxt = rusty_xml::xml_create_push_parser_ctxt(b"<a", None, None, default_parse_options());
    assert!(rusty_xml::xml_parse_chunk(&mut ctxt, b"/>", 1).unwrap().is_some());
}

#[test]
fn m3_read_io() {
    let data = b"<io/>";
    let mut off = 0;
    let doc = rusty_xml::xml_read_io(
        |buf| {
            let n = (data.len() - off).min(buf.len());
            buf[..n].copy_from_slice(&data[off..off + n]);
            off += n;
            Ok(n)
        },
        None,
        None,
        default_parse_options(),
    )
    .unwrap();
    assert_eq!(doc.name(doc.xml_doc_get_root_element().unwrap()), "io");
}

#[test]
fn m3_catalog_local() {
    let dir = tempfile_dir();
    let cat = dir.join("catalog.xml");
    std::fs::write(
        &cat,
        r#"<?xml version="1.0"?>
<catalog xmlns="urn:oasis:names:tc:entity:xmlns:xml:catalog">
  <public publicId="-//FOO" uri="foo.dtd"/>
  <system systemId="bar.dtd" uri="local-bar.dtd"/>
</catalog>"#,
    )
    .unwrap();
    let c = rusty_xml::XmlCatalog::xml_load_catalog(&cat).unwrap();
    assert!(c.xml_catalog_resolve(Some("-//FOO"), None).unwrap().ends_with("foo.dtd"));
}

#[test]
fn m4_xpath_arithmetic() {
    let doc = xml_read_memory(b"<a/>", None, None, default_parse_options()).unwrap();
    let ctx = rusty_xml::XmlXPathContext::xml_xpath_new_context(&doc);
    let obj = rusty_xml::xml_xpath_eval("1+2*3+4", &ctx).unwrap();
    let dump = rusty_xml::xml_xpath_debug_dump(&obj, &ctx);
    assert!(dump.contains("Object is a number : 11"), "{dump}");
}

#[test]
fn m4_xpath_nodeset() {
    let xml = br#"<?xml version="1.0"?><EXAMPLE prop1="gnome is great" prop2="&amp; linux too">
  <head>
   <title>Welcome to Gnome</title>
  </head>
  <chapter>
   <title>The Linux adventure</title>
   <p>bla bla bla ...</p>
   <image href="linus.gif"/>
   <p>...</p>
  </chapter>
</EXAMPLE>"#;
    let doc = xml_read_memory(xml, None, None, default_parse_options()).unwrap();
    let ctx = rusty_xml::XmlXPathContext::xml_xpath_new_context(&doc);
    let obj = rusty_xml::xml_xpath_eval("/child::EXAMPLE/child::*", &ctx).unwrap();
    match obj {
        rusty_xml::XPathObject::NodeSet(v) => {
            assert_eq!(v.len(), 2);
            assert_eq!(doc.name(v[0]), "head");
            assert_eq!(doc.name(v[1]), "chapter");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn m4_xpath_oracle_expr_base() {
    let Some(oracle) = oracle_bin() else { return };
    let doc = xml_read_memory(b"<a/>", None, None, default_parse_options()).unwrap();
    let ctx = rusty_xml::XmlXPathContext::xml_xpath_new_context(&doc);
    let exprs = ["1", "1+2", "2*3", "1+2*3+4", "(1+2)*(3+4)", "true()", "false()", "string(5)"];
    for expr in exprs {
        let obj = rusty_xml::xml_xpath_eval(expr, &ctx).unwrap_or(rusty_xml::XPathObject::Undefined);
        let ours = rusty_xml::xml_xpath_print_lint(&obj).unwrap_or_default();
        let dir = tempfile_dir();
        let f = dir.join("xpath-doc.xml");
        std::fs::write(&f, b"<a/>").unwrap();
        let out = Command::new(&oracle)
            .args(["--xpath", expr, f.to_str().unwrap()])
            .output()
            .unwrap();
        let c = String::from_utf8_lossy(&out.stdout);
        assert_eq!(ours, c.as_ref(), "xpath dump mismatch for {expr}");
    }
}

#[test]
fn m5_dtd_internal() {
    let xml = b"<!DOCTYPE a [ <!ELEMENT a (#PCDATA)> ]><a>x</a>";
    let doc = xml_read_memory(xml, None, None, default_parse_options()).unwrap();
    rusty_xml::xml_validate_document(&doc).expect("valid");
}

#[test]
fn m5_c14n_simple() {
    let doc = xml_read_memory(b"<a>x</a>", None, None, default_parse_options()).unwrap();
    let bytes = rusty_xml::xml_c14n_1_0(&doc).unwrap();
    assert_eq!(bytes, b"<a>x</a>");
}

#[test]
fn m5_c14n_oracle_example1() {
    let Some(oracle) = oracle_bin() else { return };
    let p = workspace_root().join("oracle/src/test/c14n/without-comments/example-3.xml");
    if !p.exists() {
        return;
    }
    let xml = std::fs::read(&p).unwrap();
    let doc = xml_read_memory(&xml, None, None, default_parse_options());
    let Ok(doc) = doc else { return };
    let ours = rusty_xml::xml_c14n_1_0(&doc).unwrap();
    let out = Command::new(oracle)
        .args(["--c14n", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(ours, out.stdout, "c14n mismatch");
}

#[test]
fn m5_xinclude() {
    let mut doc = xml_read_memory(
        br#"<r xmlns:xi="http://www.w3.org/2001/XInclude"><xi:include href="frag.xml"/></r>"#,
        None,
        None,
        default_parse_options(),
    )
    .unwrap();
    rusty_xml::xml_xinclude_process(&mut doc, |uri| {
        assert_eq!(uri, "frag.xml");
        Ok(b"<frag>ok</frag>".to_vec())
    })
    .unwrap();
    let root = doc.xml_doc_get_root_element().unwrap();
    let child = doc.first_child(root).unwrap();
    assert_eq!(doc.name(child), "frag");
}

#[test]
fn m6_html_implied() {
    let doc = rusty_xml::html_read_memory(b"<p>hi</p>", None, None, 0).unwrap();
    let root = doc.xml_doc_get_root_element().unwrap();
    assert_eq!(doc.name(root), "html");
}

#[test]
fn m6_relaxng_tutor() {
    let rng = br#"<element name="addressBook" xmlns="http://relaxng.org/ns/structure/1.0">
  <zeroOrMore>
    <element name="card">
      <attribute name="name"><text/></attribute>
      <attribute name="email"><text/></attribute>
    </element>
  </zeroOrMore>
</element>"#;
    let xml = br#"<addressBook><card name="John Smith" email="js@example.com"/></addressBook>"#;
    let doc = xml_read_memory(xml, None, None, default_parse_options()).unwrap();
    rusty_xml::xml_relaxng_validate_doc(rng, &doc).expect("rng valid");
}

#[test]
fn m6_xsd_sequence() {
    let xsd = br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="doc">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="a" minOccurs="1" maxOccurs="1"/>
      </xs:sequence>
    </xs:complexType>
  </xs:element>
  <xs:element name="a"/>
</xs:schema>"#;
    let doc = xml_read_memory(b"<doc><a/></doc>", None, None, default_parse_options()).unwrap();
    rusty_xml::xml_schema_validate_doc(xsd, &doc).expect("xsd valid");
}

#[test]
fn m6_schematron() {
    let sch = br#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <pattern>
    <rule context="*">
      <assert test="true()">ok</assert>
    </rule>
  </pattern>
</schema>"#;
    let doc = xml_read_memory(b"<a/>", None, None, default_parse_options()).unwrap();
    rusty_xml::xml_schematron_validate_doc(sch, &doc).expect("sch valid");
}

fn xpath_result_blocks(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for chunk in text.split("========================") {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let Some(rest) = chunk.strip_prefix("Expression:") else { continue };
        let rest = rest.trim_start();
        let Some((expr, dump)) = rest.split_once('\n') else { continue };
        out.push((expr.trim().to_string(), dump.trim_start().to_string()));
    }
    out
}

#[test]
fn m4_xpath_expr_result_files() {
    let root = workspace_root();
    let doc = xml_read_memory(b"<a/>", None, None, default_parse_options()).unwrap();
    let ctx = rusty_xml::XmlXPathContext::xml_xpath_new_context(&doc);
    for name in ["base", "functions", "floats", "strings", "equality", "compare"] {
        let expected = std::fs::read_to_string(root.join(format!("oracle/src/result/XPath/expr/{name}")))
            .unwrap_or_else(|_| String::new());
        if expected.is_empty() {
            continue;
        }
        for (expr, want) in xpath_result_blocks(&expected) {
            let obj = rusty_xml::xml_xpath_eval(&expr, &ctx)
                .unwrap_or(rusty_xml::XPathObject::Undefined);
            let dump = rusty_xml::xml_xpath_debug_dump(&obj, &ctx);
            assert_eq!(
                dump.trim_end(),
                want.trim_end(),
                "XPath expr {name}: {expr}"
            );
        }
    }
}

#[test]
fn m4_xpath_simplebase_nodes() {
    let root = workspace_root();
    let p = root.join("oracle/src/test/XPath/docs/simple");
    if !p.exists() {
        return;
    }
    let xml = std::fs::read(&p).unwrap();
    let doc = xml_read_memory(&xml, None, None, default_parse_options()).unwrap();
    let ctx = rusty_xml::XmlXPathContext::xml_xpath_new_context(&doc);
    let obj = rusty_xml::xml_xpath_eval("/child::EXAMPLE/child::*", &ctx).unwrap();
    match obj {
        rusty_xml::XPathObject::NodeSet(v) => assert_eq!(v.len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn m5_c14n_examples() {
    let Some(oracle) = oracle_bin() else { return };
    let dir = workspace_root().join("oracle/src/test/c14n/without-comments");
    for name in ["example-1.xml", "example-2.xml", "example-3.xml"] {
        let p = dir.join(name);
        if !p.exists() {
            continue;
        }
        let xml = std::fs::read(&p).unwrap();
        let Ok(doc) = xml_read_memory(&xml, None, None, default_parse_options()) else {
            continue;
        };
        let ours = rusty_xml::xml_c14n_doc_dump_memory(&doc, false, true).unwrap();
        let out = Command::new(&oracle)
            .args(["--c14n", p.to_str().unwrap()])
            .output()
            .unwrap();
        assert_eq!(ours, out.stdout, "c14n {name}");
    }
}

#[test]
fn m5_exc_c14n_oracle() {
    let Some(oracle) = oracle_bin() else { return };
    let p = workspace_root().join("oracle/src/test/c14n/exc-without-comments/test-0.xml");
    if !p.exists() {
        return;
    }
    let xml = std::fs::read(&p).unwrap();
    let Ok(doc) = xml_read_memory(&xml, None, None, default_parse_options()) else {
        return;
    };
    let ours = rusty_xml::xml_c14n_doc_dump_memory(&doc, true, true).unwrap();
    let out = Command::new(oracle)
        .args(["--exc-c14n", p.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(ours, out.stdout);
}

#[test]
fn m6_relaxng_oracle_tutor3_1() {
    let root = workspace_root();
    let rng_path = root.join("oracle/src/test/relaxng/tutor3_1.rng");
    let xml_path = root.join("oracle/src/test/relaxng/tutor3_1_1.xml");
    if !rng_path.exists() || !xml_path.exists() {
        return;
    }
    let rng = std::fs::read(&rng_path).unwrap();
    let xml = std::fs::read(&xml_path).unwrap();
    let doc = xml_read_memory(&xml, None, None, default_parse_options()).unwrap();
    rusty_xml::xml_relaxng_validate_doc(&rng, &doc).expect("tutor3_1_1");
}

/// The HTML parser skipped an unusable attribute-name character by ONE BYTE.
/// A multi-byte character there left the cursor mid-scalar and the next slice
/// panicked -- found by fuzzing, 83 distinct panic sites, all this one cause.
/// A panic in a library is an availability bug for whatever embeds it.
#[test]
fn html_multibyte_where_an_attribute_name_should_be_does_not_panic() {
    let opts = default_parse_options();
    for case in [
        &b"<r\xdd\xb7/>"[..],                       // U+0777 where a name must start
        &b"<r \xdd\xb7 a=\"1\"/>"[..],              // ... between attributes
        &b"<r\xe6\x97\xa5/>"[..],                   // 3-byte
        &b"<r\xf0\x9f\x98\x80/>"[..],               // 4-byte
        &b"<a:r xmlns:a=\"u\"\xdb\xae<a:c/></a:r>"[..],
        &b"<rs&##65;&#x42\xdd\xb7&l"[..],
    ] {
        // Must not panic. Accepting or rejecting is both fine; crashing is not.
        let _ = rusty_xml::html_read_memory(case, None, None, opts);
    }
}

/// The XML side must stay panic-free on the same shapes.
#[test]
fn xml_entry_points_do_not_panic_on_malformed_input() {
    let opts = default_parse_options();
    for case in [
        &b"<r\xdd\xb7/>"[..],
        &b"<r>\xff\xfe</r>"[..],
        &b"<r a=\"\xc3\"/>"[..],
        &b"<r>\x00</r>"[..],
        &b"<?xml"[..],
        &b"<!DOCTYPE d [<!ENTITY e \"\xdd"[..],
    ] {
        let _ = xml_read_memory(case, None, None, opts);
        let mut rec = SaxRecorder::new();
        let _ = xml_sax_parse_memory(case, opts, &mut rec);
        if let Ok(mut r) = rusty_xml::xml_reader_for_memory(case, None, None, opts) {
            let mut n = 0;
            while r.read() == 1 {
                n += 1;
                if n > 10_000 {
                    break;
                }
            }
        }
    }
}

/// Defaulted attributes are an amplification vector: 13 KB with 200 ATTLIST
/// defaults expanded to 402,002 nodes (~74 MB) unbounded. libxml2 caps entity
/// amplification for the same reason. The bound must reject that WITHOUT
/// rejecting a real DTD-heavy document, so both directions are pinned here.
#[test]
fn attlist_default_amplification_is_bounded() {
    let opts = default_parse_options();
    // abusive: ~30 defaulted attributes per input byte
    let decls: String = (0..200)
        .map(|i| format!("<!ATTLIST e a{i} CDATA \"d\">\n"))
        .collect();
    let body: String = (0..2000).map(|_| "<e/>".to_string()).collect();
    let bomb = format!("<!DOCTYPE r [<!ELEMENT e ANY>\n{decls}]><r>{body}</r>");
    assert!(
        xml_read_memory(bomb.as_bytes(), None, None, opts).is_err(),
        "a 13 KB document must not be allowed to expand to 400k attributes"
    );

    // legitimate: ~0.2 defaulted attributes per input byte, which libxml2 accepts
    let decls: String = (0..5)
        .map(|i| format!("<!ATTLIST e a{i} CDATA \"d{i}\">\n"))
        .collect();
    let body: String = (0..4000).map(|i| format!("<e id=\"{i}\">t</e>")).collect();
    let ok = format!("<!DOCTYPE r [<!ELEMENT e ANY>\n{decls}]><r>{body}</r>");
    assert!(
        xml_read_memory(ok.as_bytes(), None, None, opts).is_ok(),
        "the bound must not reject a document the C oracle accepts"
    );
}

/// Duplicate-attribute detection was a linear scan over the accepted
/// attributes, so one element with many attributes was O(n^2): 16,000 took
/// 185 ms. This pins the shape, not a wall-clock number.
#[test]
fn many_attributes_on_one_element_is_not_quadratic() {
    let opts = default_parse_options();
    let build = |k: usize| {
        let mut s = String::from("<r");
        for i in 0..k {
            s.push_str(&format!(" a{i}=\"v\""));
        }
        s.push_str("/>");
        s
    };
    for k in [1000usize, 8000] {
        let doc = xml_read_memory(build(k).as_bytes(), None, None, opts)
            .unwrap_or_else(|e| panic!("{k} attributes should parse: {e:?}"));
        assert_eq!(doc.len(), k + 2, "every attribute becomes a node");
    }
    // duplicates must still be caught once the set path is active
    let mut dup = String::from("<r");
    for i in 0..40 {
        dup.push_str(&format!(" a{i}=\"v\""));
    }
    dup.push_str(" a7=\"again\"/>");
    assert!(
        xml_read_memory(dup.as_bytes(), None, None, opts).is_err(),
        "a repeated attribute must be rejected on the set path too"
    );
}

/// A deeply nested XPath expression overflowed the stack, which ABORTS THE
/// PROCESS -- `catch_unwind` cannot recover from it, so a single bad expression
/// took down everything sharing the address space. Five shapes did it, and two
/// mechanisms: recursion while parsing, and the recursive `Drop` of the deep
/// `Box` chain the union and negation loops build.
#[test]
fn deeply_nested_xpath_is_rejected_not_fatal() {
    let doc = xml_read_memory(b"<r><a/></r>", None, None, default_parse_options()).unwrap();
    let ctx = rusty_xml::XmlXPathContext::xml_xpath_new_context(&doc);
    let n = 5000;
    let shapes = [
        ("predicates", "//*[".repeat(n)),
        ("unions", "//*|".repeat(n) + "//*"),
        ("parens", "(".repeat(n) + "1" + &")".repeat(n)),
        ("negation", "-".repeat(n) + "1"),
        ("not()", "not(".repeat(n) + "1" + &")".repeat(n)),
    ];
    for (name, expr) in shapes {
        // Must return, either Ok or Err. Reaching this line at all is the test.
        let _ = rusty_xml::xml_xpath_eval(&expr, &ctx);
        let _ = rusty_xml::xml_xpath_compile(&expr);
        assert!(!name.is_empty());
    }
}

/// The depth bound must not reject expressions people actually write.
#[test]
fn ordinary_xpath_still_compiles() {
    let doc = xml_read_memory(
        br#"<r><item id="1"><t>x</t></item><item id="2"/></r>"#,
        None,
        None,
        default_parse_options(),
    )
    .unwrap();
    let ctx = rusty_xml::XmlXPathContext::xml_xpath_new_context(&doc);
    for e in [
        "count(//item)",
        "//item[@id='1']",
        "//*[local-name()='item'][position()<3]",
        "string(//t)",
        "//a/b/c/d/e/f/g/h",
        "//x[.//y[.//z[@k]]]",
        "//*[@a or @b and not(@c)]",
        "normalize-space(//t[1])",
    ] {
        assert!(
            rusty_xml::xml_xpath_eval(e, &ctx).is_ok(),
            "ordinary expression must still evaluate: {e}"
        );
    }
    match rusty_xml::xml_xpath_eval("count(//item)", &ctx) {
        Ok(rusty_xml::XPathObject::Number(n)) => assert_eq!(n, 2.0),
        other => panic!("expected 2 items, got {other:?}"),
    }
}

/// The parser used to recurse on document depth, so the cap had to sit below
/// the stack limit -- 64, because a debug build overflowed at 95 and a stack
/// overflow aborts the process. The content loop is iterative now, so depth
/// costs no stack and the cap is a policy limit again. This must hold in BOTH
/// build profiles: debug is where the old parser died first.
#[test]
fn deep_nesting_costs_no_stack_in_any_build_profile() {
    let build = |d: usize| {
        let mut s = String::from("<r>");
        for _ in 0..d {
            s.push_str("<a>");
        }
        s.push('x');
        for _ in 0..d {
            s.push_str("</a>");
        }
        s.push_str("</r>");
        s
    };
    let def = default_parse_options();
    let huge = def | rusty_xml::XML_PARSE_HUGE;

    // Far past where the recursive parser aborted in debug (95 levels).
    for d in [100usize, 1_000, 4_000] {
        assert!(
            xml_read_memory(build(d).as_bytes(), None, None, def).is_ok(),
            "{d} levels must parse; the parser no longer recurses on depth"
        );
    }
    // Still a policy limit, and it returns an error rather than dying.
    assert!(xml_read_memory(build(6_000).as_bytes(), None, None, def).is_err());
    // HUGE lifts it, which it can now do safely.
    assert!(xml_read_memory(build(20_000).as_bytes(), None, None, huge).is_ok());
}

/// RelaxNG had two ways to kill the caller. `<start/>` with no pattern inside
/// unwrapped an Err and panicked. A definition referencing itself recursed
/// until the stack ran out, which ABORTS THE PROCESS and cannot be caught.
/// Both must now be reported as errors -- and a grammar that recurses THROUGH
/// an element is legal and must still validate.
#[test]
fn relaxng_malformed_and_cyclic_schemas_are_errors_not_crashes() {
    let doc = xml_read_memory(b"<r><a k=\"1\">t</a></r>", None, None, default_parse_options()).unwrap();
    const NS: &str = "http://relaxng.org/ns/structure/1.0";
    for bad in [
        "<grammar><start/></grammar>".to_string(),
        format!("<grammar xmlns=\"{NS}\"><define name=\"a\"><ref name=\"a\"/></define><start><ref name=\"a\"/></start></grammar>"),
        format!("<grammar xmlns=\"{NS}\"><define name=\"a\"><ref name=\"b\"/></define><define name=\"b\"><ref name=\"a\"/></define><start><ref name=\"a\"/></start></grammar>"),
        format!("<grammar xmlns=\"{NS}\"><define name=\"a\"><choice><ref name=\"a\"/></choice></define><start><ref name=\"a\"/></start></grammar>"),
    ] {
        // Reaching this line at all is the test: it must return, not abort.
        let _ = rusty_xml::xml_relaxng_validate_doc(bad.as_bytes(), &doc);
    }

    // Recursion through an element consumes input, so it terminates and is legal.
    let ok = format!(
        "<grammar xmlns=\"{NS}\"><start><ref name=\"r\"/></start>\
         <define name=\"r\"><element name=\"r\"><zeroOrMore><ref name=\"r\"/></zeroOrMore></element></define></grammar>"
    );
    let nested = xml_read_memory(b"<r><r/></r>", None, None, default_parse_options()).unwrap();
    let res = rusty_xml::xml_relaxng_validate_doc(ok.as_bytes(), &nested);
    assert!(
        res.is_ok(),
        "a grammar recursing through an element must still validate: {res:?}"
    );
}

/// Every entry point in this library materialised the whole document, including
/// the ones whose job is streaming: `xml_sax_parse_memory` built a full tree and
/// discarded it. XML_PARSE_NO_TREE lets a consumer that only wants events skip
/// it. The event stream must be IDENTICAL either way -- that is the whole
/// contract.
#[test]
fn no_tree_mode_delivers_identical_events() {
    let root = workspace_root();
    for name in ["slashdot.xml", "big-300k.xml", "big-attr.xml", "svg-lite.xml"] {
        let p = root.join("corpora").join(name);
        if !p.exists() {
            continue;
        }
        let xml = std::fs::read(&p).unwrap();
        let opts = default_parse_options();

        let mut with_tree = SaxRecorder::new();
        let d1 = xml_sax_parse_memory(&xml, opts, &mut with_tree).unwrap();
        let mut no_tree = SaxRecorder::new();
        let d2 = xml_sax_parse_memory(&xml, opts | rusty_xml::XML_PARSE_NO_TREE, &mut no_tree)
            .unwrap();

        assert_eq!(
            with_tree.to_xmllint_debug(&xml),
            no_tree.to_xmllint_debug(&xml),
            "{name}: NO_TREE changed the event stream"
        );
        assert!(
            d2.len() < d1.len(),
            "{name}: NO_TREE should build fewer nodes ({} vs {})",
            d2.len(),
            d1.len()
        );
    }
}

/// The push parser used to accumulate the whole document and parse it at
/// terminate, so "push" cost more memory than xml_read_memory, not less. It
/// streams now: content is parsed as each chunk arrives and consumed bytes are
/// released. The result must still be byte-identical to a whole-document parse
/// at EVERY chunk size, which is the only contract that matters here.
#[test]
fn push_parser_streams_and_matches_a_whole_parse() {
    let root = workspace_root();
    let opts = default_parse_options();
    for name in [
        "slashdot.xml",
        "big-300k.xml",
        "big-attr.xml",
        "svg-lite.xml",
        "atom-lite.xml",
        "title.xml",
    ] {
        let path = root.join("corpora").join(name);
        if !path.exists() {
            continue;
        }
        let d = std::fs::read(&path).unwrap();
        let whole = xml_save_doc(&xml_read_memory(&d, None, None, opts).unwrap(), 0);

        // Byte-at-a-time is the harshest split: it lands a boundary inside
        // every construct, including the CRLF that XML must fold to one LF.
        for chunk in [1usize, 2, 3, 7, 997, 65536] {
            let mut ctx = rusty_xml::xml_create_push_parser_ctxt(&[], None, None, opts);
            let mut i = 0;
            while i < d.len() {
                let n = chunk.min(d.len() - i);
                let _ = rusty_xml::xml_parse_chunk(&mut ctx, &d[i..i + n], 0);
                i += n;
            }
            let doc = rusty_xml::xml_parse_chunk(&mut ctx, &[], 1)
                .unwrap_or_else(|e| panic!("{name} chunk={chunk}: {e:?}"))
                .unwrap_or_else(|| panic!("{name} chunk={chunk}: no document"));
            assert_eq!(
                xml_save_doc(&doc, 0),
                whole,
                "{name} chunk={chunk}: streamed result differs from a whole parse"
            );
        }
    }
}

/// It must actually stream: the buffer holds the unparsed tail, not the
/// document seen so far.
#[test]
fn push_parser_does_not_hoard_the_document() {
    let path = workspace_root().join("corpora").join("big-300k.xml");
    if !path.exists() {
        return;
    }
    let d = std::fs::read(&path).unwrap();
    let mut ctx =
        rusty_xml::xml_create_push_parser_ctxt(&[], None, None, default_parse_options());
    let mut peak = 0usize;
    let mut i = 0;
    while i < d.len() {
        let n = 4096.min(d.len() - i);
        let _ = rusty_xml::xml_parse_chunk(&mut ctx, &d[i..i + n], 0);
        i += n;
        peak = peak.max(ctx.buffered());
    }
    assert!(
        rusty_xml::xml_parse_chunk(&mut ctx, &[], 1).unwrap().is_some(),
        "terminating chunk must produce the document"
    );
    assert!(
        peak < d.len() / 10,
        "peak buffered {peak} of {} bytes -- the push parser is hoarding the document",
        d.len()
    );
}
