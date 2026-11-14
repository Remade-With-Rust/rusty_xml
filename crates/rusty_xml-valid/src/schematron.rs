//! ISO Schematron: `rule/@context` + `assert/@test` / `report/@test` via XPath 1.0.

use crate::checker::{other, plain, settle, Checker, NoCheck};
use rusty_xml_tree::{InterruptCheck, InterruptError, NodeId, NodeKind, XmlDoc};
use rusty_xml_xpath::{xml_xpath_cast_to_boolean, XPathObject, XmlXPathContext};

const SCH: &[&str] = &[
    "http://purl.oclc.org/dsdl/schematron",
    "http://www.ascc.net/xml/schematron",
];

/// `xmlSchematronValidateDoc`.
#[doc(alias = "xmlSchematronValidateDoc")]
pub fn xml_schematron_validate_doc(sch: &[u8], doc: &XmlDoc) -> Result<(), String> {
    plain(schematron_inner(sch, doc, &mut NoCheck))
}

/// Like [`xml_schematron_validate_doc`], but returns `InterruptError::Interrupted` once `check`
/// reports an interrupt, also inside the rules' XPath and while the schema is being parsed; a stop
/// is never a verdict.
pub fn xml_schematron_validate_doc_with_interrupt(
    sch: &[u8],
    doc: &XmlDoc,
    check: &mut InterruptCheck<'_>,
) -> Result<(), InterruptError<String>> {
    schematron_inner(sch, doc, check)
}

fn schematron_inner<C: Checker>(
    sch: &[u8],
    doc: &XmlDoc,
    check: &mut C,
) -> Result<(), InterruptError<String>> {
    let sdoc = check.read_schema(sch)?;
    let mut errors = Vec::new();
    let walked = walk_schema(&sdoc, sdoc.xml_doc_get_root_element(), doc, &mut errors, check);
    settle(walked, check)?;
    if errors.is_empty() {
        Ok(())
    } else {
        Err(other(errors.join("; ")))
    }
}

fn is_sch(doc: &XmlDoc, id: NodeId, name: &str) -> bool {
    doc.kind(id) == NodeKind::Element
        && doc.name(id) == name
        && (doc.ns_uri(id).map(|u| SCH.contains(&u)).unwrap_or(true))
}

fn walk_schema(
    sdoc: &XmlDoc,
    node: Option<NodeId>,
    doc: &XmlDoc,
    errors: &mut Vec<String>,
    check: &mut impl Checker,
) -> Result<(), InterruptError<String>> {
    let Some(id) = node else { return Ok(()) };
    check.tick()?;
    if is_sch(sdoc, id, "rule") {
        if let Some(ctx) = sdoc.xml_get_prop(id, "context") {
            let nodes = eval_nodeset(doc, &ctx, check)?;
            let mut c = sdoc.first_child(id);
            while let Some(x) = c {
                if is_sch(sdoc, x, "assert") {
                    if let Some(test) = sdoc.xml_get_prop(x, "test") {
                        for n in &nodes {
                            if !eval_bool(doc, *n, &test, check)? {
                                errors.push(sdoc.xml_node_get_content(x));
                            }
                        }
                    }
                } else if is_sch(sdoc, x, "report") {
                    if let Some(test) = sdoc.xml_get_prop(x, "test") {
                        for n in &nodes {
                            if eval_bool(doc, *n, &test, check)? {
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
        walk_schema(sdoc, Some(x), doc, errors, check)?;
        c = sdoc.next_sibling(x);
    }
    Ok(())
}

/// The nodes an expression selects; an expression that fails to evaluate selects none, but a stop
/// is not such a failure.
fn eval_nodeset(
    doc: &XmlDoc,
    expr: &str,
    check: &mut impl Checker,
) -> Result<Vec<NodeId>, InterruptError<String>> {
    let ctx = XmlXPathContext::xml_xpath_new_context(doc);
    match check.xpath(expr, &ctx) {
        Ok(XPathObject::NodeSet(v)) => Ok(v),
        Ok(_) | Err(InterruptError::Other(_)) => Ok(vec![]),
        Err(InterruptError::Interrupted) => Err(InterruptError::Interrupted),
    }
}

/// An expression that fails to evaluate is false, but a stop is not such a failure.
fn eval_bool(
    doc: &XmlDoc,
    node: NodeId,
    expr: &str,
    check: &mut impl Checker,
) -> Result<bool, InterruptError<String>> {
    let mut ctx = XmlXPathContext::xml_xpath_new_context(doc);
    ctx.xml_xpath_set_context_node(node);
    match check.xpath(expr, &ctx) {
        Ok(o) => Ok(xml_xpath_cast_to_boolean(&o)),
        Err(InterruptError::Other(_)) => Ok(false),
        Err(InterruptError::Interrupted) => Err(InterruptError::Interrupted),
    }
}
