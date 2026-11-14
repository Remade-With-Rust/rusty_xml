mod support;

use rusty_xml::{
    xml_read_memory, xml_xpath_eval, xml_xpath_eval_with_interrupt, InterruptCheck, InterruptError,
    XPathObject, XmlDoc, XmlXPathContext,
};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use support::{is_interrupted, sweep};

fn parse(xml: &str) -> XmlDoc {
    xml_read_memory(xml.as_bytes(), None, None, 0).expect("fixture parses")
}

fn eval(
    doc: &XmlDoc,
    expr: &str,
    check: &mut InterruptCheck<'_>,
) -> Result<XPathObject, InterruptError<String>> {
    let ctx = XmlXPathContext::xml_xpath_new_context(doc);
    xml_xpath_eval_with_interrupt(expr, &ctx, check)
}

fn sweep_expr(xml: &str, expr: &str) -> u32 {
    let doc = parse(xml);
    sweep(|c| eval(&doc, expr, c), is_interrupted)
}

#[test]
fn xpath_sweep_union() {
    let xml = format!("<r>{}{}</r>", "<a/>".repeat(150), "<b/>".repeat(150));
    let total = sweep_expr(&xml, "//a | //b");
    assert!(total >= 300, "total {total}");
}

#[test]
fn xpath_sweep_cases() {
    // id(): one check per element visited inside the per-token walk.
    let tokens: Vec<String> = (0..50).map(|i| format!("k{i}")).collect();
    let elements: String = tokens.iter().map(|t| format!("<e id=\"{t}\"/>")).collect();
    sweep_expr(&format!("<r><t>{}</t>{elements}</r>", tokens.join(" ")), "id(//t)");

    // sum(): one check per node summed.
    let nested = format!("{}{}", "<a>1".repeat(20), "</a>".repeat(20));
    sweep_expr(&format!("<r>{nested}</r>"), "sum(//a)");

    // preceding axis: one check per earlier node.
    sweep_expr(&format!("<r>{}</r>", "<x/>".repeat(60)), "/descendant::x/preceding::*");

    // predicates: one check per candidate.
    sweep_expr(&format!("<r>{}</r>", "<x v=\"1\"/>".repeat(40)), "//x[@v = '1'][position() > 3]");
}

#[test]
fn xpath_result_unchanged_by_the_check() {
    let xml = format!("<r>{}{}</r>", "<a/>".repeat(30), "<b/>".repeat(30));
    let doc = parse(&xml);
    let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
    let plain = xml_xpath_eval("//b | //a", &ctx).unwrap();
    let flag = support::never();
    let checked = xml_xpath_eval_with_interrupt(
        "//b | //a",
        &ctx,
        &mut InterruptCheck::with_interval(&flag, std::num::NonZeroU32::MIN),
    )
    .unwrap();
    match (plain, checked) {
        (XPathObject::NodeSet(a), XPathObject::NodeSet(b)) => assert_eq!(a, b),
        other => panic!("unexpected results {other:?}"),
    }
}

/// Stops `expr` from another thread 100 ms after it starts and prints the time from the flag to
/// the return. The fixture doubles, at most three times, until the uninterrupted run takes 2 s.
fn latency(label: &str, expr: &str, mut size: usize, build: impl Fn(usize) -> String) {
    let (doc, uninterrupted) = {
        let mut attempt = 0;
        loop {
            let doc = parse(&build(size));
            let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
            let start = Instant::now();
            xml_xpath_eval(expr, &ctx).expect("uninterrupted run");
            let took = start.elapsed();
            if took >= Duration::from_secs(2) {
                break (doc, took);
            }
            assert!(attempt < 3, "{label}: still {took:?} at size {size}");
            attempt += 1;
            size *= 2;
        }
    };
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
    let result = eval(&doc, expr, &mut InterruptCheck::new(&*flag));
    let returned_at = Instant::now();
    setter.join().unwrap();
    assert!(matches!(result, Err(InterruptError::Interrupted)), "{label}: not stopped");
    let latency = returned_at.duration_since(flagged_at.lock().unwrap().unwrap());
    println!("{label}: size {size}, uninterrupted {uninterrupted:?}, flag to return {latency:?}");
}

#[test]
#[ignore = "release-only latency measurement"]
fn latency_sort_unique() {
    latency("sort_unique", "//a|//b", 36_000, |n| {
        format!("<r>{}{}</r>", "<a/>".repeat(n), "<b/>".repeat(n))
    });
}

#[test]
#[ignore = "release-only latency measurement"]
fn latency_id() {
    latency("id", "count(id(/r/ids))", 8_000, |n| {
        let ids: Vec<String> = (0..n).map(|i| format!("k{i}")).collect();
        let children: String = ids.iter().map(|i| format!("<c id=\"{i}\"/>")).collect();
        format!("<r><ids>{}</ids>{children}</r>", ids.join(" "))
    });
}
