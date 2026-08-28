//! W3C XML Conformance Test Suite runner.
//!
//! 2593 cases built specifically to find the well-formedness and validity gaps
//! that a hand-written corpus never will. Fetch the suite with
//! `scripts/fetch-xmlconf.ps1`; it is never vendored.
//!
//! A pass rate alone is not evidence, because libxml2 does not score 100%
//! either -- some cases are ambiguous, some need external entity loading, some
//! test editions of the spec nobody implements. So every case is run through
//! the pinned C oracle as well, on the same bytes, and what matters is the
//! DIFFERENCE. `--oracle` turns that on (one xmllint process per case, so it
//! is the slow path).
//!
//! Test semantics, from the suite's own testcases.dtd:
//!   not-wf   the parser must reject it
//!   valid    the parser must accept it, and validation must succeed
//!   invalid  the parser must accept it, and validation must FAIL
//!   error    optional: a processor may or may not report it, so not scored

use rusty_xml::{default_parse_options, xml_read_memory};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

struct Case {
    id: String,
    ty: String,
    uri: PathBuf,
    version: String,
    entities: String,
    /// Which editions of XML 1.0 the case applies to. A case not marked for
    /// the 5th is testing the OLD character rules, and libxml2's own runner
    /// parses those with XML_PARSE_OLD10 rather than scoring a 5th-edition
    /// parser against 4th-edition expectations. Not doing that counted 313
    /// inapplicable cases as failures -- for us AND for the oracle.
    edition: String,
}

/// Collect every TEST element in a catalog, including nested TESTCASES.
fn collect(catalog: &Path, out: &mut Vec<Case>) {
    let Ok(bytes) = std::fs::read(catalog) else {
        return;
    };
    let base = catalog.parent().unwrap_or(Path::new(".")).to_path_buf();
    // The catalogs are plain XML; parsing them with our own parser is the
    // first test the suite gets to run.
    //
    // Except the sun/ ones, which have several TEST elements at the top level
    // and no root: they are external entities of the master xmlconf.xml and are
    // not well-formed alone. libxml2 rejects them standalone too. Wrap them.
    let doc = match xml_read_memory(&bytes, None, None, default_parse_options()) {
        Ok(d) => d,
        Err(_) => {
            let text = String::from_utf8_lossy(&bytes);
            let body = match text.find("?>") {
                Some(i) => &text[i + 2..],
                None => &text[..],
            };
            let wrapped = format!("<TESTCASES>{body}</TESTCASES>");
            match xml_read_memory(wrapped.as_bytes(), None, None, default_parse_options()) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("catalog did not parse: {} ({e})", catalog.display());
                    return;
                }
            }
        }
    };
    let mut stack = vec![rusty_xml::NodeId::DOCUMENT];
    while let Some(id) = stack.pop() {
        let mut c = doc.first_child(id);
        while let Some(x) = c {
            if doc.kind(x) == rusty_xml::NodeKind::Element {
                if doc.name(x) == "TEST" {
                    let attr = |n: &str| -> String {
                        doc.attrs(x)
                            .into_iter()
                            .find(|a| doc.name(*a) == n)
                            .map(|a| doc.content(a).to_string())
                            .unwrap_or_default()
                    };
                    let uri = attr("URI");
                    if !uri.is_empty() {
                        let mut p = base.clone();
                        for seg in uri.split('/') {
                            p.push(seg);
                        }
                        out.push(Case {
                            id: attr("ID"),
                            ty: attr("TYPE"),
                            uri: p,
                            version: attr("VERSION"),
                            entities: attr("ENTITIES"),
                            edition: attr("EDITION"),
                        });
                    }
                }
                stack.push(x);
            }
            c = doc.next_sibling(x);
        }
    }
}

#[derive(Default)]
struct Tally {
    pass: u32,
    fail: u32,
}

impl Tally {
    fn add(&mut self, ok: bool) {
        if ok {
            self.pass += 1
        } else {
            self.fail += 1
        }
    }
    fn n(&self) -> u32 {
        self.pass + self.fail
    }
    fn rate(&self) -> f64 {
        if self.n() == 0 {
            100.0
        } else {
            100.0 * self.pass as f64 / self.n() as f64
        }
    }
}

/// Our verdict on one case: (parsed, validated).
fn ours(bytes: &[u8], want_validation: bool, old10: bool) -> (bool, bool) {
    let mut opts = default_parse_options();
    if want_validation {
        opts |= rusty_xml::XML_PARSE_DTDVALID | rusty_xml::XML_PARSE_DTDLOAD;
    }
    if old10 {
        opts |= rusty_xml::XML_PARSE_OLD10;
    }
    match xml_read_memory(bytes, None, None, opts) {
        Err(_) => (false, false),
        Ok(doc) => (true, rusty_xml::xml_validate_document(&doc).is_ok()),
    }
}

/// libxml2's verdict on the same file, so the score has something to mean.
fn oracle(path: &Path, want_validation: bool, old10: bool) -> (bool, bool) {
    let run = |extra: &[&str]| -> bool {
        let mut base: Vec<&str> = vec!["--noout"];
        if old10 {
            base.push("--oldxml10");
        }
        std::process::Command::new("oracle/bin/xmllint.exe")
            .args(base)
            .args(extra)
            .arg(path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    if !run(&[]) {
        return (false, false);
    }
    (true, if want_validation { run(&["--valid"]) } else { true })
}

fn expected(ty: &str, parsed: bool, valid: bool) -> bool {
    match ty {
        "not-wf" => !parsed,
        "valid" => parsed && valid,
        "invalid" => parsed && !valid,
        _ => true,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_oracle = args.iter().any(|a| a == "--oracle");
    let verbose = args.iter().any(|a| a == "--list-failures");
    let trace = args.iter().any(|a| a == "--trace");
    let root = Path::new("oracle/xmlconf/xmlconf");
    if !root.exists() {
        eprintln!(
            "suite not found at {}; run scripts/fetch-xmlconf.ps1",
            root.display()
        );
        std::process::exit(2);
    }

    let catalogs = [
        "xmltest/xmltest.xml",
        "sun/sun-valid.xml",
        "sun/sun-invalid.xml",
        "sun/sun-not-wf.xml",
        "sun/sun-error.xml",
        "oasis/oasis.xml",
        "ibm/ibm_oasis_valid.xml",
        "ibm/ibm_oasis_invalid.xml",
        "ibm/ibm_oasis_not-wf.xml",
        "ibm/xml-1.1/ibm_valid.xml",
        "ibm/xml-1.1/ibm_invalid.xml",
        "ibm/xml-1.1/ibm_not-wf.xml",
        "japanese/japanese.xml",
        "eduni/errata-2e/errata2e.xml",
        "eduni/errata-3e/errata3e.xml",
        "eduni/errata-4e/errata4e.xml",
        "eduni/namespaces/1.0/rmt-ns10.xml",
        "eduni/namespaces/1.1/rmt-ns11.xml",
        "eduni/namespaces/errata-1e/errata1e.xml",
        "eduni/xml-1.1/xml11.xml",
        "eduni/misc/ht-bh.xml",
    ];
    let mut cases = Vec::new();
    for c in catalogs {
        collect(&root.join(c), &mut cases);
    }

    let (mut skip_11, mut skip_err, mut skip_ext, mut missing) = (0u32, 0u32, 0u32, 0u32);
    let mut mine: BTreeMap<String, Tally> = BTreeMap::new();
    let mut theirs: BTreeMap<String, Tally> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();
    let mut disagree: Vec<String> = Vec::new();

    for c in &cases {
        // XML 1.1 is a different language and we do not implement it. Counting
        // those as failures would flatter nobody.
        if c.version == "1.1" {
            skip_11 += 1;
            continue;
        }
        // "error" cases are optional by the suite's own definition.
        if c.ty == "error" {
            skip_err += 1;
            continue;
        }
        // Cases that turn on loading external entities test a feature we do
        // not have; reported separately rather than buried in the score.
        if matches!(c.entities.as_str(), "both" | "general" | "parameter") {
            skip_ext += 1;
            continue;
        }
        let Ok(bytes) = std::fs::read(&c.uri) else {
            missing += 1;
            continue;
        };
        if trace {
            eprintln!("{} {}", c.id, c.uri.display());
        }
        let validating = c.ty == "valid" || c.ty == "invalid";
        // Exactly what libxml2's runxmlconf.c does at the same point.
        let old10 = !c.edition.is_empty() && !c.edition.contains('5');
        let (p, v) = ours(&bytes, validating, old10);
        let ok = expected(&c.ty, p, v);
        mine.entry(c.ty.clone()).or_default().add(ok);
        if !ok {
            failures.push(format!(
                "  {:<30} {:<8} parsed={} valid={}  {}",
                c.id,
                c.ty,
                if p { "Y" } else { "N" },
                if v { "Y" } else { "N" },
                c.uri.display()
            ));
        }
        if use_oracle {
            let (cp, cv) = oracle(&c.uri, validating, old10);
            let c_ok = expected(&c.ty, cp, cv);
            theirs.entry(c.ty.clone()).or_default().add(c_ok);
            if ok != c_ok {
                disagree.push(format!(
                    "  {:<30} {:<8} we {:<4} C {}",
                    c.id,
                    c.ty,
                    if ok { "pass" } else { "FAIL" },
                    if c_ok { "pass" } else { "FAIL" }
                ));
            }
        }
    }

    println!("W3C XML Conformance Test Suite");
    println!("  {} cases in the catalogs", cases.len());
    println!(
        "  skipped: {skip_11} XML 1.1, {skip_err} optional-error, \
         {skip_ext} external-entity, {missing} missing file"
    );
    println!();
    if use_oracle {
        println!("  {:<10}{:>7}   {:<18}{}", "type", "n", "rusty_xml", "libxml2");
    } else {
        println!("  {:<10}{:>7}   {}", "type", "n", "rusty_xml");
    }
    let mut tot = Tally::default();
    let mut ctot = Tally::default();
    for (ty, t) in &mine {
        tot.pass += t.pass;
        tot.fail += t.fail;
        let s = format!("{}/{} {:.1}%", t.pass, t.n(), t.rate());
        match theirs.get(ty) {
            Some(c) => {
                ctot.pass += c.pass;
                ctot.fail += c.fail;
                println!(
                    "  {:<10}{:>7}   {:<18}{}/{} {:.1}%",
                    ty,
                    t.n(),
                    s,
                    c.pass,
                    c.n(),
                    c.rate()
                );
            }
            None => println!("  {:<10}{:>7}   {}", ty, t.n(), s),
        }
    }
    println!();
    if use_oracle {
        println!(
            "  {:<10}{:>7}   {:<18}{}/{} {:.1}%",
            "TOTAL",
            tot.n(),
            format!("{}/{} {:.1}%", tot.pass, tot.n(), tot.rate()),
            ctot.pass,
            ctot.n(),
            ctot.rate()
        );
        println!("\n  {} cases where we and C disagree", disagree.len());
        for d in disagree.iter().take(4000) {
            println!("{d}");
        }
        if disagree.len() > 4000 {
            println!("  ... and {} more", disagree.len() - 60);
        }
    } else {
        println!(
            "  {:<10}{:>7}   {}/{} {:.1}%",
            "TOTAL",
            tot.n(),
            tot.pass,
            tot.n(),
            tot.rate()
        );
    }
    if verbose && !failures.is_empty() {
        println!("\nfailures ({}):", failures.len());
        for f in failures.iter().take(4000) {
            println!("{f}");
        }
        if failures.len() > 4000 {
            println!("  ... and {} more", failures.len() - 500);
        }
    }
}
