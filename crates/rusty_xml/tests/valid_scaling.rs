//! Validation paths that scaled with the product of two input sizes (#58),
//! and the verdicts that must not move while they are fixed.

use rusty_xml::*;
use std::time::{Duration, Instant};

fn parse(src: &str) -> XmlDoc {
    xml_read_memory(src.as_bytes(), None, None, default_parse_options()).expect("parses")
}

/// Item 1: every element scanned every attribute declaration, so validation
/// cost elements x declarations.
#[test]
fn attribute_declarations_are_indexed_per_element_type() {
    let mut dtd = String::from(
        "<!ELEMENT r (p*)><!ELEMENT p EMPTY><!ATTLIST p id ID #IMPLIED k CDATA #IMPLIED>",
    );
    for i in 0..8_000 {
        dtd.push_str(&format!(
            "<!ELEMENT e{i} EMPTY><!ATTLIST e{i} a CDATA #IMPLIED b CDATA #IMPLIED>"
        ));
    }
    let body: String = (0..20_000).map(|i| format!("<p id=\"i{i}\" k=\"v\"/>")).collect();
    let doc = parse(&format!("<!DOCTYPE r [{dtd}]><r>{body}</r>"));
    let t = Instant::now();
    assert_eq!(xml_validate_document(&doc), Ok(()));
    assert!(t.elapsed() < Duration::from_secs(10), "took {:?}", t.elapsed());
}

/// The checks that read the index report what they did before.
#[test]
fn attribute_checks_report_as_before() {
    let check = |dtd: &str, body: &str| {
        xml_validate_document(&parse(&format!("<!DOCTYPE r [{dtd}]>{body}")))
    };
    let base = "<!ELEMENT r EMPTY>";
    assert_eq!(
        check(&format!("{base}<!ATTLIST r a CDATA #REQUIRED>"), "<r/>"),
        Err("attribute a of r is required".into())
    );
    assert_eq!(
        check(&format!("{base}<!ATTLIST r a CDATA #FIXED 'x'>"), "<r a='y'/>"),
        Err("attribute a must be x".into())
    );
    assert_eq!(
        check(&format!("{base}<!ATTLIST r a (x|y) #IMPLIED>"), "<r a='z'/>"),
        Err("attribute a value not in enumeration".into())
    );
    assert_eq!(
        check(&format!("{base}<!ATTLIST r a ID #IMPLIED b ID #IMPLIED>"), "<r a='p' b='q'/>"),
        Err("Element r has 2 ID attributes".into())
    );
    assert_eq!(
        check(&format!("{base}<!ATTLIST r a CDATA #IMPLIED>"), "<r z='1'/>"),
        Err("No declaration for attribute z of element r".into())
    );
    // An element with several problems reports the same one every run:
    // the declarations are visited by attribute name.
    for _ in 0..8 {
        assert_eq!(
            check(
                &format!("{base}<!ATTLIST r m CDATA #REQUIRED c CDATA #REQUIRED x CDATA #REQUIRED>"),
                "<r/>"
            ),
            Err("attribute c of r is required".into())
        );
    }
}

/// Item 5: mixed content allocated the child's name once per allowed name.
#[test]
fn mixed_content_checks_each_child_once() {
    let names = 400;
    let list: String = (0..names).map(|i| format!("|m{i}")).collect();
    let decl: String = (0..names).map(|i| format!("<!ELEMENT m{i} EMPTY>")).collect();
    let body: String = (0..20_000).map(|i| format!("t<m{}/>", names - 1 - (i % 3))).collect();
    let doc = parse(&format!("<!DOCTYPE r [<!ELEMENT r (#PCDATA{list})*>{decl}<!ELEMENT z EMPTY>]><r>{body}</r>"));
    let t = Instant::now();
    assert_eq!(xml_validate_document(&doc), Ok(()));
    assert!(t.elapsed() < Duration::from_secs(10), "took {:?}", t.elapsed());
    let bad = parse(&format!(
        "<!DOCTYPE r [<!ELEMENT r (#PCDATA{list})*>{decl}<!ELEMENT z EMPTY>]><r>t<m1/><z/></r>"
    ));
    assert_eq!(xml_validate_document(&bad), Err("element z not allowed in mixed r".into()));
}

fn repeated_choice(types: usize, names: usize) -> XmlDoc {
    let mut dtd = format!(
        "<!ELEMENT r ({})>",
        (0..types).map(|t| format!("t{t}")).collect::<Vec<_>>().join(",")
    );
    for i in 0..names {
        dtd.push_str(&format!("<!ELEMENT b{i} EMPTY>"));
    }
    for t in 0..types {
        let alts = (0..names).map(|i| format!("b{i}")).collect::<Vec<_>>().join("|");
        dtd.push_str(&format!("<!ELEMENT t{t} ({alts})*>"));
    }
    let body: String = (0..types).map(|t| format!("<t{t}><b{}/><b0/></t{t}>", names - 1)).collect();
    parse(&format!("<!DOCTYPE r [{dtd}]><r>{body}</r>"))
}

/// Item 4: the follow set a repeated group adds to each of its positions is
/// one shared set, not a copy per position. A repeated choice of 2,800 or
/// 4,000 names was refused as too large; libxml2 validates it.
#[test]
fn repeated_choices_share_their_follow_set() {
    assert_eq!(xml_validate_document(&repeated_choice(1, 4_000)), Ok(()));
    let t = Instant::now();
    assert_eq!(xml_validate_document(&repeated_choice(16, 2_000)), Ok(()));
    assert!(t.elapsed() < Duration::from_secs(20), "took {:?}", t.elapsed());
    // Still matches and mismatches.
    let mut doc = repeated_choice(1, 50);
    let r = doc.xml_doc_get_root_element().unwrap();
    let t0 = doc.first_child(r).unwrap();
    let stray = doc.xml_new_node(None, "r");
    doc.xml_add_child(t0, stray);
    assert!(xml_validate_document(&doc).is_err());
}

/// The determinism pre-check is unchanged where a position holds several
/// shared sets, and a model that is too large is still refused.
#[test]
fn determinism_and_size_limits_hold_with_shared_sets() {
    let nd = |model: &str, body: &str| {
        xml_validate_document(&parse(&format!(
            "<!DOCTYPE r [<!ELEMENT r {model}><!ELEMENT a EMPTY><!ELEMENT b EMPTY><!ELEMENT c EMPTY>]>{body}"
        )))
    };
    // `a`'s position is followed by the repetition's set and the sequence's.
    let e = nd("(a*,a)", "<r><a/></r>").unwrap_err();
    assert!(e.contains("is not deterministic"), "{e}");
    let e = nd("((a,b)|(a,c))", "<r><a/><b/></r>").unwrap_err();
    assert!(e.contains("is not deterministic"), "{e}");
    assert_eq!(nd("((a|b)*,c)", "<r><a/><b/><a/><c/></r>"), Ok(()));
    assert_eq!(nd("(a,(b|c)*)", "<r><a/><c/><b/></r>"), Ok(()));
    assert!(nd("(a,(b|c)*)", "<r><b/></r>").is_err());
    // A sequence of 3,000 optional items has 4.5M distinct follow entries.
    let n = 3_000;
    let mut dtd = format!(
        "<!ELEMENT r ({})>",
        (0..n).map(|i| format!("b{i}?")).collect::<Vec<_>>().join(",")
    );
    for i in 0..n {
        dtd.push_str(&format!("<!ELEMENT b{i} EMPTY>"));
    }
    let e = xml_validate_document(&parse(&format!("<!DOCTYPE r [{dtd}]><r><b0/></r>"))).unwrap_err();
    assert!(e.contains("too large to validate against"), "{e}");
}

const SLOW_SCH: &str = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron"><pattern><rule context="/"><assert test="count(//e[count(//e) &gt;= 0]) &gt;= 0">x</assert></rule></pattern></schema>"#;

/// Item 3: one Schematron test is one XPath evaluation, and it ran to
/// completion past the interrupt, then returned success.
#[test]
fn schematron_interrupt_stops_inside_one_evaluation() {
    let doc = parse(&format!("<r>{}</r>", "<e/>".repeat(6_000)));
    let t = Instant::now();
    let deadline = t + Duration::from_millis(100);
    let by_clock = move || Instant::now() >= deadline;
    assert_eq!(
        xml_schematron_validate_doc_interruptible(SLOW_SCH.as_bytes(), &doc, &by_clock),
        Err(ValidError::Interrupted)
    );
    assert!(t.elapsed() < Duration::from_secs(3), "took {:?}", t.elapsed());
}

/// An interrupt that never fires changes nothing, including for a test that
/// fails to evaluate (read as false, as before).
#[test]
fn schematron_without_a_firing_interrupt_is_unchanged() {
    let never = std::sync::atomic::AtomicBool::new(false);
    let doc = parse("<r><e/><e/><f/></r>");
    for sch in [
        r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron"><pattern><rule context="//e"><assert test="name() = 'e'">ok</assert><report test="true()">seen e</report></rule></pattern></schema>"#,
        r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron"><pattern><rule context="/r"><assert test="count(f) = 2">two f</assert><assert test="((">broken</assert></rule></pattern></schema>"#,
    ] {
        assert_eq!(
            xml_schematron_validate_doc_interruptible(sch.as_bytes(), &doc, &never)
                .map_err(ValidError::into_message),
            xml_schematron_validate_doc(sch.as_bytes(), &doc),
            "{sch}"
        );
    }
    assert_eq!(
        xml_schematron_validate_doc(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron"><pattern><rule context="/r"><assert test="count(f) = 2">two f</assert><assert test="((">broken</assert></rule></pattern></schema>"#.as_bytes(),
            &doc
        ),
        Err("two f; broken".into())
    );
}
