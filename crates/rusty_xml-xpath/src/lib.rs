//! XPath 1.0 compile + eval matching libxml2 `xpath.h` (M4).

#![forbid(unsafe_code)]

use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};
use std::cmp::Ordering;

/// libxml2 `xmlXPathObjectType`.
pub const XPATH_UNDEFINED: i32 = 0;
pub const XPATH_NODESET: i32 = 1;
pub const XPATH_BOOLEAN: i32 = 2;
pub const XPATH_NUMBER: i32 = 3;
pub const XPATH_STRING: i32 = 4;

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

pub struct XmlXPathContext<'a> {
    pub doc: &'a XmlDoc,
    pub node: NodeId,
    pub position: usize,
    pub size: usize,
    ns: Vec<(String, String)>,
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

/// `xmlXPathOrderDocElems` — walk document order (preorder).
#[doc(alias = "xmlXPathOrderDocElems")]
pub fn xml_xpath_order_doc_elems(doc: &XmlDoc) -> Vec<NodeId> {
    let mut out = Vec::new();
    fn walk(doc: &XmlDoc, id: NodeId, out: &mut Vec<NodeId>) {
        out.push(id);
        let mut a = doc.first_attr(id);
        while let Some(x) = a {
            out.push(x);
            a = doc.next_sibling(x);
        }
        let mut c = doc.first_child(id);
        while let Some(x) = c {
            walk(doc, x, out);
            c = doc.next_sibling(x);
        }
    }
    walk(doc, NodeId::DOCUMENT, &mut out);
    out
}

/// `xmlXPathCmpNodes`.
#[doc(alias = "xmlXPathCmpNodes")]
pub fn xml_xpath_cmp_nodes(doc: &XmlDoc, a: NodeId, b: NodeId) -> i32 {
    let order = xml_xpath_order_doc_elems(doc);
    let ia = order.iter().position(|n| *n == a);
    let ib = order.iter().position(|n| *n == b);
    match (ia, ib) {
        (Some(x), Some(y)) => x.cmp(&y) as i32,
        _ => a.0.cmp(&b.0) as i32,
    }
}

/// `xmlXPathEval` / `xmlXPathEvalExpression`.
#[doc(alias = "xmlXPathEval")]
pub fn xml_xpath_eval(expr: &str, ctx: &XmlXPathContext<'_>) -> Result<XPathObject, String> {
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
    eval(&ast, ctx)
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
    ctx.doc.xml_node_get_content(id)
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
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>, bool),
    Rel(Box<Expr>, Box<Expr>, Ordering, bool),
    Add(Box<Expr>, Box<Expr>, bool),
    Mul(Box<Expr>, Box<Expr>, char),
    Neg(Box<Expr>),
    Union(Box<Expr>, Box<Expr>),
    Path(PathExpr),
    Steps { base: Box<Expr>, steps: Vec<Step> },
    Filter(Box<Expr>, Vec<Expr>),
    Literal(String),
    Number(f64),
    Var(String),
    Fun { name: String, args: Vec<Expr> },
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
        let mut e = self.parse_and()?;
        loop {
            self.skip_ws();
            if self.keyword("or") {
                let r = self.parse_and()?;
                e = Expr::Or(Box::new(e), Box::new(r));
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_eq()?;
        loop {
            self.skip_ws();
            if self.keyword("and") {
                let r = self.parse_eq()?;
                e = Expr::And(Box::new(e), Box::new(r));
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn parse_eq(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_rel()?;
        loop {
            self.skip_ws();
            if self.starts("=") {
                self.bump(1);
                let r = self.parse_rel()?;
                e = Expr::Eq(Box::new(e), Box::new(r), true);
            } else if self.starts("!=") {
                self.bump(2);
                let r = self.parse_rel()?;
                e = Expr::Eq(Box::new(e), Box::new(r), false);
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn parse_rel(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_add()?;
        loop {
            self.skip_ws();
            if self.starts("<=") {
                self.bump(2);
                let r = self.parse_add()?;
                e = Expr::Rel(Box::new(e), Box::new(r), Ordering::Less, true);
            } else if self.starts(">=") {
                self.bump(2);
                let r = self.parse_add()?;
                e = Expr::Rel(Box::new(e), Box::new(r), Ordering::Greater, true);
            } else if self.starts("<") {
                self.bump(1);
                let r = self.parse_add()?;
                e = Expr::Rel(Box::new(e), Box::new(r), Ordering::Less, false);
            } else if self.starts(">") {
                self.bump(1);
                let r = self.parse_add()?;
                e = Expr::Rel(Box::new(e), Box::new(r), Ordering::Greater, false);
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_mul()?;
        loop {
            self.skip_ws();
            if self.starts("+") {
                self.bump(1);
                let r = self.parse_mul()?;
                e = Expr::Add(Box::new(e), Box::new(r), true);
            } else if self.starts("-") && !self.is_name_start_after_minus() {
                self.bump(1);
                let r = self.parse_mul()?;
                e = Expr::Add(Box::new(e), Box::new(r), false);
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn is_name_start_after_minus(&self) -> bool {
        false
    }
    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_unary()?;
        loop {
            self.skip_ws();
            if self.starts("*") && !self.looks_like_nametest_star() {
                self.bump(1);
                let r = self.parse_unary()?;
                e = Expr::Mul(Box::new(e), Box::new(r), '*');
            } else if self.keyword("div") {
                let r = self.parse_unary()?;
                e = Expr::Mul(Box::new(e), Box::new(r), 'd');
            } else if self.keyword("mod") {
                let r = self.parse_unary()?;
                e = Expr::Mul(Box::new(e), Box::new(r), 'm');
            } else {
                break;
            }
        }
        Ok(e)
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
        let mut e = self.parse_path()?;
        // The loop is iterative but each step adds a Box level, so the tree it
        // builds is as deep as the chain is long -- and dropping it recurses.
        let mut arms = 0u32;
        loop {
            self.skip_ws();
            if self.starts("|") {
                self.bump(1);
                arms += 1;
                if arms > MAX_XPATH_DEPTH {
                    return Err("expression nested too deeply".into());
                }
                let r = self.parse_path()?;
                e = Expr::Union(Box::new(e), Box::new(r));
            } else {
                break;
            }
        }
        Ok(e)
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

fn split_qname(n: &str) -> (Option<String>, String) {
    if let Some((p, l)) = n.split_once(':') {
        (Some(p.to_string()), l.to_string())
    } else {
        (None, n.to_string())
    }
}

/* ---------------- eval ---------------- */

fn eval(expr: &Expr, ctx: &XmlXPathContext<'_>) -> Result<XPathObject, String> {
    match expr {
        Expr::Or(a, b) => {
            let x = xml_xpath_cast_to_boolean(&eval(a, ctx)?);
            if x {
                Ok(XPathObject::Boolean(true))
            } else {
                Ok(XPathObject::Boolean(xml_xpath_cast_to_boolean(&eval(b, ctx)?)))
            }
        }
        Expr::And(a, b) => {
            let x = xml_xpath_cast_to_boolean(&eval(a, ctx)?);
            if !x {
                Ok(XPathObject::Boolean(false))
            } else {
                Ok(XPathObject::Boolean(xml_xpath_cast_to_boolean(&eval(b, ctx)?)))
            }
        }
        Expr::Eq(a, b, eq) => {
            let l = eval(a, ctx)?;
            let r = eval(b, ctx)?;
            Ok(XPathObject::Boolean(compare_eq(&l, &r, ctx) == *eq))
        }
        Expr::Rel(a, b, ord, or_eq) => {
            let ln = xml_xpath_cast_to_number(&eval(a, ctx)?, ctx);
            let rn = xml_xpath_cast_to_number(&eval(b, ctx)?, ctx);
            let c = ln.partial_cmp(&rn);
            let ok = match c {
                Some(o) if o == *ord => true,
                Some(Ordering::Equal) if *or_eq => true,
                _ => false,
            };
            Ok(XPathObject::Boolean(ok))
        }
        Expr::Add(a, b, plus) => {
            let ln = xml_xpath_cast_to_number(&eval(a, ctx)?, ctx);
            let rn = xml_xpath_cast_to_number(&eval(b, ctx)?, ctx);
            Ok(XPathObject::Number(if *plus { ln + rn } else { ln - rn }))
        }
        Expr::Mul(a, b, op) => {
            let ln = xml_xpath_cast_to_number(&eval(a, ctx)?, ctx);
            let rn = xml_xpath_cast_to_number(&eval(b, ctx)?, ctx);
            Ok(XPathObject::Number(match op {
                '*' => ln * rn,
                'd' => ln / rn,
                _ => ln % rn,
            }))
        }
        Expr::Neg(a) => Ok(XPathObject::Number(-xml_xpath_cast_to_number(&eval(a, ctx)?, ctx))),
        Expr::Union(a, b) => {
            let mut ns = nodeset(eval(a, ctx)?);
            ns.extend(nodeset(eval(b, ctx)?));
            Ok(XPathObject::NodeSet(sort_unique(ctx.doc, ns)))
        }
        Expr::Path(p) => eval_path(p, ctx),
        Expr::Steps { base, steps } => {
            let mut nodes = nodeset(eval(base, ctx)?);
            for step in steps {
                let mut next = Vec::new();
                for n in nodes {
                    next.extend(axis_nodes(ctx.doc, n, step.axis, &step.test, &ctx.ns));
                }
                nodes = filter_preds(ctx, sort_unique(ctx.doc, next), &step.preds)?;
            }
            Ok(XPathObject::NodeSet(nodes))
        }
        Expr::Filter(e, preds) => {
            let o = eval(e, ctx)?;
            let ns = nodeset(o);
            Ok(XPathObject::NodeSet(filter_preds(ctx, ns, preds)?))
        }
        Expr::Literal(s) => Ok(XPathObject::String(s.clone())),
        Expr::Number(n) => Ok(XPathObject::Number(*n)),
        Expr::Var(_) => Err("variables not bound".into()),
        Expr::Fun { name, args } => eval_fun(name, args, ctx),
    }
}

fn compare_eq(l: &XPathObject, r: &XPathObject, ctx: &XmlXPathContext<'_>) -> bool {
    match (l, r) {
        (XPathObject::Boolean(_), _) | (_, XPathObject::Boolean(_)) => {
            xml_xpath_cast_to_boolean(l) == xml_xpath_cast_to_boolean(r)
        }
        (XPathObject::Number(_), _) | (_, XPathObject::Number(_)) => {
            xml_xpath_cast_to_number(l, ctx) == xml_xpath_cast_to_number(r, ctx)
        }
        _ => xml_xpath_cast_to_string(l, ctx) == xml_xpath_cast_to_string(r, ctx),
    }
}

fn nodeset(o: XPathObject) -> Vec<NodeId> {
    match o {
        XPathObject::NodeSet(v) => v,
        _ => vec![],
    }
}

fn sort_unique(doc: &XmlDoc, mut v: Vec<NodeId>) -> Vec<NodeId> {
    let order = xml_xpath_order_doc_elems(doc);
    v.sort_by_key(|n| order.iter().position(|x| x == n).unwrap_or(usize::MAX));
    v.dedup();
    v
}

fn eval_path(path: &PathExpr, ctx: &XmlXPathContext<'_>) -> Result<XPathObject, String> {
    let mut nodes = if path.abs {
        vec![NodeId::DOCUMENT]
    } else {
        vec![ctx.node]
    };
    if path.steps.is_empty() && path.abs {
        return Ok(XPathObject::NodeSet(nodes));
    }
    for step in &path.steps {
        let mut next = Vec::new();
        for n in nodes {
            next.extend(axis_nodes(ctx.doc, n, step.axis, &step.test, &ctx.ns));
        }
        nodes = filter_preds(ctx, sort_unique(ctx.doc, next), &step.preds)?;
    }
    Ok(XPathObject::NodeSet(nodes))
}

fn filter_preds(
    ctx: &XmlXPathContext<'_>,
    nodes: Vec<NodeId>,
    preds: &[Expr],
) -> Result<Vec<NodeId>, String> {
    let mut cur = nodes;
    for pred in preds {
        let size = cur.len();
        let mut kept = Vec::new();
        for (i, n) in cur.iter().enumerate() {
            let c2 = XmlXPathContext {
                doc: ctx.doc,
                node: *n,
                position: i + 1,
                size,
                ns: ctx.ns.clone(),
            };
            let v = eval(pred, &c2)?;
            let pass = match v {
                XPathObject::Number(num) => (num as usize) == c2.position,
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

fn axis_nodes(
    doc: &XmlDoc,
    n: NodeId,
    axis: Axis,
    test: &NodeTest,
    ns: &[(String, String)],
) -> Vec<NodeId> {
    let mut raw = Vec::new();
    match axis {
        Axis::SelfAxis => raw.push(n),
        Axis::Child => {
            let mut c = doc.first_child(n);
            while let Some(x) = c {
                raw.push(x);
                c = doc.next_sibling(x);
            }
        }
        Axis::Attribute => {
            let mut a = doc.first_attr(n);
            while let Some(x) = a {
                raw.push(x);
                a = doc.next_sibling(x);
            }
        }
        Axis::Parent => {
            if let Some(p) = doc.parent(n) {
                raw.push(p);
            }
        }
        Axis::Ancestor => {
            let mut p = doc.parent(n);
            while let Some(x) = p {
                raw.push(x);
                p = doc.parent(x);
            }
        }
        Axis::AncestorOrSelf => {
            raw.push(n);
            let mut p = doc.parent(n);
            while let Some(x) = p {
                raw.push(x);
                p = doc.parent(x);
            }
        }
        Axis::Descendant => collect_desc(doc, n, &mut raw, false),
        Axis::DescendantOrSelf => collect_desc(doc, n, &mut raw, true),
        Axis::FollowingSibling => {
            let mut s = doc.next_sibling(n);
            while let Some(x) = s {
                raw.push(x);
                s = doc.next_sibling(x);
            }
        }
        Axis::PrecedingSibling => {
            let mut s = doc.prev_sibling(n);
            let mut v = Vec::new();
            while let Some(x) = s {
                v.push(x);
                s = doc.prev_sibling(x);
            }
            v.reverse();
            raw.extend(v);
        }
        Axis::Following => {
            let mut cur = n;
            loop {
                if let Some(s) = doc.next_sibling(cur) {
                    collect_desc(doc, s, &mut raw, true);
                    let mut t = doc.next_sibling(s);
                    while let Some(x) = t {
                        collect_desc(doc, x, &mut raw, true);
                        t = doc.next_sibling(x);
                    }
                    cur = s;
                    // climb for more following
                    if let Some(p) = doc.parent(cur) {
                        cur = p;
                        continue;
                    }
                } else if let Some(p) = doc.parent(cur) {
                    cur = p;
                    continue;
                }
                break;
            }
        }
        Axis::Preceding => {
            // nodes before n in document order, excluding ancestors
            let order = xml_xpath_order_doc_elems(doc);
            let mut ancestors = Vec::new();
            let mut p = Some(n);
            while let Some(x) = p {
                ancestors.push(x);
                p = doc.parent(x);
            }
            if let Some(idx) = order.iter().position(|x| *x == n) {
                for &id in &order[..idx] {
                    if !ancestors.contains(&id) {
                        raw.push(id);
                    }
                }
            }
        }
        Axis::Namespace => {
            for (pre, _) in doc.ns_defs(n) {
                let dummy = n; // namespace nodes not first-class; skip
                let _ = (pre, dummy);
            }
        }
    }
    raw.into_iter()
        .filter(|id| node_test(doc, *id, test, ns, axis))
        .collect()
}

fn collect_desc(doc: &XmlDoc, n: NodeId, out: &mut Vec<NodeId>, include_self: bool) {
    if include_self {
        out.push(n);
    }
    let mut c = doc.first_child(n);
    while let Some(x) = c {
        collect_desc(doc, x, out, true);
        c = doc.next_sibling(x);
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

fn eval_fun(name: &str, args: &[Expr], ctx: &XmlXPathContext<'_>) -> Result<XPathObject, String> {
    let ev = |i: usize| eval(&args[i], ctx);
    let local = name.rsplit(':').next().unwrap_or(name);
    match local {
        "true" => Ok(XPathObject::Boolean(true)),
        "false" => Ok(XPathObject::Boolean(false)),
        "not" => Ok(XPathObject::Boolean(!xml_xpath_cast_to_boolean(&ev(0)?))),
        "boolean" => Ok(XPathObject::Boolean(xml_xpath_cast_to_boolean(&ev(0)?))),
        "number" => {
            let o = if args.is_empty() {
                XPathObject::NodeSet(vec![ctx.node])
            } else {
                ev(0)?
            };
            Ok(XPathObject::Number(xml_xpath_cast_to_number(&o, ctx)))
        }
        "string" => {
            let o = if args.is_empty() {
                XPathObject::NodeSet(vec![ctx.node])
            } else {
                ev(0)?
            };
            Ok(XPathObject::String(xml_xpath_cast_to_string(&o, ctx)))
        }
        "last" => Ok(XPathObject::Number(ctx.size as f64)),
        "position" => Ok(XPathObject::Number(ctx.position as f64)),
        "count" => Ok(XPathObject::Number(nodeset(ev(0)?).len() as f64)),
        "local-name" => {
            let ns = if args.is_empty() {
                vec![ctx.node]
            } else {
                nodeset(ev(0)?)
            };
            Ok(XPathObject::String(
                ns.first().map(|id| ctx.doc.name(*id).to_string()).unwrap_or_default(),
            ))
        }
        "name" => {
            let ns = if args.is_empty() {
                vec![ctx.node]
            } else {
                nodeset(ev(0)?)
            };
            Ok(XPathObject::String(
                ns.first().map(|id| ctx.doc.qname(*id)).unwrap_or_default(),
            ))
        }
        "namespace-uri" => {
            let ns = if args.is_empty() {
                vec![ctx.node]
            } else {
                nodeset(ev(0)?)
            };
            Ok(XPathObject::String(
                ns.first()
                    .and_then(|id| ctx.doc.ns_uri(*id).map(str::to_string))
                    .unwrap_or_default(),
            ))
        }
        "concat" => {
            let mut s = String::new();
            for a in args {
                s.push_str(&xml_xpath_cast_to_string(&eval(a, ctx)?, ctx));
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
                a.split_once(&b).map(|(x, _)| x.to_string()).unwrap_or_default(),
            ))
        }
        "substring-after" => {
            let a = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let b = xml_xpath_cast_to_string(&ev(1)?, ctx);
            Ok(XPathObject::String(
                a.split_once(&b).map(|(_, x)| x.to_string()).unwrap_or_default(),
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
                ctx.doc.xml_node_get_content(ctx.node)
            } else {
                xml_xpath_cast_to_string(&ev(0)?, ctx)
            };
            Ok(XPathObject::Number(s.chars().count() as f64))
        }
        "normalize-space" => {
            let s = if args.is_empty() {
                ctx.doc.xml_node_get_content(ctx.node)
            } else {
                xml_xpath_cast_to_string(&ev(0)?, ctx)
            };
            Ok(XPathObject::String(
                s.split_whitespace().collect::<Vec<_>>().join(" "),
            ))
        }
        "translate" => {
            let s = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let from: Vec<char> = xml_xpath_cast_to_string(&ev(1)?, ctx).chars().collect();
            let to: Vec<char> = xml_xpath_cast_to_string(&ev(2)?, ctx).chars().collect();
            let out: String = s
                .chars()
                .filter_map(|c| {
                    if let Some(i) = from.iter().position(|x| *x == c) {
                        to.get(i).copied()
                    } else {
                        Some(c)
                    }
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
                t += xpath_number(&ctx.doc.xml_node_get_content(id));
            }
            Ok(XPathObject::Number(t))
        }
        "id" => {
            let ids = xml_xpath_cast_to_string(&ev(0)?, ctx);
            let mut found = Vec::new();
            let order = xml_xpath_order_doc_elems(ctx.doc);
            for tok in ids.split_whitespace() {
                for id in &order {
                    if ctx.doc.kind(*id) == NodeKind::Element
                        && ctx.doc.xml_get_prop(*id, "id").as_deref() == Some(tok)
                    {
                        found.push(*id);
                    }
                }
            }
            Ok(XPathObject::NodeSet(sort_unique(ctx.doc, found)))
        }
        "lang" => {
            let want = xml_xpath_cast_to_string(&ev(0)?, ctx).to_ascii_lowercase();
            let mut cur = Some(ctx.node);
            let mut lang = None;
            while let Some(id) = cur {
                if let Some(l) = ctx.doc.xml_get_prop(id, "lang") {
                    lang = Some(l);
                    break;
                }
                cur = ctx.doc.parent(id);
            }
            let ok = lang
                .map(|l| {
                    let l = l.to_ascii_lowercase();
                    l == want || l.starts_with(&format!("{want}-"))
                })
                .unwrap_or(false);
            Ok(XPathObject::Boolean(ok))
        }
        _ => Err(format!("unknown function {name}")),
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
