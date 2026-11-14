mod support;

use rusty_xml::{
    xml_parse_dtd, xml_read_memory, xml_validate_document_with_interrupt,
    xml_validate_dtd_with_interrupt, InterruptCheck, InterruptError, XmlDoc,
};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
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
    let valid = parse(&with_dtd("<doc><head>h</head><item id=\"1\">a</item></doc>"));
    let invalid_order = parse(&with_dtd("<doc><item id=\"1\">a</item><head>h</head></doc>"));
    let dtd = xml_parse_dtd(DTD.as_bytes(), None, None).expect("dtd parses");
    for doc in [&valid, &invalid_order] {
        sweep(|c| xml_validate_document_with_interrupt(doc, c), is_interrupted);
        sweep(|c| xml_validate_dtd_with_interrupt(doc, &dtd, c), is_interrupted);
    }
}

#[test]
fn dtd_many_attrs_sweep() {
    let attrs = |declared: bool| {
        let names: Vec<String> = (0..40).map(|i| format!("a{i}")).collect();
        let list: String = names.iter().map(|n| format!("{n}=\"x\" ")).collect();
        let attlist: String = if declared {
            names.iter().map(|n| format!("<!ATTLIST r {n} CDATA #IMPLIED>")).collect()
        } else {
            String::new()
        };
        parse(&format!("<!DOCTYPE r [<!ELEMENT r EMPTY>{attlist}]><r {list}/>"))
    };
    let declared = attrs(true);
    let undeclared = attrs(false);
    sweep(|c| xml_validate_document_with_interrupt(&declared, c), is_interrupted);
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
    sweep(|c| xml_validate_document_with_interrupt(&doc, c), is_interrupted);
}

#[test]
#[ignore = "release-only latency measurement"]
fn latency_dtd_star() {
    let doc = star_doc(32);
    let flag = Arc::new(AtomicBool::new(false));
    let flagged_at = Arc::new(Mutex::new(None));
    let setter = {
        let (flag, flagged_at) = (Arc::clone(&flag), Arc::clone(&flagged_at));
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            *flagged_at.lock().unwrap() = Some(Instant::now());
            flag.store(true, Relaxed);
        })
    };
    let start = Instant::now();
    let result = xml_validate_document_with_interrupt(&doc, &mut InterruptCheck::new(&*flag));
    let returned_at = Instant::now();
    setter.join().unwrap();
    assert!(matches!(result, Err(InterruptError::Interrupted)), "not stopped");
    let latency = returned_at.duration_since(flagged_at.lock().unwrap().unwrap());
    println!("dtd_star: ran {:?}, flag to return {latency:?}", returned_at - start);
}
