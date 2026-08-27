//! ISO Schematron: `rule/@context` + `assert/@test` / `report/@test` via XPath 1.0.

use rusty_xml_parser::{default_parse_options, xml_read_memory};
use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};
use rusty_xml_xpath::{xml_xpath_cast_to_boolean, xml_xpath_eval, XmlXPathContext};

const SCH: &[&str] = &[
    "http://purl.oclc.org/dsdl/schematron",
    "http://www.ascc.net/xml/schematron",
];

/// `xmlSchematronValidateDoc`.
#[doc(alias = "xmlSchematronValidateDoc")]
pub fn xml_schematron_validate_doc(sch: &[u8], doc: &XmlDoc) -> Result<(), String> {
    let sdoc = xml_read_memory(sch, None, None, default_parse_options()).map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    walk_schema(&sdoc, sdoc.xml_doc_get_root_element(), doc, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn is_sch(doc: &XmlDoc, id: NodeId, name: &str) -> bool {
    doc.kind(id) == NodeKind::Element
        && doc.name(id) == name
        && (doc.ns_uri(id).map(|u| SCH.contains(&u)).unwrap_or(true))
}

fn walk_schema(sdoc: &XmlDoc, node: Option<NodeId>, doc: &XmlDoc, errors: &mut Vec<String>) {
    let Some(id) = node else { return };
    if is_sch(sdoc, id, "rule") {
        if let Some(ctx) = sdoc.xml_get_prop(id, "context") {
            let nodes = eval_nodeset(doc, &ctx);
            let mut c = sdoc.first_child(id);
            while let Some(x) = c {
                if is_sch(sdoc, x, "assert") {
                    if let Some(test) = sdoc.xml_get_prop(x, "test") {
                        for n in &nodes {
                            if !eval_bool(doc, *n, &test) {
                                errors.push(sdoc.xml_node_get_content(x));
                            }
                        }
                    }
                } else if is_sch(sdoc, x, "report") {
                    if let Some(test) = sdoc.xml_get_prop(x, "test") {
                        for n in &nodes {
                            if eval_bool(doc, *n, &test) {
                                errors.push(sdoc.xml_node_get_content(x));
                            }
                        }
                    }
                }
                c = sdoc.next_sibling(x);
            }
        }
    }
    let mut c = sdoc.first_child(id);
    while let Some(x) = c {
        walk_schema(sdoc, Some(x), doc, errors);
        c = sdoc.next_sibling(x);
    }
}

fn eval_nodeset(doc: &XmlDoc, expr: &str) -> Vec<NodeId> {
    let ctx = XmlXPathContext::xml_xpath_new_context(doc);
    match xml_xpath_eval(expr, &ctx) {
        Ok(rusty_xml_xpath::XPathObject::NodeSet(v)) => v,
        _ => vec![],
    }
}

fn eval_bool(doc: &XmlDoc, node: NodeId, expr: &str) -> bool {
    let mut ctx = XmlXPathContext::xml_xpath_new_context(doc);
    ctx.xml_xpath_set_context_node(node);
    match xml_xpath_eval(expr, &ctx) {
        Ok(o) => xml_xpath_cast_to_boolean(&o),
        Err(_) => false,
    }
}
