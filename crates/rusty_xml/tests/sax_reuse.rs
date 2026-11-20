//! Namespace resolution in a SAX parse, and the reusable [`XmlSaxParser`]
//! (#80).
//!
//! #80 changed how a prefix is found: each element's namespace is located
//! once and kept for its end tag, an element that declares nothing pushes no
//! scope frame, and a lookup compares a packed key of the prefix's first 8
//! bytes and its length. The cases below are the ones those changes could get
//! wrong: prefixes sharing their first 8 bytes, redeclaration and its undoing
//! at the end tag, the default namespace, and the indexed path past 32
//! declarations. Each is checked in both SAX modes and against the tree.
//!
//! The one-shot entries (`xml_sax_parse_memory`, `xml_read_memory`) start
//! from the buffers the thread's last parse grew, as an `XmlSaxParser` does.
//! A parse on a new thread starts from nothing, so that is the reference
//! every reused path is compared with.

use rusty_xml::{
    default_parse_options, xml_read_memory, xml_sax_parse_memory, NodeId, NodeKind, SaxAttr,
    SaxEvent, SaxHandler, SaxRecorder, XmlDoc, XmlSaxParser, XML_PARSE_NO_TREE, XML_PARSE_RECOVER,
};

/// Each start and end tag's (local, prefix, uri), and each attribute's
/// (local, uri), in order.
fn names(events: &[SaxEvent]) -> Vec<String> {
    let mut out = Vec::new();
    for e in events {
        match e {
            SaxEvent::StartElementNs {
                local,
                prefix,
                uri,
                attributes,
                ..
            } => {
                out.push(format!("<{prefix:?}:{local} {uri:?}"));
                for a in attributes {
                    out.push(format!("  @{:?}:{} {:?}", a.prefix, a.local, a.uri));
                }
            }
            SaxEvent::EndElementNs { local, prefix, uri } => {
                out.push(format!("</{prefix:?}:{local} {uri:?}"));
            }
            _ => {}
        }
    }
    out
}

fn sax(xml: &str, opts: i32) -> Vec<SaxEvent> {
    let mut rec = SaxRecorder::new();
    xml_sax_parse_memory(xml.as_bytes(), opts, &mut rec).expect("parses");
    rec.events
}

/// The URI of each element in document order, from the tree.
fn tree_uris(doc: &XmlDoc) -> Vec<Option<String>> {
    (0..doc.len() as u32)
        .map(NodeId)
        .filter(|&n| doc.kind(n) == NodeKind::Element)
        .map(|n| doc.ns_uri(n).map(str::to_string))
        .collect()
}

/// The start-tag URIs from a SAX event stream.
fn sax_uris(events: &[SaxEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter_map(|e| match e {
            SaxEvent::StartElementNs { uri, .. } => Some(uri.clone()),
            _ => None,
        })
        .collect()
}

/// Both SAX modes give the same events, the tree agrees on every element's
/// URI, and the names come out as `want`.
fn check(xml: &str, want: &[&str]) {
    let tree = sax(xml, default_parse_options());
    let no_tree = sax(xml, default_parse_options() | XML_PARSE_NO_TREE);
    assert_eq!(tree, no_tree, "no-tree events differ for {xml}");
    let doc = xml_read_memory(xml.as_bytes(), None, None, default_parse_options()).unwrap();
    assert_eq!(
        tree_uris(&doc),
        sax_uris(&no_tree),
        "tree URIs differ for {xml}"
    );
    assert_eq!(names(&no_tree), want, "for {xml}");
}

#[test]
fn prefixes_sharing_their_first_8_bytes() {
    check(
        "<abcdefgh1:r xmlns:abcdefgh1='u1' xmlns:abcdefgh2='u2' xmlns:abcdefgh='u0'>\
         <abcdefgh2:e abcdefgh1:a='1' abcdefgh:b='2'/><abcdefgh:f/></abcdefgh1:r>",
        &[
            "<Some(\"abcdefgh1\"):r Some(\"u1\")",
            "<Some(\"abcdefgh2\"):e Some(\"u2\")",
            "  @Some(\"abcdefgh1\"):a Some(\"u1\")",
            "  @Some(\"abcdefgh\"):b Some(\"u0\")",
            "</Some(\"abcdefgh2\"):e Some(\"u2\")",
            "<Some(\"abcdefgh\"):f Some(\"u0\")",
            "</Some(\"abcdefgh\"):f Some(\"u0\")",
            "</Some(\"abcdefgh1\"):r Some(\"u1\")",
        ],
    );
}

#[test]
fn a_redeclaration_ends_with_its_element() {
    check(
        "<p:r xmlns:p='outer'><p:a xmlns:p='inner'><p:b/></p:a><p:c/></p:r>",
        &[
            "<Some(\"p\"):r Some(\"outer\")",
            "<Some(\"p\"):a Some(\"inner\")",
            "<Some(\"p\"):b Some(\"inner\")",
            "</Some(\"p\"):b Some(\"inner\")",
            // The end tag reports the namespace its own start tag had.
            "</Some(\"p\"):a Some(\"inner\")",
            "<Some(\"p\"):c Some(\"outer\")",
            "</Some(\"p\"):c Some(\"outer\")",
            "</Some(\"p\"):r Some(\"outer\")",
        ],
    );
}

/// `xmlns=''` reports an empty URI, as 0.10.2 did (#80 changed lookup, not
/// what a lookup returns).
#[test]
fn the_default_namespace_set_and_undeclared() {
    check(
        "<r xmlns='d'><a xmlns=''><b/></a><c/></r>",
        &[
            "<None:r Some(\"d\")",
            "<None:a Some(\"\")",
            "<None:b Some(\"\")",
            "</None:b Some(\"\")",
            "</None:a Some(\"\")",
            "<None:c Some(\"d\")",
            "</None:c Some(\"d\")",
            "</None:r Some(\"d\")",
        ],
    );
}

#[test]
fn the_fixed_prefixes_and_an_unbound_one() {
    let xml = "<r xml:lang='en'><q:e/></r>";
    let mut rec = SaxRecorder::new();
    let opts = default_parse_options() | XML_PARSE_NO_TREE | XML_PARSE_RECOVER;
    xml_sax_parse_memory(xml.as_bytes(), opts, &mut rec).unwrap();
    let n = names(&rec.events);
    assert_eq!(
        n[1],
        "  @Some(\"xml\"):lang Some(\"http://www.w3.org/XML/1998/namespace\")"
    );
    assert_eq!(n[2], "<Some(\"q\"):e None");
    assert!(rec
        .events
        .iter()
        .any(|e| matches!(e, SaxEvent::Error(m) if m.contains("prefix q"))));
}

/// Past 32 declarations in scope lookups go through an index; it must agree
/// with the walk, follow redeclarations, and be dropped again as scopes close.
#[test]
fn many_declarations_in_scope() {
    let mut xml = String::from("<r");
    for i in 0..40 {
        xml.push_str(&format!(" xmlns:p{i}='u{i}'"));
    }
    xml.push('>');
    xml.push_str("<p39:a xmlns:p3='re3'><p3:b p0:x='1' p39:y='2'/></p39:a><p3:c/>");
    // Each child declares 5 more, crossing the threshold and back each time.
    for k in 0..3 {
        xml.push_str(&format!("<p1:d{k}"));
        for i in 0..5 {
            xml.push_str(&format!(" xmlns:q{i}='v{k}{i}'"));
        }
        xml.push_str(&format!("><q4:e/></p1:d{k}>"));
    }
    xml.push_str("</r>");
    let no_tree = sax(&xml, default_parse_options() | XML_PARSE_NO_TREE);
    let n = names(&no_tree);
    assert!(
        n.contains(&"<Some(\"p3\"):b Some(\"re3\")".to_string()),
        "{n:#?}"
    );
    assert!(n.contains(&"  @Some(\"p0\"):x Some(\"u0\")".to_string()));
    assert!(n.contains(&"  @Some(\"p39\"):y Some(\"u39\")".to_string()));
    assert!(n.contains(&"<Some(\"p3\"):c Some(\"u3\")".to_string()));
    assert!(n.contains(&"<Some(\"q4\"):e Some(\"v24\")".to_string()));
    check(&xml, &n.iter().map(String::as_str).collect::<Vec<_>>());
}

// ---- XmlSaxParser -----------------------------------------------------------

/// Stops after `after` start tags.
struct StopAfter {
    rec: SaxRecorder,
    starts: usize,
    after: usize,
}

impl SaxHandler for StopAfter {
    fn start_element_ns(
        &mut self,
        local: &str,
        prefix: Option<&str>,
        uri: Option<&str>,
        ns: &[(Option<String>, String)],
        attrs: &[SaxAttr],
        d: i32,
    ) {
        self.starts += 1;
        self.rec.start_element_ns(local, prefix, uri, ns, attrs, d);
    }
    fn end_element_ns(&mut self, local: &str, prefix: Option<&str>, uri: Option<&str>) {
        self.rec.end_element_ns(local, prefix, uri);
    }
    fn characters(&mut self, data: &str) {
        self.rec.characters(data);
    }
    fn error(&mut self, msg: &str) {
        self.rec.error(msg);
    }
    fn end_document(&mut self) {
        self.rec.end_document();
    }
    fn stop_requested(&self) -> bool {
        self.starts >= self.after
    }
}

/// Documents in an order that leaves state behind if anything is not reset:
/// declarations, open elements, attribute buffers and text, cut off by a
/// fatal error, a stop, and deep nesting.
const DOCS: &[&str] = &[
    "<p:r xmlns:p='u' xmlns='d' a='1' p:b='2'><p:e c='x'>text &amp; more</p:e></p:r>",
    "<r><e/></r>",
    // Ends mid-element with declarations in scope.
    "<p:r xmlns:p='u'><p:e xmlns:q='v' q:a='1'><q:x>",
    "<p:r/>",
    "<r xmlns:p='u'><p:a/><p:b/><p:c/><p:d/></r>",
    // Redefined attribute: fatal mid-start-tag, with the pools taken.
    "<r a='1' b='2' a='3'/>",
    "<r a='a long value that grows the buffer well past the others' b='v'/>",
    "<r><a><b><c><d><e><f>deep</f></e></d></c></b></a></r>",
    "<q:r xmlns:q='w'>&#65;&#x42;<![CDATA[c]]></q:r>",
];

/// A one-shot parse on the calling thread, starting from whatever the
/// thread's earlier parses left.
fn one_shot(xml: &[u8], opts: i32, stop: usize) -> (Vec<SaxEvent>, String) {
    let mut h = StopAfter {
        rec: SaxRecorder::new(),
        starts: 0,
        after: stop,
    };
    let r = xml_sax_parse_memory(xml, opts, &mut h);
    (h.rec.events, format!("{:?}", r.map(|d| tree_uris(&d))))
}

/// A one-shot parse on a new thread: no buffers from any earlier parse.
fn fresh(xml: &[u8], opts: i32, stop: usize) -> (Vec<SaxEvent>, String) {
    let xml = xml.to_vec();
    std::thread::spawn(move || one_shot(&xml, opts, stop))
        .join()
        .expect("parse thread")
}

fn reused(p: &mut XmlSaxParser, xml: &[u8], opts: i32, stop: usize) -> (Vec<SaxEvent>, String) {
    let mut h = StopAfter {
        rec: SaxRecorder::new(),
        starts: 0,
        after: stop,
    };
    let r = p.parse_memory(xml, opts, &mut h);
    (h.rec.events, format!("{:?}", r.map(|d| tree_uris(&d))))
}

/// A reused parser gives each document exactly what a fresh parse gives it:
/// events, errors and the returned result, in every mode, after failures and
/// stops, in several orders.
#[test]
fn a_reused_parser_matches_fresh_parses() {
    let modes = [
        default_parse_options(),
        default_parse_options() | XML_PARSE_NO_TREE,
        default_parse_options() | XML_PARSE_RECOVER,
        default_parse_options() | XML_PARSE_NO_TREE | XML_PARSE_RECOVER,
    ];
    for &opts in &modes {
        let mut p = XmlSaxParser::new();
        for round in 0..3 {
            // A different order each round (steps coprime to the length).
            let step = [1, 2, 4][round];
            for i in 0..DOCS.len() {
                let d = DOCS[(i * step + round) % DOCS.len()].as_bytes();
                // Some documents are stopped after their second start tag.
                let stop = if (i + round) % 3 == 0 { 2 } else { usize::MAX };
                assert_eq!(
                    reused(&mut p, d, opts, stop),
                    fresh(d, opts, stop),
                    "options {opts:#x}, round {round}: {}",
                    String::from_utf8_lossy(d)
                );
            }
        }
    }
}

/// One-shot parses on one thread, which start from the thread's last
/// buffers, give each document exactly what a parse on a new thread gives it.
#[test]
fn one_shot_parses_on_one_thread_match_fresh_parses() {
    let big = format!("<r a='{}'>{}</r>", "v".repeat(70_000), "t".repeat(70_000));
    let mut docs: Vec<&str> = DOCS.to_vec();
    // Grows the pools past what a thread keeps, so they are dropped.
    docs.insert(4, &big);
    for opts in [
        default_parse_options(),
        default_parse_options() | XML_PARSE_NO_TREE,
        default_parse_options() | XML_PARSE_RECOVER,
    ] {
        for round in 0..3 {
            let step = [1, 3, 7][round];
            for i in 0..docs.len() {
                let d = docs[(i * step + round) % docs.len()].as_bytes();
                let stop = if (i + round) % 3 == 0 { 2 } else { usize::MAX };
                assert_eq!(
                    one_shot(d, opts, stop),
                    fresh(d, opts, stop),
                    "options {opts:#x}, round {round}: {}",
                    String::from_utf8_lossy(&d[..d.len().min(80)])
                );
            }
        }
    }
}

/// The tree entry shares the thread's buffers: every document's whole tree
/// matches a parse on a new thread.
#[test]
fn tree_parses_on_one_thread_match_fresh_parses() {
    let tree = |xml: &[u8], opts: i32| format!("{:?}", xml_read_memory(xml, None, None, opts));
    for opts in [
        default_parse_options(),
        default_parse_options() | XML_PARSE_RECOVER,
    ] {
        for round in 0..2 {
            for i in 0..DOCS.len() {
                let d = DOCS[(i * (round + 1) + round) % DOCS.len()].as_bytes();
                let here = tree(d, opts);
                let owned = d.to_vec();
                let there = std::thread::spawn(move || tree(&owned, opts))
                    .join()
                    .expect("parse thread");
                assert_eq!(here, there, "{}", String::from_utf8_lossy(d));
            }
        }
    }
}

/// A handler that parses another document from inside a callback: the inner
/// parse runs while the outer one holds the thread's buffers, and both see
/// what they would alone.
#[test]
fn a_parse_inside_a_callback_is_independent() {
    struct Nested {
        rec: SaxRecorder,
        inner: Vec<Vec<SaxEvent>>,
    }
    impl SaxHandler for Nested {
        fn start_element_ns(
            &mut self,
            local: &str,
            prefix: Option<&str>,
            uri: Option<&str>,
            ns: &[(Option<String>, String)],
            attrs: &[SaxAttr],
            d: i32,
        ) {
            self.rec.start_element_ns(local, prefix, uri, ns, attrs, d);
            let mut r = SaxRecorder::new();
            xml_sax_parse_memory(DOCS[0].as_bytes(), default_parse_options(), &mut r)
                .expect("inner parse");
            self.inner.push(r.events);
        }
        fn end_element_ns(&mut self, local: &str, prefix: Option<&str>, uri: Option<&str>) {
            self.rec.end_element_ns(local, prefix, uri);
        }
    }
    let opts = default_parse_options();
    // Every event, as the inner parse records them, on a new thread.
    let alone = std::thread::spawn(move || {
        let mut r = SaxRecorder::new();
        xml_sax_parse_memory(DOCS[0].as_bytes(), opts, &mut r).expect("parses");
        r.events
    })
    .join()
    .expect("parse thread");
    let outer_doc = DOCS[4].as_bytes();
    let mut h = Nested {
        rec: SaxRecorder::new(),
        inner: Vec::new(),
    };
    xml_sax_parse_memory(outer_doc, opts, &mut h).expect("outer parse");
    assert_eq!(h.inner.len(), 5);
    for events in &h.inner {
        assert_eq!(events, &alone);
    }
    let names_here = names(&h.rec.events);
    let names_alone = names(&fresh(outer_doc, opts, usize::MAX).0);
    assert_eq!(names_here, names_alone);
}

#[test]
fn a_reused_parser_reads_package_shaped_input() {
    // Many small parts with the same namespaces, as in an OOXML package.
    let part = |i: usize| {
        format!(
            "<p:sld xmlns:a='http://a' xmlns:p='http://p' xmlns:r='http://r'>\
             <p:sp><a:off x='{i}' y='{}'/><a:t>part {i}</a:t><p:x r:id='rId{i}'/></p:sp></p:sld>",
            i * 2
        )
    };
    let opts = default_parse_options() | XML_PARSE_NO_TREE;
    let mut p = XmlSaxParser::new();
    for i in 0..200 {
        let xml = part(i);
        assert_eq!(
            reused(&mut p, xml.as_bytes(), opts, usize::MAX),
            fresh(xml.as_bytes(), opts, usize::MAX)
        );
    }
}
