//! Writes libxml2 reference output for the HTML fixture cases.
//!
//! Two sets are generated here. `cases/` holds hand-picked snippets (inputs committed).
//! `libxml2/` holds the libxml2 HTML test files, copied from the pinned oracle source tree.
//! Each input `<name>.html` gets `<name>.xmllint.html` beside it: the stdout of
//! `xmllint --html <input>` from the pinned binary, which must report libxml2 21503.
//! `MANIFEST` records the pin and the counts.
//!
//! Regenerate with `cargo run --release -p rusty_xml-bench --bin html-fixtures`.
//! `--check` builds the same outputs in memory and exits 1 on any difference.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const HTML5LIB_TESTS_SHA: &str = "9329e64694e7835d0dcff9811e22856ef6ad16f9";
const LIBXML_VERSION: &str = "21503";
const REGENERATE: &str = "cargo run --release -p rusty_xml-bench --bin html-fixtures";

struct OraclePin {
    tag: String,
    sha: String,
}

struct Output {
    path: PathBuf,
    bytes: Vec<u8>,
}

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

fn fixtures_dir() -> PathBuf {
    workspace_root().join("crates/rusty_xml/tests/html_fixtures")
}

fn oracle() -> PathBuf {
    let exe = if cfg!(windows) {
        "xmllint.exe"
    } else {
        "xmllint"
    };
    let default = workspace_root().join("oracle").join("bin").join(exe);
    match std::env::var_os("RUSTY_XML_ORACLE") {
        Some(path) => {
            let path = PathBuf::from(path);
            assert!(
                path.is_file(),
                "RUSTY_XML_ORACLE={} is not a file (other lookup: {})",
                path.display(),
                default.display()
            );
            path
        }
        None => {
            assert!(
                default.is_file(),
                "no xmllint: set RUSTY_XML_ORACLE, or build {} (lookups: RUSTY_XML_ORACLE, then {})",
                default.display(),
                default.display()
            );
            default
        }
    }
}

/// Exits 1 unless `xmllint --version` names the pinned libxml2. libxml2 prints the version to stderr.
fn require_pin(xmllint: &Path) {
    let out = Command::new(xmllint)
        .arg("--version")
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", xmllint.display()));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let mark = format!("using libxml version {LIBXML_VERSION}");
    if !text.contains(&mark) {
        eprintln!(
            "html-fixtures: {} is not the pinned oracle (expected `{mark}`); nothing written",
            xmllint.display()
        );
        std::process::exit(1);
    }
}

fn read_pin() -> OraclePin {
    let path = workspace_root().join("oracle/PIN");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let field = |key: &str| -> String {
        text.lines()
            .find_map(|line| line.trim_start().strip_prefix(key))
            .map(|value| value.trim().to_string())
            .unwrap_or_else(|| panic!("{} has no `{key}` line", path.display()))
    };
    OraclePin {
        tag: field("tag:"),
        sha: field("git SHA:"),
    }
}

fn xmllint_html(xmllint: &Path, input: &Path) -> Vec<u8> {
    let out = Command::new(xmllint)
        .arg("--html")
        .arg(input)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", xmllint.display()));
    if !out.status.success() {
        panic!(
            "xmllint --html failed on {} ({}): {}",
            input.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out.stdout
}

fn is_input(name: &str) -> bool {
    (name.ends_with(".html") || name.ends_with(".htm")) && !name.ends_with(".xmllint.html")
}

fn expected_path(input: &Path) -> PathBuf {
    input.with_extension("xmllint.html")
}

/// Sorted by path, so every run visits the inputs in the same order.
fn inputs(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("read_dir entry").path())
        .filter(|p| p.is_file() && p.file_name().and_then(|n| n.to_str()).is_some_and(is_input))
        .collect();
    found.sort();
    found
}

/// The libxml2 test files and the copyright notice, read from the pinned source tree.
fn libxml2_copies() -> Vec<Output> {
    let src_dir = workspace_root().join("oracle/src/test/HTML");
    let dst_dir = fixtures_dir().join("libxml2");
    let mut sources: Vec<PathBuf> = fs::read_dir(&src_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} (fetch the oracle source first): {e}",
                src_dir.display()
            )
        })
        .map(|entry| entry.expect("read_dir entry").path())
        .filter(|p| p.is_file())
        .collect();
    sources.push(workspace_root().join("oracle/src/Copyright"));
    sources.sort();
    sources
        .into_iter()
        .map(|src| {
            let name = src.file_name().expect("source has a file name").to_owned();
            let bytes =
                fs::read(&src).unwrap_or_else(|e| panic!("cannot read {}: {e}", src.display()));
            Output {
                path: dst_dir.join(name),
                bytes,
            }
        })
        .collect()
}

fn manifest(pin: &OraclePin, cases: usize, libxml2: usize) -> String {
    format!(
        "oracle: {tag} {sha} (libxml version {LIBXML_VERSION})\n\
         command: xmllint --html <input>\n\
         html5lib-tests: {HTML5LIB_TESTS_SHA}\n\
         count cases: {cases}\n\
         count libxml2: {libxml2}\n\
         regenerate: {REGENERATE}\n",
        tag = pin.tag,
        sha = pin.sha,
    )
}

/// Every file this run produces. With `write_copies`, the libxml2 copies land on disk first,
/// because xmllint reads its inputs from disk.
fn build(xmllint: &Path, pin: &OraclePin, write_copies: bool) -> Vec<Output> {
    let copies = libxml2_copies();
    if write_copies {
        fs::create_dir_all(fixtures_dir().join("libxml2")).expect("create libxml2 dir");
        for copy in &copies {
            fs::write(&copy.path, &copy.bytes)
                .unwrap_or_else(|e| panic!("cannot write {}: {e}", copy.path.display()));
        }
    }
    let cases = inputs(&fixtures_dir().join("cases"));
    let libxml2 = inputs(&fixtures_dir().join("libxml2"));

    let mut outputs = copies;
    for dir_inputs in [&cases, &libxml2] {
        let mut expected_paths = BTreeSet::new();
        for input in dir_inputs {
            let path = expected_path(input);
            assert!(
                expected_paths.insert(path.clone()),
                "collision: two inputs map to {}",
                path.display()
            );
            let bytes = xmllint_html(xmllint, input);
            outputs.push(Output { path, bytes });
        }
    }
    outputs.push(Output {
        path: fixtures_dir().join("MANIFEST"),
        bytes: manifest(pin, cases.len(), libxml2.len()).into_bytes(),
    });
    outputs
}

fn relative(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn main() -> ExitCode {
    let check = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => {
            eprintln!("usage: html-fixtures [--check]");
            return ExitCode::from(2);
        }
    };

    let xmllint = oracle();
    require_pin(&xmllint);
    let pin = read_pin();
    let outputs = build(&xmllint, &pin, !check);

    if check {
        let stale: Vec<&Output> = outputs
            .iter()
            .filter(|o| fs::read(&o.path).ok().as_deref() != Some(o.bytes.as_slice()))
            .collect();
        for o in &stale {
            eprintln!("differs: {}", relative(&o.path));
        }
        if !stale.is_empty() {
            eprintln!(
                "html-fixtures --check: {} file(s) differ; run `{REGENERATE}`",
                stale.len()
            );
            return ExitCode::from(1);
        }
        println!("html-fixtures --check: {} files match", outputs.len());
        return ExitCode::SUCCESS;
    }

    for o in &outputs {
        fs::write(&o.path, &o.bytes)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", o.path.display()));
    }
    println!("html-fixtures: wrote {} files", outputs.len());
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_path_replaces_the_last_extension() {
        assert_eq!(
            expected_path(Path::new("cases/p-closed-by-li.html")),
            PathBuf::from("cases/p-closed-by-li.xmllint.html")
        );
    }

    #[test]
    fn expected_path_for_htm_input_keeps_the_html_suffix() {
        assert_eq!(
            expected_path(Path::new("libxml2/doc3.htm")),
            PathBuf::from("libxml2/doc3.xmllint.html")
        );
    }

    #[test]
    fn input_filter_takes_html_and_htm_but_not_expected_files() {
        assert!(is_input("doc3.htm"));
        assert!(is_input("p-closed-by-li.html"));
        assert!(!is_input("p-closed-by-li.xmllint.html"));
        assert!(!is_input("Copyright"));
    }
}
