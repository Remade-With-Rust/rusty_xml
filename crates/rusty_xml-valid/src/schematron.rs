//! ISO Schematron: `rule/@context` + `assert/@test` / `report/@test` via XPath 1.0.

use crate::interrupt::{NoPoll, Poll};
use crate::ValidError;
use rusty_xml_parser::{default_parse_options, xml_read_memory};
use rusty_xml_tree::{Interrupt, InterruptPoll, NodeId, NodeKind, XmlDoc};
use rusty_xml_xpath::{xml_xpath_cast_to_boolean, xml_xpath_eval, XmlXPathContext};

const SCH: &[&str] = &[
    "http://purl.oclc.org/dsdl/schematron",
    "http://www.ascc.net/xml/schematron",
];

/// `xmlSchematronValidateDoc`.
#[doc(alias = "xmlSchematronValidateDoc")]
pub fn xml_schematron_validate_doc(sch: &[u8], doc: &XmlDoc) -> Result<(), String> {
    schematron_validate(sch, doc, &mut NoPoll).map_err(ValidError::into_message)
}

/// [`xml_schematron_validate_doc`], stopping with [`ValidError::Interrupted`]
/// once `interrupt` fires. It is polled before and after the schema is
/// parsed, and every few hundred XPath evaluations. A single evaluation runs
/// to completion.
pub fn xml_schematron_validate_doc_interruptible(
    sch: &[u8],
    doc: &XmlDoc,
    interrupt: &dyn Interrupt,
) -> Result<(), ValidError> {
    schematron_validate(sch, doc, &mut InterruptPoll::new(interrupt))
}

fn schematron_validate<P: Poll>(sch: &[u8], doc: &XmlDoc, poll: &mut P) -> Result<(), ValidError> {
    poll.check()?;
    let sdoc = xml_read_memory(sch, None, None, default_parse_options()).map_err(|e| e.to_string())?;
    poll.check()?;
    let mut errors = Vec::new();
    walk_schema(&sdoc, sdoc.xml_doc_get_root_element(), doc, &mut errors, poll)?;
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; ").into())
    }
}

fn is_sch(doc: &XmlDoc, id: NodeId, name: &str) -> bool {
    doc.kind(id) == NodeKind::Element
        && doc.name(id) == name
        && (doc.ns_uri(id).map(|u| SCH.contains(&u)).unwrap_or(true))
}

/// Run every rule, visiting the schema document in document order. It used to
/// recurse once per level of the schema, so a deeply nested schema could
/// overflow the stack.
fn walk_schema<P: Poll>(
    sdoc: &XmlDoc,
    node: Option<NodeId>,
    doc: &XmlDoc,
    errors: &mut Vec<String>,
    poll: &mut P,
) -> Result<(), ValidError> {
    let Some(top) = node else { return Ok(()) };
    let mut stack = vec![top];
    while let Some(id) = stack.pop() {
        run_rule(sdoc, id, doc, errors, poll)?;
        let mut c = sdoc.last_child(id);
        while let Some(x) = c {
            stack.push(x);
            c = sdoc.prev_sibling(x);
        }
    }
    Ok(())
}

fn run_rule<P: Poll>(
    sdoc: &XmlDoc,
    id: NodeId,
    doc: &XmlDoc,
    errors: &mut Vec<String>,
    poll: &mut P,
) -> Result<(), ValidError> {
    if is_sch(sdoc, id, "rule") {
        if let Some(ctx) = sdoc.xml_get_prop(id, "context") {
            poll.tick()?;
            let nodes = eval_nodeset(doc, &ctx);
            let mut c = sdoc.first_child(id);
            while let Some(x) = c {
                if is_sch(sdoc, x, "assert") {
                    if let Some(test) = sdoc.xml_get_prop(x, "test") {
                        for n in &nodes {
                            poll.tick()?;
                            if !eval_bool(doc, *n, &test) {
                                errors.push(sdoc.xml_node_get_content(x));
                            }
                        }
                    }
                } else if is_sch(sdoc, x, "report") {
                    if let Some(test) = sdoc.xml_get_prop(x, "test") {
                        for n in &nodes {
                            poll.tick()?;
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
    Ok(())
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
