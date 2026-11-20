//! One table of gates, run on every entry point each applies to (#71).
//!
//! A limit or a fix added for one path has been missing from another before
//! (#70: the arena reserve, character references, encodings, interrupts). So
//! each case here runs against every entry that can take it, and an entry a
//! case does not apply to is listed with the reason.
//!
//! Entries: the HTML parser, the XML parser, SAX (with and without
//! `XML_PARSE_NO_TREE`, and through one reused `XmlSaxParser`), push, the
//! reader, `xml_parse_dtd`, XInclude.

use rusty_xml::{
    default_parse_options, html_read_memory_with, xml_create_push_parser_ctxt,
    xml_parse_chunk, xml_parse_chunk_interruptible, xml_parse_dtd, xml_parse_dtd_with,
    xml_read_memory, xml_read_memory_with, xml_reader_for_memory_with, xml_sax_parse_memory_with,
    xml_xinclude_process_with, HtmlParseSettings, NullSax, ParseSettings, XmlDoc, XmlError,
    XmlParseSettings, XmlSaxParser, XML_ERR_INVALID_ENCODING, XML_ERR_USER_STOP,
    XML_PARSE_NO_TREE,
};
use std::cell::Cell;
use std::time::{Duration, Instant};

/// The XML entries that parse a whole buffer, by name, each taking settings.
type XmlEntry = fn(&[u8], &XmlParseSettings<'_>) -> Result<(), XmlError>;

fn xml_entries() -> Vec<(&'static str, XmlEntry)> {
    fn tree(b: &[u8], s: &XmlParseSettings<'_>) -> Result<(), XmlError> {
        xml_read_memory_with(b, None, None, default_parse_options(), s).map(drop)
    }
    fn sax(b: &[u8], s: &XmlParseSettings<'_>) -> Result<(), XmlError> {
        xml_sax_parse_memory_with(b, default_parse_options(), &mut NullSax, s).map(drop)
    }
    fn sax_no_tree(b: &[u8], s: &XmlParseSettings<'_>) -> Result<(), XmlError> {
        let o = default_parse_options() | XML_PARSE_NO_TREE;
        xml_sax_parse_memory_with(b, o, &mut NullSax, s).map(drop)
    }
    fn sax_reused(b: &[u8], s: &XmlParseSettings<'_>) -> Result<(), XmlError> {
        // One parser per thread, reused by every case: each gate also runs
        // after whatever the previous case left behind (#80).
        thread_local! {
            static P: std::cell::RefCell<XmlSaxParser> = std::cell::RefCell::new(XmlSaxParser::new());
        }
        let o = default_parse_options() | XML_PARSE_NO_TREE;
        P.with(|p| p.borrow_mut().parse_memory_with(b, o, &mut NullSax, s).map(drop))
    }
    fn reader(b: &[u8], s: &XmlParseSettings<'_>) -> Result<(), XmlError> {
        xml_reader_for_memory_with(b, None, None, default_parse_options(), &ParseSettings::from(*s))
            .map(drop)
    }
    fn xinclude(b: &[u8], s: &XmlParseSettings<'_>) -> Result<(), XmlError> {
        // The document under test arrives through an include.
        let mut doc = xml_read_memory(
            b"<r xmlns:xi=\"http://www.w3.org/2001/XInclude\"><xi:include href=\"t\"/></r>",
            None,
            None,
            default_parse_options(),
        )
        .unwrap();
        let body = b.to_vec();
        xml_xinclude_process_with(&mut doc, |_| Ok(body.clone()), &ParseSettings::from(*s)).map(drop)
    }
    vec![
        ("xml_read_memory_with", tree as XmlEntry),
        ("xml_sax_parse_memory_with", sax),
        ("xml_sax_parse_memory_with + NO_TREE", sax_no_tree),
        ("XmlSaxParser::parse_memory_with (reused) + NO_TREE", sax_reused),
        ("xml_reader_for_memory_with", reader),
        ("xml_xinclude_process_with (included document)", xinclude),
    ]
}

/// Push takes its settings from the context; it is run where the case does
/// not need a custom setting.
fn push(b: &[u8], interrupt: Option<&dyn rusty_xml::Interrupt>) -> Result<Option<XmlDoc>, XmlError> {
    let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, default_parse_options());
    for ch in b.chunks(64 * 1024) {
        match interrupt {
            Some(i) => xml_parse_chunk_interruptible(&mut ctxt, ch, 0, i)?,
            None => xml_parse_chunk(&mut ctxt, ch, 0)?,
        };
    }
    match interrupt {
        Some(i) => xml_parse_chunk_interruptible(&mut ctxt, &[], 1, i),
        None => xml_parse_chunk(&mut ctxt, &[], 1),
    }
}

fn nested(n: usize) -> Vec<u8> {
    format!("{}{}", "<e>".repeat(n), "</e>".repeat(n)).into_bytes()
}

/// An interrupt that fires on its `n`th poll, counting polls.
fn fires_on(n: u32) -> (impl Fn() -> bool, std::rc::Rc<Cell<u32>>) {
    let polls = std::rc::Rc::new(Cell::new(0u32));
    let p = polls.clone();
    (
        move || {
            p.set(p.get() + 1);
            p.get() >= n
        },
        polls,
    )
}

// ---- depth cap -----------------------------------------------------------

/// The nesting cap, at and past the limit, on every entry that nests.
/// Skipped: `xml_parse_dtd` (a DTD does not nest elements).
#[test]
fn gate_depth_cap() {
    let default = XmlParseSettings::new();
    let capped = XmlParseSettings::new().with_max_depth(10);
    let uncapped = XmlParseSettings::new().with_max_depth(-1);
    for (name, entry) in xml_entries() {
        assert!(entry(&nested(5_000), &default).is_ok(), "{name}: 5,000 is the default cap");
        let e = entry(&nested(5_001), &default).unwrap_err();
        assert!(e.message.contains("nesting"), "{name}: {e}");
        assert!(entry(&nested(10), &capped).is_ok(), "{name}: at a custom cap");
        assert!(entry(&nested(11), &capped).is_err(), "{name}: past a custom cap");
        assert!(entry(&nested(6_000), &uncapped).is_ok(), "{name}: no cap");
    }
    // Push: the default cap (its context carries no depth setting).
    assert!(push(&nested(5_000), None).is_ok());
    assert!(push(&nested(5_001), None).is_err());
    // HTML: libxml2's 256 by default, a custom cap, and none.
    let html = |n: usize, s: &HtmlParseSettings<'_>| {
        let d = format!("<html><body>{}x", "<div>".repeat(n));
        html_read_memory_with(d.as_bytes(), None, None, 0, s).unwrap()
    };
    assert!(html(254, &HtmlParseSettings::new()).warnings.is_empty());
    assert!(!html(255, &HtmlParseSettings::new()).warnings.is_empty());
    assert!(!html(9, &HtmlParseSettings::new().with_max_depth(10)).warnings.is_empty());
    assert!(html(1_000, &HtmlParseSettings::new().with_max_depth(-1)).warnings.is_empty());
}

// ---- many attributes -----------------------------------------------------

/// 100,000 attributes on one element stay linear on every entry that reads
/// attributes: a quadratic path takes minutes at this size (XInclude's copy
/// did, until this gate found it). Skipped: `xml_parse_dtd` (no elements).
#[test]
fn gate_many_attributes() {
    let attrs: String = (0..100_000).map(|i| format!(" a{i}=\"v\"")).collect();
    let xml = format!("<r{attrs}/>");
    let t = Instant::now();
    for (name, entry) in xml_entries() {
        assert!(entry(xml.as_bytes(), &XmlParseSettings::new()).is_ok(), "{name}");
    }
    assert!(push(xml.as_bytes(), None).is_ok());
    let h = html_read_memory_with(xml.as_bytes(), None, None, 0, &HtmlParseSettings::new()).unwrap();
    assert!(h.len() > 100_000);
    assert!(t.elapsed() < Duration::from_secs(60), "took {:?}", t.elapsed());
}

/// What an included document carries arrives whole: XInclude's copy dropped
/// attribute prefixes (`xml:lang` became `lang`), where xmllint keeps them.
#[test]
fn gate_xinclude_copies_attributes_exactly() {
    let mut doc = xml_read_memory(
        b"<r xmlns:xi=\"http://www.w3.org/2001/XInclude\"><xi:include href=\"t\"/></r>",
        None,
        None,
        default_parse_options(),
    )
    .unwrap();
    let inc = b"<i xmlns:p=\"urn:p\" xml:lang=\"en\" p:a=\"1\" b=\"2\"/>".to_vec();
    xml_xinclude_process_with(&mut doc, |_| Ok(inc.clone()), &ParseSettings::new()).unwrap();
    let out = String::from_utf8(rusty_xml::xml_save_doc(&doc, 0)).unwrap();
    // xmllint 2.15.3 --xinclude, byte for byte.
    assert_eq!(
        out,
        "<?xml version=\"1.0\"?>
<r xmlns:xi=\"http://www.w3.org/2001/XInclude\"><i xmlns:p=\"urn:p\" xml:lang=\"en\" p:a=\"1\" b=\"2\"/></r>
"
    );
}

// ---- interrupts -----------------------------------------------------------

/// An interrupt that fires part way through a large input stops every entry,
/// with `XML_ERR_USER_STOP`, and is polled during the work, not only before.
#[test]
fn gate_interrupt_part_way() {
    let big = format!("<r>{}</r>", "<a x=\"1\">t</a>".repeat(200_000));
    for (name, entry) in xml_entries() {
        let (f, polls) = fires_on(5);
        let mut s = XmlParseSettings::new();
        s.interrupt = Some(&f);
        let e = entry(big.as_bytes(), &s).unwrap_err();
        assert_eq!(e.code, XML_ERR_USER_STOP, "{name}");
        assert!(polls.get() >= 5, "{name}: polled {} times", polls.get());
    }
    let (f, _) = fires_on(5);
    assert_eq!(push(big.as_bytes(), Some(&f)).unwrap_err().code, XML_ERR_USER_STOP);
    let (f, _) = fires_on(5);
    let s = HtmlParseSettings::new().with_interrupt(&f);
    let e = html_read_memory_with(big.as_bytes(), None, None, 0, &s).unwrap_err();
    assert_eq!(e.code, XML_ERR_USER_STOP);

    // DTD loading: polled per declaration.
    let dtd: String = (0..20_000).map(|i| format!("<!ELEMENT e{i} EMPTY>\n")).collect();
    let (f, polls) = fires_on(5);
    let s = ParseSettings::new().with_interrupt(&f);
    let e = xml_parse_dtd_with(dtd.as_bytes(), None, None, &s).unwrap_err();
    assert_eq!(e.code, XML_ERR_USER_STOP);
    assert!(polls.get() >= 5);
    assert_eq!(xml_parse_dtd(dtd.as_bytes(), None, None).unwrap().elements.len(), 20_000);

    // XInclude: polled between includes.
    let includes: String = (0..10).map(|i| format!("<xi:include href=\"{i}\"/>")).collect();
    let src = format!("<r xmlns:xi=\"http://www.w3.org/2001/XInclude\">{includes}</r>");
    let mut doc = xml_read_memory(src.as_bytes(), None, None, default_parse_options()).unwrap();
    let (f, _) = fires_on(3);
    let s = ParseSettings::new().with_interrupt(&f);
    let e = xml_xinclude_process_with(&mut doc, |_| Ok(b"<i/>".to_vec()), &s).unwrap_err();
    assert_eq!(e.code, XML_ERR_USER_STOP);

    // A transcoding pass: an interrupt checked only before it would not stop
    // this ISO-8859-1 input until the parse loop began.
    let latin = [
        b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><r>".as_slice(),
        &vec![0xE9u8; 4 << 20],
        b"</r>",
    ]
    .concat();
    let (f, polls) = fires_on(3);
    let mut s = XmlParseSettings::new();
    s.interrupt = Some(&f);
    let e = xml_read_memory_with(&latin, None, None, default_parse_options(), &s).unwrap_err();
    assert_eq!(e.code, XML_ERR_USER_STOP);
    assert!(polls.get() >= 3);
}

// ---- encodings ----------------------------------------------------------

/// Every byte entry decodes by the declared encoding, and refuses invalid
/// UTF-8 where libxml2 does. HTML is the exception by specification: it
/// guesses (a `<meta>` charset, or windows-1252) instead of refusing.
#[test]
fn gate_encodings() {
    // Document.
    let doc = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><r>caf\xe9</r>";
    let d = xml_read_memory(doc, None, None, 0).unwrap();
    let r = d.xml_doc_get_root_element().unwrap();
    assert_eq!(d.xml_node_get_content(r), "caf\u{e9}");
    // DTD: a text declaration, versioned or not, and its encoding.
    let dtd = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<!ENTITY e \"caf\xe9\">\n";
    let parsed = xml_parse_dtd(dtd, None, None).unwrap();
    assert_eq!(parsed.entities.get("e").map(String::as_str), Some("caf\u{e9}"));
    assert!(xml_parse_dtd(b"<?xml encoding=\"UTF-8\"?>\n<!ENTITY e \"x\">", None, None).is_ok());
    // DTD: invalid UTF-8 with nothing declared is refused, as by libxml2.
    let e = xml_parse_dtd(b"<!ENTITY e \"caf\xe9\">\n", None, None).unwrap_err();
    assert_eq!(e.code, XML_ERR_INVALID_ENCODING);
    // XInclude text, by its encoding attribute; refused when invalid.
    let src = "<r xmlns:xi=\"http://www.w3.org/2001/XInclude\"><xi:include href=\"t\" parse=\"text\" encoding=\"ISO-8859-1\"/></r>";
    let mut d = xml_read_memory(src.as_bytes(), None, None, 0).unwrap();
    xml_xinclude_process_with(&mut d, |_| Ok(b"caf\xe9".to_vec()), &ParseSettings::new()).unwrap();
    let r = d.xml_doc_get_root_element().unwrap();
    assert_eq!(d.xml_node_get_content(r), "caf\u{e9}");
    let src = "<r xmlns:xi=\"http://www.w3.org/2001/XInclude\"><xi:include href=\"t\" parse=\"text\"/></r>";
    let mut d = xml_read_memory(src.as_bytes(), None, None, 0).unwrap();
    let e = xml_xinclude_process_with(&mut d, |_| Ok(b"caf\xe9".to_vec()), &ParseSettings::new())
        .unwrap_err();
    assert_eq!(e.code, XML_ERR_INVALID_ENCODING);
    // HTML: a declared charset is honoured; undeclared bytes are guessed.
    let h = html_read_memory_with(
        b"<meta charset=\"iso-8859-1\"><p>caf\xe9",
        None,
        None,
        0,
        &HtmlParseSettings::new(),
    )
    .unwrap();
    let out = String::from_utf8(rusty_xml::xml_save_doc(&h, 0)).unwrap();
    assert!(out.contains("caf"), "{out}");
}
