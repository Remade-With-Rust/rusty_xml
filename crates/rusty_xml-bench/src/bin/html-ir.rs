//! Deterministic instruction-count harness for the HTML parser.
//!
//! Run under `valgrind --tool=callgrind`. Reads every file named in a list
//! (one path per line) into memory first, then parses each `reps` times with
//! `html_read_memory` and walks the tree. File I/O is outside the parse loop.
//!
//! Prints work-parity anchors (element/text/attribute counts, text bytes) and
//! an FNV-1a fingerprint of the tree in document order. The fingerprint moves
//! if and only if the tree moves, so a probe that claims to be output
//! preserving must leave it byte-identical.

use rusty_xml::{NodeId, NodeKind, XmlDoc};

#[global_allocator]
static ALLOC: rusty_xml_alloc::Allocator = rusty_xml_alloc::NEW;

#[derive(Default)]
struct Tally {
    elems: u64,
    texts: u64,
    tbytes: u64,
    attrs: u64,
    comments: u64,
    fp: u64,
}

impl Tally {
    fn mix(&mut self, b: &[u8]) {
        let mut h = self.fp;
        for &x in b {
            h ^= u64::from(x);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.fp = h;
    }
}

fn walk(doc: &XmlDoc, id: NodeId, t: &mut Tally) {
    let mut c = doc.first_child(id);
    while let Some(n) = c {
        match doc.kind(n) {
            NodeKind::Element => {
                t.elems += 1;
                t.mix(b"<");
                t.mix(doc.name(n).as_bytes());
                let mut a = doc.first_attr(n);
                while let Some(at) = a {
                    t.attrs += 1;
                    t.mix(b" ");
                    t.mix(doc.name(at).as_bytes());
                    t.mix(b"=");
                    t.mix(doc.node(at).content.as_bytes());
                    a = doc.next_sibling(at);
                }
                t.mix(b">");
                walk(doc, n, t);
                t.mix(b"</>");
            }
            NodeKind::Text | NodeKind::CData => {
                t.texts += 1;
                let s = &doc.node(n).content;
                t.tbytes += s.len() as u64;
                t.mix(if doc.kind(n) == NodeKind::Text {
                    b"#"
                } else {
                    b"["
                });
                t.mix(s.as_bytes());
            }
            NodeKind::Comment => {
                t.comments += 1;
                t.mix(b"!");
                t.mix(doc.node(n).content.as_bytes());
            }
            _ => walk(doc, n, t),
        }
        c = doc.next_sibling(n);
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let list = args.next().expect("usage: html-ir LIST [REPS]");
    let reps: u32 = args.next().map_or(1, |r| r.parse().expect("reps"));
    let names = std::fs::read_to_string(&list).expect("read list");
    let bufs: Vec<Vec<u8>> = names
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|p| std::fs::read(p.trim()).unwrap_or_else(|e| panic!("{p}: {e}")))
        .collect();
    let bytes: usize = bufs.iter().map(Vec::len).sum();
    let mut t = Tally {
        fp: 0xcbf2_9ce4_8422_2325,
        ..Tally::default()
    };
    for _ in 0..reps {
        for b in &bufs {
            let doc = rusty_xml::html_read_memory(b, None, None, 0).expect("parse");
            walk(&doc, NodeId::DOCUMENT, &mut t);
        }
    }
    println!(
        "files {} bytes {} reps {} elems {} texts {} tbytes {} attrs {} comments {} fp {:016x}",
        bufs.len(),
        bytes,
        reps,
        t.elems,
        t.texts,
        t.tbytes,
        t.attrs,
        t.comments,
        t.fp
    );
}
