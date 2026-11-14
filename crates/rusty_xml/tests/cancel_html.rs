mod support;

use rusty_xml::{
    html_read_memory, html_read_memory_with_interrupt, xml_save_doc, InterruptCheck,
};
use std::fs;
use std::path::{Path, PathBuf};
use support::golden::{read_html5lib_archive, render_doc};
use support::{count_checks, is_user_stop, never, sweep};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/html_fixtures")
}

fn parse(input: &[u8], c: &mut InterruptCheck<'_>) -> Result<(), rusty_xml::XmlError> {
    html_read_memory_with_interrupt(input, None, None, 0, c).map(|_| ())
}

fn inputs() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for set in ["cases", "libxml2"] {
        let mut paths: Vec<PathBuf> = fs::read_dir(fixtures().join(set))
            .expect("fixture dir")
            .map(|e| e.expect("entry").path())
            .filter(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                (name.ends_with(".html") || name.ends_with(".htm")) && !name.ends_with(".xmllint.html")
            })
            .collect();
        paths.sort();
        for p in paths {
            out.push((format!("{set}/{}", p.display()), fs::read(&p).expect("read input")));
        }
    }
    for (name, input, _) in read_html5lib_archive(&fixtures().join("html5lib-9329e64.zst")) {
        out.push((format!("html5lib/{name}"), input));
    }
    out
}

#[test]
fn html_variant_matches_default() {
    let flag = never();
    let mut differing = Vec::new();
    let all = inputs();
    for (name, input) in &all {
        let default = html_read_memory(input, None, None, 0);
        let mut check = InterruptCheck::with_interval(&flag, std::num::NonZeroU32::MIN);
        let variant = html_read_memory_with_interrupt(input, None, None, 0, &mut check);
        let same_bytes = match (&default, &variant) {
            (Ok(a), Ok(b)) => xml_save_doc(a, 0) == xml_save_doc(b, 0),
            (Err(a), Err(b)) => a == b,
            _ => false,
        };
        if !same_bytes {
            differing.push(format!("{name}: {}", render_doc(default).lines().next().unwrap_or("")));
        }
    }
    assert!(differing.is_empty(), "{} of {} differ: {differing:?}", differing.len(), all.len());
}

#[test]
fn html_sweep() {
    sweep(
        |c| parse(b"<!DOCTYPE html><p class=a>x<!--c--><b>y</b></p>", c),
        is_user_stop,
    );
}

#[test]
fn html_token_coverage() {
    // doctype, p, "x", comment, /p, br, "y", i, "z", /i, hr, hr: 12 source tokens.
    let doc = b"<!DOCTYPE html><p class=a>x<!--c--></p><br>y<i>z</i><hr><hr>";
    let checks = count_checks(|c| {
        let _ = parse(doc, c);
    });
    assert!(checks >= 14, "checks {checks}");
}

#[test]
fn html_declared_encoding_sweep() {
    sweep(
        |c| parse(b"<meta charset=\"iso-8859-1\"><p>caf\xe9</p>", c),
        is_user_stop,
    );
}
