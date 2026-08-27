//! Work-count helper for the bench harness. Never links libxml2.

fn main() {
    eprintln!(
        "rusty_xml-bench: use bench/pinvs.ps1 to time pinned oracle/bin/xmllint against rxmlint."
    );
    eprintln!("This binary is a marker so the workspace member compiles.");
}
