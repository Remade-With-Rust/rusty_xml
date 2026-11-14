//! Parser limits and stop controls: entity amplification (#32, #40), the
//! push parser after a fatal error (#35), SAX stop (#40), cooperative
//! cancellation (#39), deep XInclude (#34) and the DTD entity graph (#41).
//!
//! Expected outcomes come from the pinned oracle, `xmllint` 2.15.3.

use rusty_xml::{
    default_parse_options, html_read_memory, html_read_memory_interruptible,
    xml_create_push_parser_ctxt, xml_ctxt_is_stopped, xml_ctxt_set_max_amplification,
    xml_parse_chunk, xml_parse_chunk_interruptible, xml_read_memory, xml_read_memory_interruptible,
    xml_read_memory_with, xml_sax_parse_memory, xml_sax_parse_memory_interruptible, xml_save_doc,
    xml_stop_parser, xml_xinclude_process, NeverInterrupt, SaxAttr, SaxHandler, SaxRecorder,
    XmlError, XmlParseSettings, XML_ERR_ENTITY_LOOP, XML_ERR_RESOURCE_LIMIT, XML_ERR_USER_STOP,
    XML_PARSE_HUGE, XML_PARSE_RECOVER,
};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

const ENTITY_1000: usize = 1000;

/// `<!ENTITY e "x" * 1000>` referenced `n` times in content.
fn flat_bomb(n: usize) -> Vec<u8> {
    format!(
        "<!DOCTYPE r [<!ENTITY e \"{}\">]><r>{}</r>",
        "x".repeat(ENTITY_1000),
        "&e;".repeat(n)
    )
    .into_bytes()
}

/// The same, referenced from one attribute value.
fn flat_bomb_attr(n: usize) -> Vec<u8> {
    format!(
        "<!DOCTYPE r [<!ENTITY e \"{}\">]><r a=\"{}\"/>",
        "x".repeat(ENTITY_1000),
        "&e;".repeat(n)
    )
    .into_bytes()
}

fn code(r: Result<rusty_xml::XmlDoc, XmlError>) -> i32 {
    match r {
        Ok(_) => 0,
        Err(e) => e.code,
    }
}

/// #32: every top-level reference is charged its expanded size plus 20, and
/// the ratio applies past 1,000,000 bytes. 980 x 1,020 = 999,600 is under the
/// allowance and xmllint accepts it; 1,000 references pass it and xmllint
/// refuses with "Maximum entity amplification factor exceeded".
#[test]
fn flat_entity_amplification_matches_libxml2() {
    let opts = default_parse_options();
    for (n, want) in [(980, 0), (1_000, XML_ERR_RESOURCE_LIMIT), (100_000, XML_ERR_RESOURCE_LIMIT)] {
        assert_eq!(code(xml_read_memory(&flat_bomb(n), None, None, opts)), want, "content, {n} refs");
    }
    for (n, want) in [(980, 0), (1_000, XML_ERR_RESOURCE_LIMIT)] {
        assert_eq!(code(xml_read_memory(&flat_bomb_attr(n), None, None, opts)), want, "attribute, {n} refs");
    }
    // XML_PARSE_HUGE does not lift the ratio in libxml2 2.15 either.
    assert_eq!(
        code(xml_read_memory(&flat_bomb(1_000), None, None, opts | XML_PARSE_HUGE)),
        XML_ERR_RESOURCE_LIMIT
    );
    let e = xml_read_memory(&flat_bomb(1_000), None, None, opts).unwrap_err();
    assert_eq!(
        e.message,
        "Maximum entity amplification factor exceeded, see xmlCtxtSetMaxAmplification."
    );

    // 100,000 references stop at the first one past the limit: the 100 MB
    // expansion this used to produce is never built.
    let doc = xml_read_memory(&flat_bomb(980), None, None, opts).unwrap();
    assert!(xml_save_doc(&doc, 0).len() > 980 * ENTITY_1000);
}

/// A replacement that holds markup is re-parsed rather than expanded as text,
/// and is charged the same way.
#[test]
fn spliced_entity_amplification_is_charged() {
    let bomb = |n: usize| {
        format!(
            "<!DOCTYPE r [<!ENTITY e \"<b>{}</b>\">]><r>{}</r>",
            "x".repeat(ENTITY_1000),
            "&e;".repeat(n)
        )
        .into_bytes()
    };
    let opts = default_parse_options();
    assert_eq!(code(xml_read_memory(&bomb(900), None, None, opts)), 0);
    assert_eq!(code(xml_read_memory(&bomb(1_000), None, None, opts)), XML_ERR_RESOURCE_LIMIT);
}

/// #40: `xmlCtxtSetMaxAmplification`, for one-shot and push parses.
#[test]
fn max_amplification_is_settable() {
    let opts = default_parse_options();
    let lenient = XmlParseSettings::new().with_max_amplification(1_000);
    assert_eq!(code(xml_read_memory_with(&flat_bomb(1_000), None, None, opts, &lenient)), 0);
    let strict = XmlParseSettings::new().with_max_amplification(1);
    assert_eq!(
        code(xml_read_memory_with(&flat_bomb(1_000), None, None, opts, &strict)),
        XML_ERR_RESOURCE_LIMIT
    );

    // Push, in small chunks: the ratio spans the whole document.
    let push = |max: Option<u32>| {
        let input = flat_bomb(1_000);
        let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, opts);
        if let Some(m) = max {
            xml_ctxt_set_max_amplification(&mut ctxt, m);
        }
        for c in input.chunks(97) {
            if let Err(e) = xml_parse_chunk(&mut ctxt, c, 0) {
                return e.code;
            }
        }
        code(xml_parse_chunk(&mut ctxt, &[], 1).map(|d| d.unwrap()))
    };
    assert_eq!(push(None), XML_ERR_RESOURCE_LIMIT);
    assert_eq!(push(Some(1_000)), 0);
}

/// Reference loops report libxml2's XML_ERR_ENTITY_LOOP, and the graph check
/// is linear: a chain of 20,000 entities took 38 s to load before.
#[test]
fn entity_graph_is_checked_in_linear_time() {
    let opts = default_parse_options();
    for doc in [
        &b"<!DOCTYPE r [<!ENTITY a \"&b;\"><!ENTITY b \"&a;\">]><r>&a;</r>"[..],
        &b"<!DOCTYPE r [<!ENTITY a \"<x>&a;</x>\">]><r>&a;</r>"[..],
    ] {
        assert_eq!(code(xml_read_memory(doc, None, None, opts)), XML_ERR_ENTITY_LOOP);
    }

    let n = 20_000;
    let mut d = String::from("<!DOCTYPE r [");
    for i in 0..n {
        d.push_str(&format!("<!ENTITY e{i} \"&e{};\">", i + 1));
    }
    d.push_str(&format!("<!ENTITY e{n} \"x\">]><r/>"));
    let t = std::time::Instant::now();
    xml_read_memory(d.as_bytes(), None, None, opts).expect("a long chain is not a loop");
    assert!(t.elapsed().as_secs() < 10, "entity graph check took {:?}", t.elapsed());

    // An ATTLIST default and an entity may share a name; that is not a loop.
    // xmllint accepts this, and it was reported as one.
    let d = b"<!DOCTYPE r [<!ENTITY a \"z\"><!ENTITY b \"&a;\"><!ATTLIST r a CDATA \"&b;\">]><r/>";
    xml_read_memory(d, None, None, opts).expect("no loop");
}

/// Feed `chunks` with the last one terminating; one entry per chunk.
fn push_all(chunks: &[&[u8]], options: i32) -> Vec<Result<bool, (i32, u32, u32)>> {
    let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, options);
    let last = chunks.len() - 1;
    chunks
        .iter()
        .enumerate()
        .map(|(i, c)| match xml_parse_chunk(&mut ctxt, c, i32::from(i == last)) {
            Ok(d) => Ok(d.is_some()),
            Err(e) => Err((e.code, e.line, e.col)),
        })
        .collect()
}

/// #35: after a fatal error every later chunk reports the same error, as
/// libxml2's `xmlParseChunk` returns `errNo` once `disableSAX` is set.
#[test]
fn push_parser_keeps_its_first_fatal_error() {
    let opts = default_parse_options();
    assert_eq!(
        push_all(&[b"<r>", b"<a>x</a></c>", b""], opts),
        vec![Ok(false), Err((76, 1, 16)), Err((76, 1, 16))]
    );
    assert_eq!(
        push_all(&[b"<r>", b"<a>text</b>", b"<x>ok</x>"], opts),
        vec![Ok(false), Err((76, 1, 15)), Err((76, 1, 15))]
    );
    // The tail used to come back as a document of its own.
    let r = push_all(&[b"<r>", b"<!DOCTYPE a><a/>", b""], opts);
    assert!(r[1].is_err());
    assert_eq!(r[2], r[1], "the leftover is never parsed as a new document");

    let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, opts);
    xml_parse_chunk(&mut ctxt, b"<r>", 0).unwrap();
    assert!(!xml_ctxt_is_stopped(&ctxt));
    assert!(xml_parse_chunk(&mut ctxt, b"</c>", 0).is_err());
    assert!(xml_ctxt_is_stopped(&ctxt));
}

/// Under XML_PARSE_RECOVER the push parser keeps the tree built before the
/// error and returns it at the end, as a one-shot recovering parse does.
#[test]
fn push_parser_recovers_like_a_one_shot_parse() {
    let opts = default_parse_options() | XML_PARSE_RECOVER;
    let chunks: [&[u8]; 3] = [b"<r><k>kept</k>", b"<a>x</a></c>", b"<z/>"];
    let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, opts);
    assert!(xml_parse_chunk(&mut ctxt, chunks[0], 0).unwrap().is_none());
    assert!(xml_parse_chunk(&mut ctxt, chunks[1], 0).unwrap().is_none());
    assert_eq!(ctxt.last_error().map(|e| e.code), Some(76));
    let doc = xml_parse_chunk(&mut ctxt, chunks[2], 1).unwrap().expect("partial tree");
    let one_shot = xml_read_memory(&chunks.concat(), None, None, opts).unwrap();
    assert_eq!(xml_save_doc(&doc, 0), xml_save_doc(&one_shot, 0));
}

/// #40: `xmlStopParser` on a push context.
#[test]
fn push_parser_can_be_stopped() {
    let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, default_parse_options());
    xml_parse_chunk(&mut ctxt, b"<r><a/>", 0).unwrap();
    xml_stop_parser(&mut ctxt);
    assert!(xml_ctxt_is_stopped(&ctxt));
    for _ in 0..2 {
        assert_eq!(xml_parse_chunk(&mut ctxt, b"<b/></r>", 1).unwrap_err().code, XML_ERR_USER_STOP);
    }
}

/// Records events and asks to stop after the `stop_after`-th start tag.
struct Stopper {
    rec: SaxRecorder,
    starts: usize,
    stop_after: usize,
    ends_document: usize,
}

impl SaxHandler for Stopper {
    fn start_document(&mut self) {
        self.rec.start_document();
    }
    fn end_document(&mut self) {
        self.ends_document += 1;
    }
    fn start_element_ns(
        &mut self,
        local: &str,
        prefix: Option<&str>,
        uri: Option<&str>,
        namespaces: &[(Option<String>, String)],
        attributes: &[SaxAttr],
        nb_defaulted: i32,
    ) {
        self.starts += 1;
        self.rec
            .start_element_ns(local, prefix, uri, namespaces, attributes, nb_defaulted);
    }
    fn end_element_ns(&mut self, local: &str, prefix: Option<&str>, uri: Option<&str>) {
        self.rec.end_element_ns(local, prefix, uri);
    }
    fn characters(&mut self, data: &str) {
        self.rec.characters(data);
    }
    fn stop_requested(&self) -> bool {
        self.starts >= self.stop_after
    }
}

/// #40: a SAX handler stops the parse from a callback. No event but
/// end_document follows, and the parse fails with XML_ERR_USER_STOP.
#[test]
fn sax_handler_can_stop_the_parse() {
    let input = b"<r><a>one</a><b>two</b><c>three</c></r>";
    let mut h = Stopper {
        rec: SaxRecorder::default(),
        starts: 0,
        stop_after: 2,
        ends_document: 0,
    };
    let e = xml_sax_parse_memory(input, default_parse_options(), &mut h).unwrap_err();
    assert_eq!(e.code, XML_ERR_USER_STOP);
    assert_eq!(h.starts, 2, "no start tag after the stop");
    let last = h.rec.events.last().unwrap();
    assert!(
        matches!(last, rusty_xml::SaxEvent::StartElementNs { local, .. } if local == "a"),
        "the stopping callback is the last event, got {last:?}"
    );
    assert_eq!(h.ends_document, 1, "endDocument is still delivered, as in libxml2");

    // Stopping on the last start tag still fails the parse.
    let mut h = Stopper {
        rec: SaxRecorder::default(),
        starts: 0,
        stop_after: 4,
        ends_document: 0,
    };
    let e = xml_sax_parse_memory(input, default_parse_options(), &mut h).unwrap_err();
    assert_eq!(e.code, XML_ERR_USER_STOP);

    // A handler that never stops sees what it always saw.
    let mut plain = SaxRecorder::default();
    let mut gated = Stopper {
        rec: SaxRecorder::default(),
        starts: 0,
        stop_after: usize::MAX,
        ends_document: 0,
    };
    xml_sax_parse_memory(input, default_parse_options(), &mut plain).unwrap();
    xml_sax_parse_memory(input, default_parse_options(), &mut gated).unwrap();
    let starts = |v: &[rusty_xml::SaxEvent]| {
        v.iter()
            .filter(|e| matches!(e, rusty_xml::SaxEvent::StartElementNs { .. }))
            .count()
    };
    assert_eq!(starts(&plain.events), starts(&gated.rec.events));
}

fn sample_docs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"<?xml version=\"1.0\"?><r a=\"1\"><!--c--><?pi x?><a>t&amp;u</a><![CDATA[<x>]]></r>".to_vec(),
        b"<!DOCTYPE r [<!ENTITY e \"<b>x</b>\"><!ATTLIST r d CDATA \"v\">]><r>&e;&e;</r>".to_vec(),
        b"<r><unclosed></r>".to_vec(),
        b"<p:r xmlns:p=\"u\"><p:a p:x=\"1\"/>text</p:r>".to_vec(),
    ];
    let mut big = String::from("<r>");
    for i in 0..5_000 {
        big.push_str(&format!("<item n=\"{i}\">value {i}</item>"));
    }
    big.push_str("</r>");
    v.push(big.into_bytes());
    v
}

fn saved(r: &Result<rusty_xml::XmlDoc, XmlError>) -> Result<Vec<u8>, XmlError> {
    r.as_ref().map(|d| xml_save_doc(d, 0)).map_err(Clone::clone)
}

/// #39: an interruptible call that is not interrupted returns exactly what
/// the plain call returns.
#[test]
fn uninterrupted_calls_match_the_plain_calls() {
    let never = AtomicBool::new(false);
    for opts in [default_parse_options(), default_parse_options() | XML_PARSE_RECOVER] {
        for d in sample_docs() {
            let plain = xml_read_memory(&d, None, None, opts);
            let polled = xml_read_memory_interruptible(&d, None, None, opts, &never);
            assert_eq!(saved(&plain), saved(&polled));
            let polled = xml_read_memory_interruptible(&d, None, None, opts, &NeverInterrupt);
            assert_eq!(saved(&plain), saved(&polled));

            let plain = html_read_memory(&d, None, None, 0);
            let polled = html_read_memory_interruptible(&d, None, None, 0, &never);
            assert_eq!(saved(&plain), saved(&polled));

            let mut a = SaxRecorder::default();
            let mut b = SaxRecorder::default();
            let ra = xml_sax_parse_memory(&d, opts, &mut a);
            let rb = xml_sax_parse_memory_interruptible(&d, opts, &mut b, &never);
            assert_eq!(saved(&ra), saved(&rb));
            assert_eq!(a.events, b.events);

            let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, opts);
            let mut ctxt2 = xml_create_push_parser_ctxt(&[], None, None, opts);
            for c in d.chunks(64) {
                let x = xml_parse_chunk(&mut ctxt, c, 0).map(|o| o.is_some());
                let y = xml_parse_chunk_interruptible(&mut ctxt2, c, 0, &never).map(|o| o.is_some());
                assert_eq!(x, y);
            }
            let x = xml_parse_chunk(&mut ctxt, &[], 1).map(|o| o.map(|d| xml_save_doc(&d, 0)));
            let y = xml_parse_chunk_interruptible(&mut ctxt2, &[], 1, &never)
                .map(|o| o.map(|d| xml_save_doc(&d, 0)));
            assert_eq!(x, y);
        }
    }
}

/// #39: once the interrupt fires, every parser stops with XML_ERR_USER_STOP
/// -- even under XML_PARSE_RECOVER -- and stops early, not at the end.
#[test]
fn interrupt_stops_every_parser() {
    let fired = AtomicBool::new(true);
    let doc = &sample_docs()[4];
    for opts in [default_parse_options(), default_parse_options() | XML_PARSE_RECOVER] {
        assert_eq!(code(xml_read_memory_interruptible(doc, None, None, opts, &fired)), XML_ERR_USER_STOP);
        let mut rec = SaxRecorder::default();
        assert_eq!(
            code(xml_sax_parse_memory_interruptible(doc, opts, &mut rec, &fired)),
            XML_ERR_USER_STOP
        );
        let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, opts);
        let e = xml_parse_chunk_interruptible(&mut ctxt, doc, 1, &fired).unwrap_err();
        assert_eq!(e.code, XML_ERR_USER_STOP);
        assert!(xml_ctxt_is_stopped(&ctxt));
        assert_eq!(xml_parse_chunk(&mut ctxt, &[], 1).unwrap_err().code, XML_ERR_USER_STOP);
    }
    assert_eq!(code(html_read_memory_interruptible(doc, None, None, 0, &fired)), XML_ERR_USER_STOP);

    // Fires on the third poll: the parse stops long before the 5,000 items.
    for parse in 0..2 {
        let polls = Cell::new(0u32);
        let third = || {
            polls.set(polls.get() + 1);
            polls.get() >= 3
        };
        let r = if parse == 0 {
            xml_read_memory_interruptible(doc, None, None, default_parse_options(), &third)
        } else {
            html_read_memory_interruptible(doc, None, None, 0, &third)
        };
        assert_eq!(code(r), XML_ERR_USER_STOP);
        assert_eq!(polls.get(), 3, "no poll after it fired");
    }

    // From another thread, the way a host cancels.
    let flag = std::sync::Arc::new(AtomicBool::new(false));
    let f2 = std::sync::Arc::clone(&flag);
    let mut huge = String::from("<r>");
    for _ in 0..400_000 {
        huge.push_str("<i>x</i>");
    }
    huge.push_str("</r>");
    let t = std::thread::spawn(move || {
        xml_read_memory_interruptible(huge.as_bytes(), None, None, default_parse_options(), &*f2)
            .map(|d| d.len())
    });
    flag.store(true, Ordering::Relaxed);
    let r = t.join().unwrap();
    // The flag may land before the first poll or after the last; either way
    // the call returns, and if it saw the flag it says so.
    if let Err(e) = r {
        assert_eq!(e.code, XML_ERR_USER_STOP);
    }
}

/// #34: XInclude walks the tree without recursing, at any depth the parser
/// accepts under XML_PARSE_HUGE, and grafts deep included documents too.
#[test]
fn xinclude_handles_any_depth() {
    let n = 100_000;
    let deep = format!("<r>{}{}</r>", "<a>".repeat(n), "</a>".repeat(n));
    let opts = default_parse_options() | XML_PARSE_HUGE;
    let mut doc = xml_read_memory(deep.as_bytes(), None, None, opts).unwrap();
    assert_eq!(xml_xinclude_process(&mut doc, |_| Err("none".into())).unwrap(), 0);

    let inc = format!(
        "<r xmlns:xi=\"http://www.w3.org/2001/XInclude\">{}<xi:include href=\"x\"/>{}</r>",
        "<a>".repeat(n),
        "</a>".repeat(n)
    );
    let mut doc = xml_read_memory(inc.as_bytes(), None, None, opts).unwrap();
    let m = 4_000;
    let included = format!("<i k=\"v\">{}t{}</i>", "<b>".repeat(m), "</b>".repeat(m));
    let got = xml_xinclude_process(&mut doc, |h| {
        assert_eq!(h, "x");
        Ok(included.clone().into_bytes())
    })
    .unwrap();
    assert_eq!(got, 1);
    let out = String::from_utf8(xml_save_doc(&doc, 0)).unwrap();
    assert!(out.contains("<i k=\"v\"><b><b>"));
    assert!(!out.contains("xi:include"));
}

/// #41: parameter entities expand one level per pass, so ten references per
/// entity grew 639 bytes into 17 MB at six levels. They are charged like
/// general entities now; xmllint refuses from five levels.
#[test]
fn parameter_entity_amplification_is_bounded() {
    let pe = |depth: usize| {
        let mut d = String::from("<!ENTITY % a0 \"<!-- xxxxxxxx -->\">");
        for i in 1..=depth {
            d.push_str(&format!("<!ENTITY % a{i} \"{}\">", format!("&#37;a{};", i - 1).repeat(10)));
        }
        format!("<!DOCTYPE r [{d}%a{depth};]><r/>").into_bytes()
    };
    let opts = default_parse_options();
    assert_eq!(code(xml_read_memory(&pe(4), None, None, opts)), 0);
    for depth in [5, 6, 9] {
        assert_eq!(code(xml_read_memory(&pe(depth), None, None, opts)), XML_ERR_RESOURCE_LIMIT, "{depth} levels");
    }
}

/// #41: re-parsing a markup entity cloned the whole DTD per reference, so
/// 100,000 references under a 500 KB subset took 6.5 s.
#[test]
fn spliced_entities_do_not_copy_the_dtd() {
    let mut d = String::from("<!DOCTYPE r [");
    for i in 0..2_000 {
        d.push_str(&format!("<!ENTITY big{i} \"{}\">", "y".repeat(500)));
    }
    d.push_str("<!ENTITY s \"<b/>\">]><r>");
    d.push_str(&"&s;".repeat(60_000));
    d.push_str("</r>");
    let t = std::time::Instant::now();
    let doc = xml_read_memory(d.as_bytes(), None, None, default_parse_options()).unwrap();
    assert!(t.elapsed().as_secs() < 10, "took {:?}", t.elapsed());
    assert!(doc.len() > 60_000);
    assert!(doc.dtd.as_ref().is_some_and(|x| x.entities.len() == 2_001), "the DTD is intact");
}

/// #41: namespace lookup walked every open element, so 200,000 nested
/// elements under XML_PARSE_HUGE took 14 s. Lookup is bounded now, also with
/// a declaration on every level and many prefixes on one element.
#[test]
fn namespace_lookup_does_not_walk_the_depth() {
    let opts = default_parse_options() | XML_PARSE_HUGE;
    let n = 300_000;
    let t = std::time::Instant::now();
    let plain = format!("<r>{}{}</r>", "<a>".repeat(n), "</a>".repeat(n));
    xml_read_memory(plain.as_bytes(), None, None, opts).unwrap();
    let declaring = format!(
        "<r xmlns:q=\"top\">{}<q:z/>{}</r>",
        "<a xmlns:p=\"u\">".repeat(n),
        "</a>".repeat(n)
    );
    let doc = xml_read_memory(declaring.as_bytes(), None, None, opts).unwrap();
    let z = (0..doc.len() as u32)
        .map(rusty_xml::NodeId)
        .find(|&i| doc.kind(i) == rusty_xml::NodeKind::Element && doc.name(i) == "z")
        .unwrap();
    assert_eq!(doc.ns_uri(z), Some("top"), "an outer binding still resolves through the index");
    let decls: String = (0..10_000).map(|i| format!(" xmlns:p{i}=\"u{i}\"")).collect();
    let wide = format!("<r{decls}>{}</r>", "<p0:e/>".repeat(50_000));
    let doc = xml_read_memory(wide.as_bytes(), None, None, opts).unwrap();
    let e = (0..doc.len() as u32)
        .map(rusty_xml::NodeId)
        .find(|&i| doc.name(i) == "e")
        .unwrap();
    assert_eq!(doc.ns_uri(e), Some("u0"));
    assert!(t.elapsed().as_secs() < 20, "took {:?}", t.elapsed());
}

/// #41: HTML has no nesting cap (libxml2's is 256), and an end tag that
/// matched nothing, or a stray <body>, scanned the whole open-element stack:
/// 80,000 of them under 80,000 open elements took 3.8 s.
#[test]
fn html_end_tags_do_not_scan_a_deep_stack() {
    let n = 200_000;
    let t = std::time::Instant::now();
    for tail in ["</i>", "<body>"] {
        let d = format!("<div>{}{}", "<b>".repeat(n), tail.repeat(n));
        let doc = html_read_memory(d.as_bytes(), None, None, 0).unwrap();
        assert!(doc.len() > n);
    }
    assert!(t.elapsed().as_secs() < 20, "took {:?}", t.elapsed());
}

/// #41: one large construct arriving in many small chunks was rescanned from
/// its start on every chunk, and so was a large prolog. Both are linear now;
/// before, these took tens of seconds.
#[test]
fn push_parser_is_linear_in_chunk_count() {
    let opts = default_parse_options();
    let t = std::time::Instant::now();
    for doc in [
        format!("<r><!--{}--></r>", "c".repeat(1_000_000)),
        format!("<r><a x=\"{}\"/></r>", "v>".repeat(500_000)),
        format!("<r><![CDATA[{}]]></r>", "d".repeat(1_000_000)),
        format!("<!DOCTYPE r [{}]><r/>", "<!ENTITY e \"x\">".repeat(60_000)),
    ] {
        let mut c = xml_create_push_parser_ctxt(&[], None, None, opts);
        for ch in doc.as_bytes().chunks(128) {
            xml_parse_chunk(&mut c, ch, 0).unwrap();
        }
        let got = xml_parse_chunk(&mut c, &[], 1).unwrap().unwrap();
        let whole = xml_read_memory(doc.as_bytes(), None, None, opts).unwrap();
        assert_eq!(xml_save_doc(&got, 0), xml_save_doc(&whole, 0));
    }
    assert!(t.elapsed().as_secs() < 30, "took {:?}", t.elapsed());
}

/// The carried scan position must find a terminator split across chunks
/// and keep the quote state of a tag: tiny chunks give the one-shot tree.
#[test]
fn push_parser_matches_one_shot_at_every_tiny_chunk_size() {
    let opts = default_parse_options();
    let docs = [
        "<r><!-- a - b - c --><x a=\"q>'\" b='>\"'>t</x><![CDATA[ ]] ]> ]]><?p ? > ?>&amp;&#65;</r>",
        "<r>\r\n<a k=\"v\"/>\u{e9}\u{4e2d}\u{1f600}<!---->x</r>",
    ];
    for d in docs {
        let whole = xml_save_doc(&xml_read_memory(d.as_bytes(), None, None, opts).unwrap(), 0);
        for size in 1..=9 {
            let mut c = xml_create_push_parser_ctxt(&[], None, None, opts);
            for ch in d.as_bytes().chunks(size) {
                xml_parse_chunk(&mut c, ch, 0).unwrap();
            }
            let got = xml_parse_chunk(&mut c, &[], 1).unwrap().unwrap();
            assert_eq!(xml_save_doc(&got, 0), whole, "chunk size {size}");
        }
    }
}
