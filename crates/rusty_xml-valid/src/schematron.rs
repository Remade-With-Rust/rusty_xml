//! ISO Schematron: `rule/@context` + `assert/@test` / `report/@test` via XPath 1.0.

use crate::interrupt::{NoPoll, Poll, PollWith};
use crate::ValidError;
use rusty_xml_parser::{default_parse_options, xml_read_memory};
use rusty_xml_tree::{Interrupt, NodeId, NodeKind, Walk, XmlDoc};
use rusty_xml_xpath::{
    xml_xpath_cast_to_boolean, xml_xpath_eval, xml_xpath_eval_interruptible, XPathError,
    XPathObject, XmlXPathContext,
};

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
/// parsed, between XPath evaluations, and inside each evaluation, so one
/// expensive test cannot run past it (#58).
pub fn xml_schematron_validate_doc_interruptible(
    sch: &[u8],
    doc: &XmlDoc,
    interrupt: &dyn Interrupt,
) -> Result<(), ValidError> {
    schematron_validate(sch, doc, &mut PollWith::new(interrupt))
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
    run_rule(sdoc, top, doc, errors, poll)?;
    let mut failed = Ok(());
    sdoc.walk(
        top,
        |id| match run_rule(sdoc, id, doc, errors, poll) {
            Ok(()) => Walk::Descend,
            Err(e) => {
                failed = Err(e);
                Walk::Stop
            }
        },
        |_| {},
    );
    failed
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
            let nodes = eval_nodeset(doc, &ctx, poll.interrupt())?;
            let mut c = sdoc.first_child(id);
            while let Some(x) = c {
                if is_sch(sdoc, x, "assert") {
                    if let Some(test) = sdoc.xml_get_prop(x, "test") {
                        for n in &nodes {
                            poll.tick()?;
                            if !eval_bool(doc, *n, &test, poll.interrupt())? {
                                errors.push(sdoc.xml_node_get_content(x));
                            }
                        }
                    }
                } else if is_sch(sdoc, x, "report") {
                    if let Some(test) = sdoc.xml_get_prop(x, "test") {
                        for n in &nodes {
                            poll.tick()?;
                            if eval_bool(doc, *n, &test, poll.interrupt())? {
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

/// Evaluate `expr`, through the interruptible entry when there is an
/// interrupt. `Ok(None)` is an evaluation error, which a test reads as false
/// and a context as no nodes; an interrupt is never read that way; it ends
/// the validation.
fn eval(
    expr: &str,
    ctx: &XmlXPathContext<'_>,
    interrupt: Option<&dyn Interrupt>,
) -> Result<Option<XPathObject>, ValidError> {
    match interrupt {
        None => Ok(xml_xpath_eval(expr, ctx).ok()),
        Some(i) => match xml_xpath_eval_interruptible(expr, ctx, i) {
            Ok(o) => Ok(Some(o)),
            Err(XPathError::Interrupted) => Err(ValidError::Interrupted),
            Err(_) => Ok(None),
        },
    }
}

fn eval_nodeset(
    doc: &XmlDoc,
    expr: &str,
    interrupt: Option<&dyn Interrupt>,
) -> Result<Vec<NodeId>, ValidError> {
    let ctx = XmlXPathContext::xml_xpath_new_context(doc);
    Ok(match eval(expr, &ctx, interrupt)? {
        Some(XPathObject::NodeSet(v)) => v,
        _ => vec![],
    })
}

fn eval_bool(
    doc: &XmlDoc,
    node: NodeId,
    expr: &str,
    interrupt: Option<&dyn Interrupt>,
) -> Result<bool, ValidError> {
    let mut ctx = XmlXPathContext::xml_xpath_new_context(doc);
    ctx.xml_xpath_set_context_node(node);
    Ok(eval(expr, &ctx, interrupt)?.is_some_and(|o| xml_xpath_cast_to_boolean(&o)))
}
