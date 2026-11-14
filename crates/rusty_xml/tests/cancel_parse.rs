mod support;

use rusty_xml::{
    xml_ctxt_read_memory_with_interrupt, xml_new_parser_ctxt, xml_parse_dtd_with_interrupt,
    xml_read_io_with_interrupt, xml_read_memory_with_interrupt,
    xml_reader_for_memory_with_interrupt, xml_save_doc, xml_sax_parse_memory_with_interrupt,
    xml_xinclude_process_with_interrupt, InterruptCheck, SaxEvent, SaxRecorder, XML_ERR_USER_STOP,
    XML_PARSE_DTDATTR, XML_PARSE_NOENT, XML_PARSE_RECOVER,
};
use std::io::Read;
use support::{count_checks, is_user_stop, sweep};

const DOC: &[u8] = b"<r a=\"1\"><!--c--><?p d?><e>&lt;t&amp;</e>tail</r>";

fn read(
    doc: &[u8],
    options: i32,
    check: &mut InterruptCheck<'_>,
) -> Result<(), rusty_xml::XmlError> {
    xml_read_memory_with_interrupt(doc, None, None, options, check).map(|_| ())
}

#[test]
fn read_memory_sweep() {
    let total = sweep(|c| read(DOC, 0, c), is_user_stop);
    assert!(total >= 6, "total {total}");
}

#[test]
fn recover_sweep() {
    let doc = b"<r a=\"1\"><!--c--><?p d?><e>&lt;t&amp;</x>tail</r>";
    sweep(|c| read(doc, XML_PARSE_RECOVER, c), is_user_stop);
}

#[test]
fn entity_expansion_sweep() {
    let with_entities =
        br#"<!DOCTYPE r [<!ENTITY a "<x/><x/>"><!ENTITY b "&a;&a;">]><r>&b;&b;</r>"#;
    let written_out = b"<r><x/><x/><x/><x/><x/><x/><x/><x/></r>";
    sweep(|c| read(with_entities, XML_PARSE_NOENT, c), is_user_stop);
    let a = count_checks(|c| {
        let _ = read(with_entities, XML_PARSE_NOENT, c);
    });
    let b = count_checks(|c| {
        let _ = read(written_out, XML_PARSE_NOENT, c);
    });
    assert!(a >= b + 6, "entities {a}, written out {b}");
}

#[test]
fn prolog_epilog_comments_checked() {
    let with_misc = count_checks(|c| {
        let _ = read(b"<!--a--><?p?><r/><!--b-->", 0, c);
    });
    let bare = count_checks(|c| {
        let _ = read(b"<r/>", 0, c);
    });
    assert_eq!(with_misc - bare, 3);
}

#[test]
fn cdata_sections_checked() {
    let with_cdata = count_checks(|c| {
        let _ = read(b"<r><![CDATA[a]]><![CDATA[b]]><![CDATA[c]]></r>", 0, c);
    });
    let empty = count_checks(|c| {
        let _ = read(b"<r></r>", 0, c);
    });
    assert_eq!(with_cdata - empty, 3);
}

#[test]
fn sax_no_event_after_stop() {
    sweep(
        |c| {
            let mut rec = SaxRecorder::new();
            let r = xml_sax_parse_memory_with_interrupt(DOC, 0, &mut rec, c);
            if r.is_err() {
                assert!(
                    !matches!(rec.events.last(), Some(SaxEvent::EndDocument)),
                    "end_document delivered after a stop"
                );
            }
            r
        },
        is_user_stop,
    );
}

#[test]
fn ctxt_sweep() {
    let previous = xml_read_memory_with_interrupt(
        b"<old/>",
        None,
        None,
        0,
        &mut InterruptCheck::new(&rusty_xml::NoInterrupt),
    )
    .unwrap();
    let previous_bytes = xml_save_doc(&previous, 0);
    sweep(
        |c| {
            let mut ctxt = xml_new_parser_ctxt();
            ctxt.doc = Some(previous.clone());
            let r = xml_ctxt_read_memory_with_interrupt(&mut ctxt, DOC, None, None, 0, c);
            if r.is_err() {
                assert_eq!(
                    ctxt.last_error.as_ref().map(|e| e.code),
                    Some(XML_ERR_USER_STOP)
                );
                assert_eq!(xml_save_doc(ctxt.doc.as_ref().unwrap(), 0), previous_bytes);
            }
            r
        },
        is_user_stop,
    );
}

fn read_io(
    bytes: &'static [u8],
    step: usize,
    check: &mut InterruptCheck<'_>,
) -> Result<(), rusty_xml::XmlError> {
    let mut cursor = std::io::Cursor::new(bytes);
    xml_read_io_with_interrupt(
        |buf: &mut [u8]| {
            let n = buf.len().min(step);
            cursor.read(&mut buf[..n])
        },
        None,
        None,
        0,
        check,
    )
    .map(|_| ())
}

#[test]
fn read_io_sweep() {
    sweep(|c| read_io(DOC, 1000, c), is_user_stop);
}

#[test]
fn read_io_loop_checked() {
    let one_read = count_checks(|c| {
        let _ = read_io(DOC, 1000, c);
    });
    let many_reads = count_checks(|c| {
        let _ = read_io(DOC, 10, c);
    });
    assert!(many_reads >= one_read + 4, "{many_reads} vs {one_read}");
}

fn chain(values: impl Fn(usize) -> String) -> String {
    (0..40)
        .map(|i| format!("<!ENTITY e{i} \"{}\">", values(i)))
        .chain(std::iter::once("<!ENTITY e40 \"x\">".to_string()))
        .collect::<Vec<_>>()
        .join("")
}

fn checks(doc: &[u8], options: i32) -> u32 {
    count_checks(|c| {
        let _ = read(doc, options, c);
    })
}

#[test]
fn dtd_defaults_sweep() {
    let doc = br#"<!DOCTYPE r [<!ATTLIST e a CDATA "1" b CDATA "2">]><r><e/><e a="x" b="y"/></r>"#;
    sweep(|c| read(doc, XML_PARSE_DTDATTR, c), is_user_stop);
    let delta = checks(doc, XML_PARSE_DTDATTR) - checks(doc, 0);
    assert!(delta >= 4, "delta {delta}");
}

#[test]
fn wide_ns_element_sweep() {
    let tag = |n: usize| {
        let decls: String = (0..n).map(|i| format!(" xmlns:p{i}=\"u{i}\"")).collect();
        let attrs: String = (0..n).map(|i| format!(" p{i}:a=\"v\"")).collect();
        format!("<e{decls}{attrs}/>").into_bytes()
    };
    let wide = tag(100);
    sweep(|c| read(&wide, 0, c), is_user_stop);
    let delta = checks(&wide, 0) - checks(&tag(10), 0);
    assert!(delta >= 2, "delta {delta}");
}

#[test]
fn internal_subset_entity_chain() {
    let doc = |subset: String| format!("<!DOCTYPE r [{subset}]><r/>").into_bytes();
    let linked = doc(chain(|i| format!("&e{};", i + 1)));
    let flat = doc(chain(|_| "x".to_string()));
    sweep(|c| read(&linked, 0, c), is_user_stop);
    let delta = checks(&linked, 0) - checks(&flat, 0);
    assert!(delta >= 820, "delta {delta}");
}

#[test]
fn internal_subset_recover_stop() {
    let linked = format!("<!DOCTYPE r [{}]><r/>", chain(|i| format!("&e{};", i + 1))).into_bytes();
    sweep(|c| read(&linked, XML_PARSE_RECOVER, c), is_user_stop);
}

fn pe_chain() -> String {
    let leaves: String = (1..=9)
        .map(|i| format!("<!ENTITY % a{i} \"<!ELEMENT x{i} (#PCDATA)>\">"))
        .collect();
    let mids: String = (0..3)
        .map(|m| {
            let parts: String = (1..=3).map(|k| format!("%a{};", m * 3 + k)).collect();
            format!("<!ENTITY % b{m} \"{parts}\">")
        })
        .collect();
    format!("{leaves}{mids}<!ENTITY % c \"%b0;%b1;%b2;\">%c;")
}

#[test]
fn pe_expansion_sweep() {
    let chained = pe_chain();
    let direct: String = (1..=9)
        .map(|i| format!("<!ELEMENT x{i} (#PCDATA)>"))
        .collect();
    let parse = |text: &str, c: &mut InterruptCheck<'_>| {
        xml_parse_dtd_with_interrupt(text.as_bytes(), None, None, c).map(|_| ())
    };
    sweep(|c| parse(&chained, c), is_user_stop);
    let delta = count_checks(|c| {
        let _ = parse(&chained, c);
    }) - count_checks(|c| {
        let _ = parse(&direct, c);
    });
    assert!(delta >= 3, "delta {delta}");
}

#[test]
fn entity_graph_sweep() {
    let parse = |text: &str, c: &mut InterruptCheck<'_>| {
        xml_parse_dtd_with_interrupt(text.as_bytes(), None, None, c).map(|_| ())
    };
    let linked = chain(|i| format!("&e{};", i + 1));
    let flat = chain(|_| "x".to_string());
    sweep(|c| parse(&linked, c), is_user_stop);
    let delta = count_checks(|c| {
        let _ = parse(&linked, c);
    }) - count_checks(|c| {
        let _ = parse(&flat, c);
    });
    assert!(delta >= 820, "delta {delta}");
}

const RSS: &[u8] = include_bytes!("cancel_fixtures/rss-feed.xml");

#[test]
fn reader_sweep() {
    sweep(
        |c| xml_reader_for_memory_with_interrupt(RSS, None, None, 0, c),
        is_user_stop,
    );
}

const FRAGMENT: &[u8] = br#"<f1 a="1" b="2" c="3"><f2 a="1" b="2" c="3"><f3 a="1" b="2" c="3"/></f2><f4 a="1" b="2" c="3"/><f5 a="1" b="2" c="3"/></f1>"#;

fn include_doc() -> rusty_xml::XmlDoc {
    let text = br#"<root xmlns:xi="http://www.w3.org/2001/XInclude"><xi:include href="a"/><xi:include href="b"/></root>"#;
    rusty_xml::xml_read_memory(text, None, None, 0).unwrap()
}

#[test]
fn xinclude_sweep() {
    let run = |c: &mut InterruptCheck<'_>| {
        let mut doc = include_doc();
        xml_xinclude_process_with_interrupt(&mut doc, |_| Ok(FRAGMENT.to_vec()), c)
    };
    sweep(run, is_user_stop);
    let total = count_checks(|c| {
        let _ = run(c);
    });
    let fragment_parse = count_checks(|c| {
        let _ = read(FRAGMENT, 0, c);
    });
    assert!(
        total >= 2 * fragment_parse + 10,
        "total {total}, one fragment parse {fragment_parse}"
    );
}

#[test]
fn xinclude_loader_error_distinct() {
    let mut doc = include_doc();
    let flag = support::never();
    let mut check = InterruptCheck::new(&flag);
    let e = xml_xinclude_process_with_interrupt(&mut doc, |_| Err("nope".to_string()), &mut check)
        .unwrap_err();
    assert_ne!(e.code, XML_ERR_USER_STOP);
}
