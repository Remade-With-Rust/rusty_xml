//! Differential and self-consistency fuzzer for rusty_xml.
//!
//! Not a coverage-guided fuzzer -- a structured generator plus a mutator, run
//! against invariants that must hold for EVERY input, valid or not:
//!
//!   1. Nothing panics or aborts. Ever. On any byte sequence.
//!   2. Streaming a document in chunks equals parsing it whole -- at every
//!      chunk size, because the bugs live exactly at the boundaries.
//!   3. Save is idempotent: parse -> save -> parse -> save is a fixed point.
//!      A round trip that keeps changing the document is losing or inventing
//!      something.
//!   4. Canonicalization succeeds or errors; it never dies.
//!
//! Deterministic: seeded xorshift, no clock, no thread_rng. A failure is
//! reproducible from its seed alone -- `fuzz <iters> <seed>` replays it.
//!
//! Every defect this has found so far was in the boundary handling: multi-byte
//! characters split across chunks, CRLF split across chunks, a BOM shifting
//! every offset, non-UTF-8 documents handed raw to a UTF-8 parser.

use rusty_xml::{
    default_parse_options, html_read_memory, xml_create_push_parser_ctxt, xml_parse_chunk,
    xml_read_memory, xml_save_doc,
};

/// xorshift64*, so a seed reproduces a failure exactly.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

/// Fragments chosen because each one has broken something at least once:
/// entity expansion, CDATA, namespaces, comments, PIs, DTD defaults,
/// astral-plane characters, CRLF, and the `]]>` / `--` sequences that the
/// incomplete-construct detector has to reason about.
const FRAGMENTS: &[&str] = &[
    "<a>", "</a>", "<b x='1'>", "</b>", "<c/>", "<n:e xmlns:n='u'>", "</n:e>",
    "text", " ", "\n", "\r\n", "\r", "\t",
    "<!-- c -->", "<!--", "-->", "--", "<?pi d?>", "<?pi", "?>",
    "<![CDATA[x]]>", "<![CDATA[", "]]>", "]]", "]",
    "&amp;", "&#65;", "&#x41;", "&undeclared;", "&", ";", "&#;", "&#x;",
    "caf\u{e9}", "\u{65e5}\u{672c}", "\u{1F600}", "\u{0}", "\u{FFFF}",
    "<!DOCTYPE d [<!ENTITY e 'v'><!ATTLIST a k CDATA 'dv'>]>",
    "&e;", "<?xml version='1.0'?>", "<?xml version='1.0' encoding='UTF-8'?>",
    "xmlns='u'", "=", "'", "\"", "<", ">", "/", "<a", "</", "<!", "<!D",
];

/// Text that is legal in content but has to survive escaping and re-parsing.
const TEXTS: &[&str] = &[
    "t", "a b", " ", "\n", "\r\n", "\r", "\t\t", "&amp;", "&lt;&gt;", "&#65;",
    "]]", "] ]>", "--", "caf\u{e9}", "\u{65e5}\u{672c}\u{8a9e}", "\u{1F600}\u{1F1EC}",
    "&#x1F600;", "  leading", "trailing  ", "&quot;&apos;",
];
const NAMES: &[&str] = &["a", "b", "c", "el", "n:q", "_x", "x-y", "x.y", "\u{e9}l"];
const ATTVALS: &[&str] = &["1", "", " ", "a b", "&amp;", "<", "\"", "'", "\n", "\u{1F600}"];

/// Build a WELL-FORMED document. Garbage finds crashes; only well-formed input
/// exercises streaming equality and round-trip idempotence, which is where the
/// subtle defects have actually been.
fn well_formed(r: &mut Rng, out: &mut String, depth: u32) {
    let name = *r.pick(NAMES);
    out.push('<');
    out.push_str(name);
    if name.starts_with("n:") {
        out.push_str(" xmlns:n='urn:u'");
    }
    for _ in 0..r.below(3) {
        out.push(' ');
        out.push_str(r.pick(&["k", "k2", "n:k", "xml:lang"]));
        if out.ends_with("n:k") {
            // A prefixed attribute needs its prefix in scope.
            out.truncate(out.len() - 3);
            out.push_str("k3");
        }
        out.push_str("='");
        let v = r.pick(ATTVALS);
        // Only these three are illegal raw inside a single-quoted value.
        for ch in v.chars() {
            match ch {
                '<' => out.push_str("&lt;"),
                '&' => out.push_str("&amp;"),
                '\'' => out.push_str("&apos;"),
                _ => out.push(ch),
            }
        }
        out.push('\'');
    }
    if depth >= 4 || r.below(4) == 0 {
        out.push_str("/>");
        return;
    }
    out.push('>');
    for _ in 0..r.below(4) {
        match r.below(6) {
            0 => out.push_str("<!-- c -->"),
            1 => out.push_str("<?pi d?>"),
            2 => out.push_str("<![CDATA[x]]>"),
            3 | 4 => out.push_str(r.pick(TEXTS)),
            _ => well_formed(r, out, depth + 1),
        }
    }
    out.push_str("</");
    out.push_str(name);
    out.push('>');
}

fn generate(r: &mut Rng) -> Vec<u8> {
    // Three inputs in four are well-formed, then lightly mutated; the fourth is
    // the fragment soup, which is what finds the crashes.
    if r.below(4) != 0 {
        let mut s = String::new();
        if r.below(3) == 0 {
            s.push_str("<?xml version='1.0' encoding='UTF-8'?>");
        }
        if r.below(4) == 0 {
            s.push_str("<!DOCTYPE a [<!ENTITY e 'v'><!ATTLIST a k CDATA 'dv'>]>");
        }
        well_formed(r, &mut s, 0);
        let mut b = s.into_bytes();
        // Mutate only sometimes -- an unmutated well-formed document is the
        // only input for which the streaming and idempotence checks are strict.
        if r.below(3) == 0 && !b.is_empty() {
            let i = r.below(b.len());
            b[i] = (r.next() & 0xFF) as u8;
        }
        return b;
    }
    generate_soup(r)
}

fn generate_soup(r: &mut Rng) -> Vec<u8> {
    let n = 1 + r.below(24);
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(r.pick(FRAGMENTS));
    }
    let mut b = s.into_bytes();
    // Bit-level mutation on top of the structured document: the generator
    // alone never produces invalid UTF-8, and invalid UTF-8 is where a parser
    // written in Rust either allocates a lossy copy or slices mid-character.
    for _ in 0..r.below(4) {
        if b.is_empty() {
            break;
        }
        let i = r.below(b.len());
        match r.below(3) {
            0 => b[i] = (r.next() & 0xFF) as u8,
            1 => b.insert(i, (r.next() & 0xFF) as u8),
            _ => {
                b.remove(i);
            }
        }
    }
    b
}

fn save(d: &[u8], opts: i32, save_opts: i32) -> Option<Vec<u8>> {
    xml_read_memory(d, None, None, opts).ok().map(|x| xml_save_doc(&x, save_opts))
}

/// Feed the document `chunk` bytes at a time and serialize the result.
fn stream(d: &[u8], chunk: usize, opts: i32) -> Option<Vec<u8>> {
    let mut c = xml_create_push_parser_ctxt(&[], None, None, opts);
    let mut i = 0;
    while i < d.len() {
        let n = chunk.min(d.len() - i);
        xml_parse_chunk(&mut c, &d[i..i + n], 0).ok()?;
        i += n;
    }
    xml_parse_chunk(&mut c, &[], 1).ok().flatten().map(|x| xml_save_doc(&x, 0))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let iters: u64 = a.get(1).and_then(|x| x.parse().ok()).unwrap_or(50_000);
    let seed: u64 = a.get(2).and_then(|x| x.parse().ok()).unwrap_or(0x9E37_79B9_7F4A_7C15);
    let mut r = Rng(seed);

    let opts = default_parse_options();
    let recover = opts | rusty_xml::XML_PARSE_RECOVER;
    let (mut parsed, mut stream_bad, mut idem_bad, mut html_bad) = (0u64, 0u64, 0u64, 0u64);

    for i in 0..iters {
        let d = generate(&mut r);
        // Invariant 1: no panic on any path, valid input or not.
        let whole = save(&d, opts, 0);
        let _ = save(&d, recover, 0);
        let _ = save(&d, opts, rusty_xml::XML_SAVE_FORMAT);
        // HTML has no well-formedness to fail, so it always produces a tree
        // -- which makes the round trip the only check with any teeth on it.
        // The nesting fix is exactly the kind of change that can make one
        // unstable, since a misparented node moves again on every pass.
        if let Ok(h1) = html_read_memory(&d, None, None, 0) {
            let once = xml_save_doc(&h1, 0);
            if let Ok(h2) = html_read_memory(&once, None, None, 0) {
                let twice = xml_save_doc(&h2, 0);
                if once != twice {
                    html_bad += 1;
                    if html_bad <= 3 {
                        eprintln!(
                            "HTML NOT IDEMPOTENT seed={seed} iter={i}
  in:  {:?}
  1st: {:?}
  2nd: {:?}",
                            String::from_utf8_lossy(&d),
                            String::from_utf8_lossy(&once),
                            String::from_utf8_lossy(&twice),
                        );
                    }
                }
            }
        }

        if let Ok(doc) = xml_read_memory(&d, None, None, opts) {
            parsed += 1;
            // Invariant 4: c14n reports, never dies.
            let _ = rusty_xml::xml_c14n_doc_dump_memory(&doc, false, true);
            let _ = rusty_xml::xml_c14n_doc_dump_memory(&doc, true, true);

            // Invariant 3: saving is a fixed point.
            let once = xml_save_doc(&doc, 0);
            if let Some(twice) = save(&once, opts, 0) {
                if once != twice {
                    idem_bad += 1;
                    if idem_bad <= 3 {
                        eprintln!(
                            "NOT IDEMPOTENT seed={seed} iter={i}\n  in:  {:?}\n  1st: {:?}\n  2nd: {:?}",
                            String::from_utf8_lossy(&d),
                            String::from_utf8_lossy(&once),
                            String::from_utf8_lossy(&twice),
                        );
                    }
                }
            }
        }

        // Invariant 2: chunked == whole, at every boundary.
        for chunk in [1usize, 2, 3, 7, 64] {
            if stream(&d, chunk, opts) != whole {
                stream_bad += 1;
                if stream_bad <= 3 {
                    eprintln!(
                        "STREAM DIVERGES seed={seed} iter={i} chunk={chunk}\n  in: {:?}",
                        String::from_utf8_lossy(&d)
                    );
                }
                break;
            }
        }
    }

    println!(
        "{iters} inputs, seed {seed}: {parsed} parsed clean, \
         {stream_bad} stream divergences, {idem_bad} non-idempotent round trips, \n         {html_bad} non-idempotent HTML round trips"
    );
    if stream_bad != 0 || idem_bad != 0 || html_bad != 0 {
        std::process::exit(1);
    }
}
