//! Loading an external DTD with `xml_parse_dtd` (#58): conditional
//! sections, parameter entities charged once, and the first declaration of a
//! parameter entity binding.
//!
//! Every verdict here is `xmllint --noout --dtdvalid x.dtd doc.xml` 2.15.3's,
//! on the same DTD and document.

use rusty_xml::{
    xml_parse_dtd, xml_read_memory, xml_validate_dtd, XML_ERR_CONDSEC_INVALID,
    XML_ERR_CONDSEC_INVALID_KEYWORD, XML_ERR_CONDSEC_NOT_FINISHED, XML_ERR_EXT_SUBSET_NOT_FINISHED,
    XML_ERR_RESOURCE_LIMIT,
};

#[derive(Debug, PartialEq)]
enum Verdict {
    Valid,
    Invalid,
    /// `xml_parse_dtd` failed with this code.
    DtdError(i32),
}
use Verdict::*;

fn verdict(dtd: &str, doc: &str) -> Verdict {
    let dtd = match xml_parse_dtd(dtd.as_bytes(), None, None) {
        Ok(d) => d,
        Err(e) => return DtdError(e.code),
    };
    let doc = xml_read_memory(doc.as_bytes(), None, None, 0).expect("document parses");
    match xml_validate_dtd(&doc, &dtd) {
        Ok(()) => Valid,
        Err(_) => Invalid,
    }
}

const TITLE: &str = "<title>x</title>";

/// INCLUDE sections were skipped like IGNORE sections, so their declarations
/// were lost: "No declaration for element title".
#[test]
fn include_sections_keep_their_declarations() {
    let cases: [(&str, Verdict); 9] = [
        ("<![ INCLUDE [ <!ELEMENT title (#PCDATA)> ]]>", Valid),
        ("<![IGNORE[ <!ELEMENT title (#PCDATA)> ]]> <!ELEMENT title EMPTY>", Invalid),
        (
            "<![IGNORE[ <![INCLUDE[ <!ELEMENT title EMPTY> ]]> ]]> <!ELEMENT title (#PCDATA)>",
            Valid,
        ),
        (
            "<![INCLUDE[ <![IGNORE[ <!ELEMENT title EMPTY> ]]> <!ELEMENT title (#PCDATA)> ]]>",
            Valid,
        ),
        // The keyword through a parameter entity, and through one that names
        // another.
        (r#"<!ENTITY % draft "INCLUDE"> <![%draft;[ <!ELEMENT title (#PCDATA)> ]]>"#, Valid),
        (
            r#"<!ENTITY % a "INCLUDE"><!ENTITY % b "%a;"> <![ %b; [ <!ELEMENT title (#PCDATA)> ]]>"#,
            Valid,
        ),
        // The first declaration of the keyword entity binds.
        (
            r#"<!ENTITY % m "INCLUDE"><!ENTITY % m "IGNORE"> <![%m;[ <!ELEMENT title (#PCDATA)> ]]>"#,
            Valid,
        ),
        // Declarations inside an IGNORE section never bind.
        (
            r#"<!ENTITY % s "INCLUDE"> <![%s;[ <!ENTITY % m "(#PCDATA)"> ]]> <![IGNORE[ <!ENTITY % m "EMPTY"> ]]> <!ELEMENT title %m;>"#,
            Valid,
        ),
        // Markers inside a comment or a literal are text.
        (r#"<!-- <![IGNORE[ --> <!ELEMENT title (#PCDATA)> <!ENTITY e "]]>">"#, Valid),
    ];
    for (dtd, want) in cases {
        assert_eq!(verdict(dtd, TITLE), want, "{dtd}");
    }
}

/// The DocBook feature-switch pattern: an IGNORE section holds SGML-only
/// syntax with a malformed parameter entity reference, which was substituted
/// anyway and failed the whole parse with error 23.
#[test]
fn ignored_text_is_not_substituted() {
    let dtd = r#"<!ENTITY % x.module "IGNORE"> <![%x.module;[ <!ELEMENT title (#PCDATA|b)* -(%ubiq.mix)> ]]> <!ELEMENT title (#PCDATA)>"#;
    assert_eq!(verdict(dtd, TITLE), Valid);
}

#[test]
fn malformed_sections_are_libxml2_errors() {
    let cases: [(&str, i32); 5] = [
        ("<![IGNORE[ <!ELEMENT title (#PCDATA)>", XML_ERR_CONDSEC_NOT_FINISHED),
        ("<![INCLUDE[ <!ELEMENT title (#PCDATA)>", XML_ERR_EXT_SUBSET_NOT_FINISHED),
        ("<![FOO[ <!ELEMENT title (#PCDATA)> ]]>", XML_ERR_CONDSEC_INVALID_KEYWORD),
        ("<![INCLUDE <!ELEMENT title (#PCDATA)> ]]>", XML_ERR_CONDSEC_INVALID),
        ("<!ELEMENT title (#PCDATA)> ]]>", XML_ERR_EXT_SUBSET_NOT_FINISHED),
    ];
    for (dtd, code) in cases {
        assert_eq!(verdict(dtd, TITLE), DtdError(code), "{dtd}");
    }
}

/// XML 1.0 section 4.2: the first declaration of an entity binds. The last
/// one did, which turned DocBook 4.5's `entry` into a content model the
/// parser rejects.
#[test]
fn the_first_parameter_entity_declaration_binds() {
    for dtd in [
        r#"<!ENTITY % m "(#PCDATA)"><!ENTITY % m "EMPTY"><!ELEMENT title %m;>"#,
        r#"<!ENTITY % m "(#PCDATA)"><![INCLUDE[ <!ENTITY % m "EMPTY"> ]]><!ELEMENT title %m;>"#,
    ] {
        assert_eq!(verdict(dtd, TITLE), Valid, "{dtd}");
    }
}

/// A value full of references was copied raw into every use, and each copy's
/// references were substituted and charged again on the next pass: a few
/// hundred uses of a 200-reference entity hit the amplification limit, as
/// TEI P5 `tei_all.dtd` did. libxml2 expands a value once, at its
/// declaration, and charges a use by its expanded length.
#[test]
fn parameter_entity_values_are_expanded_and_charged_once() {
    let dtd = |refs: usize, uses: usize| {
        let mut d = format!(r#"<!ENTITY % a " "><!ENTITY % x "{}">"#, "%a;".repeat(refs));
        for i in 0..uses {
            d.push_str(&format!("<!ELEMENT e{i} %x; (#PCDATA)>"));
        }
        d.push_str("<!ELEMENT r (#PCDATA)>");
        d
    };
    assert_eq!(verdict(&dtd(200, 400), "<r>x</r>"), Valid);
    // A real blow-up is still refused, as xmllint refuses it.
    assert_eq!(verdict(&dtd(2_000, 2_000), "<r>x</r>"), DtdError(XML_ERR_RESOURCE_LIMIT));
    // So is a doubling chain.
    let mut chain = format!(r#"<!ENTITY % e0 "{}">"#, "x".repeat(5_000));
    for i in 1..12 {
        chain.push_str(&format!(r#"<!ENTITY % e{i} "%e{j};%e{j};">"#, j = i - 1));
    }
    chain.push_str(r#"<!ELEMENT title (#PCDATA)><!ENTITY big "%e11;">"#);
    assert_eq!(verdict(&chain, TITLE), DtdError(XML_ERR_RESOURCE_LIMIT));
}
