mod support;

use rusty_xml::{
    xml_parse_dtd, xml_read_memory, xml_read_memory_with_interrupt,
    xml_relaxng_validate_doc_with_interrupt, xml_schema_validate_doc_with_interrupt,
    xml_schematron_validate_doc_with_interrupt, xml_validate_document_with_interrupt,
    xml_validate_dtd_with_interrupt, InterruptCheck, InterruptError, XmlDoc,
};
use support::{count_checks, is_interrupted, sweep};

fn parse(xml: &str) -> XmlDoc {
    xml_read_memory(xml.as_bytes(), None, None, 0).expect("fixture parses")
}

const DTD: &str = "<!ELEMENT doc (head, item+)><!ELEMENT head (#PCDATA)><!ELEMENT item (#PCDATA)>\
                   <!ATTLIST item id CDATA #REQUIRED>";

fn with_dtd(body: &str) -> String {
    format!("<!DOCTYPE doc [{DTD}]>{body}")
}

#[test]
fn dtd_valid_sweep() {
    let valid = parse(&with_dtd(
        "<doc><head>h</head><item id=\"1\">a</item></doc>",
    ));
    let invalid_order = parse(&with_dtd(
        "<doc><item id=\"1\">a</item><head>h</head></doc>",
    ));
    let dtd = xml_parse_dtd(DTD.as_bytes(), None, None).expect("dtd parses");
    for doc in [&valid, &invalid_order] {
        sweep(
            |c| xml_validate_document_with_interrupt(doc, c),
            is_interrupted,
        );
        sweep(
            |c| xml_validate_dtd_with_interrupt(doc, &dtd, c),
            is_interrupted,
        );
    }
}

#[test]
fn dtd_many_attrs_sweep() {
    let attrs = |declared: bool| {
        let names: Vec<String> = (0..40).map(|i| format!("a{i}")).collect();
        let list: String = names.iter().map(|n| format!("{n}=\"x\" ")).collect();
        let attlist: String = if declared {
            names
                .iter()
                .map(|n| format!("<!ATTLIST r {n} CDATA #IMPLIED>"))
                .collect()
        } else {
            String::new()
        };
        parse(&format!(
            "<!DOCTYPE r [<!ELEMENT r EMPTY>{attlist}]><r {list}/>"
        ))
    };
    let declared = attrs(true);
    let undeclared = attrs(false);
    sweep(
        |c| xml_validate_document_with_interrupt(&declared, c),
        is_interrupted,
    );
    let delta = count_checks(|c| {
        let _ = xml_validate_document_with_interrupt(&declared, c);
    }) - count_checks(|c| {
        let _ = xml_validate_document_with_interrupt(&undeclared, c);
    });
    assert!(delta >= 40, "delta {delta}");
}

fn star_doc(children: usize) -> XmlDoc {
    parse(&format!(
        "<!DOCTYPE r [<!ELEMENT r (a|(a,a))*><!ELEMENT a EMPTY>]><r>{}</r>",
        "<a/>".repeat(children)
    ))
}

#[test]
fn dtd_star_sweep() {
    let doc = star_doc(12);
    sweep(
        |c| xml_validate_document_with_interrupt(&doc, c),
        is_interrupted,
    );
}

#[test]
#[ignore = "release-only latency measurement"]
fn latency_dtd_star() {
    let doc = star_doc(32);
    let (result, latency) =
        support::stop_after_100ms(|c| xml_validate_document_with_interrupt(&doc, c));
    assert!(
        matches!(result, Err(InterruptError::Interrupted)),
        "not stopped"
    );
    println!("dtd_star: flag to return {latency:?}");
    assert!(
        latency <= support::LATENCY_LIMIT,
        "dtd_star: {latency:?} over the limit"
    );
}

const RNG_NS: &str = "http://relaxng.org/ns/structure/1.0";

fn rng_validate(
    schema: &str,
    doc: &XmlDoc,
    c: &mut InterruptCheck<'_>,
) -> Result<(), InterruptError<String>> {
    xml_relaxng_validate_doc_with_interrupt(schema.as_bytes(), doc, c)
}

#[test]
fn rng_choice_optional_sweep() {
    // Second choice branch, an optional attribute and a zeroOrMore: rng.rs consume swallows these
    // branches with is_ok().
    let inner = format!(
        "<element name=\"doc\" xmlns=\"{RNG_NS}\"><choice><element name=\"x\"><text/></element>\
         <element name=\"y\"><text/></element></choice><optional><attribute name=\"o\"/></optional>\
         <zeroOrMore><element name=\"z\"><text/></element></zeroOrMore></element>"
    );
    let doc = parse("<doc o=\"1\"><y>t</y><z>a</z><z>b</z></doc>");
    sweep(|c| rng_validate(&inner, &doc, c), is_interrupted);

    // A choice of elements at the top: match_element swallows the failed branch.
    let top = format!(
        "<choice xmlns=\"{RNG_NS}\"><element name=\"a\"><empty/></element>\
         <element name=\"b\"><empty/></element></choice>"
    );
    let b = parse("<b/>");
    sweep(|c| rng_validate(&top, &b, c), is_interrupted);
}

fn ref_chain(depth: usize) -> String {
    let defs: String = (0..depth)
        .map(|k| format!("<define name=\"d{k}\"><group><ref name=\"d{}\"/><ref name=\"d{}\"/></group></define>", k + 1, k + 1))
        .collect();
    format!(
        "<grammar xmlns=\"{RNG_NS}\"><start><element name=\"doc\"><ref name=\"d0\"/></element></start>\
         {defs}<define name=\"d{depth}\"><empty/></define></grammar>"
    )
}

#[test]
fn rng_ref_chain_sweep() {
    let doc = parse("<doc/>");
    let schema = ref_chain(6);
    sweep(|c| rng_validate(&schema, &doc, c), is_interrupted);
}

#[test]
#[ignore = "release-only latency measurement"]
fn latency_rng_ref_chain() {
    let doc = parse("<doc/>");
    let schema = ref_chain(22);
    let (result, latency) = support::stop_after_100ms(|c| rng_validate(&schema, &doc, c));
    assert!(
        matches!(result, Err(InterruptError::Interrupted)),
        "not stopped"
    );
    println!("rng_ref_chain: flag to return {latency:?}");
    assert!(
        latency <= support::LATENCY_LIMIT,
        "rng_ref_chain: {latency:?} over the limit"
    );
}

#[test]
fn xsd_many_attrs_sweep() {
    let schema = |declared: bool| {
        let attrs: String = if declared {
            (0..40)
                .map(|i| format!("<xs:attribute name=\"a{i}\" use=\"required\"/>"))
                .collect()
        } else {
            String::new()
        };
        format!(
            "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"><xs:element name=\"r\">\
             <xs:complexType>{attrs}</xs:complexType></xs:element></xs:schema>"
        )
    };
    let list: String = (0..40).map(|i| format!(" a{i}=\"x\"")).collect();
    let doc = parse(&format!("<r{list}/>"));
    let validate_cost = |xsd: &str| {
        let validate = count_checks(|c| {
            let _ = xml_schema_validate_doc_with_interrupt(xsd.as_bytes(), &doc, c);
        });
        let parse_only = count_checks(|c| {
            let _ = xml_read_memory_with_interrupt(xsd.as_bytes(), None, None, 0, c);
        });
        validate - parse_only
    };
    let (a, b) = (schema(true), schema(false));
    sweep(
        |c| xml_schema_validate_doc_with_interrupt(a.as_bytes(), &doc, c),
        is_interrupted,
    );
    let delta = validate_cost(&a) - validate_cost(&b);
    assert!(delta >= 40, "delta {delta}");
}

const SCHEMATRON: &str = "<schema xmlns=\"http://purl.oclc.org/dsdl/schematron\"><pattern>\
    <rule context=\"item\"><assert test=\"@id\">needs id</assert><report test=\"@bad\">bad</report></rule>\
    <rule context=\"doc\"><assert test=\"count(item) = 10\">ten</assert><report test=\"@x\">x</report></rule>\
    </pattern></schema>";

#[test]
fn schematron_sweep() {
    let items: String = (0..10).map(|i| format!("<item id=\"{i}\"/>")).collect();
    let doc = parse(&format!("<doc>{items}</doc>"));
    sweep(
        |c| xml_schematron_validate_doc_with_interrupt(SCHEMATRON.as_bytes(), &doc, c),
        is_interrupted,
    );
}

#[test]
fn schema_parse_stop_is_interrupted() {
    let doc = parse("<doc/>");
    let flag = std::sync::atomic::AtomicBool::new(true);
    let stopped = |run: &dyn Fn(&mut InterruptCheck<'_>) -> Result<(), InterruptError<String>>| {
        let r = run(&mut InterruptCheck::new(&flag));
        assert!(matches!(r, Err(InterruptError::Interrupted)), "got {r:?}");
    };
    let rng = format!("<element name=\"doc\" xmlns=\"{RNG_NS}\"><empty/></element>");
    stopped(&|c| xml_relaxng_validate_doc_with_interrupt(rng.as_bytes(), &doc, c));
    let xsd = "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"><xs:element name=\"doc\"/></xs:schema>";
    stopped(&|c| xml_schema_validate_doc_with_interrupt(xsd.as_bytes(), &doc, c));
    stopped(&|c| xml_schematron_validate_doc_with_interrupt(SCHEMATRON.as_bytes(), &doc, c));
}
