//! Work-count helper for the bench harness. Never links libxml2.

// Same allocator the shipped CLI uses, so a bench number reflects the deploy.
#[global_allocator]
static ALLOC: rusty_xml_alloc::Allocator = rusty_xml_alloc::NEW;

fn main() {
    eprintln!(
        "rusty_xml-bench: use bench/pinvs.ps1 to time pinned oracle/bin/xmllint against rxmlint."
    );
    eprintln!("This binary is a marker so the workspace member compiles.");
}
