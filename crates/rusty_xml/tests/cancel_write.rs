mod support;

use rusty_xml::{
    html_read_memory, xml_c14n_doc_dump_memory_with_interrupt, xml_node_dump_with_interrupt,
    xml_read_memory, xml_save_doc_with_interrupt, Interrupted, XmlDoc,
};
use support::{count_checks, is_interrupted, sweep};

const NAMESPACES: &[u8] = include_bytes!("cancel_fixtures/namespaces.xml");

fn namespaces() -> XmlDoc {
    xml_read_memory(NAMESPACES, None, None, 0).expect("fixture parses")
}

fn stop(_: &Interrupted) -> bool {
    true
}

#[test]
fn save_sweep() {
    let doc = namespaces();
    sweep(|c| xml_save_doc_with_interrupt(&doc, 0, c), stop);
}

#[test]
fn node_dump_sweep() {
    let doc = namespaces();
    let root = doc.xml_doc_get_root_element().expect("root");
    sweep(|c| xml_node_dump_with_interrupt(&doc, root, 0, c), stop);
}

#[test]
fn c14n_sweep() {
    let doc = namespaces();
    sweep(|c| xml_c14n_doc_dump_memory_with_interrupt(&doc, false, false, c), is_interrupted);
    sweep(|c| xml_c14n_doc_dump_memory_with_interrupt(&doc, true, true, c), is_interrupted);
}

#[test]
fn html_save_sweep() {
    let doc = html_read_memory(b"<!DOCTYPE html><p class=a>x<!--c--><br><b>y</b></p>", None, None, 0)
        .expect("html parses");
    sweep(|c| xml_save_doc_with_interrupt(&doc, 0, c), stop);
}

#[test]
fn c14n_many_ns_coverage() {
    let decls: String = (0..60).map(|i| format!(" xmlns:p{i}=\"u{i}\"")).collect();
    let doc = xml_read_memory(format!("<r{decls}><c/></r>").as_bytes(), None, None, 0).unwrap();
    let checks = count_checks(|c| {
        let _ = xml_c14n_doc_dump_memory_with_interrupt(&doc, false, false, c);
    });
    assert!(checks >= 180, "checks {checks}");
}
