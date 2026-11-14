#![allow(dead_code)]
pub mod golden;

use rusty_xml::{
    html_read_memory_with_interrupt, xml_ctxt_read_memory_with_interrupt, xml_new_parser_ctxt, xml_read_io_with_interrupt,
    xml_create_push_parser_ctxt, xml_parse_chunk_with_interrupt, xml_parse_dtd_with_interrupt,
    xml_read_memory_with_interrupt, xml_reader_for_memory_with_interrupt,
    xml_sax_parse_memory_with_interrupt, xml_xinclude_process_with_interrupt,
    xml_parse_dtd, xml_validate_document_with_interrupt, xml_validate_dtd_with_interrupt,
    xml_xpath_eval_with_interrupt, InterruptCheck,    InterruptError, SaxRecorder, XmlError, XmlXPathContext, XML_ERR_USER_STOP,
};
use std::path::Path;
use std::cell::Cell;
use std::fmt::Debug;
use std::num::NonZeroU32;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::AtomicBool;

const ONE: NonZeroU32 = NonZeroU32::new(1).unwrap();

/// Runs `run` once with an interrupt that never fires and returns how many checks it made.
pub fn count_checks(run: impl FnOnce(&mut InterruptCheck<'_>)) -> u32 {
    let calls = Cell::new(0u32);
    let counter = || {
        calls.set(calls.get() + 1);
        false
    };
    run(&mut InterruptCheck::with_interval(&counter, ONE));
    calls.get()
}

/// Stops `run` at every check k in 1..=total and asserts a stop error, no panic and no `Ok`.
/// Returns total.
pub fn sweep<T, E: Debug>(
    run: impl Fn(&mut InterruptCheck<'_>) -> Result<T, E>,
    is_stop: impl Fn(&E) -> bool,
) -> u32 {
    let total = count_checks(|check| {
        let _ = run(check);
    });
    assert!(total >= 1, "no check ran");
    for k in 1..=total {
        let calls = Cell::new(0u32);
        let stop_at_k = || {
            calls.set(calls.get() + 1);
            calls.get() >= k
        };
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run(&mut InterruptCheck::with_interval(&stop_at_k, ONE))
        }));
        match outcome {
            Err(_) => panic!("k={k} total={total} panicked"),
            Ok(Ok(_)) => panic!("k={k} total={total} returned Ok"),
            Ok(Err(e)) => assert!(is_stop(&e), "k={k} total={total} wrong error {e:?}"),
        }
        assert_eq!(calls.get(), k, "k={k} total={total}: interrupt consulted after the stop");
    }
    total
}

pub fn never() -> AtomicBool {
    AtomicBool::new(false)
}

pub fn is_user_stop(e: &XmlError) -> bool {
    e.code == XML_ERR_USER_STOP
}

pub fn is_interrupted<E>(e: &InterruptError<E>) -> bool {
    matches!(e, InterruptError::Interrupted)
}


fn flatten<T>(r: Result<T, InterruptError<String>>) -> Result<T, String> {
    r.map_err(|e| match e {
        InterruptError::Other(s) => s,
        InterruptError::Interrupted => "interrupted".to_string(),
    })
}

/// Like `golden::run_default`, but through the `_with_interrupt` variants with an interrupt that
/// never fires, consulted at every check. Kinds without a variant yet run through `run_default`.
pub fn run_interrupt(dir: &Path, case: &golden::Case) -> String {
    let flag = never();
    let mut check = InterruptCheck::with_interval(&flag, ONE);
    let a = &case.args;
    let text = match case.kind.as_str() {
        "xml" => golden::render_doc(xml_read_memory_with_interrupt(
            &golden::file(dir, &a[0]),
            None,
            None,
            golden::number(&a[1]),
            &mut check,
        )),
        "ctxt" => {
            let mut ctxt = xml_new_parser_ctxt();
            let r = xml_ctxt_read_memory_with_interrupt(
                &mut ctxt,
                &golden::file(dir, &a[0]),
                None,
                None,
                golden::number(&a[1]),
                &mut check,
            );
            golden::render_ctxt(r, &ctxt)
        }
        "io" => golden::render_doc(xml_read_io_with_interrupt(
            golden::io_reader(golden::file(dir, &a[0])),
            None,
            None,
            0,
            &mut check,
        )),
        "sax" => {
            let mut rec = SaxRecorder::new();
            let r = xml_sax_parse_memory_with_interrupt(
                &golden::file(dir, &a[0]),
                0,
                &mut rec,
                &mut check,
            );
            golden::render_sax(&rec, r)
        }
        "push" | "pushlist" => {
            let chunks = golden::push_chunks(dir, case);
            let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, 0);
            let last = chunks.len() - 1;
            let results: Vec<_> = chunks
                .iter()
                .enumerate()
                .map(|(i, c)| xml_parse_chunk_with_interrupt(&mut ctxt, c, i32::from(i == last), &mut check))
                .collect();
            golden::render_push(&results)
        }
        "reader" => golden::render_reader_walk(xml_reader_for_memory_with_interrupt(
            &golden::file(dir, &a[0]),
            None,
            None,
            0,
            &mut check,
        )),
        "xinclude" => match golden::xinclude_inputs(dir, a) {
            Ok((mut doc, fragment)) => {
                let r = xml_xinclude_process_with_interrupt(
                    &mut doc,
                    |_href| Ok(fragment.clone()),
                    &mut check,
                );
                golden::render_xinclude(r, &doc)
            }
            Err(e) => e,
        },
        "html" => golden::render_doc(html_read_memory_with_interrupt(
            &golden::file(dir, &a[0]),
            None,
            None,
            0,
            &mut check,
        )),
        "html5lib" => golden::render_html5lib(dir, &a[0], |input| {
            html_read_memory_with_interrupt(input, None, None, 0, &mut check)
        }),
        "xpath" => match golden::parse_fixture(&golden::file(dir, &a[0])) {
            Ok(doc) => {
                let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
                golden::render_xpath(flatten(xml_xpath_eval_with_interrupt(&a[1], &ctx, &mut check)))
            }
            Err(e) => e,
        },
        "dtdvalid" => match golden::parse_fixture(&golden::file(dir, &a[0])) {
            Ok(doc) => golden::render_unit(flatten(xml_validate_document_with_interrupt(&doc, &mut check))),
            Err(e) => e,
        },
        "validatedtd" => match (
            golden::parse_fixture(&golden::file(dir, &a[0])),
            xml_parse_dtd(&golden::file(dir, &a[1]), None, None),
        ) {
            (Ok(doc), Ok(dtd)) => {
                golden::render_unit(flatten(xml_validate_dtd_with_interrupt(&doc, &dtd, &mut check)))
            }
            (Err(e), _) => e,
            (_, Err(e)) => golden::render_err(&e),
        },
        "dtd" => golden::render_dtd(xml_parse_dtd_with_interrupt(
            &golden::file(dir, &a[0]),
            None,
            None,
            &mut check,
        )),
        _ => return golden::run_default(dir, case),
    };
    golden::finish(text)
}
