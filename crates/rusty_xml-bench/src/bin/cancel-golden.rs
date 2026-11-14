//! Writes `golden/<name>.out` for every case of a manifest, through the existing entry points.
//!
//! Run it in a checkout of `main` to record the expected outputs of the cancellation work:
//! `cancel-golden crates/rusty_xml/tests/cancel_fixtures`.

use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "../../../rusty_xml/tests/support/golden.rs"]
mod golden;

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("usage: cancel-golden DIR"));
    let out = dir.join("golden");
    fs::create_dir_all(&out).expect("create golden dir");
    let cases = golden::read_manifest(&dir);
    for (name, case) in &cases {
        let text = golden::run_default(&dir, case);
        fs::write(out.join(format!("{name}.out")), text).expect("write golden file");
    }
    println!("wrote {} golden files", cases.len());
}
