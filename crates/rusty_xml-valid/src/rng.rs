//! RelaxNG (simplified) matching libxml2 `relaxng.h` for the tutorial corpus.

use crate::interrupt::{NoPoll, Poll};
use crate::ValidError;
use rusty_xml_parser::{default_parse_options, xml_read_memory};
use rusty_xml_tree::{Interrupt, InterruptPoll, NodeId, NodeKind, XmlDoc};
use std::collections::HashMap;

const RNG_NS: &str = "http://relaxng.org/ns/structure/1.0";

/// How deeply schema patterns may nest, counted in elements of the schema
/// document. Compiling recurses once per level.
const MAX_SCHEMA_DEPTH: usize = 512;

/// How deeply matching may recurse, counting each pattern entered and each
/// instance element on the current path (a chain of `<ref>`s costs nothing).
///
/// The matcher backtracks through choices, so it recurses rather than walking
/// an explicit stack; without a bound, a deep instance document (the parser
/// allows 5,000 levels, a million under `XML_PARSE_HUGE`) overflowed the
/// stack, which aborts the process. Past this depth validation fails with an
/// error instead. A typical recursive grammar spends two to four of these per
/// level of the instance, so this admits 320 to 640 levels, more than the 256
/// libxml2's parser allows by default, while a release build stays inside a
/// 1 MB stack.
const MAX_MATCH_DEPTH: usize = 1280;

#[derive(Clone, Debug)]
enum Pat {
    Empty,
    Text,
    NotAllowed,
    Element { name: String, inner: Box<Pat> },
    Attribute { name: String, inner: Box<Pat> },
    Group(Vec<Pat>),
    Choice(Vec<Pat>),
    Interleave(Vec<Pat>),
    Optional(Box<Pat>),
    ZeroOrMore(Box<Pat>),
    OneOrMore(Box<Pat>),
    Value(String),
    Data,
    Ref(String),
    List(Box<Pat>),
}

struct Schema {
    start: Pat,
    defs: HashMap<String, Pat>,
}

/// `xmlRelaxNGParse` + `xmlRelaxNGValidateDoc`.
#[doc(alias = "xmlRelaxNGValidateDoc")]
pub fn xml_relaxng_validate_doc(rng_xml: &[u8], doc: &XmlDoc) -> Result<(), String> {
    relaxng_validate(rng_xml, doc, &mut NoPoll).map_err(ValidError::into_message)
}

/// [`xml_relaxng_validate_doc`], stopping with [`ValidError::Interrupted`]
/// once `interrupt` fires. It is polled before and after the schema is
/// parsed, and every few hundred elements matched, including elements
/// re-matched while trying alternatives.
pub fn xml_relaxng_validate_doc_interruptible(
    rng_xml: &[u8],
    doc: &XmlDoc,
    interrupt: &dyn Interrupt,
) -> Result<(), ValidError> {
    relaxng_validate(rng_xml, doc, &mut InterruptPoll::new(interrupt))
}

fn relaxng_validate<P: Poll>(rng_xml: &[u8], doc: &XmlDoc, poll: &mut P) -> Result<(), ValidError> {
    poll.check()?;
    let rng_doc = xml_read_memory(rng_xml, None, None, default_parse_options())
        .map_err(|e| e.to_string())?;
    poll.check()?;
    let schema = compile(&rng_doc)?;
    let root = doc.xml_doc_get_root_element().ok_or("no root")?;
    let mut cx = Cx {
        schema: &schema,
        doc,
        poll,
        depth: 0,
        aborted: false,
    };
    cx.match_element(&schema.start, root)
}

fn compile(doc: &XmlDoc) -> Result<Schema, String> {
    let root = doc.xml_doc_get_root_element().ok_or("empty rng")?;
    if max_depth(doc, root) > MAX_SCHEMA_DEPTH {
        return Err(format!(
            "RelaxNG schema nested deeper than {MAX_SCHEMA_DEPTH} levels"
        ));
    }
    let mut defs = HashMap::new();
    harvest_defs(doc, root, &mut defs);
    let start = if is_rng(doc, root, "grammar") {
        match find_child_named(doc, root, "start") {
            // `<start/>` with no pattern inside used to `.unwrap()` an Err and
            // panic. It is a malformed schema, which is an error to report, not
            // a reason to kill the caller's thread.
            Some(s) => compile_pat(doc, first_pat_child(doc, s).ok_or("empty start")?),
            None => compile_pat(doc, root),
        }
    } else {
        compile_pat(doc, root)
    };
    let schema = Schema { start, defs };
    check_no_ref_cycle(&schema)?;
    Ok(schema)
}

/// The deepest element nesting under `root`, walked iteratively.
fn max_depth(doc: &XmlDoc, root: NodeId) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(root, 1usize)];
    while let Some((id, d)) = stack.pop() {
        deepest = deepest.max(d);
        let mut c = doc.first_child(id);
        while let Some(x) = c {
            if doc.kind(x) == NodeKind::Element {
                stack.push((x, d + 1));
            }
            c = doc.next_sibling(x);
        }
    }
    deepest
}

/// Reject a schema whose definitions reference each other in a cycle without
/// consuming an element.
///
/// `<define name="a"><ref name="a"/></define>` made the matcher recurse until
/// the stack ran out, which ABORTS THE PROCESS -- a stack overflow cannot be
/// caught. Mutual cycles (a -> b -> a) and cycles through choice/group did the
/// same. Such a grammar can never match anything, so refusing it at compile
/// time loses nothing and is decidable here, unlike a depth limit in the
/// matcher which would only move the cliff.
///
/// Each definition is walked once: one that has been walked without finding a
/// cycle is not walked again. Walking every definition afresh made a chain of
/// n references cost n² steps, and the walk itself recursed once per link.
fn check_no_ref_cycle(schema: &Schema) -> Result<(), String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Active,
        Done,
    }
    fn walk<'a>(
        schema: &'a Schema,
        pat: &'a Pat,
        marks: &mut HashMap<&'a str, Mark>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > MAX_MATCH_DEPTH {
            return Err(format!(
                "RelaxNG references nested deeper than {MAX_MATCH_DEPTH} levels"
            ));
        }
        match pat {
            // An Element consumes input, so a reference under it cannot loop
            // forever; that is where recursive grammars are legitimate.
            Pat::Element { .. } => Ok(()),
            Pat::Ref(n) => {
                match marks.get(n.as_str()) {
                    Some(Mark::Active) => return Err(format!("cyclic ref {n} in schema")),
                    Some(Mark::Done) => return Ok(()),
                    None => {}
                }
                let Some((name, next)) = schema.defs.get_key_value(n) else {
                    return Ok(()); // undefined refs are reported while matching
                };
                marks.insert(name, Mark::Active);
                walk(schema, next, marks, depth + 1)?;
                marks.insert(name, Mark::Done);
                Ok(())
            }
            Pat::Attribute { inner, .. }
            | Pat::Optional(inner)
            | Pat::ZeroOrMore(inner)
            | Pat::OneOrMore(inner)
            | Pat::List(inner) => walk(schema, inner, marks, depth + 1),
            Pat::Group(v) | Pat::Choice(v) | Pat::Interleave(v) => {
                for p in v {
                    walk(schema, p, marks, depth + 1)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    let mut marks: HashMap<&str, Mark> = HashMap::new();
    walk(schema, &schema.start, &mut marks, 0)?;
    for (name, pat) in &schema.defs {
        if marks.contains_key(name.as_str()) {
            continue;
        }
        marks.insert(name, Mark::Active);
        walk(schema, pat, &mut marks, 0)?;
        marks.insert(name, Mark::Done);
    }
    Ok(())
}

/// Collect every `<define>`, walking the schema document iteratively.
fn harvest_defs(doc: &XmlDoc, root: NodeId, defs: &mut HashMap<String, Pat>) {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if is_rng(doc, id, "define") {
            if let Some(name) = doc.xml_get_prop(id, "name") {
                if let Some(ch) = first_pat_child(doc, id) {
                    defs.insert(name, compile_pat(doc, ch));
                }
            }
        }
        let mut c = doc.last_child(id);
        while let Some(x) = c {
            stack.push(x);
            c = doc.prev_sibling(x);
        }
    }
}

fn is_rng(doc: &XmlDoc, id: NodeId, name: &str) -> bool {
    doc.kind(id) == NodeKind::Element
        && doc.name(id) == name
        && (doc.ns_uri(id) == Some(RNG_NS) || doc.ns_uri(id).is_none())
}

fn first_pat_child(doc: &XmlDoc, id: NodeId) -> Option<NodeId> {
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        if doc.kind(x) == NodeKind::Element {
            return Some(x);
        }
        c = doc.next_sibling(x);
    }
    None
}

fn find_child_named(doc: &XmlDoc, id: NodeId, name: &str) -> Option<NodeId> {
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        if is_rng(doc, x, name) {
            return Some(x);
        }
        c = doc.next_sibling(x);
    }
    None
}

/// Compile one pattern. It recurses once per level of the schema document,
/// which [`compile`] bounds before calling it.
fn compile_pat(doc: &XmlDoc, id: NodeId) -> Pat {
    let name = doc.name(id);
    match name {
        "element" => {
            let n = doc.xml_get_prop(id, "name").unwrap_or_default();
            Pat::Element {
                name: n,
                inner: Box::new(compile_group_children(doc, id)),
            }
        }
        "attribute" => {
            let n = doc.xml_get_prop(id, "name").unwrap_or_default();
            Pat::Attribute {
                name: n,
                inner: Box::new(compile_group_children(doc, id)),
            }
        }
        "empty" => Pat::Empty,
        "text" => Pat::Text,
        "notAllowed" => Pat::NotAllowed,
        "optional" => Pat::Optional(Box::new(compile_group_children(doc, id))),
        "zeroOrMore" => Pat::ZeroOrMore(Box::new(compile_group_children(doc, id))),
        "oneOrMore" => Pat::OneOrMore(Box::new(compile_group_children(doc, id))),
        "choice" => Pat::Choice(compile_child_pats(doc, id)),
        "group" => Pat::Group(compile_child_pats(doc, id)),
        "interleave" => Pat::Interleave(compile_child_pats(doc, id)),
        "value" => Pat::Value(doc.xml_node_get_content(id).trim().to_string()),
        "data" => Pat::Data,
        "ref" => Pat::Ref(doc.xml_get_prop(id, "name").unwrap_or_default()),
        "list" => Pat::List(Box::new(compile_group_children(doc, id))),
        "mixed" => Pat::Interleave(vec![Pat::Text, compile_group_children(doc, id)]),
        "grammar" => find_child_named(doc, id, "start")
            .and_then(|s| first_pat_child(doc, s))
            .map(|c| compile_pat(doc, c))
            .unwrap_or(Pat::NotAllowed),
        _ => compile_group_children(doc, id),
    }
}

fn compile_child_pats(doc: &XmlDoc, id: NodeId) -> Vec<Pat> {
    let mut v = Vec::new();
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        if doc.kind(x) == NodeKind::Element {
            v.push(compile_pat(doc, x));
        }
        c = doc.next_sibling(x);
    }
    v
}

fn compile_group_children(doc: &XmlDoc, id: NodeId) -> Pat {
    let mut v = compile_child_pats(doc, id);
    match v.len() {
        0 => Pat::Empty,
        1 => v.pop().unwrap_or(Pat::Empty),
        _ => Pat::Group(v),
    }
}


/// One validation: the schema, the instance, the interrupt poll and the
/// current recursion depth.
struct Cx<'a, P: Poll> {
    schema: &'a Schema,
    doc: &'a XmlDoc,
    poll: &'a mut P,
    depth: usize,
    /// Set once the depth bound is hit. Like an interrupt, that failure is
    /// final: an `<optional>` or a `<choice>` must not take it for a
    /// mismatch and carry on to report some other error.
    aborted: bool,
}

/// The result of one matching step. An interrupt is never an ordinary
/// failure: a choice that sees one stops instead of trying the next
/// alternative.
type Step = Result<(), ValidError>;

// The matcher recurses once per pattern and once per instance element, so
// every byte of its frames is multiplied by the document's depth. Everything
// that is not recursion (building messages, gathering children, checking
// leaves) lives in the out-of-line helpers below, which keeps the recursive
// frames small.

#[cold]
#[inline(never)]
fn fail(what: &str, name: &str) -> ValidError {
    ValidError::Failed(format!("{what}{name}"))
}

#[cold]
#[inline(never)]
fn too_deep() -> ValidError {
    ValidError::Failed(format!(
        "RelaxNG validation nested deeper than {MAX_MATCH_DEPTH} patterns and elements"
    ))
}

#[cold]
#[inline(never)]
fn mismatch(name: &str, got: &str) -> ValidError {
    ValidError::Failed(format!("expected element {name}, got {got}"))
}

#[inline(never)]
fn element_kids(doc: &XmlDoc, id: NodeId) -> (Vec<(String, String)>, Vec<NodeId>) {
    let mut attrs: Vec<(String, String)> = Vec::new();
    let mut a = doc.first_attr(id);
    while let Some(x) = a {
        if !doc.name(x).starts_with("xmlns") {
            attrs.push((doc.name(x).to_string(), doc.content(x).to_string()));
        }
        a = doc.next_sibling(x);
    }
    let mut kids: Vec<NodeId> = Vec::new();
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        match doc.kind(x) {
            NodeKind::Element => kids.push(x),
            NodeKind::Text | NodeKind::CData if !doc.xml_is_blank_node(x) => kids.push(x),
            _ => {}
        }
        c = doc.next_sibling(x);
    }
    (attrs, kids)
}

#[inline(never)]
fn content_left(doc: &XmlDoc, kids: &[NodeId], attrs: &[(String, String)]) -> Step {
    if !attrs.is_empty() {
        return Err(format!("undeclared attributes {:?}", attrs).into());
    }
    if kids.iter().any(|&k| doc.kind(k) == NodeKind::Element) {
        return Err("extra element content".into());
    }
    Ok(())
}

#[inline(never)]
fn drop_text(doc: &XmlDoc, kids: &mut Vec<NodeId>) {
    kids.retain(|&k| doc.kind(k) == NodeKind::Element);
}

#[inline(never)]
fn consume_attribute(name: &str, inner: &Pat, attrs: &mut Vec<(String, String)>) -> Step {
    let Some(i) = attrs.iter().position(|(n, _)| n == name) else {
        return Err(fail("missing attribute ", name));
    };
    let val = attrs.remove(i).1;
    match inner {
        Pat::Value(v) if *v != val => Err(format!("attr {name} expected {v}").into()),
        _ => Ok(()),
    }
}

#[inline(never)]
fn consume_value(doc: &XmlDoc, v: &str, kids: &mut Vec<NodeId>) -> Step {
    let text: String = kids
        .iter()
        .filter(|k| doc.kind(**k) != NodeKind::Element)
        .map(|k| doc.content(*k))
        .collect();
    drop_text(doc, kids);
    if text.trim() == v {
        Ok(())
    } else {
        Err(format!("value {v} != {}", text.trim()).into())
    }
}

#[inline(never)]
fn take_element(doc: &XmlDoc, name: &str, kids: &mut Vec<NodeId>) -> Option<NodeId> {
    let i = kids
        .iter()
        .position(|&k| doc.kind(k) == NodeKind::Element && doc.name(k) == name)?;
    Some(kids.remove(i))
}

impl<'a, P: Poll> Cx<'a, P> {
    /// Whether a failed step ends the validation rather than letting a
    /// choice, an option or a repetition try something else.
    fn fatal(&self, e: &ValidError) -> bool {
        self.aborted || e.is_interrupted()
    }

    fn def(&self, n: &str) -> Result<&'a Pat, ValidError> {
        self.schema.defs.get(n).ok_or_else(|| fail("undefined ref ", n))
    }

    fn match_element(&mut self, pat: &'a Pat, id: NodeId) -> Step {
        self.depth += 1;
        if self.depth > MAX_MATCH_DEPTH {
            self.aborted = true;
            return Err(too_deep());
        }
        let mut pat = pat;
        while let Pat::Ref(n) = pat {
            match self.def(n) {
                Ok(p) => pat = p,
                Err(e) => {
                    self.depth -= 1;
                    return Err(e);
                }
            }
        }
        let r = match pat {
            Pat::Element { name, inner } => {
                if self.doc.name(id) != name {
                    Err(mismatch(name, self.doc.name(id)))
                } else {
                    self.match_content(inner, id)
                }
            }
            Pat::Choice(alts) => {
                let mut r = Err(ValidError::Failed("choice failed".into()));
                for a in alts {
                    match self.match_element(a, id) {
                        Err(e) if !self.fatal(&e) => {}
                        other => {
                            r = other;
                            break;
                        }
                    }
                }
                r
            }
            other => self.match_content(other, id),
        };
        self.depth -= 1;
        r
    }

    fn match_content(&mut self, pat: &'a Pat, id: NodeId) -> Step {
        self.poll.tick()?;
        self.depth += 1;
        if self.depth > MAX_MATCH_DEPTH {
            self.aborted = true;
            return Err(too_deep());
        }
        let (mut attrs, mut kids) = element_kids(self.doc, id);
        let r = match self.consume(pat, &mut kids, &mut attrs) {
            Ok(()) => content_left(self.doc, &kids, &attrs),
            e => e,
        };
        self.depth -= 1;
        r
    }

    fn consume(
        &mut self,
        pat: &'a Pat,
        kids: &mut Vec<NodeId>,
        attrs: &mut Vec<(String, String)>,
    ) -> Step {
        self.depth += 1;
        if self.depth > MAX_MATCH_DEPTH {
            self.aborted = true;
            return Err(too_deep());
        }
        // A chain of references is followed in a loop, not a frame per link;
        // check_no_ref_cycle has ruled out a cycle without an element.
        let mut pat = pat;
        while let Pat::Ref(n) = pat {
            match self.def(n) {
                Ok(p) => pat = p,
                Err(e) => {
                    self.depth -= 1;
                    return Err(e);
                }
            }
        }
        // The arms that recurse each live in their own function, so this
        // frame, which every level of the document pays for, stays small.
        let r = match pat {
            Pat::Empty => Ok(()),
            Pat::Text | Pat::Data | Pat::List(_) => {
                drop_text(self.doc, kids);
                Ok(())
            }
            Pat::NotAllowed => Err(fail("notAllowed", "")),
            Pat::Attribute { name, inner } => consume_attribute(name, inner, attrs),
            Pat::Value(v) => consume_value(self.doc, v, kids),
            Pat::Element { name, inner } => self.element(name, inner, kids),
            Pat::Group(ps) | Pat::Interleave(ps) => self.group(ps, kids, attrs),
            Pat::Choice(ps) => self.choice(ps, kids, attrs),
            Pat::Optional(p) => self.optional(p, kids, attrs),
            Pat::ZeroOrMore(p) => self.zero_or_more(p, kids, attrs),
            Pat::OneOrMore(p) => self.one_or_more(p, kids, attrs),
            Pat::Ref(_) => Ok(()), // resolved above
        };
        self.depth -= 1;
        r
    }

    #[inline(never)]
    fn element(&mut self, name: &'a str, inner: &'a Pat, kids: &mut Vec<NodeId>) -> Step {
        match take_element(self.doc, name, kids) {
            Some(n) => self.match_content(inner, n),
            None => Err(fail("missing element ", name)),
        }
    }

    #[inline(never)]
    fn group(
        &mut self,
        ps: &'a [Pat],
        kids: &mut Vec<NodeId>,
        attrs: &mut Vec<(String, String)>,
    ) -> Step {
        for p in ps {
            self.consume(p, kids, attrs)?;
        }
        Ok(())
    }

    #[inline(never)]
    fn choice(
        &mut self,
        ps: &'a [Pat],
        kids: &mut Vec<NodeId>,
        attrs: &mut Vec<(String, String)>,
    ) -> Step {
        for p in ps {
            match self.attempt(p, kids, attrs) {
                Err(e) if !self.fatal(&e) => {}
                other => return other,
            }
        }
        Err(fail("choice failed", ""))
    }

    #[inline(never)]
    fn optional(
        &mut self,
        p: &'a Pat,
        kids: &mut Vec<NodeId>,
        attrs: &mut Vec<(String, String)>,
    ) -> Step {
        match self.attempt(p, kids, attrs) {
            Err(e) if self.fatal(&e) => Err(e),
            _ => Ok(()),
        }
    }

    #[inline(never)]
    fn one_or_more(
        &mut self,
        p: &'a Pat,
        kids: &mut Vec<NodeId>,
        attrs: &mut Vec<(String, String)>,
    ) -> Step {
        self.consume(p, kids, attrs)?;
        self.zero_or_more(p, kids, attrs)
    }

    /// Try `p` on copies, keeping what it consumed only if it matched.
    #[inline(never)]
    fn attempt(
        &mut self,
        p: &'a Pat,
        kids: &mut Vec<NodeId>,
        attrs: &mut Vec<(String, String)>,
    ) -> Step {
        let mut k2 = kids.clone();
        let mut a2 = attrs.clone();
        self.consume(p, &mut k2, &mut a2)?;
        *kids = k2;
        *attrs = a2;
        Ok(())
    }

    #[inline(never)]
    fn zero_or_more(
        &mut self,
        p: &'a Pat,
        kids: &mut Vec<NodeId>,
        attrs: &mut Vec<(String, String)>,
    ) -> Step {
        loop {
            let mut k2 = kids.clone();
            let mut a2 = attrs.clone();
            match self.consume(p, &mut k2, &mut a2) {
                Ok(()) if k2.len() < kids.len() || a2.len() < attrs.len() => {
                    *kids = k2;
                    *attrs = a2;
                }
                Ok(()) => return Ok(()),
                Err(e) if self.fatal(&e) => return Err(e),
                Err(_) => return Ok(()),
            }
        }
    }
}
