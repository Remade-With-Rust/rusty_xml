//! Writes libxml2 reference output for the HTML fixture cases.
//!
//! Three sets are generated here. `cases/` holds hand-picked snippets (inputs committed).
//! `libxml2/` holds the libxml2 HTML test files, copied from the pinned oracle source tree.
//! Each input `<name>.html` gets `<name>.xmllint.html` beside it: the stdout of
//! `xmllint --html <input>` from the pinned binary, which must report libxml2 21503.
//! `html5lib/` is one zstd archive, `html5lib-9329e64.zst`, of the html5lib-tests
//! tree-construction documents and their expected output. Its layout is documented on
//! `html5lib_archive`.
//! `MANIFEST` records the pin and the counts.
//!
//! Regenerate with `cargo run --release -p rusty_xml-bench --bin html-fixtures`.
//! `--check` builds the same outputs in memory and exits 1 on any difference.

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use ruzstd::encoding::CompressionLevel;

const HTML5LIB_TESTS_SHA: &str = "9329e64694e7835d0dcff9811e22856ef6ad16f9";
const HTML5LIB_TESTS_URL: &str = "https://github.com/html5lib/html5lib-tests";
const HTML5LIB_ARCHIVE: &str = "html5lib-9329e64.zst";
/// First line of the archive, before the `oracle` line.
const ARCHIVE_MAGIC: &str = "rusty_xml html fixtures v1";
const LIBXML_VERSION: &str = "21503";
const REGENERATE: &str = "cargo run --release -p rusty_xml-bench --bin html-fixtures";

struct OraclePin {
    tag: String,
    sha: String,
}

struct Output {
    path: PathBuf,
    /// The file's content. For a `zstd` output, the content before compression.
    bytes: Vec<u8>,
    zstd: bool,
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
                zstd: false,
            }
        })
        .collect()
}

fn manifest(pin: &OraclePin, cases: usize, libxml2: usize, html5lib: usize) -> String {
    format!(
        "oracle: {tag} {sha} (libxml version {LIBXML_VERSION})\n\
         command: xmllint --html <input>\n\
         html5lib-tests: {HTML5LIB_TESTS_SHA}\n\
         count cases: {cases}\n\
         count libxml2: {libxml2}\n\
         count html5lib: {html5lib}\n\
         regenerate: {REGENERATE}\n",
        tag = pin.tag,
        sha = pin.sha,
    )
}

/// The pinned html5lib-tests checkout under `oracle/`, cloned from GitHub when missing.
/// Fails unless its HEAD is the pinned SHA.
fn html5lib_checkout() -> PathBuf {
    let dir = workspace_root().join("oracle").join("html5lib-tests");
    if !dir.exists() {
        let dir_arg = dir.to_string_lossy().into_owned();
        run_git(None, &["clone", HTML5LIB_TESTS_URL, dir_arg.as_str()]);
    }
    run_git(Some(&dir), &["checkout", "--quiet", HTML5LIB_TESTS_SHA]);
    let head = run_git(Some(&dir), &["rev-parse", "HEAD"]);
    assert_eq!(
        head.trim(),
        HTML5LIB_TESTS_SHA,
        "{} is not at the pinned html5lib-tests commit",
        dir.display()
    );
    dir
}

fn run_git(dir: Option<&Path>, args: &[&str]) -> String {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    let out = cmd
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("cannot run git: {e}"));
    if !out.status.success() {
        panic!(
            "git {} failed ({}): {}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The `#data` blocks of one `.dat` file, as `<stem>-<NNN>.html` and the input bytes.
/// Numbering counts every block, so a skipped block keeps its number.
fn split_dat(text: &[u8], stem: &str) -> Vec<(String, Vec<u8>)> {
    const DATA: &[u8] = b"#data\n";
    let starts: Vec<usize> = (0..text.len())
        .filter(|&at| (at == 0 || text[at - 1] == b'\n') && text[at..].starts_with(DATA))
        .collect();
    let mut cases = Vec::new();
    for (index, &start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(text.len());
        let block = &text[start..end];
        let number = index + 1;
        // xmllint has no fragment context, and libxml2 never runs scripts.
        if has_line(block, b"#document-fragment") || has_line(block, b"#script-on") {
            continue;
        }
        let body = &block[DATA.len()..];
        let input: &[u8] = if body.starts_with(b"#errors\n") {
            &[]
        } else {
            let end = find(body, b"\n#errors\n")
                .unwrap_or_else(|| panic!("{stem}-{number:03}: no #errors line"));
            &body[..end]
        };
        cases.push((format!("{stem}-{number:03}.html"), input.to_vec()));
    }
    cases
}

fn has_line(block: &[u8], line: &[u8]) -> bool {
    block.split(|&b| b == b'\n').any(|l| l == line)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The document cases of `tree-construction/*.dat`, sorted by name. Not recursive:
/// `scripted/` repeats the stems `adoption01` and `webkit01`.
fn html5lib_cases(tree_construction: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<PathBuf> = fs::read_dir(tree_construction)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", tree_construction.display()))
        .map(|entry| entry.expect("read_dir entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "dat"))
        .collect();
    files.sort();
    let mut cases = Vec::new();
    for file in files {
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("utf-8 file name")
            .to_string();
        let text =
            fs::read(&file).unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
        cases.extend(split_dat(&text, &stem));
    }
    cases.sort_by(|a, b| a.0.cmp(&b.0));
    cases
}

/// A directory under the system temp dir, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    /// Fails when the path already exists, so a planted link or directory is never reused.
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("rusty_xml-html5lib-{}", std::process::id()));
        fs::create_dir(&path).unwrap_or_else(|e| {
            panic!("cannot create {} (it must not exist): {e}", path.display())
        });
        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Set 3, as one archive of the uncompressed layout below, zstd-compressed on write.
/// Returns the output and the number of cases.
///
/// Layout:
/// ```text
/// rusty_xml html fixtures v1\n
/// oracle <tag> <sha>\n
/// html5lib-tests <sha>\n
/// license <len>\n<len bytes of html5lib-tests/LICENSE>
/// then per case, sorted by name: <name>\n<input len>\n<input bytes><expected len>\n<expected bytes>
/// ```
fn html5lib_archive(xmllint: &Path, pin: &OraclePin) -> (Output, usize) {
    let checkout = html5lib_checkout();
    let license = fs::read(checkout.join("LICENSE"))
        .unwrap_or_else(|e| panic!("cannot read the html5lib-tests LICENSE: {e}"));
    let inputs = html5lib_cases(&checkout.join("tree-construction"));

    let temp = TempDir::new();
    let mut cases = Vec::with_capacity(inputs.len());
    for (name, input) in inputs {
        let path = temp.0.join(&name);
        fs::write(&path, &input).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
        let expected = xmllint_html(xmllint, &path);
        cases.push((name, input, expected));
    }

    let mut stream = format!(
        "{ARCHIVE_MAGIC}\noracle {tag} {sha}\nhtml5lib-tests {HTML5LIB_TESTS_SHA}\nlicense {len}\n",
        tag = pin.tag,
        sha = pin.sha,
        len = license.len(),
    )
    .into_bytes();
    stream.extend_from_slice(&license);
    for (name, input, expected) in &cases {
        stream.extend_from_slice(format!("{name}\n{}\n", input.len()).as_bytes());
        stream.extend_from_slice(input);
        stream.extend_from_slice(format!("{}\n", expected.len()).as_bytes());
        stream.extend_from_slice(expected);
    }
    let output = Output {
        path: fixtures_dir().join(HTML5LIB_ARCHIVE),
        bytes: stream,
        zstd: true,
    };
    (output, cases.len())
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
            outputs.push(Output {
                path,
                bytes,
                zstd: false,
            });
        }
    }
    let (archive, html5lib) = html5lib_archive(xmllint, pin);
    outputs.push(archive);
    outputs.push(Output {
        path: fixtures_dir().join("MANIFEST"),
        bytes: manifest(pin, cases.len(), libxml2.len(), html5lib).into_bytes(),
        zstd: false,
    });
    outputs
}

fn relative(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Decompresses a zstd frame, or `None` when the bytes are not one.
fn unzstd(packed: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    ruzstd::decoding::StreamingDecoder::new(packed)
        .ok()?
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

/// Compares what is on disk with the output. A zstd output is compared decompressed.
fn on_disk_matches(o: &Output) -> bool {
    let Ok(raw) = fs::read(&o.path) else {
        return false;
    };
    let on_disk = if o.zstd {
        match unzstd(&raw) {
            Some(bytes) => bytes,
            None => return false,
        }
    } else {
        raw
    };
    on_disk == o.bytes
}

/// Writes a zstd output through `<path>.tmp`, then renames it into place.
fn write_output(o: &Output) {
    if o.zstd {
        let packed =
            ruzstd::encoding::compress_to_vec(o.bytes.as_slice(), CompressionLevel::Fastest);
        let tmp = PathBuf::from(format!("{}.tmp", o.path.display()));
        fs::write(&tmp, &packed).unwrap_or_else(|e| panic!("cannot write {}: {e}", tmp.display()));
        fs::rename(&tmp, &o.path)
            .unwrap_or_else(|e| panic!("cannot rename {} into place: {e}", tmp.display()));
    } else {
        fs::write(&o.path, &o.bytes)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", o.path.display()));
    }
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
        let stale: Vec<&Output> = outputs.iter().filter(|o| !on_disk_matches(o)).collect();
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
        write_output(o);
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

    fn split(text: &str) -> Vec<(String, Vec<u8>)> {
        split_dat(text.as_bytes(), "t")
    }

    #[test]
    fn split_dat_skips_a_fragment_block() {
        let text = "#data\n<p>a\n#errors\n#document\n| <html>\n\
                    #data\n<p>b\n#errors\n#document-fragment\ndiv\n#document\n| <html>\n";
        assert_eq!(
            split(text),
            vec![("t-001.html".to_string(), b"<p>a".to_vec())]
        );
    }

    #[test]
    fn split_dat_skips_a_script_on_block() {
        let text = "#data\n<p>a\n#errors\n#script-on\n#document\n| <html>\n\
                    #data\n<p>b\n#errors\n#document\n| <html>\n";
        assert_eq!(
            split(text),
            vec![("t-002.html".to_string(), b"<p>b".to_vec())]
        );
    }

    #[test]
    fn split_dat_excludes_the_errors_delimiter_and_keeps_inner_newlines() {
        let text = "#data\n<p>a\n<b>\n#errors\n1:1 expected\n#document\n| <html>\n";
        assert_eq!(
            split(text),
            vec![("t-001.html".to_string(), b"<p>a\n<b>".to_vec())]
        );
    }

    #[test]
    fn split_dat_gives_empty_input_when_the_block_starts_with_errors() {
        let text = "#data\n#errors\n#document\n| <html>\n";
        assert_eq!(split(text), vec![("t-001.html".to_string(), Vec::new())]);
    }

    #[test]
    fn split_dat_numbers_skipped_blocks_so_names_keep_their_position() {
        let text = "#data\n<a>\n#errors\n#document-fragment\ndiv\n#document\n| <html>\n\
                    #data\n<b>\n#errors\n#document\n| <html>\n\
                    #data\n<c>\n#errors\n#document\n| <html>\n";
        assert_eq!(
            split(text),
            vec![
                ("t-002.html".to_string(), b"<b>".to_vec()),
                ("t-003.html".to_string(), b"<c>".to_vec()),
            ]
        );
    }
}
