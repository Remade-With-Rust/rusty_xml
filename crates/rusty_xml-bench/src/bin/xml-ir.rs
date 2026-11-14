//! Deterministic instruction-count harness for the XML parser, XPath and writer.
//!
//! Run under `valgrind --tool=callgrind`. Reads every file named in a list (one path per line)
//! into memory first, then for each rep and file: `xml_read_memory`, one XPath expression (files
//! under 50 KB only) and `xml_save_doc`. File I/O is outside the loop.
//!
//! Prints work-parity anchors and an FNV-1a fingerprint over error codes, every node id the XPath
//! returned, in order, and the saved bytes. A change that claims to preserve output must leave the
//! fingerprint byte-identical.

use rusty_xml::{xml_read_memory, xml_save_doc, xml_xpath_eval, NodeId, XPathObject, XmlDoc, XmlXPathContext};

#[global_allocator]
static ALLOC: rusty_xml_alloc::Allocator = rusty_xml_alloc::NEW;

const XPATH_LIMIT: usize = 50_000;
const XPATH_EXPR: &str = "//*[@*]";

struct Tally {
    nodes: u64,
    xpath: u64,
    out: u64,
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

fn count_nodes(doc: &XmlDoc, id: NodeId) -> u64 {
    let mut n = 0;
    let mut c = doc.first_child(id);
    while let Some(child) = c {
        n += 1 + count_nodes(doc, child);
        c = doc.next_sibling(child);
    }
    n
}

fn main() {
    let mut args = std::env::args().skip(1);
    let list = args.next().expect("usage: xml-ir LIST [REPS]");
    let reps: u32 = args.next().map_or(1, |r| r.parse().expect("reps"));
    let names = std::fs::read_to_string(&list).expect("read list");
    let bufs: Vec<Vec<u8>> = names
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|p| std::fs::read(p.trim()).unwrap_or_else(|e| panic!("{p}: {e}")))
        .collect();
    let bytes: usize = bufs.iter().map(Vec::len).sum();
    let mut t = Tally {
        nodes: 0,
        xpath: 0,
        out: 0,
        fp: 0xcbf2_9ce4_8422_2325,
    };
    for _ in 0..reps {
        for b in &bufs {
            let doc = match xml_read_memory(b, None, None, 0) {
                Ok(doc) => doc,
                Err(e) => {
                    t.mix(&e.code.to_le_bytes());
                    continue;
                }
            };
            t.nodes += count_nodes(&doc, NodeId::DOCUMENT);
            if b.len() < XPATH_LIMIT {
                let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
                match xml_xpath_eval(XPATH_EXPR, &ctx) {
                    Ok(XPathObject::NodeSet(ids)) => {
                        t.xpath += ids.len() as u64;
                        for id in ids {
                            t.mix(&id.0.to_le_bytes());
                        }
                    }
                    Ok(_) => t.mix(b"other"),
                    Err(e) => t.mix(e.as_bytes()),
                }
            }
            let out = xml_save_doc(&doc, 0);
            t.out += out.len() as u64;
            t.mix(&out);
        }
    }
    println!(
        "files {} bytes {} reps {} nodes {} xpath {} out {} fp {:016x}",
        bufs.len(),
        bytes,
        reps,
        t.nodes,
        t.xpath,
        t.out,
        t.fp
    );
}
