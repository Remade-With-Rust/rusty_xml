//! Resource limits and cancellation for the validators, C14N, the writer and
//! the tree: #36 (DTD content models), #39 (interruptible calls) and the #41
//! review of everything that walks a document or a schema.

use rusty_xml::*;
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn parse(src: &str) -> XmlDoc {
    xml_read_memory(src.as_bytes(), None, None, default_parse_options()).expect("parses")
}

fn parse_huge(src: &str) -> XmlDoc {
    xml_read_memory(src.as_bytes(), None, None, default_parse_options() | XML_PARSE_HUGE)
        .expect("parses")
}

/// Run `f` on a thread with a 1 MB stack, the size of a Windows main thread,
/// so a walk that recurses per level overflows here rather than passing on a
/// generous test-thread stack.
fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("no panic and no stack overflow")
}

fn deep(d: usize) -> String {
    format!("{}{}", "<a>x".repeat(d), "</a>".repeat(d))
}

// ---------------------------------------------------------------- #36

#[test]
fn dtd_star_over_a_repeatable_body_is_not_exponential() {
    // (a*)* could consume the same children in many ways, and the old matcher
    // expanded every one: 26 children took 24 seconds.
    for n in [26, 10_000] {
        let src = format!(
            "<!DOCTYPE r [<!ELEMENT r (a*)*><!ELEMENT a EMPTY>]><r>{}</r>",
            "<a/>".repeat(n)
        );
        let doc = parse(&src);
        let t = Instant::now();
        xml_validate_document(&doc).expect("valid");
        assert!(t.elapsed() < Duration::from_secs(2), "{n} children took {:?}", t.elapsed());
    }
}

#[test]
fn dtd_content_models_still_match_and_mismatch() {
    let cases: &[(&str, &str, bool)] = &[
        ("(a,b)", "<a/><b/>", true),
        ("(a,b)", "<b/><a/>", false),
        ("(a,(b|c)*,d)", "<a/><b/><c/><b/><d/>", true),
        ("(a,(b|c)*,d)", "<a/><b/><c/>", false),
        ("(a+)", "", false),
        ("(a,b)?", "", true),
        ("((a,b)|(c,d))", "<c/><d/>", true),
        ("((a,b)|(c,d))", "<a/><d/>", false),
    ];
    for &(model, kids, ok) in cases {
        let src = format!(
            "<!DOCTYPE r [<!ELEMENT r {model}><!ELEMENT a EMPTY><!ELEMENT b EMPTY>\
             <!ELEMENT c EMPTY><!ELEMENT d EMPTY>]><r>{kids}</r>"
        );
        assert_eq!(xml_validate_document(&parse(&src)).is_ok(), ok, "{model} {kids}");
    }
}

#[test]
fn dtd_non_deterministic_models_are_reported_as_libxml2_reports_them() {
    // Every message is xmllint 2.15.3's own text for the same document.
    let cases: &[(&str, Option<&str>)] = &[
        ("(a|(a,a))*", Some("Content model of r is not deterministic: (a | (a , a))*")),
        ("(a|a)", Some("Content model of r is not deterministic: (a | a)")),
        ("((a,b)|(a,c))", Some("Content model of r is not deterministic: ((a , b) | (a , c))")),
        ("(a*,a)", Some("Content model of r is not deterministic: (a* , a)")),
        ("((a?),a?)*", Some("Content model of r is not deterministic: (a? , a?)*")),
        // libxml2 merges the states of these, so it accepts them.
        ("(a|a)*", None),
        ("((a|b|a)*,c)", None),
        ("(a*)*", None),
        ("((a,b?)*)", None),
        ("(a,(a|b))", None),
    ];
    for &(model, want) in cases {
        let src = format!(
            "<!DOCTYPE r [<!ELEMENT r {model}><!ELEMENT a EMPTY><!ELEMENT b EMPTY>\
             <!ELEMENT c EMPTY>]><r/>"
        );
        let got = xml_validate_document(&parse(&src)).err();
        match want {
            Some(msg) => assert_eq!(got.as_deref(), Some(msg), "{model}"),
            None => assert!(
                !got.as_deref().unwrap_or("").contains("not deterministic"),
                "{model}: {got:?}"
            ),
        }
    }
}

#[test]
fn dtd_non_determinism_is_only_reported_for_element_types_in_use() {
    // As in libxml2, which builds a content model when an element of that
    // type is validated.
    let src = "<!DOCTYPE r [<!ELEMENT r (a)*><!ELEMENT x (a|a)><!ELEMENT a EMPTY>]><r><a/></r>";
    xml_validate_document(&parse(src)).expect("x is never used");
    let mixed = "<!DOCTYPE r [<!ELEMENT r (#PCDATA|a)*><!ELEMENT a ANY>]><r>t<a/>t</r>";
    xml_validate_document(&parse(mixed)).expect("mixed content");
}

// ---------------------------------------------------------------- #39

fn big_dtd_doc() -> XmlDoc {
    parse(&format!(
        "<!DOCTYPE r [<!ELEMENT r (a)*><!ELEMENT a (#PCDATA)>]><r>{}</r>",
        "<a>t</a>".repeat(20_000)
    ))
}

/// An interrupt that fires on its `n`th poll, counting the polls.
fn fires_after(n: u32) -> (impl Fn() -> bool, std::rc::Rc<Cell<u32>>) {
    let polls = std::rc::Rc::new(Cell::new(0));
    let p = polls.clone();
    (
        move || {
            p.set(p.get() + 1);
            p.get() >= n
        },
        polls,
    )
}

#[test]
fn an_interrupt_that_has_fired_stops_every_call() {
    let set = AtomicBool::new(true);
    let doc = big_dtd_doc();
    assert_eq!(xml_validate_document_interruptible(&doc, &set), Err(ValidError::Interrupted));
    assert!(xml_validate_dtd_interruptible(&doc, doc.dtd.as_ref().unwrap(), &set)
        .unwrap_err()
        .is_interrupted());
    assert_eq!(
        xml_c14n_doc_dump_memory_interruptible(&doc, false, false, &set),
        Err(ValidError::Interrupted)
    );
    assert_eq!(xml_save_doc_interruptible(&doc, 0, &set), Err(Interrupted));
    let rng = br#"<element name="r" xmlns="http://relaxng.org/ns/structure/1.0"><zeroOrMore><element name="a"><text/></element></zeroOrMore></element>"#;
    assert_eq!(xml_relaxng_validate_doc_interruptible(rng, &doc, &set), Err(ValidError::Interrupted));
    let xsd = br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="r"><xs:complexType><xs:sequence><xs:element name="a" maxOccurs="unbounded"/></xs:sequence></xs:complexType></xs:element></xs:schema>"#;
    assert_eq!(xml_schema_validate_doc_interruptible(xsd, &doc, &set), Err(ValidError::Interrupted));
    let sch = br#"<schema xmlns="http://purl.oclc.org/dsdl/schematron"><pattern><rule context="//a"><assert test="true()">x</assert></rule></pattern></schema>"#;
    assert_eq!(xml_schematron_validate_doc_interruptible(sch, &doc, &set), Err(ValidError::Interrupted));
}

#[test]
fn an_interrupt_fires_part_way_through_a_large_document() {
    let doc = big_dtd_doc();
    let (f, polls) = fires_after(5);
    assert_eq!(xml_validate_document_interruptible(&doc, &f), Err(ValidError::Interrupted));
    assert_eq!(polls.get(), 5, "polled once up front and every few hundred elements");
    let (f, _) = fires_after(5);
    assert_eq!(xml_c14n_doc_dump_memory_interruptible(&doc, true, true, &f), Err(ValidError::Interrupted));
    let (f, _) = fires_after(5);
    assert_eq!(xml_save_doc_interruptible(&doc, XML_SAVE_FORMAT, &f), Err(Interrupted));
    let rng = br#"<element name="r" xmlns="http://relaxng.org/ns/structure/1.0"><zeroOrMore><element name="a"><text/></element></zeroOrMore></element>"#;
    let (f, _) = fires_after(5);
    assert_eq!(xml_relaxng_validate_doc_interruptible(rng, &doc, &f), Err(ValidError::Interrupted));
}

#[test]
fn an_interrupt_that_never_fires_changes_nothing() {
    let never = NeverInterrupt;
    let docs = [
        big_dtd_doc(),
        parse("<!DOCTYPE r [<!ELEMENT r (a)><!ELEMENT a EMPTY>]><r><b/></r>"),
        parse("<!DOCTYPE r [<!ELEMENT r (a|(a,a))*><!ELEMENT a EMPTY>]><r/>"),
        parse(r#"<r xmlns="u" xmlns:p="v"><p:a b="1" a="2">t<!--c--></p:a><?pi x?></r>"#),
    ];
    let rng = br#"<element name="r" xmlns="http://relaxng.org/ns/structure/1.0"><zeroOrMore><element name="a"><text/></element></zeroOrMore></element>"#;
    let xsd = br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="r"><xs:complexType><xs:sequence><xs:element name="a" maxOccurs="unbounded"/></xs:sequence></xs:complexType></xs:element></xs:schema>"#;
    let sch = br#"<schema xmlns="http://purl.oclc.org/dsdl/schematron"><pattern><rule context="//a"><assert test="false()">no a</assert></rule></pattern></schema>"#;
    for doc in &docs {
        let msg = |r: Result<(), ValidError>| r.map_err(ValidError::into_message);
        assert_eq!(msg(xml_validate_document_interruptible(doc, &never)), xml_validate_document(doc));
        for (ex, cm) in [(false, false), (true, true)] {
            assert_eq!(
                xml_c14n_doc_dump_memory_interruptible(doc, ex, cm, &never).map_err(ValidError::into_message),
                xml_c14n_doc_dump_memory(doc, ex, cm)
            );
        }
        for opts in [0, XML_SAVE_FORMAT, XML_SAVE_NO_DECL] {
            assert_eq!(xml_save_doc_interruptible(doc, opts, &never), Ok(xml_save_doc(doc, opts)));
        }
        assert_eq!(msg(xml_relaxng_validate_doc_interruptible(rng, doc, &never)), xml_relaxng_validate_doc(rng, doc));
        assert_eq!(msg(xml_schema_validate_doc_interruptible(xsd, doc, &never)), xml_schema_validate_doc(xsd, doc));
        assert_eq!(msg(xml_schematron_validate_doc_interruptible(sch, doc, &never)), xml_schematron_validate_doc(sch, doc));
    }
    let html = html_read_memory(b"<p>a<br>b</p>", None, None, 0).expect("html");
    assert_eq!(xml_save_doc_interruptible(&html, 0, &never), Ok(xml_save_doc(&html, 0)));
}

#[test]
fn an_interrupt_inside_a_relaxng_choice_is_not_taken_for_a_mismatch() {
    // A choice whose first branch is interrupted must stop, not go on to try
    // the second branch and report that one's result.
    let rng = br#"<choice xmlns="http://relaxng.org/ns/structure/1.0"><element name="r"><zeroOrMore><element name="a"><text/></element></zeroOrMore></element><element name="r"><zeroOrMore><element name="a"><text/></element></zeroOrMore></element></choice>"#;
    let doc = big_dtd_doc();
    let (f, _) = fires_after(3);
    assert_eq!(xml_relaxng_validate_doc_interruptible(rng, &doc, &f), Err(ValidError::Interrupted));
}

// ---------------------------------------------------------------- #41

const DEEP: usize = 20_000;

#[test]
fn deep_documents_never_overflow_the_stack_in_validators_c14n_save_or_content() {
    on_small_stack(|| {
        let doc = parse_huge(&deep(DEEP));
        assert_eq!(doc.xml_node_get_content(NodeId::DOCUMENT).len(), DEEP);
        assert!(xml_c14n_1_0(&doc).is_ok());
        assert!(xml_exc_c14n_1_0(&doc).is_ok());
        assert!(!xml_save_doc(&doc, XML_SAVE_FORMAT).is_empty());
        let xsd = br#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="a"><xs:complexType><xs:sequence><xs:element ref="a" minOccurs="0"/></xs:sequence></xs:complexType></xs:element></xs:schema>"#;
        xml_schema_validate_doc(xsd, &doc).expect("XSD walks iteratively");
    });
}

/// RelaxNG backtracks, so it recurses, up to a bound sized for a 1 MB stack
/// in a release build. Debug frames are about two and a half times larger,
/// so this runs on Rust's default 2 MB thread stack.
#[test]
fn deep_documents_hit_the_relaxng_bound_instead_of_overflowing() {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(|| {
            let doc = parse_huge(&deep(DEEP));
            let rng = br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><ref name="a"/></start><define name="a"><element name="a"><text/><optional><ref name="a"/></optional></element></define></grammar>"#;
            let e = xml_relaxng_validate_doc(rng, &doc).unwrap_err();
            assert!(e.contains("nested deeper than"), "the depth bound, not a masked mismatch: {e}");
            let rng = br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><ref name="a"/></start><define name="a"><element name="a"><choice><text/><ref name="a"/></choice></element></define></grammar>"#;
            assert!(xml_relaxng_validate_doc(rng, &doc).is_err());
        })
        .expect("spawn")
        .join()
        .expect("no panic and no stack overflow");
}

#[test]
fn deep_dtd_validation_never_overflows_the_stack() {
    on_small_stack(|| {
        let src = format!("<!DOCTYPE a [<!ELEMENT a (#PCDATA|a)*>]>{}", deep(DEEP));
        xml_validate_document(&parse_huge(&src)).expect("valid");
    });
}

#[test]
fn relaxng_validates_documents_as_deep_as_libxml2_parses_by_default() {
    std::thread::Builder::new().stack_size(2 << 20).spawn(|| {
        let doc = parse(&deep(256));
        let rng = br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><ref name="a"/></start><define name="a"><element name="a"><text/><optional><ref name="a"/></optional></element></define></grammar>"#;
        xml_relaxng_validate_doc(rng, &doc).expect("256 levels");
    }).expect("spawn").join().expect("no stack overflow");
}

#[test]
fn relaxng_reference_chains_are_walked_once_and_bounded() {
    // A chain of n <ref>s was walked afresh from every definition, n² steps,
    // and each walk recursed once per link.
    let chain = |n: usize| {
        let mut s = String::from(
            r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start><ref name="d0"/></start>"#,
        );
        for i in 0..n {
            s.push_str(&format!(r#"<define name="d{i}"><ref name="d{}"/></define>"#, i + 1));
        }
        s.push_str(&format!(r#"<define name="d{n}"><element name="r"><empty/></element></define></grammar>"#));
        s
    };
    let doc = parse("<r/>");
    on_small_stack(move || {
        let t = Instant::now();
        xml_relaxng_validate_doc(chain(500).as_bytes(), &doc).expect("a 500-link chain");
        assert!(t.elapsed() < Duration::from_secs(5));
        let e = xml_relaxng_validate_doc(chain(20_000).as_bytes(), &doc).unwrap_err();
        assert!(e.contains("nested deeper than"), "{e}");
    });
}

#[test]
fn deeply_nested_schemas_are_refused_not_overflowed() {
    on_small_stack(|| {
        let doc = parse("<r/>");
        let rng = format!(
            r#"<element name="r" xmlns="http://relaxng.org/ns/structure/1.0">{}<empty/>{}</element>"#,
            "<group>".repeat(50_000),
            "</group>".repeat(50_000)
        );
        let sdoc = rng.into_bytes();
        let e = xml_relaxng_validate_doc(&sdoc, &doc);
        assert!(e.is_err());
        let sch = format!(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">{}<pattern><rule context="/r"><assert test="false()">deep</assert></rule></pattern>{}</schema>"#,
            "<x>".repeat(50_000),
            "</x>".repeat(50_000)
        );
        let parsed = xml_read_memory(sch.as_bytes(), None, None, default_parse_options() | XML_PARSE_HUGE);
        assert!(parsed.is_ok());
        // The schema parse itself uses default options and refuses the depth,
        // which is an error, not a crash.
        let _ = xml_schematron_validate_doc(sch.as_bytes(), &doc);
    });
}

#[test]
fn node_content_is_in_document_order() {
    let doc = parse("<r>a<x>b<y>c</y>d<![CDATA[e]]></x>f<z/>g<!--no-->h</r>");
    assert_eq!(doc.xml_node_get_content(NodeId::DOCUMENT), "abcdefgh");
    let root = doc.xml_doc_get_root_element().unwrap();
    let x = doc.first_child(root).and_then(|t| doc.next_sibling(t)).unwrap();
    assert_eq!(doc.xml_node_get_content(x), "bcde");
}
