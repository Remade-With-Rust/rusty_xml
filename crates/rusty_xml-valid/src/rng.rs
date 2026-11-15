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

/// Entries the backtracking memo may hold (#56). Past it, matching goes on
/// uncached rather than growing without bound.
const MEMO_MAX: usize = 1 << 20;

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
        memo: HashMap::default(),
        backtracked: false,
        reach: HashMap::default(),
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
    /// Outcomes of [`Cx::match_content`] by pattern and element (#56). A
    /// choice whose alternatives reach the same definition matched the same
    /// subtree once per alternative, so n nested two-way choices over a
    /// failing document cost 2^n: depth 22 took 1.4 s and depth 40 did not
    /// finish. The schema and the document never change during a validation,
    /// so the outcome depends on the pair alone; the one exception, reaching
    /// the nesting bound, ends the validation and is never stored. Leaving
    /// the depth out of the key (0.9.4 had it) also lets a second pass (#57)
    /// reuse what the first pass and the passes below it found, so an
    /// element is matched once however many of its ancestors reconsider it.
    memo: HashMap<(usize, u32), Step, std::hash::BuildHasherDefault<Mix>>,
    /// A choice, option or repetition has failed and been undone. Until then
    /// nothing is memoised, so a document that never backtracks pays nothing:
    /// an eager table cost +10% to +45% on 32,000 siblings.
    backtracked: bool,
    /// [`Reach`] of each content pattern a second pass was considered for,
    /// by address.
    reach: HashMap<usize, Reach<'a>, std::hash::BuildHasherDefault<Mix>>,
}

/// What a content pattern can consume without entering a child element: the
/// names of the element and attribute patterns it reaches, and whether one
/// of its patterns takes text. A second pass can only accept content whose
/// children, attributes and text all fall inside it, so most alternatives a
/// choice tries and drops are refused without one: on DocBook, running the
/// pass for every failed element content cost +4.4%.
struct Reach<'a> {
    elems: NameSet<'a>,
    attrs: NameSet<'a>,
    text: bool,
}

/// A set of schema names. Only the schema inserts into it; a document can
/// only look names up, so a plain FNV hash is safe here, and SipHash on every
/// child name cost DocBook 0.7M instructions.
type NameSet<'a> = std::collections::HashSet<&'a str, std::hash::BuildHasherDefault<Fnv>>;

#[derive(Default)]
struct Fnv(u64);

impl std::hash::Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 { 0xcbf2_9ce4_8422_2325 } else { self.0 };
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = h;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// A hasher for the matcher's integer keys (pattern addresses, node ids,
/// depths, positions). None of them is a value a document can choose, so
/// SipHash bought nothing there and cost the memo and the second pass on
/// every lookup.
#[derive(Default)]
struct Mix(u64);

impl std::hash::Hasher for Mix {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(u64::from(b));
        }
    }

    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn write_u32(&mut self, x: u32) {
        self.write_u64(u64::from(x));
    }

    fn write_usize(&mut self, x: usize) {
        self.write_u64(x as u64);
    }

    fn write_u8(&mut self, x: u8) {
        self.write_u64(u64::from(x));
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// The result of one matching step. An interrupt is never an ordinary
/// failure: a choice that sees one stops instead of trying the next
/// alternative.
type Step = Result<(), ValidError>;

/// Extra depth a second pass (#57) is charged on entry, for its own frames.
const SECOND_PASS_DEPTH: usize = 2;
/// Live states past which a second pass gives up and the first pass's
/// verdict stands.
const SECOND_PASS_MAX_STATES: usize = 1 << 20;
/// Attributes past which a second pass gives up: the used set is a bitmask.
const SECOND_PASS_MAX_ATTRS: usize = 62;

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

/// The children and attributes of one instance element, read once. Matching
/// its content moves an [`St`] over them and never copies them: the old
/// matcher cloned the remaining children for every repetition and every
/// trial and removed taken ones from the middle of a `Vec`, so `zeroOrMore`
/// over k siblings moved O(k^2) ids (#58).
struct Content {
    /// Element children, in document order.
    els: Vec<NodeId>,
    /// Text and CDATA children that are not blank.
    texts: Vec<NodeId>,
    /// Attributes other than namespace declarations, in document order.
    attrs: Vec<(String, String)>,
}

#[inline(never)]
fn element_content(doc: &XmlDoc, id: NodeId) -> Content {
    let mut attrs: Vec<(String, String)> = Vec::new();
    let mut a = doc.first_attr(id);
    while let Some(x) = a {
        if !doc.name(x).starts_with("xmlns") {
            attrs.push((doc.name(x).to_string(), doc.content(x).to_string()));
        }
        a = doc.next_sibling(x);
    }
    let mut els: Vec<NodeId> = Vec::new();
    let mut texts: Vec<NodeId> = Vec::new();
    let mut c = doc.first_child(id);
    while let Some(x) = c {
        match doc.kind(x) {
            NodeKind::Element => els.push(x),
            NodeKind::Text | NodeKind::CData if !doc.xml_is_blank_node(x) => texts.push(x),
            _ => {}
        }
        c = doc.next_sibling(x);
    }
    Content { els, texts, attrs }
}

/// Which attributes a content match has used: a bitmask, or a flag per
/// attribute on an element with more than 64.
#[derive(Clone)]
enum Used {
    Small(u64),
    Big(Box<[bool]>),
}

/// Where a match stands in an element's content. Every element child before
/// `pos` is consumed, and so is every index in `taken`: elements an
/// `interleave` took out of order, all past `pos`, ascending. `text` is set
/// once a text, data, list or value pattern has taken the element's text.
/// Saving and restoring one is a copy of a few words (`taken` is empty
/// outside `interleave`), not of the children.
#[derive(Clone)]
struct St {
    pos: usize,
    taken: Vec<usize>,
    used: Used,
    text: bool,
}

impl St {
    fn new(c: &Content) -> St {
        St {
            pos: 0,
            taken: Vec::new(),
            used: if c.attrs.len() <= 64 {
                Used::Small(0)
            } else {
                Used::Big(vec![false; c.attrs.len()].into_boxed_slice())
            },
            text: false,
        }
    }

    /// The first element child not yet consumed.
    fn next(&self, n: usize) -> Option<usize> {
        let mut i = self.pos;
        for &t in &self.taken {
            match t.cmp(&i) {
                std::cmp::Ordering::Equal => i += 1,
                std::cmp::Ordering::Greater => break,
                std::cmp::Ordering::Less => {}
            }
        }
        (i < n).then_some(i)
    }

    /// Consume element child `i`; `next` is [`St::next`].
    fn take(&mut self, i: usize, next: usize) {
        if i == next {
            self.taken.retain(|&t| t > i);
            self.pos = i + 1;
            while self.taken.first() == Some(&self.pos) {
                self.taken.remove(0);
                self.pos += 1;
            }
        } else {
            let at = self.taken.partition_point(|&t| t < i);
            self.taken.insert(at, i);
        }
    }

    fn consumed(&self) -> usize {
        self.pos + self.taken.len()
    }

    fn is_used(&self, i: usize) -> bool {
        match &self.used {
            Used::Small(b) => (b >> i) & 1 != 0,
            Used::Big(v) => v[i],
        }
    }

    fn use_attr(&mut self, i: usize) {
        match &mut self.used {
            Used::Small(b) => *b |= 1 << i,
            Used::Big(v) => v[i] = true,
        }
    }

    fn used_count(&self) -> usize {
        match &self.used {
            Used::Small(b) => b.count_ones() as usize,
            Used::Big(v) => v.iter().filter(|&&x| x).count(),
        }
    }

    /// Whether `self` consumed more than `before`: a repetition that matches
    /// without consuming anything ends.
    fn progressed(&self, before: &St, c: &Content) -> bool {
        self.consumed() > before.consumed()
            || self.used_count() > before.used_count()
            || (self.text && !before.text && !c.texts.is_empty())
    }
}

#[inline(never)]
fn content_left(c: &Content, st: &St) -> Step {
    if st.used_count() < c.attrs.len() {
        let left: Vec<&(String, String)> = c
            .attrs
            .iter()
            .enumerate()
            .filter(|(i, _)| !st.is_used(*i))
            .map(|(_, a)| a)
            .collect();
        return Err(format!("undeclared attributes {:?}", left).into());
    }
    if st.next(c.els.len()).is_some() {
        return Err("extra element content".into());
    }
    Ok(())
}

#[inline(never)]
fn consume_attribute(name: &str, inner: &Pat, c: &Content, st: &mut St) -> Step {
    let Some(i) = (0..c.attrs.len()).find(|&i| !st.is_used(i) && c.attrs[i].0 == name) else {
        return Err(fail("missing attribute ", name));
    };
    st.use_attr(i);
    match inner {
        Pat::Value(v) if *v != c.attrs[i].1 => Err(format!("attr {name} expected {v}").into()),
        _ => Ok(()),
    }
}

/// The element's text as a value pattern sees it: empty once taken.
fn text_of(doc: &XmlDoc, c: &Content, taken: bool) -> String {
    if taken {
        String::new()
    } else {
        c.texts.iter().map(|k| doc.content(*k)).collect()
    }
}

#[inline(never)]
fn consume_value(doc: &XmlDoc, v: &str, c: &Content, st: &mut St) -> Step {
    let text = text_of(doc, c, st.text);
    st.text = true;
    if text.trim() == v {
        Ok(())
    } else {
        Err(format!("value {v} != {}", text.trim()).into())
    }
}

/// The first element child not consumed, at or past `st.pos`, with this
/// name: how an `interleave` takes elements, in any order.
#[inline(never)]
fn find_named(doc: &XmlDoc, c: &Content, st: &St, name: &str) -> Option<usize> {
    let mut t = st.taken.iter().peekable();
    for i in st.pos..c.els.len() {
        while t.peek().is_some_and(|&&x| x < i) {
            t.next();
        }
        if t.peek() == Some(&&i) {
            continue;
        }
        if doc.name(c.els[i]) == name {
            return Some(i);
        }
    }
    None
}

/// A state of the second pass: the next element child, the attributes used
/// and whether the text has been taken.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct S2 {
    pos: u32,
    used: u64,
    text: bool,
}

/// Why a second pass stopped without a set of states.
enum Stop {
    /// Beyond what the pass handles; the first pass's verdict stands.
    GiveUp,
    /// An interrupt: the validation ends.
    Fatal(ValidError),
}

/// Sort and deduplicate a state set, giving up past the state bound.
fn norm(mut v: Vec<S2>) -> Result<Vec<S2>, Stop> {
    v.sort_unstable();
    v.dedup();
    if v.len() > SECOND_PASS_MAX_STATES {
        return Err(Stop::GiveUp);
    }
    Ok(v)
}

#[inline(never)]
fn s2_text(input: Vec<S2>) -> Result<Vec<S2>, Stop> {
    norm(input.into_iter().map(|s| S2 { text: true, ..s }).collect())
}

#[inline(never)]
fn s2_value(doc: &XmlDoc, v: &str, c: &Content, input: Vec<S2>) -> Result<Vec<S2>, Stop> {
    let mut out = Vec::new();
    for s in input {
        if text_of(doc, c, s.text).trim() == v {
            out.push(S2 { text: true, ..s });
        }
    }
    norm(out)
}

#[inline(never)]
fn s2_attribute(name: &str, inner: &Pat, c: &Content, input: Vec<S2>) -> Vec<S2> {
    let mut out = Vec::new();
    for s in input {
        let Some(i) = (0..c.attrs.len()).find(|&i| (s.used >> i) & 1 == 0 && c.attrs[i].0 == name)
        else {
            continue;
        };
        if let Pat::Value(v) = inner {
            if *v != c.attrs[i].1 {
                continue;
            }
        }
        out.push(S2 { used: s.used | (1 << i), ..s });
    }
    out
}

/// What the second pass knows about element children: whether a pattern
/// matched the content of the child at a position, asked once per pair.
type ChildCache = HashMap<(usize, u32), bool, std::hash::BuildHasherDefault<Mix>>;

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
                        Err(e) if !self.fatal(&e) => self.backtracked = true,
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
        if !self.backtracked {
            return self.match_content_once(pat, id);
        }
        let key = (pat as *const Pat as usize, id.0);
        if let Some(r) = self.memo.get(&key) {
            return r.clone();
        }
        let r = self.match_content_once(pat, id);
        // An interrupt or the depth bound ends the whole validation; neither
        // is an outcome of this pair.
        let final_ = match &r {
            Err(e) => self.fatal(e),
            Ok(()) => self.aborted,
        };
        if !final_ && self.memo.len() < MEMO_MAX {
            self.memo.insert(key, r.clone());
        }
        r
    }

    fn match_content_once(&mut self, pat: &'a Pat, id: NodeId) -> Step {
        self.poll.tick()?;
        self.depth += 1;
        if self.depth > MAX_MATCH_DEPTH {
            self.aborted = true;
            return Err(too_deep());
        }
        let c = element_content(self.doc, id);
        let mut st = St::new(&c);
        let r = match self.consume(pat, &c, &mut st, false) {
            Ok(()) => content_left(&c, &st),
            e => e,
        };
        let r = match r {
            Err(e)
                if !self.fatal(&e)
                    && c.attrs.len() <= SECOND_PASS_MAX_ATTRS
                    && self.second_pass_may_accept(pat, &c) =>
            {
                self.second_pass(pat, &c, e)
            }
            other => other,
        };
        self.depth -= 1;
        r
    }

    /// Match `pat` against what is left of an element's content. Outside an
    /// `interleave` (`any` unset) a sequence takes the next element child, as
    /// libxml2 does; the old matcher took the first child of that name from
    /// anywhere, so `<a/><b/>` and `<b/><a/>` both matched a sequence a, b
    /// (#57). Inside an `interleave` (and `mixed`, compiled as one) an
    /// element is still taken from anywhere.
    fn consume(&mut self, pat: &'a Pat, c: &Content, st: &mut St, any: bool) -> Step {
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
                st.text = true;
                Ok(())
            }
            Pat::NotAllowed => Err(fail("notAllowed", "")),
            Pat::Attribute { name, inner } => consume_attribute(name, inner, c, st),
            Pat::Value(v) => consume_value(self.doc, v, c, st),
            Pat::Element { name, inner } => self.element(name, inner, c, st, any),
            Pat::Group(ps) => self.group(ps, c, st, any),
            Pat::Interleave(ps) => self.group(ps, c, st, true),
            Pat::Choice(ps) => self.choice(ps, c, st, any),
            Pat::Optional(p) => self.optional(p, c, st, any),
            Pat::ZeroOrMore(p) => self.zero_or_more(p, c, st, any),
            Pat::OneOrMore(p) => self.one_or_more(p, c, st, any),
            Pat::Ref(_) => Ok(()), // resolved above
        };
        self.depth -= 1;
        r
    }

    #[inline(never)]
    fn element(
        &mut self,
        name: &'a str,
        inner: &'a Pat,
        c: &Content,
        st: &mut St,
        any: bool,
    ) -> Step {
        let Some(next) = st.next(c.els.len()) else {
            return Err(fail("missing element ", name));
        };
        let i = if any {
            match find_named(self.doc, c, st, name) {
                Some(i) => i,
                None => return Err(fail("missing element ", name)),
            }
        } else if self.doc.name(c.els[next]) == name {
            next
        } else {
            return Err(fail("missing element ", name));
        };
        self.match_content(inner, c.els[i])?;
        st.take(i, next);
        Ok(())
    }

    #[inline(never)]
    fn group(&mut self, ps: &'a [Pat], c: &Content, st: &mut St, any: bool) -> Step {
        for p in ps {
            self.consume(p, c, st, any)?;
        }
        Ok(())
    }

    #[inline(never)]
    fn choice(&mut self, ps: &'a [Pat], c: &Content, st: &mut St, any: bool) -> Step {
        for p in ps {
            match self.attempt(p, c, st, any) {
                Err(e) if !self.fatal(&e) => self.backtracked = true,
                other => return other,
            }
        }
        Err(fail("choice failed", ""))
    }

    #[inline(never)]
    fn optional(&mut self, p: &'a Pat, c: &Content, st: &mut St, any: bool) -> Step {
        match self.attempt(p, c, st, any) {
            Err(e) if self.fatal(&e) => Err(e),
            Err(_) => {
                self.backtracked = true;
                Ok(())
            }
            Ok(()) => Ok(()),
        }
    }

    #[inline(never)]
    fn one_or_more(&mut self, p: &'a Pat, c: &Content, st: &mut St, any: bool) -> Step {
        self.consume(p, c, st, any)?;
        self.zero_or_more(p, c, st, any)
    }

    /// Try `p` on a copy of the state, keeping it only if `p` matched.
    #[inline(never)]
    fn attempt(&mut self, p: &'a Pat, c: &Content, st: &mut St, any: bool) -> Step {
        let mut s2 = st.clone();
        self.consume(p, c, &mut s2, any)?;
        *st = s2;
        Ok(())
    }

    #[inline(never)]
    fn zero_or_more(&mut self, p: &'a Pat, c: &Content, st: &mut St, any: bool) -> Step {
        loop {
            let mut s2 = st.clone();
            match self.consume(p, c, &mut s2, any) {
                Ok(()) if s2.progressed(st, c) => *st = s2,
                Ok(()) => return Ok(()),
                Err(e) if self.fatal(&e) => return Err(e),
                Err(_) => {
                    self.backtracked = true;
                    return Ok(());
                }
            }
        }
    }

    /// Whether every child element, attribute and text of `c` has a pattern
    /// in `pat` that could take it: a second pass cannot accept otherwise.
    #[inline(never)]
    fn second_pass_may_accept(&mut self, pat: &'a Pat, c: &Content) -> bool {
        let key = pat as *const Pat as usize;
        if !self.reach.contains_key(&key) {
            let r = self.reach_of(pat);
            self.reach.insert(key, r);
        }
        let r = &self.reach[&key];
        (c.texts.is_empty() || r.text)
            && c.els.iter().all(|&e| r.elems.contains(self.doc.name(e)))
            && c.attrs.iter().all(|(n, _)| r.attrs.contains(n.as_str()))
    }

    /// [`Reach`] of a content pattern, walked iteratively through references.
    fn reach_of(&self, pat: &'a Pat) -> Reach<'a> {
        let mut r = Reach {
            elems: Default::default(),
            attrs: Default::default(),
            text: false,
        };
        let mut seen: NameSet<'a> = Default::default();
        let mut stack = vec![pat];
        while let Some(p) = stack.pop() {
            match p {
                Pat::Element { name, .. } => {
                    r.elems.insert(name);
                }
                Pat::Attribute { name, .. } => {
                    r.attrs.insert(name);
                }
                Pat::Text | Pat::Data | Pat::List(_) | Pat::Value(_) => r.text = true,
                Pat::Group(ps) | Pat::Choice(ps) | Pat::Interleave(ps) => stack.extend(ps.iter()),
                Pat::Optional(q) | Pat::ZeroOrMore(q) | Pat::OneOrMore(q) => stack.push(q),
                Pat::Ref(n) => {
                    if seen.insert(n) {
                        if let Some(d) = self.schema.defs.get(n) {
                            stack.push(d);
                        }
                    }
                }
                Pat::Empty | Pat::NotAllowed => {}
            }
        }
        r
    }

    /// The first pass commits to the first alternative that matches and to
    /// the longest repetition, and never reconsiders them when what follows
    /// fails: `choice(a, group(a, b))` rejected `<a/><b/>`, and a recursive
    /// `choice(text, ref)` rejected `<a><a><a/></a></a>` (#57). Before such a
    /// failure is reported, this pass follows every way the patterns can
    /// split the children, as sets of [`S2`] states, and accepts if one
    /// consumes every element child and attribute (and the text, if there is
    /// any). It only runs after the first pass failed, so a document that
    /// validates never pays for it.
    ///
    /// It gives up, leaving the first pass's verdict and message, on an
    /// `interleave` or `mixed` pattern, on more than
    /// [`SECOND_PASS_MAX_ATTRS`] attributes, past
    /// [`SECOND_PASS_MAX_STATES`] states and at the depth bound. An interrupt
    /// ends the validation as usual.
    #[cold]
    #[inline(never)]
    fn second_pass(&mut self, pat: &'a Pat, c: &Content, first: ValidError) -> Step {
        self.backtracked = true;
        let base = self.depth;
        let was_aborted = self.aborted;
        self.depth += SECOND_PASS_DEPTH;
        let mut cache = ChildCache::default();
        let start = vec![S2 {
            pos: 0,
            used: 0,
            text: false,
        }];
        let r = self.s2(pat, c, start, &mut cache);
        self.depth = base;
        match r {
            Ok(states) => {
                let n = c.els.len() as u32;
                let all = if c.attrs.is_empty() {
                    0
                } else {
                    u64::MAX >> (64 - c.attrs.len())
                };
                let text_ok = |s: &S2| s.text || c.texts.is_empty();
                if states.iter().any(|s| s.pos == n && s.used == all && text_ok(s)) {
                    Ok(())
                } else {
                    Err(first)
                }
            }
            Err(Stop::GiveUp) => {
                // The depth bound inside this pass only ends the pass.
                self.aborted = was_aborted;
                Err(first)
            }
            Err(Stop::Fatal(e)) => Err(e),
        }
    }

    /// The states `pat` can reach from `input`.
    #[inline(never)]
    fn s2(
        &mut self,
        pat: &'a Pat,
        c: &Content,
        input: Vec<S2>,
        cache: &mut ChildCache,
    ) -> Result<Vec<S2>, Stop> {
        if input.is_empty() {
            return Ok(input);
        }
        self.depth += 1;
        let r = if self.depth > MAX_MATCH_DEPTH {
            Err(Stop::GiveUp)
        } else {
            self.s2_step(pat, c, input, cache)
        };
        self.depth -= 1;
        r
    }

    fn s2_step(
        &mut self,
        pat: &'a Pat,
        c: &Content,
        input: Vec<S2>,
        cache: &mut ChildCache,
    ) -> Result<Vec<S2>, Stop> {
        let mut pat = pat;
        while let Pat::Ref(n) = pat {
            match self.schema.defs.get(n) {
                Some(p) => pat = p,
                None => return Ok(Vec::new()),
            }
        }
        match pat {
            Pat::Empty => Ok(input),
            Pat::NotAllowed => Ok(Vec::new()),
            Pat::Text | Pat::Data | Pat::List(_) => s2_text(input),
            Pat::Value(v) => s2_value(self.doc, v, c, input),
            Pat::Attribute { name, inner } => Ok(s2_attribute(name, inner, c, input)),
            Pat::Element { name, inner } => self.s2_element(name, inner, c, input, cache),
            Pat::Group(ps) => self.s2_group(ps, c, input, cache),
            Pat::Choice(ps) => self.s2_choice(ps, c, input, cache),
            Pat::Interleave(_) => Err(Stop::GiveUp),
            Pat::Optional(p) => {
                let mut out = self.s2(p, c, input.clone(), cache)?;
                out.extend(input);
                norm(out)
            }
            Pat::ZeroOrMore(p) => self.s2_star(p, c, input, cache),
            Pat::OneOrMore(p) => {
                let once = self.s2(p, c, input, cache)?;
                self.s2_star(p, c, once, cache)
            }
            Pat::Ref(_) => Ok(input), // resolved above
        }
    }

    #[inline(never)]
    fn s2_group(
        &mut self,
        ps: &'a [Pat],
        c: &Content,
        input: Vec<S2>,
        cache: &mut ChildCache,
    ) -> Result<Vec<S2>, Stop> {
        let mut s = input;
        for p in ps {
            s = self.s2(p, c, s, cache)?;
            if s.is_empty() {
                break;
            }
        }
        Ok(s)
    }

    #[inline(never)]
    fn s2_choice(
        &mut self,
        ps: &'a [Pat],
        c: &Content,
        input: Vec<S2>,
        cache: &mut ChildCache,
    ) -> Result<Vec<S2>, Stop> {
        let mut out = Vec::new();
        for p in ps {
            out.extend(self.s2(p, c, input.clone(), cache)?);
        }
        norm(out)
    }

    /// A repetition: every state reachable by any number of steps, each state
    /// expanded once.
    #[inline(never)]
    fn s2_star(
        &mut self,
        p: &'a Pat,
        c: &Content,
        input: Vec<S2>,
        cache: &mut ChildCache,
    ) -> Result<Vec<S2>, Stop> {
        let mut seen: std::collections::HashSet<S2, std::hash::BuildHasherDefault<Mix>> =
            input.iter().copied().collect();
        let mut all = input.clone();
        let mut frontier = input;
        loop {
            let next = self.s2(p, c, frontier, cache)?;
            frontier = next.into_iter().filter(|s| seen.insert(*s)).collect();
            if frontier.is_empty() {
                break;
            }
            all.extend_from_slice(&frontier);
            if all.len() > SECOND_PASS_MAX_STATES {
                return Err(Stop::GiveUp);
            }
        }
        Ok(all)
    }

    #[inline(never)]
    fn s2_element(
        &mut self,
        name: &'a str,
        inner: &'a Pat,
        c: &Content,
        input: Vec<S2>,
        cache: &mut ChildCache,
    ) -> Result<Vec<S2>, Stop> {
        let mut out = Vec::new();
        for s in input {
            let p = s.pos as usize;
            let Some(&el) = c.els.get(p) else { continue };
            if self.doc.name(el) != name {
                continue;
            }
            let key = (inner as *const Pat as usize, s.pos);
            let ok = match cache.get(&key) {
                Some(&ok) => ok,
                None => {
                    let ok = match self.match_content(inner, el) {
                        Ok(()) => true,
                        Err(e) if e.is_interrupted() => return Err(Stop::Fatal(e)),
                        Err(_) if self.aborted => return Err(Stop::GiveUp),
                        Err(_) => false,
                    };
                    cache.insert(key, ok);
                    ok
                }
            };
            if ok {
                out.push(S2 { pos: s.pos + 1, ..s });
            }
        }
        // Distinct inputs map to distinct outputs.
        Ok(out)
    }
}
