mod support;

use std::fs;
use std::path::{Path, PathBuf};
use support::golden::{read_manifest, run_default, Case};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cancel_fixtures")
}

fn first_difference(got: &str, want: &str) -> String {
    for (i, (g, w)) in got.lines().zip(want.lines()).enumerate() {
        if g != w {
            return format!("line {}: got {:?}, want {:?}", i + 1, g, w);
        }
    }
    format!(
        "line counts differ: got {}, want {}",
        got.lines().count(),
        want.lines().count()
    )
}

fn check_all(run: impl Fn(&Path, &Case) -> String) {
    let dir = fixtures();
    let mut failures = Vec::new();
    let cases = read_manifest(&dir);
    for (name, case) in &cases {
        let path = dir.join("golden").join(format!("{name}.out"));
        let want = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let got = run(&dir, case);
        if got != want {
            failures.push(format!("{name}: {}", first_difference(&got, &want)));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ from the golden output:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn golden_default() {
    check_all(run_default);
}

#[test]
fn golden_interrupt() {
    check_all(support::run_interrupt);
}
