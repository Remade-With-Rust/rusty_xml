//! XPath resource bounds and libxml2 parity: #34 (depth), #37 (ordering
//! cost), #38 (node comparison), #39 (interrupt), #40 (operation limit), and
//! the step and comparison semantics the #41 review pass checked against
//! xmllint 2.15.3.

use rusty_xml::{
    default_parse_options, xml_read_memory, xml_xpath_cast_to_string, xml_xpath_cmp_nodes,
    xml_xpath_eval, xml_xpath_eval_interruptible, xml_xpath_order_doc_elems, NeverInterrupt,
    NodeId, XPathError, XPathObject, XmlDoc, XmlXPathContext, XML_PARSE_HUGE,
};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

fn doc(s: &str) -> XmlDoc {
    xml_read_memory(s.as_bytes(), None, None, default_parse_options()).expect("parses")
}

fn flat(n: usize) -> XmlDoc {
    doc(&format!("<r>{}</r>", "<a/>".repeat(n)))
}

fn s(d: &XmlDoc, e: &str) -> String {
    let ctx = XmlXPathContext::xml_xpath_new_context(d);
    let o = xml_xpath_eval(e, &ctx).unwrap_or_else(|err| panic!("{e}: {err}"));
    xml_xpath_cast_to_string(&o, &ctx)
}

fn count(d: &XmlDoc, e: &str) -> usize {
    let ctx = XmlXPathContext::xml_xpath_new_context(d);
    match xml_xpath_eval(e, &ctx) {
        Ok(XPathObject::NodeSet(v)) => v.len(),
        other => panic!("{e}: expected a node-set, got {other:?}"),
    }
}

/// Runs `f` on a thread with a small stack, so a walk that recurses once
/// per level overflows even in a debug build.
fn small_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|sc| {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn_scoped(sc, f)
            .expect("spawn")
            .join()
            .expect("no panic")
    })
}

/// #34: every XPath walk over the tree follows links instead of recursing,
/// so a 100,000-deep document parsed with XML_PARSE_HUGE is evaluated on a
/// 512 KiB stack. Each of these aborted the process at 20,000 levels on the
/// main thread, and far sooner on this stack. (A debug build uses 25,000
/// levels: parsing that deep is quadratic there, about 80 s at 100,000.)
#[test]
fn xpath_walks_do_not_recurse_per_level() {
    let n = if cfg!(debug_assertions) {
        25_000
    } else {
        100_000
    };
    let text = format!("<a>{}t{}</a>", "<a>".repeat(n - 1), "</a>".repeat(n - 1));
    let d = xml_read_memory(
        text.as_bytes(),
        None,
        None,
        default_parse_options() | XML_PARSE_HUGE,
    )
    .expect("parses under XML_PARSE_HUGE");
    small_stack(|| {
        let ctx = XmlXPathContext::xml_xpath_new_context(&d);
        let num = |e: &str| match xml_xpath_eval(e, &ctx).unwrap() {
            XPathObject::Number(x) => x,
            o => panic!("{e}: {o:?}"),
        };
        assert_eq!(num("count(//a)"), n as f64);
        assert_eq!(num("count(//text()/ancestor::a)"), n as f64);
        assert_eq!(num("count(//a[not(a)]/preceding::node())"), 0.0);
        assert_eq!(num("count(/a/following::node())"), 0.0);
        assert_eq!(num("string-length(/)"), 1.0);
        match xml_xpath_eval("//a", &ctx).unwrap() {
            XPathObject::NodeSet(v) => assert_eq!(v.len(), n),
            o => panic!("{o:?}"),
        }
        // A union and a multi-node step both sort through the order index.
        assert_eq!(num("count(//a | //text())"), (n + 1) as f64);
        assert_eq!(num("count(//a/a)"), (n - 1) as f64);
        assert_eq!(xml_xpath_order_doc_elems(&d).len(), n + 2);
        let deepest = match xml_xpath_eval("//a[not(a)]", &ctx).unwrap() {
            XPathObject::NodeSet(v) => v[0],
            o => panic!("{o:?}"),
        };
        let root = d.xml_doc_get_root_element().unwrap();
        assert_eq!(xml_xpath_cmp_nodes(&d, root, deepest), 1);
        assert_eq!(xml_xpath_cmp_nodes(&d, deepest, root), -1);
    });
}

/// Operator chains are kept flat: as nested boxes, a long `1+1+...` made an
/// expression tree as deep as it was long, and evaluating or dropping it
/// recursed that deep.
#[test]
fn long_operator_chains_do_not_recurse() {
    let d = doc("<r><a>1</a></r>");
    small_stack(|| {
        let ctx = XmlXPathContext::xml_xpath_new_context(&d);
        let n = 50_000;
        let sum = format!("{}1", "1+".repeat(n));
        match xml_xpath_eval(&sum, &ctx) {
            Ok(XPathObject::Number(x)) => assert_eq!(x, (n + 1) as f64),
            o => panic!("{o:?}"),
        }
        let ors = format!("{}true()", "false() or ".repeat(n));
        assert!(matches!(
            xml_xpath_eval(&ors, &ctx),
            Ok(XPathObject::Boolean(true))
        ));
        let ands = format!("{}false()", "true() and ".repeat(n));
        assert!(matches!(
            xml_xpath_eval(&ands, &ctx),
            Ok(XPathObject::Boolean(false))
        ));
        let eqs = format!("{}1", "1=".repeat(n));
        assert!(xml_xpath_eval(&eqs, &ctx).is_ok());
        let union = format!("{}//a", "//a|".repeat(5_000));
        match xml_xpath_eval(&union, &ctx) {
            Ok(XPathObject::NodeSet(v)) => assert_eq!(v.len(), 1),
            o => panic!("{o:?}"),
        }
    });
}

/// #37: a step whose result is most of the document is linear. The axis
/// work is checked through the operation count, which counts every node an
/// axis visits: `//a/following::a` from n context nodes visited O(n^2)
/// nodes before the merge and visits O(n) now.
#[test]
fn step_merging_is_linear() {
    let n = 20_000;
    let d = flat(n);
    let mut ctx = XmlXPathContext::xml_xpath_new_context(&d);
    ctx.set_op_limit(10 * n as u64);
    for (e, want) in [
        ("count(//a)", n),
        ("count(//a/following::a)", n - 1),
        ("count(//a/preceding::a)", n - 1),
        ("count(//a/following-sibling::a)", n - 1),
        ("count(//a/preceding-sibling::a)", n - 1),
        ("count(//a/..)", 1),
        ("count(//a/ancestor::*)", 1),
        ("count(//*//a)", n),
        ("count(//a | //a)", n),
    ] {
        match xml_xpath_eval_interruptible(e, &ctx, &NeverInterrupt) {
            Ok(XPathObject::Number(x)) => assert_eq!(x, want as f64, "{e}"),
            o => panic!("{e} within 10 ops per node: {o:?}"),
        }
    }
    // Wall clock as a backstop for the sort itself (4,000 elements took
    // 8 ms and 32,000 took 490 ms in release when it was quadratic).
    let big = flat(100_000);
    let t = std::time::Instant::now();
    assert_eq!(count(&big, "//a"), 100_000);
    assert_eq!(count(&big, "//a/following-sibling::a"), 99_999);
    assert!(t.elapsed().as_secs() < 20, "took {:?}", t.elapsed());
}

/// #38: libxml2's contract, 1 when the first node comes first.
#[test]
fn cmp_nodes_matches_libxml2() {
    let d = doc("<r x='1' y='2'><a/><a><b/></a><a/></r>");
    let r = d.xml_doc_get_root_element().unwrap();
    let a1 = d.first_child(r).unwrap();
    let a2 = d.next_sibling(a1).unwrap();
    let a3 = d.last_child(r).unwrap();
    let b = d.first_child(a2).unwrap();
    let x = d.first_attr(r).unwrap();
    let y = d.next_sibling(x).unwrap();
    assert_eq!(xml_xpath_cmp_nodes(&d, r, a1), 1);
    assert_eq!(xml_xpath_cmp_nodes(&d, a1, a3), 1);
    assert_eq!(xml_xpath_cmp_nodes(&d, a3, a1), -1);
    assert_eq!(xml_xpath_cmp_nodes(&d, a1, a1), 0);
    assert_eq!(xml_xpath_cmp_nodes(&d, b, a3), 1);
    assert_eq!(xml_xpath_cmp_nodes(&d, a3, b), -1);
    assert_eq!(xml_xpath_cmp_nodes(&d, a1, b), 1);
    assert_eq!(xml_xpath_cmp_nodes(&d, NodeId::DOCUMENT, b), 1);
    // Attributes come after their element and before its children.
    assert_eq!(xml_xpath_cmp_nodes(&d, r, x), 1);
    assert_eq!(xml_xpath_cmp_nodes(&d, x, r), -1);
    assert_eq!(xml_xpath_cmp_nodes(&d, x, y), 1);
    assert_eq!(xml_xpath_cmp_nodes(&d, y, x), -1);
    assert_eq!(xml_xpath_cmp_nodes(&d, x, a1), 1);
    assert_eq!(xml_xpath_cmp_nodes(&d, b, y), -1);
    // Not comparable: an id outside the document, or a detached node.
    assert_eq!(xml_xpath_cmp_nodes(&d, a1, NodeId(1_000_000)), -2);
    let mut d2 = doc("<r><a/></r>");
    let lone = d2.xml_new_node(None, "lone");
    let a = d2
        .first_child(d2.xml_doc_get_root_element().unwrap())
        .unwrap();
    assert_eq!(xml_xpath_cmp_nodes(&d2, lone, a), -2);
    // Agrees with the document-order walk on every pair.
    let order = xml_xpath_order_doc_elems(&d);
    for (i, &p) in order.iter().enumerate() {
        for (j, &q) in order.iter().enumerate() {
            let want = match i.cmp(&j) {
                std::cmp::Ordering::Less => 1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => -1,
            };
            assert_eq!(xml_xpath_cmp_nodes(&d, p, q), want, "{i} vs {j}");
        }
    }
    // The indexed form agrees, including on what it cannot compare.
    let ctx = XmlXPathContext::xml_xpath_new_context(&d);
    for &p in &order {
        for &q in &order {
            assert_eq!(ctx.cmp_nodes(p, q), xml_xpath_cmp_nodes(&d, p, q));
        }
    }
    assert_eq!(ctx.cmp_nodes(a1, NodeId(1_000_000)), -2);
    let ctx2 = XmlXPathContext::xml_xpath_new_context(&d2);
    assert_eq!(ctx2.cmp_nodes(lone, a), -2);
    assert_eq!(ctx2.cmp_nodes(lone, lone), 0);
    // First against last of a flat document: the walk scans the siblings
    // between them, the index answers at once.
    let big = flat(100_000);
    let r = big.xml_doc_get_root_element().unwrap();
    let (first, last) = (big.first_child(r).unwrap(), big.last_child(r).unwrap());
    let ctx = XmlXPathContext::xml_xpath_new_context(&big);
    assert_eq!(ctx.cmp_nodes(first, last), 1);
    assert_eq!(ctx.cmp_nodes(last, first), -1);
    assert_eq!(xml_xpath_cmp_nodes(&big, last, first), -1);
}

/// Predicates apply per context node, with reverse axes counting backwards,
/// and node-set comparisons are existential. Every expected value here is
/// xmllint 2.15.3's answer; before this change eight of them differed.
#[test]
fn step_and_comparison_semantics_match_libxml2() {
    let d = doc("<r><a><b>1</b><b>2</b></a><a><b>3</b><b>x</b></a>\
         <c>p</c><c>q</c><c>s</c><n>5</n><n>2</n><n>abc</n></r>");
    assert_eq!(count(&d, "/r/a/b[1]"), 2);
    assert_eq!(count(&d, "/r/a/b[last()]"), 2);
    assert_eq!(s(&d, "/r/c[3]/preceding-sibling::c[1]"), "q");
    assert_eq!(s(&d, "/r/a[2]/b[1]/ancestor::*[1]/b[2]"), "x");
    assert_eq!(count(&d, "//b/ancestor::*[1]"), 2);
    assert_eq!(s(&d, "//b[.='x']/preceding::b[1]"), "3");
    assert_eq!(count(&d, "//b[2]/preceding::b"), 3);
    assert_eq!(count(&d, "//b/following::*"), 10);
    assert_eq!(count(&d, "//b[1.5]"), 0);
    for (e, want) in [
        ("//b='x'", true),
        ("//b!='x'", true),
        ("//b='zz'", false),
        ("//b!=//c", true),
        ("/r/c[1]!=/r/c[1]", false),
        ("//n>4", true),
        ("//n<=1", false),
        ("//n=5", true),
        ("//n!=5", true),
        ("//n=2.0", true),
        ("//zz=false()", true),
        ("//b=true()", true),
        ("//b>true()", false),
        ("'4'>//n", true),
        ("//zz!=//b", false),
        ("//zz<1", false),
    ] {
        let ctx = XmlXPathContext::xml_xpath_new_context(&d);
        match xml_xpath_eval(e, &ctx) {
            Ok(XPathObject::Boolean(b)) => assert_eq!(b, want, "{e}"),
            o => panic!("{e}: {o:?}"),
        }
    }
}

/// #40: `op_limit` caps one evaluation; 0 means no cap.
#[test]
fn op_limit_stops_evaluation() {
    let d = flat(1_000);
    let mut ctx = XmlXPathContext::xml_xpath_new_context(&d);
    assert_eq!(ctx.op_limit(), 0);
    let e = "count(//a[count(following-sibling::a) >= 0])";
    let want = xml_xpath_eval(e, &ctx).unwrap();
    ctx.set_op_limit(1_000);
    assert_eq!(
        xml_xpath_eval_interruptible(e, &ctx, &NeverInterrupt).unwrap_err(),
        XPathError::OpLimitExceeded
    );
    assert_eq!(
        XPathError::OpLimitExceeded.code(),
        rusty_xml::XPATH_OP_LIMIT_EXCEEDED
    );
    let plain = xml_xpath_eval(e, &ctx).unwrap_err();
    assert!(plain.contains("operation limit"), "{plain}");
    // The count restarts for every evaluation, and a cap the work fits
    // under changes nothing.
    ctx.set_op_limit(10_000_000);
    for _ in 0..3 {
        let got = xml_xpath_eval_interruptible(e, &ctx, &NeverInterrupt).unwrap();
        assert_eq!(format!("{got:?}"), format!("{want:?}"));
    }
    // translate() is charged as libxml2 charges it.
    ctx.set_op_limit(50);
    let big = format!(
        "translate('{}', '{}', '')",
        "x".repeat(200),
        "y".repeat(200)
    );
    assert!(xml_xpath_eval(&big, &ctx).is_err());
    ctx.set_op_limit(0);
    assert_eq!(s(&d, &big), "x".repeat(200));
}

/// #39: an interrupt stops an evaluation and is otherwise invisible.
#[test]
fn interrupt_stops_evaluation() {
    let d = flat(2_000);
    let ctx = XmlXPathContext::xml_xpath_new_context(&d);
    let e = "count(//a[count(preceding-sibling::a) >= 0])";
    let fired = AtomicBool::new(true);
    assert_eq!(
        xml_xpath_eval_interruptible(e, &ctx, &fired).unwrap_err(),
        XPathError::Interrupted
    );
    // Fires on the third poll: the evaluation was well under way.
    let polls = Cell::new(0u32);
    let later = || {
        polls.set(polls.get() + 1);
        polls.get() >= 3
    };
    assert_eq!(
        xml_xpath_eval_interruptible(e, &ctx, &later).unwrap_err(),
        XPathError::Interrupted
    );
    assert_eq!(polls.get(), 3);
    // Never fired: the same result as the plain call.
    fired.store(false, Ordering::Relaxed);
    let a = xml_xpath_eval_interruptible(e, &ctx, &fired).unwrap();
    let b = xml_xpath_eval(e, &ctx).unwrap();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    // Compile errors keep their text.
    assert!(matches!(
        xml_xpath_eval_interruptible("1 +", &ctx, &NeverInterrupt),
        Err(XPathError::Eval(_))
    ));
}

/// A function called with too few arguments is an error, as in libxml2
/// (XPATH_INVALID_ARITY); it indexed past the argument list and panicked.
#[test]
fn missing_function_arguments_are_errors_not_panics() {
    let d = doc("<r/>");
    let ctx = XmlXPathContext::xml_xpath_new_context(&d);
    for e in [
        "count()",
        "substring('a')",
        "translate('a','b')",
        "not()",
        "concat()",
        "starts-with('a')",
    ] {
        let r = std::panic::catch_unwind(|| xml_xpath_eval(e, &ctx));
        match r {
            Ok(Err(msg)) => assert!(msg.contains("Invalid number of arguments"), "{e}: {msg}"),
            Ok(Ok(o)) if e == "concat()" => assert!(matches!(o, XPathObject::String(_))),
            other => panic!("{e}: {other:?}"),
        }
    }
}

/// translate() maps through a table now; it scanned `from` for every
/// character of the input.
#[test]
fn translate_is_linear_and_unchanged() {
    let d = doc("<r/>");
    assert_eq!(s(&d, "translate('bar','abc','ABC')"), "BAr");
    assert_eq!(s(&d, "translate('--aaa--','abc-','ABC')"), "AAA");
    assert_eq!(s(&d, "translate('abc','aa','xy')"), "xbc");
    let from: String = (0..5_000u32)
        .filter_map(|c| char::from_u32(0x4e00 + c))
        .collect();
    let input = "\u{4e00}z".repeat(50_000);
    let e = format!("string-length(translate('{input}', '{from}', ''))");
    assert_eq!(s(&d, &e), "50000");
}
