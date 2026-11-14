#![allow(dead_code)]
pub mod golden;

use rusty_xml::{
    xml_ctxt_read_memory_with_interrupt, xml_new_parser_ctxt, xml_read_io_with_interrupt,
    xml_read_memory_with_interrupt, xml_sax_parse_memory_with_interrupt, InterruptCheck,
    InterruptError, SaxRecorder, XmlDoc, XmlError, XML_ERR_USER_STOP,
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


fn unwrap_other(r: Result<XmlDoc, InterruptError<String>>) -> Result<XmlDoc, String> {
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
        _ => return golden::run_default(dir, case),
    };
    golden::finish(text)
}
