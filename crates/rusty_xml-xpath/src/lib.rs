//! XPath 1.0 compile + eval matching libxml2 `xpath.h` (M4).
//!
//! Nothing here recurses once per level of document nesting: axis walks,
//! document-order indexing, node comparison and string values all follow the
//! first-child / next-sibling / parent links, so a document parsed with
//! `XML_PARSE_HUGE` (a million levels) cannot exhaust the stack (#34).
//! Recursion follows only the expression, whose nesting the parser bounds.

#![forbid(unsafe_code)]

use rusty_xml_tree::{Interrupt, Interrupted, NodeId, NodeKind, XmlDoc};
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::OnceLock;

/// libxml2 `xmlXPathObjectType`.
pub const XPATH_UNDEFINED: i32 = 0;
pub const XPATH_NODESET: i32 = 1;
pub const XPATH_BOOLEAN: i32 = 2;
pub const XPATH_NUMBER: i32 = 3;
pub const XPATH_STRING: i32 = 4;

/// libxml2 `xmlXPathError` `XPATH_OP_LIMIT_EXCEEDED`: the evaluation passed
/// the context's [operation limit](XmlXPathContext::set_op_limit).
pub const XPATH_OP_LIMIT_EXCEEDED: i32 = 25;

#[derive(Clone, Debug)]
pub enum XPathObject {
    Undefined,
    NodeSet(Vec<NodeId>),
    Boolean(bool),
    Number(f64),
    String(String),
}

impl XPathObject {
    pub fn object_type(&self) -> i32 {
        match self {
            XPathObject::Undefined => XPATH_UNDEFINED,
            XPathObject::NodeSet(_) => XPATH_NODESET,
            XPathObject::Boolean(_) => XPATH_BOOLEAN,
            XPathObject::Number(_) => XPATH_NUMBER,
            XPathObject::String(_) => XPATH_STRING,
        }
    }
}

/// Why an evaluation failed, as the typed entry points report it
/// ([`xml_xpath_eval_interruptible`]). The plain entry points report the
/// same failures as their `Display` text.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum XPathError {
    /// The expression could not be compiled or evaluated; the text is the
    /// message the plain entry points return.
    Eval(String),
    /// The evaluation passed the context's operation limit
    /// (libxml2 `XPATH_OP_LIMIT_EXCEEDED`).
    OpLimitExceeded,
    /// The caller's [`Interrupt`] fired.
    Interrupted,
}

impl XPathError {
    /// The libxml2 `xmlXPathError` value closest to this error.
    pub fn code(&self) -> i32 {
        match self {
            // XPATH_EXPR_ERROR
            XPathError::Eval(_) => 7,
            XPathError::OpLimitExceeded => XPATH_OP_LIMIT_EXCEEDED,
            // No libxml2 counterpart: XML_ERR_USER_STOP, as the parsers report.
            XPathError::Interrupted => 111,
        }
    }
}

impl fmt::Display for XPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XPathError::Eval(s) => f.write_str(s),
            XPathError::OpLimitExceeded => f.write_str("XPath operation limit exceeded"),
            XPathError::Interrupted => f.write_str("XPath evaluation interrupted"),
        }
    }
}

impl std::error::Error for XPathError {}

impl From<Interrupted> for XPathError {
    fn from(_: Interrupted) -> Self {
        XPathError::Interrupted
    }
}

impl From<String> for XPathError {
    fn from(s: String) -> Self {
        XPathError::Eval(s)
    }
}

impl From<&str> for XPathError {
    fn from(s: &str) -> Self {
        XPathError::Eval(s.to_string())
    }
}

pub struct XmlXPathContext<'a> {
    pub doc: &'a XmlDoc,
    pub node: NodeId,
    pub position: usize,
    pub size: usize,
    ns: Vec<(String, String)>,
    op_limit: u64,
    /// Document-order index, built on first need and kept for the life of
    /// the context: the context borrows the document immutably, so it cannot
    /// change underneath (libxml2's `xmlXPathOrderDocElems`, done lazily).
    index: OnceLock<DocIndex>,
}

impl<'a> XmlXPathContext<'a> {
    /// `xmlXPathNewContext`.
    #[doc(alias = "xmlXPathNewContext")]
    pub fn xml_xpath_new_context(doc: &'a XmlDoc) -> Self {
        Self {
            node: doc.xml_doc_get_root_element().unwrap_or(NodeId::DOCUMENT),
            doc,
            position: 1,
            size: 1,
            ns: Vec::new(),
            op_limit: 0,
            index: OnceLock::new(),
        }
    }

    /// `xmlXPathSetContextNode`.
    #[doc(alias = "xmlXPathSetContextNode")]
    pub fn xml_xpath_set_context_node(&mut self, node: NodeId) {
        self.node = node;
    }

    pub fn register_ns(&mut self, prefix: &str, href: &str) {
        self.ns.push((prefix.to_string(), href.to_string()));
    }

    /// Caps the operations one evaluation may perform; 0, the default, means
    /// no cap (libxml2's `xmlXPathContext.opLimit`).
    ///
    /// An operation is counted, as libxml2 counts them, for each expression
    /// node evaluated, each node an axis visits, each node merged by a union,
    /// and for `translate()` by the product of its string lengths. A plain
    /// evaluation that passes the cap fails with a message; the typed
    /// [`xml_xpath_eval_interruptible`] fails with
    /// [`XPathError::OpLimitExceeded`]. Unlike libxml2, whose `opCount`
    /// accumulates until the caller resets it, the count starts at zero for
    /// every evaluation.
    pub fn set_op_limit(&mut self, limit: u64) {
        self.op_limit = limit;
    }

    /// The operation cap set by [`XmlXPathContext::set_op_limit`] (0: none).
    pub fn op_limit(&self) -> u64 {
        self.op_limit
    }

    /// [`xml_xpath_cmp_nodes`] through this context's document-order index:
    /// O(1) per call once the index exists (the first call, or any
    /// evaluation that needed it, builds it in one pass). libxml2 gets the
    /// same from `xmlXPathOrderDocElems`. Same results, including -2.
    pub fn cmp_nodes(&self, a: NodeId, b: NodeId) -> i32 {
        let idx = self.index();
        let (pa, pb) = (idx.pos(a), idx.pos(b));
        if pa == u32::MAX || pb == u32::MAX {
            // Outside the document tree: only the walk can tell.
            return xml_xpath_cmp_nodes(self.doc, a, b);
        }
        match pa.cmp(&pb) {
            Ordering::Less => 1,
            Ordering::Equal => 0,
            Ordering::Greater => -1,
        }
    }

    fn index(&self) -> &DocIndex {
        self.index.get_or_init(|| DocIndex::build(self.doc))
    }
}

/// `xmlXPathIsNaN`.
#[doc(alias = "xmlXPathIsNaN")]
pub fn xml_xpath_is_nan(v: f64) -> bool {
    v.is_nan()
}

/// `xmlXPathIsInf`.
#[doc(alias = "xmlXPathIsInf")]
pub fn xml_xpath_is_inf(v: f64) -> i32 {
    if v.is_infinite() {
        if v.is_sign_positive() { 1 } else { -1 }
    } else {
        0
    }
}

/// Visits every node reachable from `doc`'s document node in document order:
/// each node, then its attributes, then its children. Iterative, so depth
/// costs no stack. `leave(id, last)` runs once a node's subtree is done, with
/// the last value `enter` returned inside it.
fn walk_doc_order(doc: &XmlDoc, mut enter: impl FnMut(NodeId), mut leave: impl FnMut(NodeId)) {
    let mut cur = NodeId::DOCUMENT;
    loop {
        enter(cur);
        let mut a = doc.first_attr(cur);
        while let Some(x) = a {
            enter(x);
            leave(x);
            a = doc.next_sibling(x);
        }
        if let Some(c) = doc.first_child(cur) {
            cur = c;
            continue;
        }
        loop {
            leave(cur);
            if cur == NodeId::DOCUMENT {
                return;
            }
            if let Some(s) = doc.next_sibling(cur) {
                cur = s;
                break;
            }
            match doc.parent(cur) {
                Some(p) => cur = p,
                None => return,
            }
        }
    }
}

/// `xmlXPathOrderDocElems` — walk document order (preorder).
#[doc(alias = "xmlXPathOrderDocElems")]
pub fn xml_xpath_order_doc_elems(doc: &XmlDoc) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_doc_order(doc, |id| out.push(id), |_| {});
    out
}

/// Document-order position of every node reachable from the document node,
/// and the position of the last node in its subtree; `u32::MAX` for a node
/// the walk does not reach.
struct DocIndex {
    pos: Vec<u32>,
    end: Vec<u32>,
}

impl DocIndex {
    fn build(doc: &XmlDoc) -> DocIndex {
        let n = doc.len();
        let mut pos = vec![u32::MAX; n];
        let mut end = vec![u32::MAX; n];
        let next = Cell::new(0u32);
        walk_doc_order(
            doc,
            |id| {
                pos[id.index()] = next.get();
                next.set(next.get() + 1);
            },
            |id| end[id.index()] = next.get() - 1,
        );
        DocIndex { pos, end }
    }

    #[inline]
    fn pos(&self, id: NodeId) -> u32 {
        self.pos.get(id.index()).copied().unwrap_or(u32::MAX)
    }

    #[inline]
    fn end(&self, id: NodeId) -> u32 {
        self.end.get(id.index()).copied().unwrap_or(u32::MAX)
    }

    /// Sorts `v` into document order and drops duplicates. The sort is
    /// adaptive, so input already in order costs one pass.
    fn sort_unique(&self, v: &mut Vec<NodeId>) {
        v.sort_by_key(|n| (self.pos(*n), n.0));
        v.dedup();
    }
}

/// `xmlXPathCmpNodes`: 1 if `a` comes before `b` in document order, 0 if they
/// are the same node, -1 if `a` comes after `b`, and -2 if they cannot be
/// compared (an id outside the document, or nodes in different trees, such
/// as one in a detached subtree).
///
/// As in libxml2, it walks up to the common ancestor and then along one
/// sibling list (an element's attributes come before its children), so it
/// costs O(depth + siblings) and allocates nothing.
///
/// Before 0.9.1 this returned the opposite sign (-1 when `a` came first) and
/// walked the whole document on every call.
#[doc(alias = "xmlXPathCmpNodes")]
pub fn xml_xpath_cmp_nodes(doc: &XmlDoc, a: NodeId, b: NodeId) -> i32 {
    let n = doc.len();
    if a.index() >= n || b.index() >= n {
        return -2;
    }
    if a == b {
        return 0;
    }
    let depth_root = |mut x: NodeId| {
        let mut d = 0usize;
        while let Some(p) = doc.parent(x) {
            if p == x {
                break;
            }
            d += 1;
            x = p;
        }
        (d, x)
    };
    let (mut da, ra) = depth_root(a);
    let (mut db, rb) = depth_root(b);
    if ra != rb {
        return -2;
    }
    let (mut x, mut y) = (a, b);
    while da > db {
        x = doc.parent(x).unwrap_or(x);
        da -= 1;
    }
    if x == b {
        // b is an ancestor of a (or a's element, for an attribute)
        return -1;
    }
    while db > da {
        y = doc.parent(y).unwrap_or(y);
        db -= 1;
    }
    if y == a {
        return 1;
    }
    loop {
        let (px, py) = (doc.parent(x), doc.parent(y));
        if px == py {
            break;
        }
        match (px, py) {
            (Some(p), Some(q)) => {
                x = p;
                y = q;
            }
            _ => return -2,
        }
    }
    let ax = doc.kind(x) == NodeKind::Attribute;
    let ay = doc.kind(y) == NodeKind::Attribute;
    if ax != ay {
        return if ax { 1 } else { -1 };
    }
    // Search both ways from x at once, so the cost is the distance between
    // the two, whichever comes first.
    let (mut fwd, mut back) = (doc.next_sibling(x), doc.prev_sibling(x));
    while fwd.is_some() || back.is_some() {
        if let Some(s) = fwd {
            if s == y {
                return 1;
            }
            fwd = doc.next_sibling(s);
        }
        if let Some(s) = back {
            if s == y {
                return -1;
            }
            back = doc.prev_sibling(s);
        }
    }
    -2
}

fn parse_xpath(expr: &str) -> Result<Expr, String> {
    let mut p = Parser {
        src: expr.trim(),
        pos: 0,
        depth: 0,
    };
    let ast = p.parse_expr()?;
    p.skip_ws();
    if p.pos < p.src.len() && !p.src[p.pos..].chars().all(|c| c.is_whitespace()) {
        return Err(format!("trailing junk in XPath: {}", &p.src[p.pos..]));
    }
    Ok(ast)
}

/// `xmlXPathEval` / `xmlXPathEvalExpression`.
#[doc(alias = "xmlXPathEval")]
pub fn xml_xpath_eval(expr: &str, ctx: &XmlXPathContext<'_>) -> Result<XPathObject, String> {
    let ast = parse_xpath(expr)?;
    eval_root(&ast, ctx, None).map_err(|e| e.to_string())
}

/// [`xml_xpath_eval`], stoppable from another thread (#39).
///
/// `interrupt` is polled about every [`rusty_xml_tree::INTERRUPT_POLL_EVERY`]
/// operations (the units [`XmlXPathContext::set_op_limit`] counts); once it
/// fires the evaluation returns [`XPathError::Interrupted`]. An evaluation
/// that is not interrupted returns exactly what [`xml_xpath_eval`] returns,
/// with its error text in [`XPathError::Eval`]. Pass
/// [`rusty_xml_tree::NeverInterrupt`] to get typed errors only.
pub fn xml_xpath_eval_interruptible(
    expr: &str,
    ctx: &XmlXPathContext<'_>,
    interrupt: &dyn Interrupt,
) -> Result<XPathObject, XPathError> {
    if interrupt.is_interrupted() {
        return Err(XPathError::Interrupted);
    }
    let ast = parse_xpath(expr)?;
    eval_root(&ast, ctx, Some(interrupt))
}

/// `xmlXPathCastToBoolean`.
#[doc(alias = "xmlXPathCastToBoolean")]
pub fn xml_xpath_cast_to_boolean(o: &XPathObject) -> bool {
    match o {
        XPathObject::Boolean(b) => *b,
        XPathObject::Number(n) => *n != 0.0 && !n.is_nan(),
        XPathObject::String(s) => !s.is_empty(),
        XPathObject::NodeSet(v) => !v.is_empty(),
        XPathObject::Undefined => false,
    }
}

/// `xmlXPathCastToNumber`.
#[doc(alias = "xmlXPathCastToNumber")]
pub fn xml_xpath_cast_to_number(o: &XPathObject, ctx: &XmlXPathContext<'_>) -> f64 {
    match o {
        XPathObject::Number(n) => *n,
        XPathObject::Boolean(b) => {
            if *b { 1.0 } else { 0.0 }
        }
        XPathObject::String(s) => xpath_number(s),
        XPathObject::NodeSet(v) => xml_xpath_cast_to_number(&XPathObject::String(string_val(ctx, v)), ctx),
        XPathObject::Undefined => f64::NAN,
    }
}

/// `xmlXPathCastToString`.
#[doc(alias = "xmlXPathCastToString")]
pub fn xml_xpath_cast_to_string(o: &XPathObject, ctx: &XmlXPathContext<'_>) -> String {
    match o {
        XPathObject::String(s) => s.clone(),
        XPathObject::Boolean(b) => if *b { "true".into() } else { "false".into() },
        XPathObject::Number(n) => number_to_string(*n),
        XPathObject::NodeSet(v) => string_val(ctx, v),
        XPathObject::Undefined => String::new(),
    }
}

fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n.is_sign_positive() {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if n == 0.0 {
        return "0".into();
    }
    if n > i32::MIN as f64 && n < i32::MAX as f64 && n == (n as i32) as f64 {
        return format!("{}", n as i32);
    }
    let abs = n.abs();
    if abs > 1e9 || abs < 1e-5 {
        let s = format!("{n:.14e}");
        let Some(idx) = s.find('e') else { return s };
        let mant = &s[..idx];
        let es = &s[idx..];
        let mut mant = mant.to_string();
        if mant.contains('.') {
            while mant.ends_with('0') {
                mant.pop();
            }
            if mant.ends_with('.') {
                mant.pop();
            }
        }
        let es = if es.starts_with("e-") || es.starts_with("e+") {
            es.to_string()
        } else {
            format!("e+{}", &es[1..])
        };
        format!("{mant}{es}")
    } else {
        let mut s = format!("{n}");
        if let Some(rest) = s.strip_prefix("-") {
            if rest.starts_with('.') {
                s = format!("-0{rest}");
            }
        } else if s.starts_with('.') {
            s = format!("0{s}");
        }
        s
    }
}

fn dump_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n.is_sign_positive() {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if n == 0.0 {
        return "0".into();
    }
    // libxml2 debug dump uses `%0g` (6 significant digits, scientific outside [1e-4, 1e6)).
    let exp = n.abs().log10().floor() as i32;
    if exp < -4 || exp >= 6 {
        let s = format!("{n:.5e}");
        let Some(idx) = s.find('e') else { return s };
        let mant = &s[..idx];
        let es = &s[idx..];
        let mut mant = mant.to_string();
        if mant.contains('.') {
            while mant.ends_with('0') {
                mant.pop();
            }
            if mant.ends_with('.') {
                mant.pop();
            }
        }
        let es = if es.starts_with("e-") || es.starts_with("e+") {
            es.to_string()
        } else {
            format!("e+{}", &es[1..])
        };
        format!("{mant}{es}")
    } else {
        format!("{n}")
    }
}

fn xpath_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn string_val(ctx: &XmlXPathContext<'_>, nodes: &[NodeId]) -> String {
    let Some(&id) = nodes.first() else { return String::new() };
    string_value(ctx.doc, id)
}

/// Dump matching libxml2 `xmlXPathDebugDumpObject` used by `xmllint --xpath` / testXPath.
pub fn xml_xpath_debug_dump(obj: &XPathObject, ctx: &XmlXPathContext<'_>) -> String {
    match obj {
        XPathObject::Undefined => "Object is empty (NULL)\n".into(),
        XPathObject::Number(n) => format!("Object is a number : {}\n", dump_number(*n)),
        XPathObject::Boolean(b) => format!(
            "Object is a Boolean : {}\n",
            if *b { "true" } else { "false" }
        ),
        XPathObject::String(s) => format!("Object is a string : {}\n", debug_string(s)),
        XPathObject::NodeSet(v) => {
            let mut s = format!("Object is a Node Set :\nSet contains {} nodes:\n", v.len());
            for (i, id) in v.iter().enumerate() {
                s.push_str(&format!("{}", i + 1));
                dump_one(ctx.doc, *id, 1, &mut s);
            }
            s
        }
    }
}

fn debug_string(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i >= 40 {
            out.push_str("...");
            break;
        }
        if c.is_whitespace() {
            out.push(' ');
        } else if (c as u32) >= 0x80 {
            out.push_str(&format!("#{:X}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

fn spaces(depth: i32) -> String {
    "  ".repeat(depth.max(0) as usize)
}

fn dump_one(doc: &XmlDoc, id: NodeId, depth: i32, out: &mut String) {
    match doc.kind(id) {
        NodeKind::Document | NodeKind::HtmlDocument => {
            out.push_str(&spaces(depth));
            out.push_str(" /\n");
        }
        NodeKind::Element => {
            out.push_str(&spaces(depth));
            out.push_str("ELEMENT ");
            out.push_str(&doc.qname(id));
            out.push('\n');
            for (pre, href) in doc.ns_defs(id) {
                out.push_str(&spaces(depth + 1));
                match pre {
                    Some(p) => out.push_str(&format!("namespace {p} href={href}\n")),
                    None => out.push_str(&format!("default namespace href={href}\n")),
                }
            }
            let mut a = doc.first_attr(id);
            while let Some(x) = a {
                dump_attr(doc, x, depth + 1, out);
                a = doc.next_sibling(x);
            }
        }
        NodeKind::Attribute => dump_attr(doc, id, depth, out),
        NodeKind::Text => {
            out.push_str(&spaces(depth));
            out.push_str("TEXT\n");
            out.push_str(&spaces(depth + 1));
            out.push_str("content=");
            out.push_str(&debug_string(doc.content(id)));
            out.push('\n');
        }
        NodeKind::CData => {
            out.push_str(&spaces(depth));
            out.push_str("CDATA_SECTION\n");
        }
        NodeKind::Comment => {
            out.push_str(&spaces(depth));
            out.push_str("COMMENT\n");
        }
        NodeKind::Pi => {
            out.push_str(&spaces(depth));
            out.push_str(&format!("PI {}\n", doc.name(id)));
        }
        _ => {
            out.push_str(&spaces(depth));
            out.push_str(&format!("{:?}\n", doc.kind(id)));
        }
    }
}

fn dump_attr(doc: &XmlDoc, id: NodeId, depth: i32, out: &mut String) {
    out.push_str(&spaces(depth));
    out.push_str("ATTRIBUTE ");
    out.push_str(doc.name(id));
    out.push('\n');
    out.push_str(&spaces(depth + 1));
    out.push_str("TEXT\n");
    out.push_str(&spaces(depth + 2));
    out.push_str("content=");
    out.push_str(&debug_string(doc.content(id)));
    out.push('\n');
}

/// `xmllint --xpath` scalar printer (`%0g` / true / false / string).
pub fn xml_xpath_print_lint(obj: &XPathObject) -> Option<String> {
    match obj {
        XPathObject::Undefined => Some(String::new()),
        XPathObject::Number(n) => Some(format!("{}\n", dump_number(*n))),
        XPathObject::Boolean(b) => Some(format!("{}\n", if *b { "true" } else { "false" })),
        XPathObject::String(s) => Some(format!("{s}\n")),
        XPathObject::NodeSet(_) => None,
    }
}
#[doc(alias = "xmlXPathCompile")]
pub fn xml_xpath_compile(expr: &str) -> Result<String, String> {
    let mut p = Parser {
        src: expr.trim(),
        pos: 0,
        depth: 0,
    };
    let _ = p.parse_expr()?;
    Ok(expr.to_string())
}

/// `xmlXPathCompiledEval`.
#[doc(alias = "xmlXPathCompiledEval")]
pub fn xml_xpath_compiled_eval(
    expr: &str,
    ctx: &XmlXPathContext<'_>,
) -> Result<XPathObject, String> {
    xml_xpath_eval(expr, ctx)
}

/* ---------------- parser / AST ---------------- */

#[derive(Clone, Debug)]
enum Expr {
    // A chain of one operator is kept flat (`a or b or c` is one `Or` of
    // three arms, `1+2-3` one `Bin`), and evaluated left to right in a loop.
    // As nested boxes a long chain made a tree as deep as the chain was
    // long, and both evaluating it and dropping it recursed that deep.
    Or(Vec<Expr>),
    And(Vec<Expr>),
    Bin(Box<Expr>, Vec<(BinOp, Expr)>),
    Neg(Box<Expr>),
    Union(Vec<Expr>),
    Path(PathExpr),
    Steps { base: Box<Expr>, steps: Vec<Step> },
    Filter(Box<Expr>, Vec<Expr>),
    Literal(String),
    Number(f64),
    Var(String),
    Fun { name: String, args: Vec<Expr> },
}

#[derive(Clone, Copy, Debug)]
enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Clone, Debug)]
struct PathExpr {
    abs: bool,
    steps: Vec<Step>,
}

#[derive(Clone, Debug)]
struct Step {
    axis: Axis,
    test: NodeTest,
    preds: Vec<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    Child,
    Descendant,
    Parent,
    Ancestor,
    FollowingSibling,
    PrecedingSibling,
    Following,
    Preceding,
    Attribute,
    Namespace,
    SelfAxis,
    DescendantOrSelf,
    AncestorOrSelf,
}

#[derive(Clone, Debug)]
enum NodeTest {
    Star,
    Name(String, Option<String>),
    Node,
    Text,
    Comment,
    Pi(Option<String>),
}

/// Nesting limit for a compiled expression.
///
/// The recursive-descent parser had no bound, so `((((...1...))))` or a chain
/// of `//*|//*|...` around a thousand deep overflowed the stack. That is worse
/// than a panic: a stack overflow ABORTS THE PROCESS and `catch_unwind` cannot
/// recover from it. The nesting also builds a `Box` chain whose recursive
/// `Drop` overflows on its own, so bounding the parse bounds both.
///
/// The limit is 64, not something larger, because one level of XPath nesting
/// costs about THIRTEEN stack frames as it descends the precedence chain
/// (expr -> or -> and -> eq -> rel -> add -> mul -> unary -> union -> path ->
/// relative -> step -> preds -> expr). A limit of 256 was measured to overflow
/// at 250 before the guard could fire. Real expressions nest a handful deep.
const MAX_XPATH_DEPTH: u32 = 64;

struct Parser<'a> {
    src: &'a str,
    pos: usize,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn enter(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_XPATH_DEPTH {
            return Err("expression nested too deeply".into());
        }
        Ok(())
    }
    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.src[self.pos..].chars().next() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }
    fn starts(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }
    fn bump(&mut self, n: usize) {
        self.pos += n;
    }
    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.enter()?;
        let r = self.parse_or();
        self.leave();
        r
    }
    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut arms = vec![self.parse_and()?];
        loop {
            self.skip_ws();
            if self.keyword("or") {
                arms.push(self.parse_and()?);
            } else {
                break;
            }
        }
        Ok(if arms.len() == 1 {
            arms.pop().unwrap_or(Expr::Number(0.0))
        } else {
            Expr::Or(arms)
        })
    }
    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut arms = vec![self.parse_eq()?];
        loop {
            self.skip_ws();
            if self.keyword("and") {
                arms.push(self.parse_eq()?);
            } else {
                break;
            }
        }
        Ok(if arms.len() == 1 {
            arms.pop().unwrap_or(Expr::Number(0.0))
        } else {
            Expr::And(arms)
        })
    }
    fn parse_eq(&mut self) -> Result<Expr, String> {
        let first = self.parse_rel()?;
        let mut rest = Vec::new();
        loop {
            self.skip_ws();
            if self.starts("=") {
                self.bump(1);
                rest.push((BinOp::Eq, self.parse_rel()?));
            } else if self.starts("!=") {
                self.bump(2);
                rest.push((BinOp::Ne, self.parse_rel()?));
            } else {
                break;
            }
        }
        Ok(chain(first, rest))
    }
    fn parse_rel(&mut self) -> Result<Expr, String> {
        let first = self.parse_add()?;
        let mut rest = Vec::new();
        loop {
            self.skip_ws();
            let op = if self.starts("<=") {
                self.bump(2);
                BinOp::Le
            } else if self.starts(">=") {
                self.bump(2);
                BinOp::Ge
            } else if self.starts("<") {
                self.bump(1);
                BinOp::Lt
            } else if self.starts(">") {
                self.bump(1);
                BinOp::Gt
            } else {
                break;
            };
            rest.push((op, self.parse_add()?));
        }
        Ok(chain(first, rest))
    }
    fn parse_add(&mut self) -> Result<Expr, String> {
        let first = self.parse_mul()?;
        let mut rest = Vec::new();
        loop {
            self.skip_ws();
            if self.starts("+") {
                self.bump(1);
                rest.push((BinOp::Add, self.parse_mul()?));
            } else if self.starts("-") && !self.is_name_start_after_minus() {
                self.bump(1);
                rest.push((BinOp::Sub, self.parse_mul()?));
            } else {
                break;
            }
        }
        Ok(chain(first, rest))
    }
    fn is_name_start_after_minus(&self) -> bool {
        false
    }
    fn parse_mul(&mut self) -> Result<Expr, String> {
        let first = self.parse_unary()?;
        let mut rest = Vec::new();
        loop {
            self.skip_ws();
            if self.starts("*") && !self.looks_like_nametest_star() {
                self.bump(1);
                rest.push((BinOp::Mul, self.parse_unary()?));
            } else if self.keyword("div") {
                rest.push((BinOp::Div, self.parse_unary()?));
            } else if self.keyword("mod") {
                rest.push((BinOp::Mod, self.parse_unary()?));
            } else {
                break;
            }
        }
        Ok(chain(first, rest))
    }
    fn looks_like_nametest_star(&self) -> bool {
        false
    }
    fn parse_unary(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        if self.starts("-") {
            self.bump(1);
            self.enter()?;
            let inner = self.parse_unary();
            self.leave();
            Ok(Expr::Neg(Box::new(inner?)))
        } else {
            self.parse_union()
        }
    }
    fn parse_union(&mut self) -> Result<Expr, String> {
        // Flat, like the operator chains: any number of arms, no depth.
        let mut arms = vec![self.parse_path()?];
        loop {
            self.skip_ws();
            if self.starts("|") {
                self.bump(1);
                arms.push(self.parse_path()?);
            } else {
                break;
            }
        }
        Ok(if arms.len() == 1 {
            arms.pop().unwrap_or(Expr::Number(0.0))
        } else {
            Expr::Union(arms)
        })
    }
    fn parse_path(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        if self.looks_like_primary() {
            return self.parse_filter_or_primary();
        }
        let abs = if self.starts("//") {
            self.bump(2);
            let mut steps = vec![Step {
                axis: Axis::DescendantOrSelf,
                test: NodeTest::Node,
                preds: vec![],
            }];
            steps.extend(self.parse_relative()?);
            return Ok(Expr::Path(PathExpr { abs: true, steps }));
        } else if self.starts("/") {
            self.bump(1);
            true
        } else {
            false
        };
        let steps = self.parse_relative()?;
        if abs || !steps.is_empty() {
            Ok(Expr::Path(PathExpr { abs, steps }))
        } else {
            self.parse_filter_or_primary()
        }
    }
    fn looks_like_primary(&self) -> bool {
        let s = self.src[self.pos..].trim_start();
        match s.chars().next() {
            Some('(' | '$' | '\'' | '"') => true,
            Some(d) if d.is_ascii_digit() => true,
            Some('.') => s[1..].chars().next().map(|x| x.is_ascii_digit()).unwrap_or(false),
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                let i = s
                    .find(|ch: char| !(ch.is_ascii_alphanumeric() || "-._:".contains(ch)))
                    .unwrap_or(s.len());
                s[i..].trim_start().starts_with('(')
            }
            _ => false,
        }
    }
    fn parse_relative(&mut self) -> Result<Vec<Step>, String> {
        let mut steps = Vec::new();
        self.skip_ws();
        if self.peek().is_none() || matches!(self.peek(), Some(')' | ',' | '|' | ']')) {
            return Ok(steps);
        }
        if self.starts("/") && !self.starts("//") {
            return Ok(steps);
        }
        steps.push(self.parse_step()?);
        loop {
            self.skip_ws();
            if self.starts("//") {
                self.bump(2);
                steps.push(Step {
                    axis: Axis::DescendantOrSelf,
                    test: NodeTest::Node,
                    preds: vec![],
                });
                steps.push(self.parse_step()?);
            } else if self.starts("/") {
                self.bump(1);
                steps.push(self.parse_step()?);
            } else {
                break;
            }
        }
        Ok(steps)
    }
    fn parse_step(&mut self) -> Result<Step, String> {
        self.skip_ws();
        if self.starts("..") {
            self.bump(2);
            return Ok(Step {
                axis: Axis::Parent,
                test: NodeTest::Node,
                preds: self.parse_preds()?,
            });
        }
        if self.starts(".") {
            self.bump(1);
            return Ok(Step {
                axis: Axis::SelfAxis,
                test: NodeTest::Node,
                preds: self.parse_preds()?,
            });
        }
        if self.starts("@") {
            self.bump(1);
            let test = self.parse_node_test()?;
            return Ok(Step {
                axis: Axis::Attribute,
                test,
                preds: self.parse_preds()?,
            });
        }
        let save = self.pos;
        if let Some(axis) = self.try_axis() {
            let test = self.parse_node_test()?;
            return Ok(Step {
                axis,
                test,
                preds: self.parse_preds()?,
            });
        }
        self.pos = save;
        let test = self.parse_node_test()?;
        Ok(Step {
            axis: Axis::Child,
            test,
            preds: self.parse_preds()?,
        })
    }
    fn try_axis(&mut self) -> Option<Axis> {
        self.skip_ws();
        let names = [
            ("descendant-or-self", Axis::DescendantOrSelf),
            ("following-sibling", Axis::FollowingSibling),
            ("preceding-sibling", Axis::PrecedingSibling),
            ("ancestor-or-self", Axis::AncestorOrSelf),
            ("descendant", Axis::Descendant),
            ("attribute", Axis::Attribute),
            ("following", Axis::Following),
            ("namespace", Axis::Namespace),
            ("preceding", Axis::Preceding),
            ("ancestor", Axis::Ancestor),
            ("parent", Axis::Parent),
            ("child", Axis::Child),
            ("self", Axis::SelfAxis),
        ];
        for (n, ax) in names {
            if self.src[self.pos..].starts_with(n) {
                let after = self.pos + n.len();
                if self.src[after..].starts_with("::") {
                    self.pos = after + 2;
                    return Some(ax);
                }
            }
        }
        None
    }
    fn parse_node_test(&mut self) -> Result<NodeTest, String> {
        self.skip_ws();
        if self.starts("*") {
            self.bump(1);
            return Ok(NodeTest::Star);
        }
        if self.fn_test("node") {
            return Ok(NodeTest::Node);
        }
        if self.fn_test("text") {
            return Ok(NodeTest::Text);
        }
        if self.fn_test("comment") {
            return Ok(NodeTest::Comment);
        }
        if self.src[self.pos..].starts_with("processing-instruction") {
            self.pos += "processing-instruction".len();
            self.skip_ws();
            if !self.starts("(") {
                return Err("expected (".into());
            }
            self.bump(1);
            self.skip_ws();
            let lit = if self.starts("'") || self.starts("\"") {
                Some(self.parse_literal_raw()?)
            } else {
                None
            };
            self.skip_ws();
            if self.starts(")") {
                self.bump(1);
            }
            return Ok(NodeTest::Pi(lit));
        }
        let name = self.parse_qname()?;
        let (prefix, local) = split_qname(&name);
        Ok(NodeTest::Name(local, prefix))
    }
    fn fn_test(&mut self, n: &str) -> bool {
        if self.src[self.pos..].starts_with(n) {
            let after = self.pos + n.len();
            let rest = &self.src[after..];
            let trimmed = rest.trim_start();
            if trimmed.starts_with("()") || trimmed.starts_with('(') {
                // node() 
                if let Some(idx) = rest.find(')') {
                    self.pos = after + idx + 1;
                    return true;
                }
            }
        }
        false
    }
    fn parse_preds(&mut self) -> Result<Vec<Expr>, String> {
        let mut v = Vec::new();
        loop {
            self.skip_ws();
            if self.starts("[") {
                self.bump(1);
                v.push(self.parse_expr()?);
                self.skip_ws();
                if self.starts("]") {
                    self.bump(1);
                } else {
                    return Err("expected ]".into());
                }
            } else {
                break;
            }
        }
        Ok(v)
    }
    fn parse_filter_or_primary(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_primary()?;
        let preds = self.parse_preds()?;
        if !preds.is_empty() {
            e = Expr::Filter(Box::new(e), preds);
        }
        self.skip_ws();
        if self.starts("/") {
            let mut steps = Vec::new();
            if self.starts("//") {
                self.bump(2);
                steps.push(Step {
                    axis: Axis::DescendantOrSelf,
                    test: NodeTest::Node,
                    preds: vec![],
                });
            } else {
                self.bump(1);
            }
            steps.extend(self.parse_relative()?);
            return Ok(Expr::Steps {
                base: Box::new(e),
                steps,
            });
        }
        Ok(e)
    }
    fn parse_primary(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        if self.starts("(") {
            self.bump(1);
            let e = self.parse_expr()?;
            self.skip_ws();
            if self.starts(")") {
                self.bump(1);
            }
            return Ok(e);
        }
        if self.starts("$") {
            self.bump(1);
            return Ok(Expr::Var(self.parse_qname()?));
        }
        if self.starts("'") || self.starts("\"") {
            return Ok(Expr::Literal(self.parse_literal_raw()?));
        }
        if self.peek().map(|c| c.is_ascii_digit() || c == '.').unwrap_or(false) {
            return Ok(Expr::Number(self.parse_number()?));
        }
        let name = self.parse_qname()?;
        self.skip_ws();
        if self.starts("(") {
            self.bump(1);
            let mut args = Vec::new();
            self.skip_ws();
            if !self.starts(")") {
                args.push(self.parse_expr()?);
                loop {
                    self.skip_ws();
                    if self.starts(",") {
                        self.bump(1);
                        args.push(self.parse_expr()?);
                    } else {
                        break;
                    }
                }
            }
            self.skip_ws();
            if self.starts(")") {
                self.bump(1);
            }
            return Ok(Expr::Fun { name, args });
        }
        // name as child nametest path
        let (prefix, local) = split_qname(&name);
        Ok(Expr::Path(PathExpr {
            abs: false,
            steps: vec![Step {
                axis: Axis::Child,
                test: NodeTest::Name(local, prefix),
                preds: vec![],
            }],
        }))
    }
    fn parse_literal_raw(&mut self) -> Result<String, String> {
        let q = self.peek().ok_or("literal")?;
        self.bump(1);
        if let Some(end) = self.src[self.pos..].find(q) {
            let s = self.src[self.pos..self.pos + end].to_string();
            self.pos += end + 1;
            Ok(s)
        } else {
            Err("unterminated string".into())
        }
    }
    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            self.bump(1);
        }
        if self.peek() == Some('.') {
            self.bump(1);
            while self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                self.bump(1);
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.bump(1);
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump(1);
            }
            while self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                self.bump(1);
            }
        }
        let s = &self.src[start..self.pos];
        Ok(s.parse::<f64>().unwrap_or(f64::NAN))
    }
    fn parse_qname(&mut self) -> Result<String, String> {
        self.skip_ws();
        let start = self.pos;
        let first = self.peek().ok_or_else(|| format!("expected name at {}", &self.src[self.pos..]))?;
        if !(first.is_ascii_alphabetic() || first == '_' || first == ':') {
            return Err(format!("expected name at {}", &self.src[self.pos..]));
        }
        self.bump(first.len_utf8());
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ':' {
                self.bump(c.len_utf8());
            } else {
                break;
            }
        }
        Ok(self.src[start..self.pos].to_string())
    }
    fn keyword(&mut self, kw: &str) -> bool {
        self.skip_ws();
        if self.src[self.pos..].starts_with(kw) {
            let after = self.pos + kw.len();
            let next = self.src[after..].chars().next();
            if next.map(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-').unwrap_or(false) {
                return false;
            }
            self.pos = after;
            true
        } else {
            false
        }
    }
}

fn chain(first: Expr, rest: Vec<(BinOp, Expr)>) -> Expr {
    if rest.is_empty() {
        first
    } else {
        Expr::Bin(Box::new(first), rest)
    }
}

fn split_qname(n: &str) -> (Option<String>, String) {
    if let Some((p, l)) = n.split_once(':') {
        (Some(p.to_string()), l.to_string())
    } else {
        (None, n.to_string())
    }
}

/* ---------------- eval ---------------- */

/// Work units between two polls of an [`Interrupt`].
const POLL_EVERY: u64 = rusty_xml_tree::INTERRUPT_POLL_EVERY as u64;

/// State for one evaluation: the context, the operation count, and the
/// interrupt. The count is kept even when nothing limits it; with no limit
/// and no interrupt `next_check` is `u64::MAX`, so a charge is an add and a
/// compare that never fires.
struct Env<'e, 'a> {
    ctx: &'e XmlXPathContext<'a>,
    doc: &'a XmlDoc,
    ops: Cell<u64>,
    next_check: Cell<u64>,
    limit: u64,
    interrupt: Option<&'e dyn Interrupt>,
}

impl<'e, 'a> Env<'e, 'a> {
    fn new(ctx: &'e XmlXPathContext<'a>, interrupt: Option<&'e dyn Interrupt>) -> Self {
        let env = Env {
            ctx,
            doc: ctx.doc,
            ops: Cell::new(0),
            next_check: Cell::new(u64::MAX),
            limit: ctx.op_limit,
            interrupt,
        };
        env.next_check.set(env.next_after(0));
        env
    }

    fn next_after(&self, count: u64) -> u64 {
        let mut n = u64::MAX;
        if self.limit != 0 {
            n = n.min(self.limit.saturating_add(1));
        }
        if self.interrupt.is_some() {
            n = n.min(count.saturating_add(POLL_EVERY));
        }
        n
    }

    /// Counts `n` operations, failing once the count passes the limit or the
    /// interrupt has fired.
    #[inline]
    fn charge(&self, n: u64) -> Result<(), XPathError> {
        let c = self.ops.get().saturating_add(n);
        self.ops.set(c);
        if c >= self.next_check.get() {
            self.checkpoint(c)
        } else {
            Ok(())
        }
    }

    #[cold]
    #[inline(never)]
    fn checkpoint(&self, count: u64) -> Result<(), XPathError> {
        if self.limit != 0 && count > self.limit {
            return Err(XPathError::OpLimitExceeded);
        }
        if let Some(i) = self.interrupt {
            if i.is_interrupted() {
                return Err(XPathError::Interrupted);
            }
        }
        self.next_check.set(self.next_after(count));
        Ok(())
    }
}

/// The context node, position and size an expression is evaluated against.
#[derive(Clone, Copy)]
struct Focus {
    node: NodeId,
    position: usize,
    size: usize,
}

fn eval_root(
    expr: &Expr,
    ctx: &XmlXPathContext<'_>,
    interrupt: Option<&dyn Interrupt>,
) -> Result<XPathObject, XPathError> {
    let env = Env::new(ctx, interrupt);
    let focus = Focus {
        node: ctx.node,
        position: ctx.position,
        size: ctx.size,
    };
    eval(expr, &env, focus)
}

fn eval(expr: &Expr, env: &Env<'_, '_>, f: Focus) -> Result<XPathObject, XPathError> {
    env.charge(1)?;
    match expr {
        Expr::Or(arms) => {
            for a in arms {
                if xml_xpath_cast_to_boolean(&eval(a, env, f)?) {
                    return Ok(XPathObject::Boolean(true));
                }
            }
            Ok(XPathObject::Boolean(false))
        }
        Expr::And(arms) => {
            for a in arms {
                if !xml_xpath_cast_to_boolean(&eval(a, env, f)?) {
                    return Ok(XPathObject::Boolean(false));
                }
            }
            Ok(XPathObject::Boolean(true))
        }
        Expr::Bin(first, rest) => {
            let mut acc = eval(first, env, f)?;
            for (op, e) in rest {
                let r = eval(e, env, f)?;
                acc = apply_bin(*op, &acc, &r, env.ctx);
            }
            Ok(acc)
        }
        Expr::Neg(a) => Ok(XPathObject::Number(-xml_xpath_cast_to_number(
            &eval(a, env, f)?,
            env.ctx,
        ))),
        Expr::Union(arms) => {
            let mut ns = Vec::new();
            for a in arms {
                let s = nodeset(eval(a, env, f)?);
                env.charge(s.len() as u64)?;
                ns.extend(s);
            }
            env.ctx.index().sort_unique(&mut ns);
            Ok(XPathObject::NodeSet(ns))
        }
        Expr::Path(p) => {
            let start = if p.abs { NodeId::DOCUMENT } else { f.node };
            Ok(XPathObject::NodeSet(eval_steps(
                env,
                vec![start],
                &p.steps,
            )?))
        }
        Expr::Steps { base, steps } => {
            let nodes = nodeset(eval(base, env, f)?);
            Ok(XPathObject::NodeSet(eval_steps(env, nodes, steps)?))
        }
        Expr::Filter(e, preds) => {
            let ns = nodeset(eval(e, env, f)?);
            Ok(XPathObject::NodeSet(filter_preds(env, ns, preds)?))
        }
        Expr::Literal(s) => Ok(XPathObject::String(s.clone())),
        Expr::Number(n) => Ok(XPathObject::Number(*n)),
        Expr::Var(_) => Err("variables not bound".into()),
        Expr::Fun { name, args } => eval_fun(name, args, env, f),
    }
}

fn apply_bin(
    op: BinOp,
    l: &XPathObject,
    r: &XPathObject,
    ctx: &XmlXPathContext<'_>,
) -> XPathObject {
    let num = |g: fn(f64, f64) -> f64| {
        XPathObject::Number(g(
            xml_xpath_cast_to_number(l, ctx),
            xml_xpath_cast_to_number(r, ctx),
        ))
    };
    match op {
        BinOp::Eq => XPathObject::Boolean(compare_equal(l, r, ctx, true)),
        BinOp::Ne => XPathObject::Boolean(compare_equal(l, r, ctx, false)),
        BinOp::Lt => XPathObject::Boolean(compare_rel(l, r, ctx, Ordering::Less, false)),
        BinOp::Le => XPathObject::Boolean(compare_rel(l, r, ctx, Ordering::Less, true)),
        BinOp::Gt => XPathObject::Boolean(compare_rel(l, r, ctx, Ordering::Greater, false)),
        BinOp::Ge => XPathObject::Boolean(compare_rel(l, r, ctx, Ordering::Greater, true)),
        BinOp::Add => num(|a, b| a + b),
        BinOp::Sub => num(|a, b| a - b),
        BinOp::Mul => num(|a, b| a * b),
        BinOp::Div => num(|a, b| a / b),
        BinOp::Mod => num(|a, b| a % b),
    }
}

/// `=` (`eq`) or `!=` per XPath 1.0, 3.4 (libxml2 `xmlXPathEqualValues`): a
/// node-set compares true if SOME member's string-value does, so
/// `//b = 'x'` asks whether any `b` is `x`, not whether the first one is.
fn compare_equal(l: &XPathObject, r: &XPathObject, ctx: &XmlXPathContext<'_>, eq: bool) -> bool {
    use XPathObject::{Boolean, NodeSet, Number, String as Str};
    let doc = ctx.doc;
    match (l, r) {
        (NodeSet(a), NodeSet(b)) => {
            if a.is_empty() || b.is_empty() {
                return false;
            }
            // One set of strings, then one lookup per member of the other:
            // linear, where pairwise would be quadratic.
            let sa: HashSet<String> = a.iter().map(|n| string_value(doc, *n)).collect();
            if eq {
                b.iter().any(|n| sa.contains(&string_value(doc, *n)))
            } else if sa.len() > 1 {
                true
            } else {
                let only = sa.iter().next();
                b.iter().any(|n| Some(&string_value(doc, *n)) != only)
            }
        }
        (NodeSet(a), other) | (other, NodeSet(a)) => match other {
            Boolean(b) => (!a.is_empty() == *b) == eq,
            Number(x) => a
                .iter()
                .any(|n| (xpath_number(&string_value(doc, *n)) == *x) == eq),
            Str(s) => a.iter().any(|n| (string_value(doc, *n) == *s) == eq),
            _ => false,
        },
        (Boolean(_), _) | (_, Boolean(_)) => {
            (xml_xpath_cast_to_boolean(l) == xml_xpath_cast_to_boolean(r)) == eq
        }
        (Number(_), _) | (_, Number(_)) => {
            (xml_xpath_cast_to_number(l, ctx) == xml_xpath_cast_to_number(r, ctx)) == eq
        }
        _ => (xml_xpath_cast_to_string(l, ctx) == xml_xpath_cast_to_string(r, ctx)) == eq,
    }
}

/// `<`, `<=`, `>`, `>=` per XPath 1.0, 3.4 (libxml2 `xmlXPathCompareValues`):
/// existential over node-sets, by number. Two node-sets compare by their
/// extremes, which is the same answer as trying every pair.
fn compare_rel(
    l: &XPathObject,
    r: &XPathObject,
    ctx: &XmlXPathContext<'_>,
    ord: Ordering,
    or_eq: bool,
) -> bool {
    use XPathObject::{Boolean, NodeSet};
    let doc = ctx.doc;
    let test = |a: f64, b: f64| match a.partial_cmp(&b) {
        Some(o) if o == ord => true,
        Some(Ordering::Equal) => or_eq,
        _ => false,
    };
    let nums = |v: &[NodeId]| -> Vec<f64> {
        v.iter()
            .map(|n| xpath_number(&string_value(doc, *n)))
            .filter(|x| !x.is_nan())
            .collect()
    };
    let bool_num = |b: bool| if b { 1.0 } else { 0.0 };
    match (l, r) {
        (NodeSet(a), NodeSet(b)) => {
            let (na, nb) = (nums(a), nums(b));
            let min = |v: &[f64]| v.iter().copied().reduce(f64::min);
            let max = |v: &[f64]| v.iter().copied().reduce(f64::max);
            let pair = if ord == Ordering::Less {
                (min(&na), max(&nb))
            } else {
                (max(&na), min(&nb))
            };
            match pair {
                (Some(x), Some(y)) => test(x, y),
                _ => false,
            }
        }
        (NodeSet(a), Boolean(b)) => test(bool_num(!a.is_empty()), bool_num(*b)),
        (Boolean(b), NodeSet(a)) => test(bool_num(*b), bool_num(!a.is_empty())),
        (NodeSet(a), other) => {
            let y = xml_xpath_cast_to_number(other, ctx);
            nums(a).into_iter().any(|x| test(x, y))
        }
        (other, NodeSet(b)) => {
            let x = xml_xpath_cast_to_number(other, ctx);
            nums(b).into_iter().any(|y| test(x, y))
        }
        _ => test(
            xml_xpath_cast_to_number(l, ctx),
            xml_xpath_cast_to_number(r, ctx),
        ),
    }
}

fn nodeset(o: XPathObject) -> Vec<NodeId> {
    match o {
        XPathObject::NodeSet(v) => v,
        _ => vec![],
    }
}

impl Axis {
    /// Reverse axes number their proximity positions from the context node
    /// backwards (XPath 1.0, 2.4).
    fn is_reverse(self) -> bool {
        matches!(
            self,
            Axis::Ancestor | Axis::AncestorOrSelf | Axis::Preceding | Axis::PrecedingSibling
        )
    }
}

/// Applies `steps` to a node-set in document order, returning one.
fn eval_steps(
    env: &Env<'_, '_>,
    mut nodes: Vec<NodeId>,
    steps: &[Step],
) -> Result<Vec<NodeId>, XPathError> {
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];
        // `//x` is descendant-or-self::node()/child::x; with no predicate on
        // either step that is descendant::x, which needs no merge or sort
        // (libxml2 makes the same rewrite).
        if step.axis == Axis::DescendantOrSelf
            && matches!(step.test, NodeTest::Node)
            && step.preds.is_empty()
        {
            if let Some(next) = steps.get(i + 1) {
                if next.axis == Axis::Child && next.preds.is_empty() {
                    nodes = step_nodes(env, &nodes, Axis::Descendant, &next.test, &[])?;
                    i += 2;
                    continue;
                }
            }
        }
        nodes = step_nodes(env, &nodes, step.axis, &step.test, &step.preds)?;
        i += 1;
    }
    Ok(nodes)
}

/// One location step from every node of `ctxs` (document order, no
/// duplicates); the result is in document order with no duplicates.
///
/// Predicates are applied per context node, with proximity positions taken
/// along the axis (reverse axes count backwards), and only then are the
/// per-node results merged. Without predicates the merge has shortcuts that
/// keep it linear: the union of `following` (or `preceding`) over many nodes
/// is that axis from one of them, and nested `descendant` walks are skipped.
fn step_nodes(
    env: &Env<'_, '_>,
    ctxs: &[NodeId],
    axis: Axis,
    test: &NodeTest,
    preds: &[Expr],
) -> Result<Vec<NodeId>, XPathError> {
    let doc = env.doc;
    let mut out = Vec::new();
    if ctxs.is_empty() {
        return Ok(out);
    }
    if preds.is_empty() {
        if ctxs.len() > 1 {
            let simple = || {
                let idx = env.ctx.index();
                ctxs.iter()
                    .all(|&n| idx.pos(n) != u32::MAX && doc.kind(n) != NodeKind::Attribute)
            };
            match axis {
                // following(x) is every node after x's subtree, so the union
                // is following() of the node whose subtree ends first.
                // preceding(x) is every node whose subtree ends before x, so
                // the union is preceding() of the last node.
                Axis::Following | Axis::Preceding if simple() => {
                    let idx = env.ctx.index();
                    let pick = if axis == Axis::Following {
                        ctxs.iter().copied().min_by_key(|&n| idx.end(n))
                    } else {
                        ctxs.iter().copied().max_by_key(|&n| idx.pos(n))
                    };
                    if let Some(n) = pick {
                        axis_into(env, n, axis, test, &mut out)?;
                    }
                    return Ok(out);
                }
                // Context nodes are in document order: one inside a subtree
                // already walked adds nothing new, and the subtrees left are
                // disjoint and in order, so the output needs no sort.
                Axis::Descendant | Axis::DescendantOrSelf if simple() => {
                    let idx = env.ctx.index();
                    let mut covered: Option<u32> = None;
                    for &n in ctxs {
                        if covered.is_some_and(|e| idx.pos(n) <= e) {
                            continue;
                        }
                        axis_into(env, n, axis, test, &mut out)?;
                        covered = Some(idx.end(n));
                    }
                    return Ok(out);
                }
                // The siblings after the first context node under a parent
                // hold those after every later one; before, the last one.
                Axis::FollowingSibling | Axis::PrecedingSibling if simple() => {
                    let mut seen = HashSet::new();
                    let mut take = |n: NodeId| -> Result<(), XPathError> {
                        if seen.insert(doc.parent(n).unwrap_or(n)) {
                            axis_into(env, n, axis, test, &mut out)?;
                        }
                        Ok(())
                    };
                    if axis == Axis::FollowingSibling {
                        ctxs.iter().try_for_each(|&n| take(n))?;
                    } else {
                        ctxs.iter().rev().try_for_each(|&n| take(n))?;
                    }
                    env.ctx.index().sort_unique(&mut out);
                    return Ok(out);
                }
                // Climb from each node only until the chain joins one
                // already climbed.
                Axis::Ancestor | Axis::AncestorOrSelf => {
                    let mut seen = HashSet::new();
                    for &n in ctxs {
                        let start = out.len();
                        let mut p = if axis == Axis::AncestorOrSelf {
                            Some(n)
                        } else {
                            doc.parent(n)
                        };
                        while let Some(x) = p {
                            if !seen.insert(x) {
                                break;
                            }
                            out.push(x);
                            p = doc.parent(x);
                        }
                        keep_matching(env, &mut out, start, test, axis)?;
                    }
                    env.ctx.index().sort_unique(&mut out);
                    return Ok(out);
                }
                _ => {}
            }
        }
        for &n in ctxs {
            axis_into(env, n, axis, test, &mut out)?;
        }
        if ctxs.len() > 1 {
            env.ctx.index().sort_unique(&mut out);
        }
        return Ok(out);
    }
    let reverse = axis.is_reverse();
    let mut s = Vec::new();
    for &n in ctxs {
        s.clear();
        axis_into(env, n, axis, test, &mut s)?;
        if reverse {
            s.reverse();
        }
        let mut kept = filter_preds(env, std::mem::take(&mut s), preds)?;
        if reverse {
            kept.reverse();
        }
        out.extend_from_slice(&kept);
    }
    if ctxs.len() > 1 {
        env.ctx.index().sort_unique(&mut out);
    }
    Ok(out)
}

/// Filters `nodes` (in the order that numbers their proximity positions)
/// through each predicate in turn.
fn filter_preds(
    env: &Env<'_, '_>,
    nodes: Vec<NodeId>,
    preds: &[Expr],
) -> Result<Vec<NodeId>, XPathError> {
    let mut cur = nodes;
    for pred in preds {
        let size = cur.len();
        let mut kept = Vec::new();
        for (i, n) in cur.iter().enumerate() {
            let focus = Focus {
                node: *n,
                position: i + 1,
                size,
            };
            let pass = match eval(pred, env, focus)? {
                // xmlXPathEvaluatePredicateResult: an exact comparison, so
                // [1.5] selects nothing.
                XPathObject::Number(num) => num == focus.position as f64,
                other => xml_xpath_cast_to_boolean(&other),
            };
            if pass {
                kept.push(*n);
            }
        }
        cur = kept;
    }
    Ok(cur)
}

/// Appends to `out` the nodes on `axis` from `n` that pass `test`, in
/// document order, and charges the nodes visited.
fn axis_into(
    env: &Env<'_, '_>,
    n: NodeId,
    axis: Axis,
    test: &NodeTest,
    out: &mut Vec<NodeId>,
) -> Result<(), XPathError> {
    let doc = env.doc;
    let is_attr = doc.kind(n) == NodeKind::Attribute;
    let start = out.len();
    match axis {
        Axis::SelfAxis => out.push(n),
        Axis::Child => {
            if !is_attr {
                let mut c = doc.first_child(n);
                while let Some(x) = c {
                    out.push(x);
                    c = doc.next_sibling(x);
                }
            }
        }
        Axis::Attribute => {
            if !is_attr {
                let mut a = doc.first_attr(n);
                while let Some(x) = a {
                    out.push(x);
                    a = doc.next_sibling(x);
                }
            }
        }
        Axis::Parent => {
            if let Some(p) = doc.parent(n) {
                out.push(p);
            }
        }
        Axis::Ancestor | Axis::AncestorOrSelf => {
            if axis == Axis::AncestorOrSelf {
                out.push(n);
            }
            let mut p = doc.parent(n);
            while let Some(x) = p {
                out.push(x);
                p = doc.parent(x);
            }
            out[start..].reverse();
        }
        Axis::Descendant | Axis::DescendantOrSelf => {
            if axis == Axis::DescendantOrSelf {
                out.push(n);
            }
            if !is_attr {
                collect_desc(doc, n, out);
            }
        }
        Axis::FollowingSibling => {
            if !is_attr {
                let mut s = doc.next_sibling(n);
                while let Some(x) = s {
                    out.push(x);
                    s = doc.next_sibling(x);
                }
            }
        }
        Axis::PrecedingSibling => {
            if !is_attr {
                let mut s = doc.prev_sibling(n);
                while let Some(x) = s {
                    out.push(x);
                    s = doc.prev_sibling(x);
                }
                out[start..].reverse();
            }
        }
        Axis::Following => {
            // An attribute's following nodes start with its element's
            // children; neither axis ever holds attributes.
            let mut cur = n;
            if is_attr {
                match doc.parent(n) {
                    Some(e) => {
                        collect_desc(doc, e, out);
                        cur = e;
                    }
                    None => return env.charge(1),
                }
            }
            loop {
                let mut s = doc.next_sibling(cur);
                while let Some(x) = s {
                    out.push(x);
                    collect_desc(doc, x, out);
                    s = doc.next_sibling(x);
                }
                match doc.parent(cur) {
                    Some(p) => cur = p,
                    None => break,
                }
            }
        }
        Axis::Preceding => {
            // Built backwards (nearest first), then reversed: for the node
            // and each ancestor, every earlier sibling's subtree. Ancestors
            // themselves are never added.
            let mut cur = n;
            if is_attr {
                match doc.parent(n) {
                    Some(e) => cur = e,
                    None => return env.charge(1),
                }
            }
            let mut sub = Vec::new();
            loop {
                let mut s = doc.prev_sibling(cur);
                while let Some(x) = s {
                    sub.clear();
                    sub.push(x);
                    collect_desc(doc, x, &mut sub);
                    out.extend(sub.iter().rev());
                    s = doc.prev_sibling(x);
                }
                match doc.parent(cur) {
                    Some(p) => cur = p,
                    None => break,
                }
            }
            out[start..].reverse();
        }
        // Namespace nodes are not first-class in this tree.
        Axis::Namespace => {}
    }
    keep_matching(env, out, start, test, axis)
}

/// Charges the nodes an axis visited (`out[start..]`) and keeps those that
/// pass `test`.
fn keep_matching(
    env: &Env<'_, '_>,
    out: &mut Vec<NodeId>,
    start: usize,
    test: &NodeTest,
    axis: Axis,
) -> Result<(), XPathError> {
    let doc = env.doc;
    env.charge((out.len() - start) as u64 + 1)?;
    let ns = &env.ctx.ns;
    let mut w = start;
    for r in start..out.len() {
        let id = out[r];
        if node_test(doc, id, test, ns, axis) {
            out[w] = id;
            w += 1;
        }
    }
    out.truncate(w);
    Ok(())
}

/// Appends every descendant of `root` (not `root` itself, no attributes) in
/// document order. Follows the links instead of recursing (#34).
fn collect_desc(doc: &XmlDoc, root: NodeId, out: &mut Vec<NodeId>) {
    let Some(mut cur) = doc.first_child(root) else {
        return;
    };
    loop {
        out.push(cur);
        if let Some(c) = doc.first_child(cur) {
            cur = c;
            continue;
        }
        loop {
            if let Some(s) = doc.next_sibling(cur) {
                cur = s;
                break;
            }
            match doc.parent(cur) {
                Some(p) if p != root => cur = p,
                _ => return,
            }
        }
    }
}

/// XPath string-value of a node: its own text for a leaf, else the text of
/// every descendant text node reached through elements, as
/// `XmlDoc::xml_node_get_content` gives it, but without recursion.
fn string_value(doc: &XmlDoc, id: NodeId) -> String {
    match doc.kind(id) {
        NodeKind::Text
        | NodeKind::CData
        | NodeKind::Comment
        | NodeKind::Pi
        | NodeKind::Attribute => doc.content(id).to_string(),
        _ => {
            let mut out = String::new();
            let Some(mut cur) = doc.first_child(id) else {
                return out;
            };
            loop {
                match doc.kind(cur) {
                    NodeKind::Text | NodeKind::CData => out.push_str(doc.content(cur)),
                    NodeKind::Element => {
                        if let Some(c) = doc.first_child(cur) {
                            cur = c;
                            continue;
                        }
                    }
                    _ => {}
                }
                loop {
                    if let Some(s) = doc.next_sibling(cur) {
                        cur = s;
                        break;
                    }
                    match doc.parent(cur) {
                        Some(p) if p != id => cur = p,
                        _ => return out,
                    }
                }
            }
        }
    }
}

fn node_test(
    doc: &XmlDoc,
    id: NodeId,
    test: &NodeTest,
    ns: &[(String, String)],
    axis: Axis,
) -> bool {
    match test {
        NodeTest::Node => true,
        NodeTest::Star => {
            if axis == Axis::Attribute {
                doc.kind(id) == NodeKind::Attribute
            } else {
                doc.kind(id) == NodeKind::Element
            }
        }
        NodeTest::Text => doc.kind(id) == NodeKind::Text || doc.kind(id) == NodeKind::CData,
        NodeTest::Comment => doc.kind(id) == NodeKind::Comment,
        NodeTest::Pi(t) => {
            doc.kind(id) == NodeKind::Pi && t.as_deref().map(|x| x == doc.name(id)).unwrap_or(true)
        }
        NodeTest::Name(local, prefix) => {
            let kind_ok = if axis == Axis::Attribute {
                doc.kind(id) == NodeKind::Attribute
            } else {
                doc.kind(id) == NodeKind::Element
            };
            if !kind_ok || doc.name(id) != local {
                return false;
            }
            if let Some(p) = prefix {
                let href = ns.iter().find(|(a, _)| a == p).map(|(_, h)| h.as_str());
                match href {
                    Some(h) => doc.ns_uri(id) == Some(h),
                    None => doc.prefix(id) == Some(p.as_str()),
                }
            } else {
                true
            }
        }
    }
}

fn eval_fun(
    name: &str,
    args: &[Expr],
    env: &Env<'_, '_>,
    f: Focus,
) -> Result<XPathObject, XPathError> {
    let ctx = env.ctx;
    let doc = env.doc;
    // A missing argument is libxml2's XPATH_INVALID_ARITY. Indexing `args`
    // directly panicked on `count()` or `substring('a')`, which aborts the
    // caller's thread on an expression it may not control.
    let ev = |i: usize| match args.get(i) {
        Some(a) => eval(a, env, f),
        None => Err(XPathError::Eval(format!(
            "Invalid number of arguments to {name}()"
        ))),
    };
    let local = name.rsplit(':').next().unwrap_or(name);
    match local {
        "true" => Ok(XPathObject::Boolean(true)),
        "false" => Ok(XPathObject::Boolean(false)),
        "not" => Ok(XPathObject::Boolean(!xml_xpath_cast_to_boolean(&ev(0)?))),
        "boolean" => Ok(XPathObject::Boolean(xml_xpath_cast_to_boolean(&ev(0)?))),
        "number" => {
            let o = if args.is_empty() {
                XPathObject::NodeSet(vec![f.node])
            } else {
                ev(0)?
            };
            Ok(XPathObject::Number(xml_xpath_cast_to_number(&o, ctx)))
        }
        "string" => {
            let o = if args.is_empty() {
                XPathObject::NodeSet(vec![f.node])
            } else {
                ev(0)?
            };
            Ok(XPathObject::String(xml_xpath_cast_to_string(&o, ctx)))
        }
        "last" => Ok(XPathObject::Number(f.size as f64)),
        "position" => Ok(XPathObject::Number(f.position as f64)),
        "count" => Ok(XPathObject::Number(nodeset(ev(0)?).len() as f64)),
        "local-name" => {
            let ns = if args.is_empty() {
                vec![f.node]
            } else {
                nodeset(ev(0)?)
            };
            Ok(XPathObject::String(
                ns.first()
                    .map(|id| doc.name(*id).to_string())
                    .unwrap_or_default(),
            ))
        }
        "name" => {
            let ns = if args.is_empty() {
                vec![f.node]
            } else {
                nodeset(ev(0)?)
            };
            Ok(XPathObject::String(
                ns.first().map(|id| doc.qname(*id)).unwrap_or_default(),
            ))
        }
        "namespace-uri" => {
            let ns = if args.is_empty() {
                vec![f.node]
            } else {
                nodeset(ev(0)?)
            };
            Ok(XPathObject::String(
                ns.first()
                    .and_then(|id| doc.ns_uri(*id).map(str::to_string))
                    .unwrap_or_default(),
            ))
        }
        "concat" => {
            let mut s = String::new();
            for a in args {
                s.push_str(&xml_xpath_cast_to_string(&eval(a, env, f)?, ctx));
            }
            Ok(XPathObject::String(s))
        }
        "starts-with" => {
            let a = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let b = xml_xpath_cast_to_string(&ev(1)?, ctx);
            Ok(XPathObject::Boolean(a.starts_with(&b)))
        }
        "contains" => {
            let a = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let b = xml_xpath_cast_to_string(&ev(1)?, ctx);
            Ok(XPathObject::Boolean(a.contains(&b)))
        }
        "substring-before" => {
            let a = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let b = xml_xpath_cast_to_string(&ev(1)?, ctx);
            Ok(XPathObject::String(
                a.split_once(&b)
                    .map(|(x, _)| x.to_string())
                    .unwrap_or_default(),
            ))
        }
        "substring-after" => {
            let a = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let b = xml_xpath_cast_to_string(&ev(1)?, ctx);
            Ok(XPathObject::String(
                a.split_once(&b)
                    .map(|(_, x)| x.to_string())
                    .unwrap_or_default(),
            ))
        }
        "substring" => {
            let s = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let start = xpath_round(xml_xpath_cast_to_number(&ev(1)?, ctx));
            let end = if args.len() > 2 {
                start + xpath_round(xml_xpath_cast_to_number(&ev(2)?, ctx))
            } else {
                f64::INFINITY
            };
            let out: String = s
                .chars()
                .enumerate()
                .filter(|(i, _)| {
                    let pos = (*i as f64) + 1.0;
                    pos >= start && pos < end
                })
                .map(|(_, c)| c)
                .collect();
            Ok(XPathObject::String(out))
        }
        "string-length" => {
            let s = if args.is_empty() {
                string_value(doc, f.node)
            } else {
                xml_xpath_cast_to_string(&ev(0)?, ctx)
            };
            Ok(XPathObject::Number(s.chars().count() as f64))
        }
        "normalize-space" => {
            let s = if args.is_empty() {
                string_value(doc, f.node)
            } else {
                xml_xpath_cast_to_string(&ev(0)?, ctx)
            };
            Ok(XPathObject::String(
                s.split_whitespace().collect::<Vec<_>>().join(" "),
            ))
        }
        "translate" => {
            let s = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let from = xml_xpath_cast_to_string(&ev(1)?, ctx);
            let to: Vec<char> = xml_xpath_cast_to_string(&ev(2)?, ctx).chars().collect();
            // libxml2 charges translate() as quadratic, which its loop is;
            // the same charge keeps an op limit refusing the same calls.
            if env.limit != 0 && !s.is_empty() && !from.is_empty() {
                let f1 = s.len() as u64 / 10 + 1;
                let f2 = from.len() as u64 / 10 + 1;
                env.charge(f1.saturating_mul(f2))?;
            }
            // First occurrence in `from` wins; a char past the end of `to`
            // is deleted. One map lookup per char instead of a scan of
            // `from`, which made this O(len(s) * len(from)).
            let mut map: HashMap<char, Option<char>> = HashMap::new();
            for (i, c) in from.chars().enumerate() {
                map.entry(c).or_insert_with(|| to.get(i).copied());
            }
            let out: String = s
                .chars()
                .filter_map(|c| match map.get(&c) {
                    Some(m) => *m,
                    None => Some(c),
                })
                .collect();
            Ok(XPathObject::String(out))
        }
        "floor" => Ok(XPathObject::Number(
            xml_xpath_cast_to_number(&ev(0)?, ctx).floor(),
        )),
        "ceiling" => Ok(XPathObject::Number(
            xml_xpath_cast_to_number(&ev(0)?, ctx).ceil(),
        )),
        "round" => {
            let n = xml_xpath_cast_to_number(&ev(0)?, ctx);
            Ok(XPathObject::Number(xpath_round(n)))
        }
        "sum" => {
            let ns = nodeset(ev(0)?);
            let mut t = 0.0;
            for id in ns {
                t += xpath_number(&string_value(doc, id));
            }
            Ok(XPathObject::Number(t))
        }
        "id" => {
            let ids = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let toks: HashSet<&str> = ids.split_whitespace().collect();
            env.charge(toks.len() as u64)?;
            let mut found = Vec::new();
            if !toks.is_empty() {
                // One pass over the document, not one per token.
                let mut els = Vec::new();
                walk_doc_order(doc, |id| els.push(id), |_| {});
                env.charge(els.len() as u64)?;
                for id in els {
                    if doc.kind(id) == NodeKind::Element {
                        if let Some(v) = doc.xml_get_prop(id, "id") {
                            if toks.contains(v.as_str()) {
                                found.push(id);
                            }
                        }
                    }
                }
            }
            Ok(XPathObject::NodeSet(found))
        }
        "lang" => {
            let want = xml_xpath_cast_to_string(&ev(0)?, ctx).to_ascii_lowercase();
            let mut cur = Some(f.node);
            let mut lang = None;
            while let Some(id) = cur {
                if let Some(l) = doc.xml_get_prop(id, "lang") {
                    lang = Some(l);
                    break;
                }
                cur = doc.parent(id);
            }
            let ok = lang
                .map(|l| {
                    let l = l.to_ascii_lowercase();
                    l == want || l.starts_with(&format!("{want}-"))
                })
                .unwrap_or(false);
            Ok(XPathObject::Boolean(ok))
        }
        _ => Err(format!("unknown function {name}").into()),
    }
}

fn xpath_round(n: f64) -> f64 {
    if n.is_nan() || n.is_infinite() {
        return n;
    }
    if n == 0.0 {
        return n;
    }
    // XPath round: floor(n+0.5) except negative half toward +inf
    if n >= 0.0 {
        (n + 0.5).floor()
    } else {
        let f = (n.abs() + 0.5).floor();
        if (n.abs() - n.abs().floor()) == 0.5 {
            -n.abs().floor()
        } else {
            -f
        }
    }
}
