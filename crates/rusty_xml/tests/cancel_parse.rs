mod support;

use rusty_xml::{
    xml_ctxt_read_memory_with_interrupt, xml_new_parser_ctxt, xml_read_io_with_interrupt,
    xml_read_memory_with_interrupt, xml_sax_parse_memory_with_interrupt, xml_save_doc,
    InterruptCheck, SaxEvent, SaxRecorder, XML_ERR_USER_STOP, XML_PARSE_NOENT, XML_PARSE_RECOVER,
};
use std::io::Read;
use support::{count_checks, is_user_stop, sweep};

const DOC: &[u8] = b"<r a=\"1\"><!--c--><?p d?><e>&lt;t&amp;</e>tail</r>";

fn read(doc: &[u8], options: i32, check: &mut InterruptCheck<'_>) -> Result<(), rusty_xml::XmlError> {
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
    let with_entities = br#"<!DOCTYPE r [<!ENTITY a "<x/><x/>"><!ENTITY b "&a;&a;">]><r>&b;&b;</r>"#;
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
                assert_eq!(ctxt.last_error.as_ref().map(|e| e.code), Some(XML_ERR_USER_STOP));
                assert_eq!(xml_save_doc(ctxt.doc.as_ref().unwrap(), 0), previous_bytes);
            }
            r
        },
        is_user_stop,
    );
}

fn read_io(bytes: &'static [u8], step: usize, check: &mut InterruptCheck<'_>) -> Result<(), rusty_xml::XmlError> {
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

