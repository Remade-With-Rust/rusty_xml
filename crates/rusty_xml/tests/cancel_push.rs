mod support;

use rusty_xml::{
    xml_create_push_parser_ctxt, xml_parse_chunk, xml_parse_chunk_with_interrupt, InterruptCheck,
    NoInterrupt, XmlDoc, XmlError, XmlPushParserCtxt, XML_ERR_USER_STOP,
};
use std::cell::Cell;
use std::num::NonZeroU32;
use support::{is_user_stop, sweep};

const ONE: NonZeroU32 = NonZeroU32::new(1).unwrap();

fn ctxt() -> XmlPushParserCtxt {
    xml_create_push_parser_ctxt(&[], None, None, 0)
}

fn feed(
    ctxt: &mut XmlPushParserCtxt,
    chunk: &[u8],
    terminate: i32,
    check: &mut InterruptCheck<'_>,
) -> Result<Option<XmlDoc>, XmlError> {
    xml_parse_chunk_with_interrupt(ctxt, chunk, terminate, check)
}

#[test]
fn push_stop_is_sticky() {
    let mut ctxt = ctxt();
    let calm = || false;
    assert!(feed(
        &mut ctxt,
        b"<r>",
        0,
        &mut InterruptCheck::with_interval(&calm, ONE)
    )
    .unwrap()
    .is_none());

    let calls = Cell::new(0u32);
    let stop_on_second = || {
        calls.set(calls.get() + 1);
        calls.get() >= 2
    };
    let stopped = feed(
        &mut ctxt,
        b"<a>text</a>",
        0,
        &mut InterruptCheck::with_interval(&stop_on_second, ONE),
    )
    .unwrap_err();
    assert_eq!(stopped.code, XML_ERR_USER_STOP);
    assert_eq!(ctxt.last_error(), Some(&stopped));

    let mut never = InterruptCheck::new(&NoInterrupt);
    assert_eq!(
        feed(&mut ctxt, b"more", 0, &mut never).unwrap_err(),
        stopped
    );
    assert_eq!(
        feed(&mut ctxt, b"</r>", 1, &mut never).unwrap_err(),
        stopped
    );
    assert_eq!(xml_parse_chunk(&mut ctxt, b"x", 1).unwrap_err(), stopped);
}

#[test]
fn push_sweep_terminate() {
    let doc = br#"<r a="1"><!--c--><?p d?><e>&lt;t&amp;</e>tail</r>"#;
    sweep(|c| feed(&mut ctxt(), doc, 1, c), is_user_stop);
}

#[test]
fn push_prolog_sweep() {
    let total = sweep(
        |c| {
            feed(
                &mut ctxt(),
                b"<?xml version=\"1.0\"?><!--a--><!--b-->",
                0,
                c,
            )
        },
        is_user_stop,
    );
    assert!(total >= 3, "total {total}");
}

#[test]
fn push_no_stream_sweep() {
    let utf16: Vec<u8> = [0xFFu8, 0xFE]
        .into_iter()
        .chain("<r><e/></r>".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let (first, second) = utf16.split_at(8);
    sweep(
        |c| {
            let mut ctxt = ctxt();
            assert!(xml_parse_chunk(&mut ctxt, first, 0).unwrap().is_none());
            feed(&mut ctxt, second, 1, c)
        },
        is_user_stop,
    );
}

#[test]
fn push_terminate_before_root_sweep() {
    sweep(|c| feed(&mut ctxt(), b"<!--x-->", 1, c), is_user_stop);
}
