//! Issue #16: HTML output checked against libxml2 2.15.3 on every test run.
//!
//! Each input is parsed with `html_read_memory(.., 0)` and saved with
//! `xml_save_doc(.., 0)`, then compared byte for byte with the expected file
//! beside it. Two sets are committed here:
//!
//! - `cases/`: hand-picked snippets. `<name>.html` is the input and
//!   `<name>.xmllint.html` the output of `xmllint --html`.
//! - `libxml2/`: the libxml2 HTML test files, copied from the pinned oracle
//!   source tree, with their expected output in the same layout.
//!
//! The expected files come from the pinned oracle (`oracle/PIN`, libxml2
//! 21503). Regenerate them with
//! `cargo run --release -p rusty_xml-bench --bin html-fixtures`.
//!
//! A mismatch fails the test unless `known-differences.txt` names it with a
//! reason key. A listed entry that starts to match also fails, so the list
//! stays current.

use std::collections::BTreeMap;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use rusty_xml::{html_read_memory, xml_save_doc};

/// Sets a `known-differences.txt` key may name. The html5lib set is checked
/// by the html5lib test added with its fixtures.
const SETS: [&str; 3] = ["cases", "libxml2", "html5lib"];

/// Reason keys, defined in the header of `known-differences.txt`.
const REASON_KEYS: [&str; 4] = [
    "writer-nonascii-refs",
    "writer-nonascii-attr-name",
    "writer-attr-escaping",
    "parser-input-encoding",
];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/html_fixtures")
}

/// Parses `input` as HTML and serializes it. A panic in either step is
/// returned as `Err` with the panic message.
fn ours(input: &[u8]) -> Result<Vec<u8>, String> {
    let run = || -> Result<Vec<u8>, String> {
        let doc = html_read_memory(input, None, None, 0).map_err(|e| format!("{e:?}"))?;
        Ok(xml_save_doc(&doc, 0))
    };
    panic::catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|payload| {
        let text = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".to_string());
        Err(format!("panic: {text}"))
    })
}

/// Parses `known-differences.txt`: `<set>/<file name>` TAB reason key per line.
fn known_differences() -> BTreeMap<String, &'static str> {
    let path = fixtures_dir().join("known-differences.txt");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut map = BTreeMap::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, reason) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("known-differences line has no tab: {line:?}"));
        let reason = *REASON_KEYS
            .iter()
            .find(|k| **k == reason)
            .unwrap_or_else(|| panic!("unknown reason key {reason:?} for {key}"));
        assert!(
            map.insert(key.to_string(), reason).is_none(),
            "duplicate known-differences entry {key}"
        );
    }
    map
}

fn is_input(name: &str) -> bool {
    (name.ends_with(".html") || name.ends_with(".htm")) && !name.ends_with(".xmllint.html")
}

/// Loads every input of `set` in file-name order, with its expected output.
fn load_set(set: &str) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let dir = fixtures_dir().join(set);
    let mut inputs: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("read_dir entry").path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(is_input))
        .collect();
    inputs.sort();
    inputs
        .into_iter()
        .map(|input| {
            let name = input
                .file_name()
                .and_then(|n| n.to_str())
                .expect("utf-8 file name")
                .to_string();
            let expected_path = input.with_extension("xmllint.html");
            let read = |p: &Path| {
                fs::read(p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
            };
            let input_bytes = read(&input);
            let expected = read(&expected_path);
            (name, input_bytes, expected)
        })
        .collect()
}

/// A window of both outputs around their first differing byte, for the failure message.
fn describe(got: &Result<Vec<u8>, String>, expected: &[u8]) -> String {
    let got = match got {
        Ok(bytes) => bytes.as_slice(),
        Err(message) => return format!("      {message}\n"),
    };
    let at = got
        .iter()
        .zip(expected)
        .position(|(a, b)| a != b)
        .unwrap_or(got.len().min(expected.len()));
    let window = |bytes: &[u8]| {
        let start = at.saturating_sub(40);
        let end = (at + 60).min(bytes.len());
        String::from_utf8_lossy(&bytes[start..end]).into_owned()
    };
    format!(
        "      at byte {at}\n        ours:     {:?}\n        expected: {:?}\n",
        window(got),
        window(expected)
    )
}

/// Compares every case of `set` with its expected output. Fails once, listing
/// the mismatches that are not known differences, the known differences that
/// now match, and the known differences whose input is not in `cases`.
fn check_set(set: &str, cases: &[(String, Vec<u8>, Vec<u8>)]) {
    assert!(!cases.is_empty(), "{set}: no cases found");
    let known = known_differences();
    let mut mismatches = String::new();
    let mut now_matching = Vec::new();
    let mut names = Vec::new();
    for (name, input, expected) in cases {
        let key = format!("{set}/{name}");
        names.push(key.clone());
        let got = ours(input);
        let matches = got.as_deref() == Ok(expected.as_slice());
        match (matches, known.contains_key(&key)) {
            (false, false) => {
                mismatches.push_str(&format!("    {key}\n{}", describe(&got, expected)));
            }
            (true, true) => now_matching.push(key),
            _ => {}
        }
    }
    let prefix = format!("{set}/");
    let missing: Vec<&String> = known
        .keys()
        .filter(|key| key.starts_with(&prefix) && !names.contains(key))
        .collect();
    if mismatches.is_empty() && now_matching.is_empty() && missing.is_empty() {
        return;
    }
    panic!(
        "{set}: HTML output differs from libxml2\n\
         mismatches not in known-differences.txt:\n{mismatches}\
         known differences that now match (remove them):\n    {now_matching:?}\n\
         known differences with no input in {set}/:\n    {missing:?}\n"
    );
}

#[test]
fn cases_match_libxml2() {
    check_set("cases", &load_set("cases"));
}

#[test]
fn libxml2_test_suite_matches_libxml2() {
    check_set("libxml2", &load_set("libxml2"));
}

#[test]
fn every_known_difference_names_a_case() {
    for (key, _) in known_differences() {
        let (set, file) = key
            .split_once('/')
            .unwrap_or_else(|| panic!("known-differences key has no set: {key}"));
        assert!(SETS.contains(&set), "{key}: unknown set {set}");
        // html5lib names are checked when that set is added.
        if set == "html5lib" {
            continue;
        }
        let input = fixtures_dir().join(set).join(file);
        assert!(input.is_file(), "{key}: no input file {}", input.display());
    }
}
