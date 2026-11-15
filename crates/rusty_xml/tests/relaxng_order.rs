//! RelaxNG verdicts that follow libxml2 (#57) and sibling runs in linear
//! time (#58).
//!
//! Expected verdicts are `xmllint --relaxng` 2.15.3's.

use rusty_xml::*;
use std::cell::Cell;
use std::time::{Duration, Instant};

const NS: &str = r#"xmlns="http://relaxng.org/ns/structure/1.0""#;

fn parse(src: &str) -> XmlDoc {
    xml_read_memory(src.as_bytes(), None, None, 0).expect("parses")
}

fn check(rng: &str, doc: &str) -> Result<(), String> {
    xml_relaxng_validate_doc(rng.as_bytes(), &parse(doc))
}

/// A sequence takes its elements in order; the old matcher took each from
/// anywhere among the children.
#[test]
fn a_sequence_enforces_element_order() {
    let rng = format!(
        r#"<element name="r" {NS}><element name="a"><empty/></element><element name="b"><empty/></element></element>"#
    );
    assert_eq!(check(&rng, "<r><a/><b/></r>"), Ok(()));
    assert_eq!(
        check(&rng, "<r><b/><a/></r>"),
        Err("missing element a".to_string())
    );
    // Inside an interleave the order is free, as before.
    let rng = format!(
        r#"<element name="r" {NS}><interleave><element name="a"><empty/></element><element name="b"><empty/></element></interleave></element>"#
    );
    assert_eq!(check(&rng, "<r><a/><b/></r>"), Ok(()));
    assert_eq!(check(&rng, "<r><b/><a/></r>"), Ok(()));
}

/// A choice that took an alternative too early is reconsidered when what
/// follows fails.
#[test]
fn a_choice_tries_the_other_alternatives_when_the_rest_fails() {
    let rng = format!(
        r#"<element name="r" {NS}><choice><element name="a"><empty/></element><group><element name="a"><empty/></element><element name="b"><empty/></element></group></choice></element>"#
    );
    assert_eq!(check(&rng, "<r><a/></r>"), Ok(()));
    assert_eq!(check(&rng, "<r><a/><b/></r>"), Ok(()));
    assert_eq!(
        check(&rng, "<r><a/><b/><b/></r>"),
        Err("extra element content".to_string())
    );
    // An optional that took too much, and a repetition followed by the same
    // element.
    let rng = format!(
        r#"<element name="r" {NS}><zeroOrMore><element name="a"><empty/></element></zeroOrMore><element name="a"><empty/></element></element>"#
    );
    assert_eq!(check(&rng, "<r><a/><a/><a/></r>"), Ok(()));
    assert!(check(&rng, "<r></r>").is_err());
}

/// `choice(text, ref a)` took the empty text and left the child element.
#[test]
fn a_recursive_grammar_accepts_its_nested_documents() {
    let rng = format!(
        r#"<grammar {NS}><start><ref name="a"/></start><define name="a"><element name="a"><choice><text/><ref name="a"/></choice></element></define></grammar>"#
    );
    assert_eq!(check(&rng, "<a><a><a/></a></a>"), Ok(()));
    assert_eq!(check(&rng, "<a>text</a>"), Ok(()));
    let deep = format!("{}{}", "<a>".repeat(150), "</a>".repeat(150));
    assert_eq!(check(&rng, &deep), Ok(()));
    assert!(check(&rng, "<a><b/></a>").is_err());
}

/// A repetition saved and restored the remaining children by cloning them,
/// and took elements out of the middle of a `Vec`: 100,000 siblings took
/// 1.1 s and 100,000 choice items 20 s in a release build.
#[test]
fn sibling_runs_are_linear() {
    let elem = format!(
        r#"<element name="r" {NS}><zeroOrMore><element name="c"><empty/></element></zeroOrMore></element>"#
    );
    let choice = format!(
        r#"<element name="r" {NS}><zeroOrMore><choice><element name="a"><empty/></element><element name="b"><empty/></element><element name="c"><empty/></element></choice></zeroOrMore></element>"#
    );
    let n = 100_000;
    let siblings = format!("<r>{}</r>", "<c/>".repeat(n));
    let items: String = (0..n).map(|i| ["<a/>", "<b/>", "<c/>"][i % 3]).collect();
    let t = Instant::now();
    assert_eq!(check(&elem, &siblings), Ok(()));
    assert_eq!(check(&choice, &format!("<r>{items}</r>")), Ok(()));
    // A run that fails at its end takes the second pass, also linear.
    assert_eq!(
        check(&elem, &format!("<r>{}<z/></r>", "<c/>".repeat(n))),
        Err("extra element content".to_string())
    );
    assert_eq!(
        check(&choice, &format!("<r>{items}<z/></r>")),
        Err("extra element content".to_string())
    );
    assert!(t.elapsed() < Duration::from_secs(20), "took {:?}", t.elapsed());
}

/// An interrupt that fires while the second pass runs ends the validation;
/// it is not read as a mismatch.
#[test]
fn an_interrupt_during_the_second_pass_is_returned() {
    let rng = format!(
        r#"<element name="r" {NS}><zeroOrMore><choice><element name="a"><empty/></element><element name="b"><empty/></element></choice></zeroOrMore></element>"#
    );
    let items: String = (0..60_000).map(|i| ["<a/>", "<b/>"][i % 2]).collect();
    let doc = parse(&format!("<r>{items}<z/></r>"));
    // Count the polls of a whole run, then fire three quarters of the way
    // in: the first pass is about half of the work, the second pass the rest.
    let polls = Cell::new(0u32);
    let counting = || {
        polls.set(polls.get() + 1);
        false
    };
    assert_eq!(
        xml_relaxng_validate_doc_interruptible(rng.as_bytes(), &doc, &counting),
        Err(ValidError::Failed("extra element content".into()))
    );
    let total = polls.get();
    assert!(total > 8, "too few polls to place an interrupt: {total}");
    let fired = Cell::new(0u32);
    let late = || {
        fired.set(fired.get() + 1);
        fired.get() >= total * 3 / 4
    };
    assert_eq!(
        xml_relaxng_validate_doc_interruptible(rng.as_bytes(), &doc, &late),
        Err(ValidError::Interrupted)
    );
}

/// The #56 cases still hold with the second pass: alternatives that reach
/// the same definition are matched once per pair, failing or matching.
#[test]
fn twin_choices_stay_linear_with_the_second_pass() {
    fn schema(n: usize) -> String {
        let mut s = format!(r#"<grammar {NS}><start><ref name="L0"/></start>"#);
        for i in 0..n {
            s.push_str(&format!(
                r#"<define name="L{i}"><element name="c"><choice><ref name="L{j}"/><ref name="L{j}"/></choice></element></define>"#,
                j = i + 1
            ));
        }
        s.push_str(&format!(
            r#"<define name="L{n}"><element name="c"><empty/></element></define></grammar>"#
        ));
        s
    }
    let t = Instant::now();
    for n in [10, 22, 40, 200] {
        let rng = schema(n);
        let failing = format!("{}<x/>{}", "<c>".repeat(n + 1), "</c>".repeat(n + 1));
        assert_eq!(check(&rng, &failing), Err("choice failed".to_string()), "depth {n}");
        let matching = format!("{}{}", "<c>".repeat(n + 1), "</c>".repeat(n + 1));
        assert_eq!(check(&rng, &matching), Ok(()), "depth {n}");
    }
    assert!(t.elapsed() < Duration::from_secs(10), "took {:?}", t.elapsed());
}
