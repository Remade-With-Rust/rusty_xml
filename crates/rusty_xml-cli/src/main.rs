//! rxmlint — xmllint-shaped CLI. Never installed as `xmllint`.

#![forbid(unsafe_code)]

// The allocator is a property of the deliverable, never of the library. rxmlint
// ships rusty_alloc; the pin lives in the rusty_xml-alloc seam.
#[global_allocator]
static ALLOC: rusty_xml_alloc::Allocator = rusty_xml_alloc::NEW;

use rusty_xml::{
    default_parse_options, html_read_memory, xml_c14n_doc_dump_memory, xml_create_push_parser_ctxt,
    xml_parse_chunk, xml_read_memory, xml_reader_for_memory, xml_relaxng_validate_doc,
    xml_sax_parse_memory, xml_save_doc, xml_schema_validate_doc, xml_schematron_validate_doc,
    xml_validate_document, xml_xpath_eval, SaxRecorder, XmlXPathContext, XML_SAVE_FORMAT,
    XML_SAVE_NO_DECL, XML_SAVE_NO_EMPTY,
};
use std::env;
use std::io::{self, Read, Write};
use std::process;

fn usage() -> ! {
    eprintln!(
        "Usage: rxmlint [--noout] [--recover] [--sax] [--stream] [--format] [--xpath EXPR] [--c14n] [--exc-c14n] [--html] [--push] [--dtdvalid] [--relaxng FILE] [--schema FILE] [--schematron FILE] [--repeat] [file|-]\n  --repeat : first sets 100 inner parses, each extra --repeat multiplies by 10 (xmllint; rxmlint-repeat-flag-v1)"
    );
    process::exit(1);
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut noout = false;
    let mut sax = false;
    let mut stream = false;
    let mut format = false;
    let mut no_decl = false;
    let mut no_empty = false;
    let mut bench_counts = false;
    let mut html = false;
    let mut push = false;
    let mut recover = false;
    let mut c14n = false;
    let mut exc_c14n = false;
    let mut dtdvalid = false;
    let mut xpath: Option<String> = None;
    let mut relaxng: Option<String> = None;
    let mut schema: Option<String> = None;
    let mut schematron: Option<String> = None;
    let mut files: Vec<String> = Vec::new();
    // xmllint: first --repeat → 100, each extra ×10. 1 means "flag absent".
    let mut repeat: u32 = 1;

    while !args.is_empty() {
        let a = args.remove(0);
        match a.as_str() {
            "--noout" => noout = true,
            "--sax" | "--sax1" => sax = true,
            "--stream" => stream = true,
            "--format" => format = true,
            "--no-decl" => no_decl = true,
            "--no-empty" => no_empty = true,
            "--bench-counts" => bench_counts = true,
            "--html" => html = true,
            "--push" => push = true,
            "--c14n" => c14n = true,
            "--exc-c14n" => exc_c14n = true,
            "--dtdvalid" => dtdvalid = true,
            "--xpath" => {
                xpath = Some(args.first().cloned().unwrap_or_default());
                if !args.is_empty() {
                    args.remove(0);
                }
            }
            "--relaxng" => {
                relaxng = Some(args.first().cloned().unwrap_or_default());
                if !args.is_empty() {
                    args.remove(0);
                }
            }
            "--schema" => {
                schema = Some(args.first().cloned().unwrap_or_default());
                if !args.is_empty() {
                    args.remove(0);
                }
            }
            "--schematron" => {
                schematron = Some(args.first().cloned().unwrap_or_default());
                if !args.is_empty() {
                    args.remove(0);
                }
            }
            "--recover" => recover = true,
            "--repeat" => {
                if repeat > 1 {
                    repeat = repeat.saturating_mul(10);
                } else {
                    repeat = 100;
                }
            }
            "--help" | "-h" => usage(),
            "--" => {
                files.extend(args.drain(..));
            }
            s if s.starts_with('-') && s != "-" => {
                eprintln!("rxmlint: unknown option {s}");
                usage();
            }
            s => files.push(s.to_string()),
        }
    }
    if files.is_empty() {
        files.push("-".into());
    }

    let options = default_parse_options()
        | if recover {
            rusty_xml::XML_PARSE_RECOVER
        } else {
            0
        };
    let mut save_opts = 0;
    if format {
        save_opts |= XML_SAVE_FORMAT;
    }
    if no_decl {
        save_opts |= XML_SAVE_NO_DECL;
    }
    if no_empty {
        save_opts |= XML_SAVE_NO_EMPTY;
    }

    let mut status = 0;
    for f in files {
        let (bytes, url) = if f == "-" {
            let mut b = Vec::new();
            io::stdin().read_to_end(&mut b).unwrap();
            (b, None)
        } else {
            match std::fs::read(&f) {
                Ok(b) => (b, Some(f.as_str())),
                Err(e) => {
                    eprintln!("rxmlint: {f}: {e}");
                    status = 2;
                    continue;
                }
            }
        };

        for iter in 0..repeat {
            let emit = iter + 1 == repeat;

            if html {
                match html_read_memory(&bytes, url, None, options) {
                    Ok(doc) => {
                        if emit && !noout {
                            let out = xml_save_doc(&doc, save_opts);
                            let _ = io::stdout().write_all(&out);
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        status = 4;
                        break;
                    }
                }
                continue;
            }

            if stream {
                match xml_reader_for_memory(&bytes, url, None, options) {
                    Ok(mut r) => {
                        let mut ticks = 0u64;
                        while r.read() == 1 {
                            ticks += 1;
                        }
                        if emit && bench_counts {
                            eprintln!("bytes={} reader_ticks={}", bytes.len(), ticks);
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        status = 4;
                        break;
                    }
                }
                continue;
            }

            if sax {
                let mut rec = SaxRecorder::new();
                match xml_sax_parse_memory(&bytes, options, &mut rec) {
                    Ok(_) => {
                        if emit && bench_counts {
                            eprintln!("bytes={} sax_events={}", bytes.len(), rec.events.len());
                        }
                        if emit && !noout {
                            let dump = rec.to_xmllint_debug(&bytes);
                            let _ = io::stdout().write_all(dump.as_bytes());
                        }
                    }
                    Err(e) => {
                        if emit && !noout {
                            let dump = rec.to_xmllint_debug(&bytes);
                            let _ = io::stdout().write_all(dump.as_bytes());
                        }
                        eprintln!("{e}");
                        status = 4;
                        break;
                    }
                }
                continue;
            }

            let parsed = if push {
                let mut ctxt = xml_create_push_parser_ctxt(&[], url, None, options);
                const N: usize = 4;
                let mut i = 0;
                let mut last = Ok(None);
                while i < bytes.len() {
                    let end = (i + N).min(bytes.len());
                    let term = if end == bytes.len() { 1 } else { 0 };
                    last = xml_parse_chunk(&mut ctxt, &bytes[i..end], term);
                    i = end;
                }
                match last {
                    Ok(Some(d)) => Ok(d),
                    Ok(None) => xml_parse_chunk(&mut ctxt, &[], 1).map(|o| o.unwrap()),
                    Err(e) => Err(e),
                }
            } else {
                xml_read_memory(&bytes, url, None, options)
            };

            match parsed {
                Ok(mut doc) => {
                    if emit && bench_counts {
                        let elems = (0..doc.len())
                            .filter(|&i| {
                                doc.kind(rusty_xml::NodeId(i as u32))
                                    == rusty_xml::NodeKind::Element
                            })
                            .count();
                        eprintln!("bytes={} elements={}", bytes.len(), elems);
                    }
                    if let Some(expr) = &xpath {
                        let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
                        match xml_xpath_eval(expr, &ctx) {
                            Ok(obj) => {
                                if !noout {
                                    if let Some(s) = rusty_xml::xml_xpath_print_lint(&obj) {
                                        let _ = io::stdout().write_all(s.as_bytes());
                                    } else if let rusty_xml::XPathObject::NodeSet(v) = obj {
                                        for id in v {
                                            let b = rusty_xml::xml_node_dump(
                                                &doc,
                                                id,
                                                rusty_xml::XML_SAVE_NO_DECL,
                                            );
                                            let _ = io::stdout().write_all(&b);
                                            let _ = io::stdout().write_all(b"\n");
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                eprintln!("XPath compilation failure");
                                status = 10;
                            }
                        }
                        continue;
                    }
                    if c14n || exc_c14n {
                        match xml_c14n_doc_dump_memory(&doc, exc_c14n, true) {
                            Ok(b) => {
                                if !noout {
                                    let _ = io::stdout().write_all(&b);
                                }
                            }
                            Err(e) => {
                                eprintln!("{e}");
                                status = 4;
                            }
                        }
                        continue;
                    }
                    if dtdvalid {
                        if let Err(e) = xml_validate_document(&doc) {
                            eprintln!("{e}");
                            status = 3;
                        }
                    }
                    if let Some(p) = &relaxng {
                        match std::fs::read(p) {
                            Ok(rng) => {
                                if let Err(e) = xml_relaxng_validate_doc(&rng, &doc) {
                                    eprintln!("{e}");
                                    status = 3;
                                }
                            }
                            Err(e) => {
                                eprintln!("{e}");
                                status = 2;
                            }
                        }
                    }
                    if let Some(p) = &schema {
                        match std::fs::read(p) {
                            Ok(xsd) => {
                                if let Err(e) = xml_schema_validate_doc(&xsd, &doc) {
                                    eprintln!("{e}");
                                    status = 3;
                                }
                            }
                            Err(e) => {
                                eprintln!("{e}");
                                status = 2;
                            }
                        }
                    }
                    if let Some(p) = &schematron {
                        match std::fs::read(p) {
                            Ok(sch) => {
                                if let Err(e) = xml_schematron_validate_doc(&sch, &doc) {
                                    eprintln!("{e}");
                                    status = 3;
                                }
                            }
                            Err(e) => {
                                eprintln!("{e}");
                                status = 2;
                            }
                        }
                    }
                    if emit && !noout {
                        let out = xml_save_doc(&doc, save_opts);
                        let _ = io::stdout().write_all(&out);
                    }
                    let _ = &mut doc;
                }
                Err(e) => {
                    eprintln!("{e}");
                    status = 4;
                    break;
                }
            }
        }
    }
    process::exit(status);
}
