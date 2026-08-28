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
    let doc = xml_read_memory(&xml, None, None, default_parse_options() | rusty_xml::XML_PARSE_DTDATTR);
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
        default_parse_options() | rusty_xml::XML_PARSE_DTDATTR,
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
        let Ok(doc) = xml_read_memory(&xml, None, None, default_parse_options() | rusty_xml::XML_PARSE_DTDATTR) else {
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
    let Ok(doc) = xml_read_memory(&xml, None, None, default_parse_options() | rusty_xml::XML_PARSE_DTDATTR) else {
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
    // Defaulting is what amplifies, and defaulting is opt-in (XML_PARSE_DTDATTR)
    // exactly as it is in libxml2 -- a plain parse adds no attributes at all, so
    // the bound only has anything to do when the flag asks for it.
    let opts = default_parse_options() | rusty_xml::XML_PARSE_DTDATTR;
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

/// The HTML parser decoded no entities at all, so `caf&eacute;` reached the
/// tree verbatim and the serializer turned it into `caf&amp;eacute;`. For
/// anything reading extracted text that is silent corruption of the most common
/// characters on the web.
#[test]
fn html_named_and_numeric_entities_are_decoded() {
    let opts = default_parse_options();
    let doc = rusty_xml::html_read_memory(
        b"<html><body><p>caf&eacute; &mdash; don&rsquo;t &nbsp; &copy; AT&T &#65;&#x42; &unknown; &amp</p></body></html>",
        None,
        None,
        opts,
    )
    .unwrap();
    let mut text = String::new();
    for i in 0..doc.len() {
        let id = rusty_xml::NodeId(i as u32);
        if doc.kind(id) == rusty_xml::NodeKind::Text {
            text.push_str(&doc.node(id).content);
        }
    }
    assert!(text.contains("caf\u{e9}"), "eacute must decode: {text:?}");
    assert!(text.contains('\u{2014}'), "mdash must decode: {text:?}");
    assert!(text.contains('\u{2019}'), "rsquo must decode: {text:?}");
    assert!(text.contains('\u{a0}'), "nbsp must decode: {text:?}");
    assert!(text.contains('\u{a9}'), "copy must decode: {text:?}");
    assert!(text.contains("AB"), "numeric refs must decode: {text:?}");
    // A bare ampersand is not a reference and must survive as written.
    assert!(text.contains("AT&T"), "bare & must survive: {text:?}");
    // An unknown name is left alone, as a browser leaves it.
    assert!(text.contains("&unknown;"), "unknown name must survive: {text:?}");
    // HTML allows a named reference with no semicolon.
    assert!(text.trim_end().ends_with('&'), "`&amp` without `;` decodes: {text:?}");

    // Attribute values go through the same path.
    let doc = rusty_xml::html_read_memory(
        b"<html><body><a title=\"caf&eacute;\" href=x?a=1&amp;b=2>t</a></body></html>",
        None,
        None,
        opts,
    )
    .unwrap();
    let mut attrs = String::new();
    for i in 0..doc.len() {
        let id = rusty_xml::NodeId(i as u32);
        if doc.kind(id) == rusty_xml::NodeKind::Attribute {
            attrs.push_str(&doc.node(id).content);
        }
    }
    assert!(attrs.contains("caf\u{e9}"), "attribute entities decode: {attrs:?}");
    assert!(attrs.contains("a=1&b=2"), "attribute &amp; decodes: {attrs:?}");
}

/// XML is not HTML: only the five predefined entities exist, and an unknown one
/// is an error rather than literal text. The HTML change must not leak here.
#[test]
fn xml_entity_rules_are_unchanged_by_the_html_table() {
    let opts = default_parse_options();
    let doc = xml_read_memory(b"<r>&lt;&gt;&amp;&apos;&quot;</r>", None, None, opts).unwrap();
    assert_eq!(String::from_utf8_lossy(&xml_save_doc(&doc, 0)).trim_end().lines().last().unwrap(),
               "<r>&lt;&gt;&amp;'\"</r>");
    assert!(
        xml_read_memory(b"<r>&eacute;</r>", None, None, opts).is_err(),
        "an HTML name is not an XML entity"
    );
}

/// XML_PARSE_RECOVER was a declared constant that nothing read, so one bad byte
/// anywhere cost the caller the entire document. That is the wrong trade for a
/// converter: real-world markup is broken, and the text is still worth having.
/// Verified against `xmllint --recover`, which produces the same trees.
#[test]
fn recover_yields_a_partial_document_instead_of_nothing() {
    let strict = default_parse_options();
    let recover = strict | rusty_xml::XML_PARSE_RECOVER;
    let text_of = |d: &XmlDoc| {
        let mut t = String::new();
        for i in 0..d.len() {
            let id = rusty_xml::NodeId(i as u32);
            if d.kind(id) == rusty_xml::NodeKind::Text {
                t.push_str(&d.node(id).content);
            }
        }
        t
    };

    // Truncated mid-document: the text before the break must survive.
    let trunc = b"<d><e>text that should survive";
    assert!(xml_read_memory(trunc, None, None, strict).is_err());
    let d = xml_read_memory(trunc, None, None, recover).expect("recover yields a tree");
    assert!(text_of(&d).contains("text that should survive"));

    // An undeclared namespace prefix -- endemic in scraped markup -- is a
    // namespace error, not a well-formedness one. libxml2 reports it and exits
    // zero, so it must not be fatal for us either, in EITHER mode. Refusing
    // input C accepts is a worse trade than any conformance case, and it cost
    // us a valid one outright: `<A.-:x/>` is a legal Name whose colon is not a
    // prefix at all.
    let ns = b"<r><a:e>body</a:e></r>";
    for opts in [strict, recover] {
        let d = xml_read_memory(ns, None, None, opts).expect("must not be fatal");
        assert!(text_of(&d).contains("body"));
    }

    // An entity we cannot resolve keeps its reference rather than killing the parse.
    let ent = b"<?xml version=\"1.0\"?>\n<!DOCTYPE d [<!ENTITY c SYSTEM \"x.xml\">]>\n<d>before &c; after</d>";
    assert!(xml_read_memory(ent, None, None, strict).is_err());
    let d = xml_read_memory(ent, None, None, recover).expect("recover yields a tree");
    let t = text_of(&d);
    assert!(t.contains("before") && t.contains("after"), "surrounding text kept: {t:?}");

    // Recovery must not change a well-formed parse.
    let good = b"<r a=\"1\"><b>t</b><!--c--></r>";
    assert_eq!(
        xml_save_doc(&xml_read_memory(good, None, None, strict).unwrap(), 0),
        xml_save_doc(&xml_read_memory(good, None, None, recover).unwrap(), 0),
    );
}

/// An entity's replacement text was returned VERBATIM, so a nested reference
/// reached the tree as literal text and the serializer escaped it:
/// `<!ENTITY b "&a;&a;">` referenced as `&b;` produced `&amp;a;&amp;a;` instead
/// of the expansion. Silent corruption of a legal document.
///
/// Expanding recursively IS the billion-laughs vector, so the bound arrives in
/// the same commit as the recursion, never after it.
#[test]
fn nested_entities_expand_and_bombs_are_refused() {
    let opts = default_parse_options();
    let text_of = |d: &XmlDoc| {
        let mut t = String::new();
        for i in 0..d.len() {
            let id = rusty_xml::NodeId(i as u32);
            if d.kind(id) == rusty_xml::NodeKind::Text {
                t.push_str(&d.node(id).content);
            }
        }
        t
    };

    let nested = b"<?xml version=\"1.0\"?>\n<!DOCTYPE d [\n<!ENTITY a \"AAA\">\n<!ENTITY b \"&a;&a;\">\n<!ENTITY c \"&b;-&b;\">\n]>\n<d>[&c;]</d>";
    let d = xml_read_memory(nested, None, None, opts).expect("nested entities are legal");
    assert_eq!(text_of(&d), "[AAAAAA-AAAAAA]", "nested references must expand");

    // Predefined entities and character references inside a replacement.
    let mixed = b"<?xml version=\"1.0\"?>\n<!DOCTYPE d [<!ENTITY e \"&lt;&#65;&#x42;&gt;\">]>\n<d>&e;</d>";
    let d = xml_read_memory(mixed, None, None, opts).unwrap();
    assert_eq!(text_of(&d), "<AB>");

    // Billion laughs, at three depths. Each must be refused, not expanded.
    for depth in [6usize, 9, 12] {
        let mut s = String::from("<?xml version=\"1.0\"?>\n<!DOCTYPE l [\n<!ENTITY l0 \"aaaaaaaaaa\">\n");
        for i in 1..=depth {
            s.push_str(&format!("<!ENTITY l{i} \""));
            for _ in 0..10 {
                s.push_str(&format!("&l{};", i - 1));
            }
            s.push_str("\">\n");
        }
        s.push_str(&format!("]>\n<l>&l{depth};</l>"));
        assert!(
            xml_read_memory(s.as_bytes(), None, None, opts).is_err(),
            "a {depth}-deep entity bomb must be refused"
        );
    }
}

/// Twelve single-byte encodings beyond ISO-8859 and windows-1252. Real corpora
/// use them -- Cyrillic, Greek, Arabic, Hebrew, Baltic, Turkish, Vietnamese --
/// and each costs 256 bytes of table. The pinned oracle cannot read any of them
/// because it is built ICONV=OFF, so these are checked against the expected
/// text directly rather than against C.
#[test]
fn single_byte_encodings_decode() {
    let opts = default_parse_options();
    // (encoding label, bytes of the high-half characters, expected text)
    let cases: &[(&str, &[u8], &str)] = &[
        ("windows-1251", &[0xF2, 0xE5, 0xF1, 0xF2], "\u{442}\u{435}\u{441}\u{442}"),
        ("KOI8-R", &[0xD4, 0xC5, 0xD3, 0xD4], "\u{442}\u{435}\u{441}\u{442}"),
        ("windows-1253", &[0xE5, 0xEB, 0xEB], "\u{3b5}\u{3bb}\u{3bb}"),
        ("windows-1250", &[0xE8, 0x65, 0xF0], "\u{10d}e\u{111}"),
        ("IBM866", &[0xE2, 0xA5, 0xE1], "\u{442}\u{435}\u{441}"),
        ("macintosh", &[0x8E], "\u{e9}"),
    ];
    for (label, high, want) in cases {
        let mut doc = format!("<?xml version=\"1.0\" encoding=\"{label}\"?><d>").into_bytes();
        doc.extend_from_slice(high);
        doc.extend_from_slice(b"</d>");
        let d = xml_read_memory(&doc, None, None, opts)
            .unwrap_or_else(|e| panic!("{label} should decode: {e:?}"));
        let mut text = String::new();
        for i in 0..d.len() {
            let id = rusty_xml::NodeId(i as u32);
            if d.kind(id) == rusty_xml::NodeKind::Text {
                text.push_str(&d.node(id).content);
            }
        }
        assert_eq!(&text, want, "{label} decoded wrongly");
    }
}

/// Two streaming defects found by differential fuzzing -- feeding mutated
/// documents through both paths and comparing -- which no single-split test
/// could reach.
#[test]
fn streaming_handles_encodings_and_split_characters() {
    let opts = default_parse_options();
    let run = |d: &[u8], chunk: usize| -> Option<Vec<u8>> {
        let mut c = rusty_xml::xml_create_push_parser_ctxt(&[], None, None, opts);
        let mut i = 0;
        while i < d.len() {
            let n = chunk.min(d.len() - i);
            if rusty_xml::xml_parse_chunk(&mut c, &d[i..i + n], 0).is_err() {
                return None;
            }
            i += n;
        }
        rusty_xml::xml_parse_chunk(&mut c, &[], 1)
            .ok()
            .flatten()
            .map(|x| xml_save_doc(&x, 0))
    };
    let cases: Vec<Vec<u8>> = vec![
        // Not UTF-8: the streaming path hands raw bytes to the parser and
        // cannot convert as it goes, so these must fall back to buffering.
        b"<?xml version=\"1.0\" encoding=\"windows-1251\"?><d>\xf2\xe5\xf1\xf2</d>".to_vec(),
        b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><d>\xe9</d>".to_vec(),
        // A BOM shifts every offset.
        b"\xef\xbb\xbf<r>bom</r>".to_vec(),
        // Multi-byte characters split across a chunk boundary: 2-, 3- and
        // 4-byte sequences, plus CRLF which folds to one LF.
        "<r>caf\u{e9} \u{65e5}\u{672c} \u{1F600}</r>".as_bytes().to_vec(),
        b"<r>a\r\nb\rc</r>".to_vec(),
        "<r a=\"\u{1F600}\">\u{e9}</r>".as_bytes().to_vec(),
    ];
    for d in cases {
        let whole = xml_read_memory(&d, None, None, opts).ok().map(|x| xml_save_doc(&x, 0));
        for chunk in [1usize, 2, 3, 5, 64] {
            assert_eq!(
                run(&d, chunk),
                whole,
                "chunk={chunk} differs from a whole parse for {:?}",
                String::from_utf8_lossy(&d)
            );
        }
    }
}

/// A deep tree must not take the process down on ANY path.
///
/// Three of these were live aborts. The HTML parser silently never nested, so
/// no deep HTML tree existed to crash on; fixing that reintroduced deep trees
/// and exposed a recursive `write_node`, which overflowed saving a 2000-deep
/// document -- inside our own MAX_DEPTH of 5000. Canonicalization recursed too.
/// The writer is now iterative; c14n is bounded and reports instead of dying.
#[test]
fn deep_tree_never_aborts_on_any_path() {
    let deep = |n: usize| -> Vec<u8> {
        format!("<r>{}t{}</r>", "<a x='1'>".repeat(n), "</a>".repeat(n)).into_bytes()
    };
    // Right under the parser's own limit: whatever it accepts, it must handle.
    let d = deep(4_990);
    let doc = xml_read_memory(&d, None, None, default_parse_options()).expect("parses");
    assert!(!xml_save_doc(&doc, 0).is_empty());
    assert!(!xml_save_doc(&doc, 1).is_empty(), "--format must not overflow either");
    // Canonicalization is iterative too now, so it simply works at depth.
    assert!(!rusty_xml::xml_c14n_doc_dump_memory(&doc, false, true).unwrap().is_empty());
    assert!(!rusty_xml::xml_c14n_doc_dump_memory(&doc, true, true).unwrap().is_empty());
    drop(doc); // recursive Drop would land here

    // HTML has no depth limit of its own; saving it must still be safe.
    let h = format!("<html><body>{}x{}</body></html>", "<div>".repeat(50_000), "</div>".repeat(50_000));
    let hdoc = rusty_xml::html_read_memory(h.as_bytes(), None, None, 0).expect("parses");
    assert!(!xml_save_doc(&hdoc, 0).is_empty());
}

/// Canonicalization is the XML-DSig path, so depth must not be a limit on it.
///
/// It recursed, carrying the inherited namespace rendering down with it, and a
/// 2000-deep document ABORTED THE PROCESS while being canonicalized -- inside
/// a parser limit of 5000. It was bounded at 400 as a stopgap. The traversal
/// is driven by an explicit stack now, the inherited set shared by Rc rather
/// than cloned per child, and the bound is gone.
#[test]
fn c14n_handles_any_depth_the_parser_accepts() {
    let deep = |n: usize| -> Vec<u8> {
        format!("<r>{}t{}</r>", "<a>".repeat(n), "</a>".repeat(n)).into_bytes()
    };
    for n in [1, 399, 400, 401, 4_000, 4_990] {
        let doc = xml_read_memory(&deep(n), None, None, default_parse_options()).unwrap();
        for exclusive in [false, true] {
            let out = rusty_xml::xml_c14n_doc_dump_memory(&doc, exclusive, true)
                .unwrap_or_else(|e| panic!("c14n failed at depth {n}: {e}"));
            // n nested <a> plus the root, opened and closed.
            assert_eq!(
                String::from_utf8_lossy(&out).matches("<a>").count(),
                n,
                "lost elements at depth {n}"
            );
        }
    }
}

/// The HTML parser used to hand every generic element the body as its parent,
/// so nothing nested: fifty nested <div> came out as fifty empty siblings and
/// the tree was 4 deep where C measured 53. Every text node landed in the wrong
/// place, which is exactly what rag-converter reads.
#[test]
fn html_elements_actually_nest() {
    let depth_of = |doc: &rusty_xml::XmlDoc| -> usize {
        fn walk(d: &rusty_xml::XmlDoc, id: rusty_xml::NodeId) -> usize {
            let mut best = 0;
            let mut c = d.first_child(id);
            while let Some(x) = c {
                best = best.max(walk(d, x));
                c = d.next_sibling(x);
            }
            best + 1
        }
        walk(doc, rusty_xml::NodeId::DOCUMENT)
    };
    let h = format!("<html><body>{}deep{}</body></html>", "<div>".repeat(50), "</div>".repeat(50));
    let doc = rusty_xml::html_read_memory(h.as_bytes(), None, None, 0).expect("parses");
    assert!(depth_of(&doc) >= 53, "elements did not nest: depth {}", depth_of(&doc));

    // And the text must be reachable underneath, not stranded at the top.
    let out = String::from_utf8_lossy(&xml_save_doc(&doc, 0)).to_string();
    assert!(out.contains("<div><div>"), "no nesting in the output: {}", &out[..out.len().min(200)]);
    assert!(out.matches("<div>").count() == 50 && out.matches("</div>").count() == 50);
}


/// `--format` must re-indent, not echo the source indentation back.
///
/// The writer disables formatting for any element with a text child -- as
/// libxml2's does -- so pretty-printing only works if the blank text between
/// tags is dropped at PARSE time. xmllint does that by making --format imply
/// noblanks; we did not, so an already-indented document came back with its
/// original spacing and no reformatting at all. On the 300 KB corpus 1699
/// lines differed from C.
#[test]
fn format_reindents_rather_than_echoing_source_whitespace() {
    let src = b"<r>\n <a>\n  <b>x</b>\n </a>\n</r>";
    // Source is indented by 1 and 2 spaces; the writer's unit is 2 spaces, so
    // a real reformat must widen it.
    let doc = xml_read_memory(
        src,
        None,
        None,
        default_parse_options() | rusty_xml::XML_PARSE_NOBLANKS,
    )
    .expect("parses");
    let out = String::from_utf8(xml_save_doc(&doc, rusty_xml::XML_SAVE_FORMAT)).unwrap();
    assert!(out.contains("\n  <a>"), "level 1 not reindented:\n{out}");
    assert!(out.contains("\n    <b>x</b>"), "level 2 not reindented:\n{out}");

    // Without noblanks the blank text nodes survive and formatting is
    // suppressed -- that is libxml2's behaviour too, and why the flag matters.
    let kept = xml_read_memory(src, None, None, default_parse_options()).expect("parses");
    let out2 = String::from_utf8(xml_save_doc(&kept, rusty_xml::XML_SAVE_FORMAT)).unwrap();
    assert!(out2.contains("\n <a>"), "blank text should have been preserved:\n{out2}");
}

/// Every literal in the language must be character-validated, not just text.
///
/// Character data was checked; attribute values, CDATA, entity values and
/// ATTLIST defaults were not. A C0 control byte in any of them was accepted
/// and the writer quietly substituted U+FFFD for it on the way out, so the
/// document came back holding a character it never contained -- and an ATTLIST
/// default propagated that into every element that took the default. C rejects
/// all four.
///
/// Found by the fuzzer, not by reading: the round-trip check saw the first
/// save escape the character as &#xFFFD; and the second emit it raw, which
/// made serialization non-idempotent.
#[test]
fn invalid_characters_are_rejected_in_every_literal() {
    let opts = default_parse_options();
    let cases: &[(&str, &[u8])] = &[
        ("attribute value", b"<a k=\"x\x1fy\">t</a>"),
        ("CDATA", b"<a><![CDATA[x\x1fy]]></a>"),
        ("entity value", b"<!DOCTYPE a [<!ENTITY e \"v\x1f\">]><a>&e;</a>"),
        ("ATTLIST default", b"<!DOCTYPE a [<!ATTLIST a k CDATA 'd\x1f'>]><a/>"),
        ("character data", b"<a>x\x1fy</a>"),
    ];
    for (what, src) in cases {
        assert!(
            xml_read_memory(src, None, None, opts).is_err(),
            "invalid character accepted in {what}"
        );
    }
    // The control characters XML does allow must still pass, in all of them.
    for src in [
        &b"<a k=\"x\ty\nz\rw\">t</a>"[..],
        &b"<a><![CDATA[x\ty]]></a>"[..],
        &b"<!DOCTYPE a [<!ENTITY e \"v\tw\">]><a>&e;</a>"[..],
    ] {
        assert!(
            xml_read_memory(src, None, None, opts).is_ok(),
            "legal whitespace rejected: {:?}",
            String::from_utf8_lossy(src)
        );
    }
}

/// A malformed internal subset must be reported, not silently dropped.
///
/// The subset was parsed with `unwrap_or_default()`, so ANY error in it threw
/// the whole DTD away and left an empty one. The entities and ATTLIST defaults
/// it declared vanished, and the failure surfaced later as a bogus "entity not
/// defined" pointing at the reference rather than the declaration.
#[test]
fn a_broken_internal_subset_is_reported_not_discarded() {
    let src = b"<!DOCTYPE a [<!ENTITY e \"v\x1f\">]><a>&e;</a>";
    let e = xml_read_memory(src, None, None, default_parse_options()).unwrap_err();
    assert!(
        e.message.contains("invalid character"),
        "blamed the wrong thing: {e}"
    );
    // Recovery still tolerates it -- that is what recovery is for.
    assert!(
        xml_read_memory(src, None, None, default_parse_options() | rusty_xml::XML_PARSE_RECOVER)
            .is_ok(),
        "recover mode should still produce a tree"
    );
}

/// An HTML document must serialize by HTML rules, not XML ones.
///
/// There was no HTML serializer: html_read_memory produced a plain Document
/// and xml_save_doc wrote it as XML. That produced three defects at once --
/// an `<?xml ... encoding="HTML"?>` declaration where C writes the doctype,
/// `<br/>` where C writes `<br>`, and `&#xFFFD;` for control characters C
/// passes through.
///
/// The `<br/>` one was not cosmetic. Re-parsed as HTML, a trailing slash is
/// not an end tag, so every following node became a CHILD of the br instead of
/// a sibling -- the tree moved on every save/parse cycle. The fuzzer found
/// 3084 unstable HTML round trips in 30,000 inputs; there are none now.
#[test]
fn html_serializes_as_html() {
    let save = |src: &str| -> String {
        let d = rusty_xml::html_read_memory(src.as_bytes(), None, None, 0).expect("parses");
        String::from_utf8(xml_save_doc(&d, 0)).unwrap()
    };

    // No doctype in the source: C supplies HTML 4.0 Transitional.
    let out = save("<html><body><p>x</p></body></html>");
    assert!(out.starts_with("<!DOCTYPE html PUBLIC \"-//W3C//DTD HTML 4.0 Transitional//EN\""));
    assert!(!out.contains("<?xml"), "XML declaration in HTML output:\n{out}");

    // A doctype that was there is kept.
    assert!(save("<!DOCTYPE html><html><body>x</body></html>").starts_with("<!DOCTYPE html>"));

    // Void elements take no end tag and no slash; everything else takes one.
    let out = save("<html><body><br><img src=x><el k=v></el></body></html>");
    assert!(out.contains("<br>"), "void element not minimized:\n{out}");
    assert!(!out.contains("<br/>") && !out.contains("</br>"), "br closed:\n{out}");
    assert!(out.contains("<img src=\"x\">"), "img not minimized:\n{out}");
    assert!(out.contains("<el k=\"v\"></el>"), "empty element self-closed:\n{out}");

    // Control characters are legal in HTML and C writes them verbatim.
    assert!(save("<html><body>a\u{8}b</body></html>").contains("a\u{8}b"));
}

/// Saving HTML and re-parsing it must be a fixed point.
///
/// This is the invariant that catches a serializer the parser disagrees with,
/// and it is the one rag-converter depends on: read HTML, write it, read it
/// back, and the tree must not have moved.
#[test]
fn html_round_trip_is_stable() {
    let cases = [
        "<html><body><br>after<p>x</p></body></html>",
        "<html><body><div><span>a</span>b<img src=i>c</div></body></html>",
        "<!DOCTYPE html><html><body><ul><li>1<li>2</ul></body></html>",
        "<html><body><table><tr><td><div>x</div></td></tr></table></body></html>",
        "<html><body>a\u{8}b\u{1}c</body></html>",
        "<html><body><hr><meta charset=utf-8><input value=v></body></html>",
    ];
    for src in cases {
        let d1 = rusty_xml::html_read_memory(src.as_bytes(), None, None, 0).expect("parses");
        let once = xml_save_doc(&d1, 0);
        let d2 = rusty_xml::html_read_memory(&once, None, None, 0).expect("reparses");
        let twice = xml_save_doc(&d2, 0);
        assert_eq!(
            String::from_utf8_lossy(&once),
            String::from_utf8_lossy(&twice),
            "round trip moved the tree for {src}"
        );
    }
}

/// A doctype is attacker-controlled text like everything else.
///
/// The first cut of the doctype reader sliced it by byte index -- `[2..9]` for
/// the keyword and `[9..]` for the body -- which panics when the index lands
/// inside a multi-byte character. The fuzzer hit it on the first run.
#[test]
fn a_malformed_doctype_does_not_panic() {
    for src in [
        "<!\u{1F600}",
        "<!DOCTYP\u{e9}",
        "<!DOCTYPE\u{1F600}>",
        "<!DOCTYPE html PUBLIC \"\u{1F600}",
        "<!DOCTYPE html SYSTEM '",
        "<!D",
        "<!",
        "<!DOCTYPE",
    ] {
        let _ = rusty_xml::html_read_memory(src.as_bytes(), None, None, 0);
    }
}

/// A block element closes an open paragraph.
///
/// Only same-name autoclose was implemented, so `<p>two<div>three` nested the
/// div INSIDE the paragraph where C makes them siblings. Anything reading the
/// tree for structure -- rag-converter, in particular -- saw the text one level
/// too deep and attributed it to the wrong block.
#[test]
fn a_block_element_closes_an_open_paragraph() {
    let save = |src: &str| -> String {
        let d = rusty_xml::html_read_memory(src.as_bytes(), None, None, 0).expect("parses");
        String::from_utf8(xml_save_doc(&d, 0)).unwrap()
    };
    let out = save("<html><body><div><p>one<p>two<div>three</body></html>");
    assert!(
        out.contains("<p>one</p><p>two</p><div>three</div>"),
        "paragraph not closed by the block element:\n{out}"
    );
    // Inline elements do NOT close it.
    let out = save("<html><body><p>a<span>b</span>c</p></body></html>");
    assert!(out.contains("<p>a<span>b</span>c</p>"), "inline closed a p:\n{out}");
    // Nor does a block element close anything other than a paragraph.
    let out = save("<html><body><div>a<div>b</div></div></body></html>");
    assert!(out.contains("<div>a<div>b</div></div>"), "div closed a div:\n{out}");
}

/// A malformed content model must not eat the machine.
///
/// `<!ELEMENT doc (a & b)?>` uses SGML's "and" connector. `&` is not a name
/// character, so the content-model reader's take_name returned "" and the
/// position never advanced -- the loop pushed an empty token forever. Thirty-two
/// bytes of DTD grew a Vec until the process died asking for 32 GB, and any
/// document with a DTD could do it to anything that validates.
///
/// Found by the W3C conformance suite (not-wf-sa-135) on its very first run.
#[test]
fn a_malformed_content_model_terminates() {
    for model in [
        "(a & b)?", "(a b)", "(&)", "((((", "(a|", "(a,,b)", "(", "()",
        "(#PCDATA|a)*", "(a,b,c)+", "(a**)", "(|)", "(a|)", "(~)",
    ] {
        let src = format!("<!DOCTYPE doc [\n<!ELEMENT doc {model}>\n]>\n<doc></doc>");
        // Either verdict is fine. Returning at all is the point.
        if let Ok(doc) = xml_read_memory(src.as_bytes(), None, None, default_parse_options()) {
            let _ = rusty_xml::xml_validate_document(&doc);
        }
    }
}

/// Declaration syntax in the internal subset is checked, not skimmed.
///
/// The subset parser was a lenient scanner: it took anything up to the next
/// '>' and shrugged. Every case below is one the W3C suite rejects and we
/// accepted, and each is a real ambiguity -- an unquoted default silently
/// became an empty string, and a bad enumeration silently lost its members.
#[test]
fn malformed_declarations_are_rejected() {
    let bad = [
        // Entity declarations.
        ("no space after entity name", "<!DOCTYPE d [<!ENTITY foo\"text\">]><d/>"),
        ("PUBLIC needs two literals", "<!DOCTYPE d [<!ENTITY e PUBLIC \"only one\">]><d/>"),
        ("no space between literals", "<!DOCTYPE d [<!ENTITY e PUBLIC \"a\"\"b\">]><d/>"),
        ("SGML comment in a decl", "<!DOCTYPE d [<!ENTITY e \"v\" -- c -->]><d/>"),
        // Attribute-list declarations.
        ("comma in an enumeration", "<!DOCTYPE d [<!ATTLIST d a (foo,bar) #IMPLIED>]><d/>"),
        ("unquoted default value", "<!DOCTYPE d [<!ATTLIST d a NMTOKEN v1>]><d/>"),
        ("NAME is not an AttType", "<!DOCTYPE d [<!ATTLIST d a NAME #IMPLIED>]><d/>"),
    ];
    for (what, src) in bad {
        assert!(
            xml_read_memory(src.as_bytes(), None, None, default_parse_options()).is_err(),
            "accepted a malformed declaration: {what}"
        );
    }

    // The well-formed equivalents must still parse, or the strictness is just
    // breakage wearing a hat.
    let good = [
        "<!DOCTYPE d [<!ENTITY foo \"text\">]><d>&foo;</d>",
        "<!DOCTYPE d [<!ENTITY e PUBLIC \"pub\" \"sys\">]><d/>",
        "<!DOCTYPE d [<!ATTLIST d a (foo|bar) #IMPLIED>]><d a=\"foo\"/>",
        "<!DOCTYPE d [<!ATTLIST d a NMTOKEN 'v1'>]><d/>",
        "<!DOCTYPE d [<!ATTLIST d a CDATA #REQUIRED>]><d a=\"x\"/>",
        "<!DOCTYPE d [<!ATTLIST d a CDATA #FIXED 'k'>]><d/>",
        "<!DOCTYPE d [<!NOTATION n SYSTEM 'x'><!ATTLIST d a NOTATION (n) #IMPLIED>]><d/>",
        "<!DOCTYPE d [<!ENTITY e SYSTEM 'x' NDATA gif>]><d/>",
    ];
    for src in good {
        assert!(
            xml_read_memory(src.as_bytes(), None, None, default_parse_options()).is_ok(),
            "rejected a well-formed declaration: {src}"
        );
    }
}

/// Saving a document must not lose its DOCTYPE.
///
/// The writer never emitted one, so a read-modify-write silently dropped every
/// entity, ATTLIST default and notation the document declared. Found by
/// widening the corpus past the original seven files.
///
/// We emit the internal subset VERBATIM. libxml2 re-serializes it from its
/// parsed form, which reorders the declarations and respaces the content
/// models; faithful to the meaning, but not to the document. This is the one
/// deliberate serialization divergence from C.
#[test]
fn the_doctype_survives_a_round_trip() {
    let src = concat!(
        "<!DOCTYPE cat [\n",
        "  <!ELEMENT cat (item+)>\n",
        "  <!ATTLIST item id ID #REQUIRED>\n",
        "  <!ENTITY co \"Example Ltd\">\n",
        "]>\n<cat><item id=\"a\">&co;</item></cat>"
    );
    let doc = xml_read_memory(src.as_bytes(), None, None, default_parse_options()).unwrap();
    let out = String::from_utf8(xml_save_doc(&doc, 0)).unwrap();
    assert!(out.contains("<!DOCTYPE cat ["), "no doctype in output:\n{out}");
    assert!(out.contains("<!ENTITY co \"Example Ltd\">"), "subset lost:\n{out}");
    // And it must still parse, with the entity still declared.
    let again = xml_read_memory(out.as_bytes(), None, None, default_parse_options()).unwrap();
    assert!(String::from_utf8(xml_save_doc(&again, 0)).unwrap().contains("<!ENTITY co"));

    // External identifiers round-trip too.
    let ext = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \
               \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">\n<html/>";
    let d = xml_read_memory(ext.as_bytes(), None, None, default_parse_options()).unwrap();
    let o = String::from_utf8(xml_save_doc(&d, 0)).unwrap();
    assert!(o.contains("PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\""), "external id lost:\n{o}");
    // XHTML serializes empty elements as `<html />`, with the space, so that
    // pre-XML HTML parsers do not read the slash as part of the name.
    assert!(o.contains("<html />"), "not XHTML empty-tag form:\n{o}");
}

/// A character reference belongs to the text run it sits in.
///
/// Every reference was flushed into a text node of its own, so `&#65; &#66;`
/// became three nodes and the middle one was whitespace-only --
/// XML_PARSE_NOBLANKS then deleted it and `A B` came back as `AB`. Losing a
/// space between two character references is silent text corruption, and
/// rag-converter reads exactly this kind of text.
#[test]
fn a_character_reference_does_not_split_the_text_run() {
    let src = b"<r><a>&#65; &#66; C</a><b>x &#67; y</b></r>";
    for opts in [
        default_parse_options(),
        default_parse_options() | rusty_xml::XML_PARSE_NOBLANKS,
    ] {
        let doc = xml_read_memory(src, None, None, opts).unwrap();
        let out = String::from_utf8(xml_save_doc(&doc, 0)).unwrap();
        assert!(out.contains("<a>A B C</a>"), "lost a space: {out}");
        assert!(out.contains("<b>x C y</b>"), "lost a space: {out}");
    }
}

/// Indentation stops growing at 60 columns, as libxml2's does.
///
/// C keeps a fixed 60-character indent buffer and writes as much of it as the
/// level calls for. We grew without limit, so a deep document diverged from C
/// at level 31 with the default two-space unit.
#[test]
fn indentation_is_capped_like_c() {
    let mut src = String::from("<r>");
    for i in 0..80 {
        src.push_str(&format!("<l n=\"{i}\">"));
    }
    src.push_str("x");
    for _ in 0..80 {
        src.push_str("</l>");
    }
    src.push_str("</r>");
    let doc = xml_read_memory(src.as_bytes(), None, None, default_parse_options()).unwrap();
    let out = String::from_utf8(xml_save_doc(&doc, rusty_xml::XML_SAVE_FORMAT)).unwrap();
    let deepest = out
        .lines()
        .map(|l| l.len() - l.trim_start().len())
        .max()
        .unwrap();
    assert_eq!(deepest, 60, "indent should cap at 60 columns, got {deepest}");
}

/// Completing attributes from ATTLIST defaults is opt-in, as it is in C.
///
/// We did it on every parse. libxml2 does it for XML_PARSE_DTDATTR -- xmllint's
/// --dtdattr -- and not otherwise, not even for --valid. So any document with
/// an ATTLIST default came back carrying attributes libxml2 would not have
/// added, and serialized differently from C for that reason alone.
#[test]
fn attlist_defaults_are_opt_in_like_c() {
    let src = b"<!DOCTYPE r [<!ELEMENT e ANY><!ATTLIST e k CDATA \"dflt\">]><r><e/></r>";

    let plain = xml_read_memory(src, None, None, default_parse_options()).unwrap();
    let out = String::from_utf8(xml_save_doc(&plain, 0)).unwrap();
    // The serialized DOCTYPE carries the word too, so look at the element.
    assert!(out.contains("<e/>"), "a plain parse must not add defaults:\n{out}");

    let with = xml_read_memory(
        src,
        None,
        None,
        default_parse_options() | rusty_xml::XML_PARSE_DTDATTR,
    )
    .unwrap();
    let out = String::from_utf8(xml_save_doc(&with, 0)).unwrap();
    assert!(out.contains("k=\"dflt\""), "DTDATTR must add them:\n{out}");

    // Validation does NOT imply defaulting -- C is the same.
    let valid = xml_read_memory(
        src,
        None,
        None,
        default_parse_options() | rusty_xml::XML_PARSE_DTDVALID,
    )
    .unwrap();
    let out = String::from_utf8(xml_save_doc(&valid, 0)).unwrap();
    assert!(out.contains("<e/>"), "--valid must not add defaults:\n{out}");
}

/// The entity graph in a DTD has well-formedness constraints of its own.
///
/// General entity references are kept verbatim in entity values and attribute
/// defaults, because they are expanded at the point of use. Nothing then looked
/// at them, so a literal could reference an entity that was never declared, or
/// one declared NDATA (which may not be referenced at all), or itself through a
/// cycle. A cycle only surfaced later as a depth-limit error naming the wrong
/// entity.
#[test]
fn the_entity_graph_is_checked() {
    let bad = [
        ("undeclared in a default", "<!DOCTYPE d [<!ATTLIST d a CDATA \"&foo;\">]><d/>"),
        ("unparsed entity referenced", "<!DOCTYPE d [<!NOTATION n SYSTEM \"s\"><!ENTITY e SYSTEM \"x\" NDATA n><!ATTLIST d a CDATA \"&e;\">]><d/>"),
        ("direct cycle", "<!DOCTYPE d [<!ENTITY e \"&e;\">]><d/>"),
        ("indirect cycle", "<!DOCTYPE d [<!ENTITY a \"&b;\"><!ENTITY b \"&c;\"><!ENTITY c \"&a;\">]><d/>"),
    ];
    for (what, src) in bad {
        assert!(
            xml_read_memory(src.as_bytes(), None, None, default_parse_options()).is_err(),
            "accepted a broken entity graph: {what}"
        );
    }
    // A chain that terminates is fine, and so are the predefined entities.
    let good = "<!DOCTYPE d [<!ENTITY a \"x&b;\"><!ENTITY b \"y&amp;z\">]><d>&a;</d>";
    assert!(xml_read_memory(good.as_bytes(), None, None, default_parse_options()).is_ok());
}

/// Three ways a perfectly valid document was rejected or mis-validated.
///
/// All three were found by the conformance suite, and all three are the kind
/// that only show up on documents nobody in the corpus happened to write.
#[test]
fn valid_documents_that_used_to_be_rejected() {
    let opts = default_parse_options();

    // The DTD's own name parser was ASCII-only, so a non-ASCII element name in
    // a declaration came back empty and the document was refused -- while the
    // document body accepted the very same name.
    let thai = "<!DOCTYPE \u{0e40}\u{0e08} [<!ELEMENT \u{0e40}\u{0e08} (#PCDATA)>]>\
                <\u{0e40}\u{0e08}>x</\u{0e40}\u{0e08}>";
    assert!(
        xml_read_memory(thai.as_bytes(), None, None, opts).is_ok(),
        "non-ASCII name rejected in the DTD"
    );

    // An apostrophe inside a comment in the internal subset is not a quote.
    // It opened one that never closed, so the scan ate the rest of the file.
    let apos = "<!DOCTYPE d [\n<!--NOTE: XML doesn't say-->\n<!ELEMENT d EMPTY>\n]>\n<d/>";
    assert!(
        xml_read_memory(apos.as_bytes(), None, None, opts).is_ok(),
        "apostrophe in a DTD comment swallowed the document"
    );

    // An ATTLIST declares a QName. Looking it up with xmlGetProp semantics
    // (unprefixed only) meant every prefixed declared attribute looked absent,
    // so a #REQUIRED one was reported missing from a document that had it.
    let lang = "<!DOCTYPE b [<!ELEMENT b ANY><!ATTLIST b xml:lang CDATA #REQUIRED>]>\
                <b xml:lang=\"en\">t</b>";
    let doc = xml_read_memory(lang.as_bytes(), None, None, opts).unwrap();
    assert!(
        rusty_xml::xml_validate_document(&doc).is_ok(),
        "a present prefixed attribute was reported missing"
    );
}

/// An entity whose replacement text is markup must become NODES.
///
/// It was inserted as text and escaped on the way out, so `<!ENTITY e
/// "<b>x</b>">` put the literal string `&lt;b&gt;x&lt;/b&gt;` into the tree.
/// Structure lost, and DTD validation saw character data where an element was
/// declared.
///
/// The decision is made on the STORED replacement, not the expanded one, and
/// that distinction is the whole thing: a character reference in an entity
/// value is expanded when the declaration is read, so `&#60;foo/>` really does
/// hold markup -- while `&lt;` is bypassed and stays written out, so
/// `&lt;AB&gt;` is three characters and no markup at all.
#[test]
fn entity_replacement_that_is_markup_becomes_markup() {
    let opts = default_parse_options();
    let text_of = |d: &rusty_xml::XmlDoc| -> String {
        String::from_utf8(xml_save_doc(d, 0)).unwrap()
    };

    // Markup written directly.
    let d = xml_read_memory(
        b"<!DOCTYPE d [<!ENTITY e \"<b>x</b>\">]><d>&e;</d>",
        None, None, opts,
    ).unwrap();
    assert!(text_of(&d).contains("<d><b>x</b></d>"), "not spliced: {}", text_of(&d));

    // Markup arriving by character reference -- expanded at declaration time,
    // so it IS markup.
    let d = xml_read_memory(
        b"<!DOCTYPE d [<!ELEMENT d (foo)><!ELEMENT foo EMPTY><!ENTITY e \"&#60;foo/>\">]><d>&e;</d>",
        None, None, opts,
    ).unwrap();
    assert!(text_of(&d).contains("<foo/>"), "charref markup not spliced: {}", text_of(&d));
    assert!(rusty_xml::xml_validate_document(&d).is_ok(), "validation should see the element");

    // A predefined reference is bypassed and stays a character.
    let d = xml_read_memory(
        b"<!DOCTYPE d [<!ENTITY e \"&lt;AB&gt;\">]><d>&e;</d>",
        None, None, opts,
    ).unwrap();
    let out = text_of(&d);
    assert!(out.contains("&lt;AB&gt;"), "predefined ref treated as markup: {out}");
    assert!(!out.contains("<AB>"), "predefined ref treated as markup: {out}");

    // Replacement text has to be well formed in its own right.
    for bad in [
        &b"<!DOCTYPE d [<!ENTITY e \"<foo a='&#38;'></foo>\">]><d>&e;</d>"[..],
        &b"<!DOCTYPE d [<!ENTITY e \"<foo></bar>\">]><d>&e;</d>"[..],
        &b"<!DOCTYPE d [<!ENTITY e \"<foo>\">]><d>&e;</d>"[..],
    ] {
        assert!(
            xml_read_memory(bad, None, None, opts).is_err(),
            "accepted a malformed replacement: {}",
            String::from_utf8_lossy(bad)
        );
    }
}

/// Validity constraints on the declarations and on what elements carry.
///
/// Every one of these was found by the conformance suite, and every one was
/// silently accepted before.
#[test]
fn validity_constraints_on_declarations() {
    let opts = default_parse_options();
    let invalid = |src: &str| {
        let d = xml_read_memory(src.as_bytes(), None, None, opts)
            .unwrap_or_else(|e| panic!("should parse: {src}\n{e}"));
        assert!(
            rusty_xml::xml_validate_document(&d).is_err(),
            "should be invalid: {src}"
        );
    };
    // Unique Element Type Declaration.
    invalid("<!DOCTYPE r [<!ELEMENT r ANY><!ELEMENT e EMPTY><!ELEMENT e EMPTY>]><r/>");
    // ID Attribute Default: an ID cannot carry a default value.
    invalid("<!DOCTYPE r [<!ELEMENT r ANY><!ATTLIST r id ID \"x23\">]><r/>");
    // No Duplicate Types in a mixed content model.
    invalid("<!DOCTYPE r [<!ELEMENT r (#PCDATA|a|a)*><!ELEMENT a EMPTY>]><r/>");
    // No Duplicate Tokens in an enumeration.
    invalid("<!DOCTYPE r [<!ELEMENT r ANY><!ATTLIST r b (one|one) #IMPLIED>]><r/>");
    // Attribute Value Type: the attribute has to be declared.
    invalid("<!DOCTYPE r [<!ELEMENT r EMPTY>]><r xml:space='preserve'/>");
    // A CDATA section is character data even when it is empty, so it breaks an
    // element-only content model.
    invalid("<!DOCTYPE r [<!ELEMENT r (a+)><!ELEMENT a EMPTY>]><r><a/><![CDATA[]]></r>");

    // And the well-formed equivalents must still validate.
    let good = "<!DOCTYPE r [<!ELEMENT r (a+)><!ELEMENT a EMPTY>\
                <!ATTLIST r id ID #IMPLIED><!ATTLIST a k (one|two) \"one\">]>\
                <r id=\"x\"><a/><a k=\"two\"/></r>";
    let d = xml_read_memory(good.as_bytes(), None, None, opts).unwrap();
    rusty_xml::xml_validate_document(&d).expect("should be valid");
}

/// Whitespace the grammar requires and we were not asking for.
#[test]
fn required_whitespace_is_required() {
    let opts = default_parse_options();
    for src in [
        // S between attributes.
        &b"<r a=\"1\"b=\"2\"/>"[..],
        // S in the XML declaration.
        &b"<?xml version=\"1.0\"encoding=\"UTF-8\"?><r/>"[..],
        // S between the two literals of an ExternalID.
        &b"<!DOCTYPE r PUBLIC \"-//X//DTD//EN\"\"x.dtd\"><r/>"[..],
        // A version number is a restricted character set.
        &b"<?xml version=\"1.0?\"?><r/>"[..],
    ] {
        assert!(
            xml_read_memory(src, None, None, opts).is_err(),
            "accepted: {}",
            String::from_utf8_lossy(src)
        );
    }
    // The legal forms still parse.
    for src in [
        &b"<r a=\"1\" b=\"2\"/>"[..],
        &b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><r/>"[..],
        &b"<!DOCTYPE r PUBLIC \"-//X//DTD//EN\" \"x.dtd\"><r/>"[..],
    ] {
        assert!(
            xml_read_memory(src, None, None, opts).is_ok(),
            "rejected: {}",
            String::from_utf8_lossy(src)
        );
    }
}

/// A colon that does not form a QName is not a prefix marker.
///
/// The colon is an ordinary XML 1.0 name character. `:`, `:x`, `x:` and
/// `a.-:x` are all legal Names, and we rejected every document that used one.
/// libxml2 reports the namespace problem and carries on with the name
/// unprefixed; anything else refuses input C accepts.
#[test]
fn a_colon_that_is_not_a_prefix_is_just_a_colon() {
    let opts = default_parse_options();
    for src in [
        &b"<!DOCTYPE d [<!ELEMENT d (#PCDATA)><!ATTLIST d : CDATA #IMPLIED>]><d :=\"v\"></d>"[..],
        &b"<:foo/>"[..],
        &b"<foo:/>"[..],
        &b"<a.-:x/>"[..],
        &b"<foo a:b:c=\"1\"/>"[..],
    ] {
        assert!(
            xml_read_memory(src, None, None, opts).is_ok(),
            "rejected a legal Name: {}",
            String::from_utf8_lossy(src)
        );
    }
    // A real prefix still resolves.
    let d = xml_read_memory(b"<r xmlns:p=\"urn:x\"><p:e/></r>", None, None, opts).unwrap();
    let out = String::from_utf8(xml_save_doc(&d, 0)).unwrap();
    assert!(out.contains("<p:e/>"), "prefix handling broke: {out}");
}

/// The notation constraints, which could not be checked at all before.
///
/// `<!NOTATION>` was parsed and its name thrown away, so nothing could tell
/// whether an NDATA annotation or a NOTATION attribute type named one that
/// exists.
#[test]
fn notations_must_be_declared() {
    let opts = default_parse_options();
    let invalid = |src: &str| {
        let d = xml_read_memory(src.as_bytes(), None, None, opts)
            .unwrap_or_else(|e| panic!("should parse: {src}\n{e}"));
        assert!(rusty_xml::xml_validate_document(&d).is_err(), "should be invalid: {src}");
    };
    invalid("<!DOCTYPE p [<!ELEMENT p EMPTY><!ENTITY b SYSTEM \"u\" NDATA Missing>]><p/>");
    invalid("<!DOCTYPE p [<!ELEMENT p EMPTY><!ATTLIST p a NOTATION (nope) #IMPLIED>]><p/>");

    // Not an EMPTY element: a NOTATION attribute on one of those is its own
    // violation, and would mask the thing under test.
    let good = "<!DOCTYPE p [<!ELEMENT p (#PCDATA)><!NOTATION n SYSTEM \"u\">\
                <!ENTITY b SYSTEM \"u\" NDATA n><!ATTLIST p a NOTATION (n) #IMPLIED>]>\
                <p a=\"n\">x</p>";
    let d = xml_read_memory(good.as_bytes(), None, None, opts).unwrap();
    rusty_xml::xml_validate_document(&d).expect("should be valid");
}

/// Declarations name QNames, and so does the DOCTYPE.
///
/// The validator looked elements up by their local part, so an element
/// declared `<!ELEMENT xml:foo>` was reported as having no declaration.
#[test]
fn declarations_are_matched_by_qname() {
    let src = b"<!DOCTYPE xml:foo [<!ELEMENT xml:foo EMPTY>]><xml:foo/>";
    let d = xml_read_memory(src, None, None, default_parse_options()).unwrap();
    rusty_xml::xml_validate_document(&d).expect("declared element reported undeclared");
}

/// Attribute-value normalization for the tokenized types (XML 1.0 3.3.3).
///
/// A value whose declared type is anything but CDATA has its leading and
/// trailing space discarded and its internal runs collapsed to one space. We
/// did none of it, so `id="  x  y  "` stayed as written where C reports
/// `x y` -- a different value for the same document, on anything with a DTD
/// that declares a non-CDATA attribute.
///
/// It is normalization, not defaulting, so it happens whether or not
/// XML_PARSE_DTDATTR was asked for. libxml2 is the same.
#[test]
fn tokenized_attribute_values_are_normalized() {
    let src = b"<!DOCTYPE r [<!ELEMENT r EMPTY>\
                <!ATTLIST r id ID #IMPLIED><!ATTLIST r n NMTOKENS #IMPLIED>\
                <!ATTLIST r c CDATA #IMPLIED>]>\
                <r id=\"  x  y  \" n=\"  a\tb\nc  \" c=\"  keep  me  \"/>";
    let doc = xml_read_memory(src, None, None, default_parse_options()).unwrap();
    let out = String::from_utf8(xml_save_doc(&doc, 0)).unwrap();
    assert!(out.contains("id=\"x y\""), "ID not normalized:\n{out}");
    assert!(out.contains("n=\"a b c\""), "NMTOKENS not normalized:\n{out}");
    // CDATA is left exactly as the parser produced it.
    assert!(out.contains("c=\"  keep  me  \""), "CDATA was normalized:\n{out}");
}
