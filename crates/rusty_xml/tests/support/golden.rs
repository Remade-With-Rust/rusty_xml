//! Golden-output support: runs a manifest case through the default entry points and renders the
//! result as text. Uses only the public API of rusty_xml as `main` has it, so the `cancel-golden`
//! bin builds this file unchanged in a worktree of `main` to record the expected outputs.

use rusty_xml::*;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Payloads and whole case outputs of at least this many bytes render as length plus FNV-1a hash.
const BIG: usize = 50_000;

/// First line of the html5lib archive.
const ARCHIVE_MAGIC: &str = "rusty_xml html fixtures v1";

pub struct Case {
    pub kind: String,
    pub args: Vec<String>,
}

/// Number of arguments of a manifest kind; the last one is the rest of the line.
fn arity(kind: &str) -> usize {
    match kind {
        "io" | "sax" | "pushlist" | "dtd" | "reader" | "dtdvalid" | "nodedump" | "html"
        | "html5lib" => 1,
        "xml" | "ctxt" | "push" | "xinclude" | "xpath" | "validatedtd" | "rng" | "xsd" | "sch"
        | "save" => 2,
        "c14n" => 3,
        _ => panic!("unknown manifest kind {kind}"),
    }
}

/// Reads `manifest.txt`: `name kind arg1 ... argN` per line, `#` lines and blank lines skipped.
pub fn read_manifest(dir: &Path) -> Vec<(String, Case)> {
    let path = dir.join("manifest.txt");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut cases = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut head = line.splitn(3, ' ');
        let name = head.next().expect("name").to_string();
        let kind = head
            .next()
            .unwrap_or_else(|| panic!("{line}: no kind"))
            .to_string();
        let rest = head.next().unwrap_or("");
        let args: Vec<String> = rest.splitn(arity(&kind), ' ').map(str::to_string).collect();
        assert_eq!(args.len(), arity(&kind), "{line}: wrong argument count");
        cases.push((name, Case { kind, args }));
    }
    cases
}

pub struct Html5libHeader {
    pub oracle_tag: String,
    pub oracle_sha: String,
    pub html5lib_sha: String,
    pub license: Vec<u8>,
}

/// Cursor over the decompressed archive.
struct ArchiveReader<'a> {
    rest: &'a [u8],
}

impl<'a> ArchiveReader<'a> {
    /// The next line, without its newline.
    fn line(&mut self) -> &'a str {
        let end = self
            .rest
            .iter()
            .position(|&b| b == b'\n')
            .expect("archive: unterminated line");
        let (line, rest) = self.rest.split_at(end);
        self.rest = &rest[1..];
        std::str::from_utf8(line).expect("archive: line is not UTF-8")
    }

    /// The next line as a length.
    fn length(&mut self) -> usize {
        self.line()
            .parse()
            .expect("archive: length line is not a number")
    }

    /// The next `len` bytes, with no newline after them.
    fn bytes(&mut self, len: usize) -> &'a [u8] {
        assert!(len <= self.rest.len(), "archive: truncated record");
        let (bytes, rest) = self.rest.split_at(len);
        self.rest = rest;
        bytes
    }
}

/// Decompresses `html5lib-9329e64.zst` and parses it into its header and its
/// cases: name, input, expected output. Layout, after the zstd frame:
///
/// ```text
/// rusty_xml html fixtures v1\n
/// oracle <tag> <sha>\n
/// html5lib-tests <sha>\n
/// license <len>\n<len bytes of html5lib-tests/LICENSE>
/// then per case, sorted by name: <name>\n<input len>\n<input bytes><expected len>\n<expected bytes>
/// ```
pub fn read_html5lib_archive_with_header(
    path: &Path,
) -> (Html5libHeader, Vec<(String, Vec<u8>, Vec<u8>)>) {
    let packed = fs::read(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let bytes = rusty_zstd::decompress(&packed)
        .unwrap_or_else(|e| panic!("{}: cannot decompress: {e}", path.display()));

    let mut r = ArchiveReader { rest: &bytes };
    assert_eq!(
        r.line(),
        ARCHIVE_MAGIC,
        "{}: bad magic line",
        path.display()
    );
    let oracle = r
        .line()
        .strip_prefix("oracle ")
        .expect("archive: second line is not `oracle <tag> <sha>`");
    let (oracle_tag, oracle_sha) = oracle
        .split_once(' ')
        .expect("archive: `oracle` line has no SHA");
    let html5lib_sha = r
        .line()
        .strip_prefix("html5lib-tests ")
        .expect("archive: third line is not `html5lib-tests <sha>`")
        .to_string();
    let license_len = r
        .line()
        .strip_prefix("license ")
        .expect("archive: fourth line is not `license <len>`")
        .parse::<usize>()
        .expect("archive: license length is not a number");
    let license = r.bytes(license_len).to_vec();

    let header = Html5libHeader {
        oracle_tag: oracle_tag.to_string(),
        oracle_sha: oracle_sha.to_string(),
        html5lib_sha,
        license,
    };
    let mut cases = Vec::new();
    while !r.rest.is_empty() {
        let name = r.line().to_string();
        let input_len = r.length();
        let input = r.bytes(input_len).to_vec();
        let expected_len = r.length();
        let expected = r.bytes(expected_len).to_vec();
        cases.push((name, input, expected));
    }
    (header, cases)
}

pub fn read_html5lib_archive(path: &Path) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    read_html5lib_archive_with_header(path).1
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

pub fn payload(bytes: &[u8]) -> String {
    if bytes.len() >= BIG {
        return format!("LEN {} FNV {:016x}", bytes.len(), fnv(bytes));
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            format!("HEX {hex}")
        }
    }
}

pub fn finish(text: String) -> String {
    if text.len() >= BIG {
        format!("LEN {} FNV {:016x}\n", text.len(), fnv(text.as_bytes()))
    } else {
        text
    }
}

pub fn render_err(e: &XmlError) -> String {
    format!("ERR {} {}:{} {}\n", e.code, e.line, e.col, e.message)
}

pub fn render_doc(r: Result<XmlDoc, XmlError>) -> String {
    match r {
        Ok(doc) => format!("OK\n{}\n", payload(&xml_save_doc(&doc, 0))),
        Err(e) => render_err(&e),
    }
}

pub fn render_xpath(r: Result<XPathObject, String>) -> String {
    match r {
        Ok(XPathObject::NodeSet(ids)) => {
            let ids: Vec<String> = ids.iter().map(|n| n.0.to_string()).collect();
            format!("NODES {}\n", ids.join(" "))
        }
        Ok(other) => format!("{other:?}\n"),
        Err(m) => format!("ERR {m}\n"),
    }
}

pub fn render_unit(r: Result<(), String>) -> String {
    match r {
        Ok(()) => "OK\n".to_string(),
        Err(m) => format!("ERR {m}\n"),
    }
}

pub fn render_bytes(r: Result<Vec<u8>, String>) -> String {
    match r {
        Ok(b) => format!("OK\n{}\n", payload(&b)),
        Err(m) => format!("ERR {m}\n"),
    }
}

pub fn render_dtd(r: Result<XmlDtd, XmlError>) -> String {
    let d = match r {
        Ok(d) => d,
        Err(e) => return render_err(&e),
    };
    let sorted_map = |m: &std::collections::HashMap<String, String>| {
        let mut v: Vec<(&String, &String)> = m.iter().collect();
        v.sort();
        format!("{v:?}")
    };
    let sorted_set = |s: &std::collections::HashSet<String>| {
        let mut v: Vec<&String> = s.iter().collect();
        v.sort();
        format!("{v:?}")
    };
    let mut elements: Vec<&String> = d.elements.keys().collect();
    elements.sort();
    let mut attributes: Vec<&(String, String)> = d.attributes.keys().collect();
    attributes.sort();
    format!(
        "DTD name {:?} public {:?} system {:?}\nentities {}\nparameter_entities {}\nunparsed_entities {}\nnotations {}\nelements {:?}\nattributes {:?}\n",
        d.name,
        d.public_id,
        d.system_id,
        sorted_map(&d.entities),
        sorted_map(&d.parameter_entities),
        sorted_set(&d.unparsed_entities),
        sorted_set(&d.notations),
        elements,
        attributes
    )
}

/// One line per chunk result; consecutive equal lines collapse to one line with `xN`.
pub fn render_push(chunks: &[Result<Option<XmlDoc>, XmlError>]) -> String {
    let lines: Vec<String> = chunks
        .iter()
        .map(|r| match r {
            Ok(None) => "OK none\n".to_string(),
            Ok(Some(doc)) => format!("OK doc\n{}\n", payload(&xml_save_doc(doc, 0))),
            Err(e) => render_err(e),
        })
        .collect();
    let mut out = String::new();
    let mut i = 0;
    while i < lines.len() {
        let run = lines[i..].iter().take_while(|l| **l == lines[i]).count();
        out.push_str(&lines[i]);
        if run > 1 {
            out.push_str(&format!("x{run}\n"));
        }
        i += run;
    }
    out
}

pub fn render_reader(seq: &[(i32, String)]) -> String {
    seq.iter().map(|(t, s)| format!("{t} {s}\n")).collect()
}

/// A read callback over `bytes` that hands out at most 1000 bytes per call.
pub fn io_reader(bytes: Vec<u8>) -> impl FnMut(&mut [u8]) -> Result<usize, std::io::Error> {
    let mut cursor = std::io::Cursor::new(bytes);
    move |buf: &mut [u8]| {
        let n = buf.len().min(1000);
        cursor.read(&mut buf[..n])
    }
}

/// The result of a context read plus the context's `last_error` and whether it holds a document.
pub fn render_ctxt(r: Result<XmlDoc, XmlError>, ctxt: &XmlParserCtxt) -> String {
    format!(
        "{}last_error {:?}
doc {}
",
        render_doc(r),
        ctxt.last_error,
        ctxt.doc.is_some()
    )
}

/// The recorder's event count and a hash of its events, then the result.
pub fn render_sax(rec: &SaxRecorder, r: Result<XmlDoc, XmlError>) -> String {
    format!(
        "events {} fp {:016x}
{}",
        rec.events.len(),
        fnv(format!("{:?}", rec.events).as_bytes()),
        render_doc(r)
    )
}

/// The chunks a `push` (fixed size) or `pushlist` (one per line) case feeds; the last terminates.
pub fn push_chunks(dir: &Path, case: &Case) -> Vec<Vec<u8>> {
    let bytes = file(dir, &case.args[0]);
    if case.kind == "push" {
        let size = number(&case.args[1]) as usize;
        return bytes.chunks(size).map(<[u8]>::to_vec).collect();
    }
    let text = String::from_utf8(bytes).expect("utf-8 fixture");
    text.lines().map(|l| l.as_bytes().to_vec()).collect()
}

/// Walks a reader to the end, one line per node.
pub fn render_reader_walk(r: Result<XmlTextReader, XmlError>) -> String {
    let mut reader = match r {
        Ok(reader) => reader,
        Err(e) => return render_err(&e),
    };
    let mut seq = Vec::new();
    while reader.read() == 1 {
        seq.push((
            reader.node_type() as i32,
            format!(
                "depth {} name {:?} value {:?} empty {} attrs {}",
                reader.depth(),
                reader.name(),
                reader.value(),
                reader.is_empty_element(),
                reader.attribute_count()
            ),
        ));
    }
    render_reader(&seq)
}

/// The document of an `xinclude` case and the fragment its loader returns for every href.
pub fn xinclude_inputs(dir: &Path, args: &[String]) -> Result<(XmlDoc, Vec<u8>), String> {
    let doc = parse_fixture(&file(dir, &args[0]))?;
    Ok((doc, file(dir, &args[1])))
}

/// The return value of an XInclude run and the document it edited.
pub fn render_xinclude(r: Result<i32, XmlError>, doc: &XmlDoc) -> String {
    let head = match r {
        Ok(n) => format!(
            "OK {n}
"
        ),
        Err(e) => render_err(&e),
    };
    format!(
        "{head}{}
",
        payload(&xml_save_doc(doc, 0))
    )
}

/// One line per html5lib case: its name, and the length and hash of the rendered result of `parse`.
pub fn render_html5lib(
    dir: &Path,
    archive: &str,
    mut parse: impl FnMut(&[u8]) -> Result<XmlDoc, XmlError>,
) -> String {
    let mut out = String::new();
    for (name, input, _expected) in read_html5lib_archive(&dir.join(archive)) {
        let text = render_doc(parse(&input));
        out.push_str(&format!(
            "{name} LEN {} FNV {:016x}
",
            text.len(),
            fnv(text.as_bytes())
        ));
    }
    out
}

pub fn file(dir: &Path, name: &str) -> Vec<u8> {
    let path = dir.join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub fn number(s: &str) -> i32 {
    s.parse().unwrap_or_else(|_| panic!("not a number: {s}"))
}

/// Parses a fixture with the XML parser at options 0, or renders its error.
pub fn parse_fixture(bytes: &[u8]) -> Result<XmlDoc, String> {
    xml_read_memory(bytes, None, None, 0).map_err(|e| render_err(&e))
}

/// Runs `case` through the existing functions and renders the outcome.
pub fn run_default(dir: &Path, case: &Case) -> String {
    let a = &case.args;
    let text = match case.kind.as_str() {
        "xml" => render_doc(xml_read_memory(
            &file(dir, &a[0]),
            None,
            None,
            number(&a[1]),
        )),
        "ctxt" => {
            let mut ctxt = xml_new_parser_ctxt();
            let r = xml_ctxt_read_memory(&mut ctxt, &file(dir, &a[0]), None, None, number(&a[1]));
            render_ctxt(r, &ctxt)
        }
        "io" => render_doc(xml_read_io(io_reader(file(dir, &a[0])), None, None, 0)),
        "sax" => {
            let mut rec = SaxRecorder::new();
            let r = xml_sax_parse_memory(&file(dir, &a[0]), 0, &mut rec);
            render_sax(&rec, r)
        }
        "push" | "pushlist" => {
            let chunks = push_chunks(dir, case);
            let mut ctxt = xml_create_push_parser_ctxt(&[], None, None, 0);
            let last = chunks.len() - 1;
            let results: Vec<_> = chunks
                .iter()
                .enumerate()
                .map(|(i, c)| xml_parse_chunk(&mut ctxt, c, i32::from(i == last)))
                .collect();
            render_push(&results)
        }
        "dtd" => render_dtd(xml_parse_dtd(&file(dir, &a[0]), None, None)),
        "reader" => render_reader_walk(xml_reader_for_memory(&file(dir, &a[0]), None, None, 0)),
        "xinclude" => match xinclude_inputs(dir, a) {
            Ok((mut doc, fragment)) => {
                let r = xml_xinclude_process(&mut doc, |_href| Ok(fragment.clone()));
                render_xinclude(r, &doc)
            }
            Err(e) => e,
        },
        "xpath" => match parse_fixture(&file(dir, &a[0])) {
            Ok(doc) => {
                let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
                render_xpath(xml_xpath_eval(&a[1], &ctx))
            }
            Err(e) => e,
        },
        "dtdvalid" => match parse_fixture(&file(dir, &a[0])) {
            Ok(doc) => render_unit(xml_validate_document(&doc)),
            Err(e) => e,
        },
        "validatedtd" => match (
            parse_fixture(&file(dir, &a[0])),
            xml_parse_dtd(&file(dir, &a[1]), None, None),
        ) {
            (Ok(doc), Ok(dtd)) => render_unit(xml_validate_dtd(&doc, &dtd)),
            (Err(e), _) => e,
            (_, Err(e)) => render_err(&e),
        },
        "rng" | "xsd" | "sch" => match parse_fixture(&file(dir, &a[1])) {
            Ok(doc) => {
                let schema = file(dir, &a[0]);
                render_unit(match case.kind.as_str() {
                    "rng" => xml_relaxng_validate_doc(&schema, &doc),
                    "xsd" => xml_schema_validate_doc(&schema, &doc),
                    _ => xml_schematron_validate_doc(&schema, &doc),
                })
            }
            Err(e) => e,
        },
        "c14n" => match parse_fixture(&file(dir, &a[0])) {
            Ok(doc) => render_bytes(xml_c14n_doc_dump_memory(&doc, a[1] == "1", a[2] == "1")),
            Err(e) => e,
        },
        "save" => {
            let bytes = file(dir, &a[0]);
            let doc = if a[0].ends_with(".html") {
                html_read_memory(&bytes, None, None, 0).map_err(|e| render_err(&e))
            } else {
                parse_fixture(&bytes)
            };
            match doc {
                Ok(doc) => format!("{}\n", payload(&xml_save_doc(&doc, number(&a[1])))),
                Err(e) => e,
            }
        }
        "nodedump" => match parse_fixture(&file(dir, &a[0])) {
            Ok(doc) => match doc.xml_doc_get_root_element() {
                Some(root) => format!("{}\n", payload(&xml_node_dump(&doc, root, 0))),
                None => "NO ROOT\n".to_string(),
            },
            Err(e) => e,
        },
        "html" => render_doc(html_read_memory(&file(dir, &a[0]), None, None, 0)),
        "html5lib" => render_html5lib(dir, &a[0], |input| html_read_memory(input, None, None, 0)),
        other => panic!("unknown manifest kind {other}"),
    };
    finish(text)
}
