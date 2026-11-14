//! DTD element content models (`<!ELEMENT r (a, (b | c)*)>`): parsed the way
//! libxml2 parses them, compiled to a Glushkov position automaton, checked for
//! determinism, and matched against an element's children.
//!
//! The previous matcher backtracked over the model: a `*` whose body could
//! consume the same children in more than one way, such as `(a*)*`, expanded
//! every position once per path, so 26 children took about 24 seconds (#36).
//! The automaton is built once per element type and matches in one pass over
//! the children. For a deterministic model every step has at most one next
//! position.
//!
//! libxml2 also refuses non-deterministic models (XML 1.0 Appendix E): it
//! compiles the model to an automaton and reports "Content model of r is not
//! deterministic: (a | (a , a))*" (`xmlValidBuildContentModel`, valid.c).
//! A model is deterministic exactly when no two positions that can follow the
//! same point carry the same name, which the Glushkov follow sets show
//! directly.
//!
//! Nothing here recurses per level of the model or per item in a group. A
//! group of n items is a right-nested chain n deep, as libxml2 builds it, so
//! every walk uses an explicit stack.

use std::collections::HashMap;

/// libxml2 `xmlElementContentOccur`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Occ {
    Once,
    Opt,
    Mult,
    Plus,
}

/// libxml2 `xmlElementContentType`. `#PCDATA` never reaches a children model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Element(u32),
    Seq,
    Or,
}

const NONE: u32 = u32::MAX;

/// One `xmlElementContent` node. Sequences and choices are binary and
/// right-nested, `(a , b , c)` being `SEQ(a, SEQ(b, c))`, because that is the
/// shape the normalization rules and the printer below are written against.
#[derive(Clone, Copy, Debug)]
struct Node {
    kind: Kind,
    occ: Occ,
    c1: u32,
    c2: u32,
}

/// The deepest group nesting accepted: the DTD parser's own limit, so this
/// only refuses an `XmlDtd` built by hand. libxml2's determinism check
/// recurses once per group, which this bounds.
const MAX_GROUP_DEPTH: usize = 256;

/// The most follow-set entries one model may produce. A sequence of n
/// optional items has about n²/2 of them, so this admits several thousand
/// optional items while keeping a hostile DTD from asking for gigabytes.
const MAX_FOLLOW_ENTRIES: usize = 1 << 22;

/// The size of the buffer libxml2 prints a model into (`char expr[5000]`).
const PRINT_SIZE: usize = 5000;

struct Tree {
    nodes: Vec<Node>,
    names: Vec<String>,
    root: u32,
}

/// A parse frame for one open group, holding libxml2's `ret`, `cur` and
/// `last` for that call of `xmlParseElementChildrenContentDeclPriv`.
struct Frame {
    ret: u32,
    cur: u32,
    last: u32,
    sep: u8,
}

fn parse(spec: &str) -> Result<Tree, String> {
    let b = spec.as_bytes();
    let mut i = 0usize;
    let mut nodes: Vec<Node> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut ids: HashMap<String, u32> = HashMap::new();
    let ws = |i: &mut usize| {
        while matches!(b.get(*i), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            *i += 1;
        }
    };
    let bad = || format!("malformed content model {spec}");
    ws(&mut i);
    if b.get(i) != Some(&b'(') {
        return Err(bad());
    }
    i += 1;
    let mut frames: Vec<Frame> = vec![Frame {
        ret: NONE,
        cur: NONE,
        last: NONE,
        sep: 0,
    }];
    // A finished child (an element, or a closed group) waiting to be placed in
    // the innermost open frame.
    let mut done: Option<u32>;
    loop {
        // Read one cp: a name with its occurrence, or the start of a group.
        ws(&mut i);
        if b.get(i) == Some(&b'(') {
            i += 1;
            if frames.len() >= MAX_GROUP_DEPTH {
                return Err(format!("content model {spec} nested too deeply"));
            }
            frames.push(Frame {
                ret: NONE,
                cur: NONE,
                last: NONE,
                sep: 0,
            });
            continue;
        }
        let start = i;
        while let Some(&c) = b.get(i) {
            if matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')' | b'|' | b',' | b'?' | b'*' | b'+') {
                break;
            }
            i += 1;
        }
        if i == start {
            return Err(bad());
        }
        let name = &spec[start..i];
        let next_id = names.len() as u32;
        let id = *ids.entry(name.to_string()).or_insert_with(|| {
            names.push(name.to_string());
            next_id
        });
        let occ = match b.get(i) {
            Some(b'?') => Occ::Opt,
            Some(b'*') => Occ::Mult,
            Some(b'+') => Occ::Plus,
            _ => Occ::Once,
        };
        if occ != Occ::Once {
            i += 1;
        }
        nodes.push(Node {
            kind: Kind::Element(id),
            occ,
            c1: NONE,
            c2: NONE,
        });
        done = Some(nodes.len() as u32 - 1);

        // Place finished children and close groups for as long as we can.
        while let Some(child) = done.take() {
            let f = frames.last_mut().expect("a frame is open while reading");
            if f.ret == NONE {
                f.ret = child;
                f.cur = child;
            } else {
                f.last = child;
            }
            ws(&mut i);
            match b.get(i) {
                Some(&c @ (b',' | b'|')) => {
                    if f.sep == 0 {
                        f.sep = c;
                    } else if f.sep != c {
                        return Err(bad());
                    }
                    i += 1;
                    let op = nodes.len() as u32;
                    nodes.push(Node {
                        kind: if c == b',' { Kind::Seq } else { Kind::Or },
                        occ: Occ::Once,
                        c1: NONE,
                        c2: NONE,
                    });
                    if f.last == NONE {
                        nodes[op as usize].c1 = f.ret;
                        f.ret = op;
                        f.cur = op;
                    } else {
                        nodes[f.cur as usize].c2 = op;
                        nodes[op as usize].c1 = f.last;
                        f.cur = op;
                        f.last = NONE;
                    }
                    // The next cp follows; go back to reading.
                }
                Some(b')') => {
                    i += 1;
                    if f.cur != NONE && f.last != NONE {
                        nodes[f.cur as usize].c2 = f.last;
                    }
                    let (ret, mut cur) = (f.ret, f.cur);
                    frames.pop();
                    match b.get(i) {
                        Some(b'?') => {
                            i += 1;
                            let r = &mut nodes[ret as usize];
                            r.occ = if matches!(r.occ, Occ::Plus | Occ::Mult) {
                                Occ::Mult
                            } else {
                                Occ::Opt
                            };
                        }
                        Some(b'*') => {
                            i += 1;
                            nodes[ret as usize].occ = Occ::Mult;
                            // (a | b* | c?)* == (a | b | c)*
                            cur = ret;
                            while cur != NONE && nodes[cur as usize].kind == Kind::Or {
                                for c in [nodes[cur as usize].c1, nodes[cur as usize].c2] {
                                    if c != NONE && matches!(nodes[c as usize].occ, Occ::Opt | Occ::Mult) {
                                        nodes[c as usize].occ = Occ::Once;
                                    }
                                }
                                cur = nodes[cur as usize].c2;
                            }
                        }
                        Some(b'+') => {
                            i += 1;
                            let r = &mut nodes[ret as usize];
                            r.occ = if matches!(r.occ, Occ::Opt | Occ::Mult) {
                                Occ::Mult
                            } else {
                                Occ::Plus
                            };
                            // (a | b*)+ == (a | b)*, starting where libxml2
                            // starts: at `cur`, the last connector of the
                            // chain, not at its head.
                            let mut found = false;
                            while cur != NONE && nodes[cur as usize].kind == Kind::Or {
                                for c in [nodes[cur as usize].c1, nodes[cur as usize].c2] {
                                    if c != NONE && matches!(nodes[c as usize].occ, Occ::Opt | Occ::Mult) {
                                        nodes[c as usize].occ = Occ::Once;
                                        found = true;
                                    }
                                }
                                cur = nodes[cur as usize].c2;
                            }
                            if found {
                                nodes[ret as usize].occ = Occ::Mult;
                            }
                        }
                        _ => {}
                    }
                    if frames.is_empty() {
                        ws(&mut i);
                        if i != b.len() {
                            return Err(bad());
                        }
                        return Ok(Tree {
                            nodes,
                            names,
                            root: ret,
                        });
                    }
                    done = Some(ret);
                }
                _ => return Err(bad()),
            }
        }
    }
}

/// `xmlSnprintfElementContent(buf, 5000, content, 1)`, including its
/// truncation to " ..." once the buffer runs short.
fn print(t: &Tree) -> String {
    enum Act {
        Enter(u32, bool),
        Mid(u32),
        Exit(u32, bool),
    }
    let mut buf = String::new();
    let mut stack = vec![Act::Enter(t.root, true)];
    let size = PRINT_SIZE;
    let room = |buf: &String| size.saturating_sub(buf.len());
    let short = |buf: &mut String| {
        if room(buf) > 4 && !buf.ends_with('.') {
            buf.push_str(" ...");
        }
    };
    while let Some(a) = stack.pop() {
        match a {
            Act::Enter(n, englob) => {
                let left = room(&buf);
                if left < 50 {
                    short(&mut buf);
                    continue;
                }
                if englob {
                    buf.push('(');
                }
                let node = t.nodes[n as usize];
                match node.kind {
                    Kind::Element(id) => {
                        let name = &t.names[id as usize];
                        if left < name.len() + 10 {
                            buf.push_str(" ...");
                            continue;
                        }
                        buf.push_str(name);
                        stack.push(Act::Exit(n, englob));
                    }
                    Kind::Seq | Kind::Or => {
                        stack.push(Act::Exit(n, englob));
                        stack.push(Act::Mid(n));
                        let c1 = t.nodes[node.c1 as usize];
                        stack.push(Act::Enter(node.c1, matches!(c1.kind, Kind::Seq | Kind::Or)));
                    }
                }
            }
            Act::Mid(n) => {
                let node = t.nodes[n as usize];
                if room(&buf) < 50 {
                    short(&mut buf);
                    stack.pop(); // this node's Exit
                    continue;
                }
                let c2 = t.nodes[node.c2 as usize];
                let (sep, other) = if node.kind == Kind::Seq {
                    (" , ", Kind::Or)
                } else {
                    (" | ", Kind::Seq)
                };
                buf.push_str(sep);
                let englob = (c2.kind == other || c2.occ != Occ::Once)
                    && !matches!(c2.kind, Kind::Element(_));
                stack.push(Act::Enter(node.c2, englob));
            }
            Act::Exit(n, englob) => {
                if room(&buf) <= 2 {
                    continue;
                }
                if englob {
                    buf.push(')');
                }
                match t.nodes[n as usize].occ {
                    Occ::Once => {}
                    Occ::Opt => buf.push('?'),
                    Occ::Mult => buf.push('*'),
                    Occ::Plus => buf.push('+'),
                }
            }
        }
    }
    buf
}

/// The work allowed for libxml2's determinism check on one model. Its
/// epsilon elimination is far from linear (libxml2 itself takes about a
/// minute on a sequence of 3,000 optional elements), so past this budget the
/// check is skipped and the model is treated as deterministic: a check
/// libxml2 makes for compatibility is not worth an unbounded stall, and
/// matching itself does not depend on it.
const DETERMINISM_BUDGET: u64 = 1 << 25;

/// libxml2's own determinism verdict for a content model, by its own route:
/// `xmlValidBuildAContentModel` builds an automaton with epsilon
/// transitions, `xmlFAEliminateEpsilonTransitions` removes them, and
/// `xmlFAComputesDeterminism` looks for a state with two transitions on the
/// same name to different states.
///
/// It is ported rather than derived from the Glushkov sets because the two
/// disagree. libxml2 merges states joined by a lone epsilon, so `(a | a)*`
/// passes, while `(a | a)` fails because its states keep a dead transition
/// slot that blocks the merge. Strict 1-unambiguity would reject both.
mod det {
    use super::{Kind, Occ, Tree, NONE};

    const EPS: u32 = NONE;
    const MARK_NORMAL: u8 = 0;
    const MARK_START: u8 = 1;
    const MARK_VISITED: u8 = 2;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Ty {
        Normal,
        Start,
        Final,
        Unreach,
        Sink,
    }

    /// `xmlRegTrans`: `atom` is an atom identity (`EPS` for an epsilon),
    /// `name` the element name it matches, `to` the target (-1 eliminated,
    /// -2 being reduced).
    #[derive(Clone, Copy)]
    struct Tr {
        atom: u32,
        name: u32,
        to: i32,
    }

    struct St {
        ty: Ty,
        trans: Vec<Tr>,
        trans_to: Vec<u32>,
        mark: u8,
    }

    struct Am {
        states: Vec<Option<St>>,
        atoms: u32,
        work: u64,
    }

    struct OutOfBudget;

    impl Am {
        fn spend(&mut self, n: usize) -> Result<(), OutOfBudget> {
            self.work += n as u64 + 1;
            if self.work > super::DETERMINISM_BUDGET {
                Err(OutOfBudget)
            } else {
                Ok(())
            }
        }

        fn st(&mut self, i: usize) -> &mut St {
            self.states[i].as_mut().expect("live state")
        }

        fn len(&self, i: usize) -> usize {
            self.states[i].as_ref().map_or(0, |s| s.trans.len())
        }

        /// `xmlRegStatePush`.
        fn push(&mut self) -> u32 {
            self.states.push(Some(St {
                ty: Ty::Normal,
                trans: Vec::new(),
                trans_to: Vec::new(),
                mark: MARK_NORMAL,
            }));
            self.states.len() as u32 - 1
        }

        /// `xmlRegStateAddTrans`, which ignores an exact duplicate.
        fn add(&mut self, from: u32, atom: u32, name: u32, to: u32) -> Result<(), OutOfBudget> {
            let n = self.len(from as usize);
            self.spend(n)?;
            let s = self.st(from as usize);
            if s.trans.iter().rev().any(|t| t.atom == atom && t.to == to as i32) {
                return Ok(());
            }
            s.trans.push(Tr {
                atom,
                name,
                to: to as i32,
            });
            self.st(to as usize).trans_to.push(from);
            Ok(())
        }

        /// `xmlAutomataNewTransition`, which makes a fresh atom every call.
        fn transition(&mut self, from: u32, to: Option<u32>, name: u32) -> Result<u32, OutOfBudget> {
            let atom = self.atoms;
            self.atoms += 1;
            let to = match to {
                Some(t) => t,
                None => self.push(),
            };
            self.add(from, atom, name, to)?;
            Ok(to)
        }

        /// `xmlAutomataNewEpsilon`.
        fn epsilon(&mut self, from: u32, to: Option<u32>) -> Result<u32, OutOfBudget> {
            let to = match to {
                Some(t) => t,
                None => self.push(),
            };
            self.add(from, EPS, NONE, to)?;
            Ok(to)
        }

        /// `xmlValidBuildAContentModel`. It recurses once per nested group,
        /// which the parser caps, and walks each chain in a loop.
        fn build(&mut self, t: &Tree, mut n: u32, state: &mut u32) -> Result<(), OutOfBudget> {
            let node = t.nodes[n as usize];
            match node.kind {
                Kind::Element(name) => {
                    let old = *state;
                    match node.occ {
                        Occ::Once => *state = self.transition(*state, None, name)?,
                        Occ::Opt => {
                            *state = self.transition(*state, None, name)?;
                            self.epsilon(old, Some(*state))?;
                        }
                        Occ::Plus => {
                            *state = self.transition(*state, None, name)?;
                            self.transition(*state, Some(*state), name)?;
                        }
                        Occ::Mult => {
                            *state = self.epsilon(*state, None)?;
                            self.transition(*state, Some(*state), name)?;
                        }
                    }
                }
                Kind::Seq => {
                    let mut old = *state;
                    let occ = node.occ;
                    if occ != Occ::Once {
                        *state = self.epsilon(old, None)?;
                        old = *state;
                    }
                    loop {
                        let cur = t.nodes[n as usize];
                        self.build(t, cur.c1, state)?;
                        n = cur.c2;
                        let next = t.nodes[n as usize];
                        if !(next.kind == Kind::Seq && next.occ == Occ::Once) {
                            break;
                        }
                    }
                    self.build(t, n, state)?;
                    let oldend = *state;
                    *state = self.epsilon(oldend, None)?;
                    self.close(occ, old, oldend, *state)?;
                }
                Kind::Or => {
                    let occ = node.occ;
                    if matches!(occ, Occ::Plus | Occ::Mult) {
                        *state = self.epsilon(*state, None)?;
                    }
                    let old = *state;
                    let oldend = self.push();
                    loop {
                        let cur = t.nodes[n as usize];
                        *state = old;
                        self.build(t, cur.c1, state)?;
                        self.epsilon(*state, Some(oldend))?;
                        n = cur.c2;
                        let next = t.nodes[n as usize];
                        if !(next.kind == Kind::Or && next.occ == Occ::Once) {
                            break;
                        }
                    }
                    *state = old;
                    self.build(t, n, state)?;
                    self.epsilon(*state, Some(oldend))?;
                    *state = self.epsilon(oldend, None)?;
                    self.close(occ, old, oldend, *state)?;
                }
            }
            Ok(())
        }

        fn close(&mut self, occ: Occ, old: u32, oldend: u32, state: u32) -> Result<(), OutOfBudget> {
            match occ {
                Occ::Once => {}
                Occ::Opt => {
                    self.epsilon(old, Some(state))?;
                }
                Occ::Mult => {
                    self.epsilon(old, Some(state))?;
                    self.epsilon(oldend, Some(old))?;
                }
                Occ::Plus => {
                    self.epsilon(oldend, Some(old))?;
                }
            }
            Ok(())
        }

        /// `xmlFAEliminateSimpleEpsilonTransitions`. A state's transition
        /// count includes slots already eliminated, exactly as in C.
        fn eliminate_simple(&mut self) -> Result<(), OutOfBudget> {
            for sn in 0..self.states.len() {
                let Some(s) = self.states[sn].as_ref() else { continue };
                if s.trans.len() != 1 || matches!(s.ty, Ty::Unreach | Ty::Final) {
                    continue;
                }
                let t0 = s.trans[0];
                if !(t0.atom == EPS && t0.to >= 0 && t0.to as usize != sn) {
                    continue;
                }
                if s.ty == Ty::Start {
                    continue;
                }
                let newto = t0.to as u32;
                let incoming = s.trans_to.clone();
                for tmp in incoming {
                    let tmp = tmp as usize;
                    let mut j = 0;
                    while j < self.len(tmp) {
                        let tr = self.st(tmp).trans[j];
                        if tr.to == sn as i32 {
                            self.st(tmp).trans[j].to = -1;
                            self.add(tmp as u32, tr.atom, tr.name, newto)?;
                        }
                        j += 1;
                    }
                }
                let s = self.st(sn);
                s.trans.clear();
                s.ty = Ty::Unreach;
            }
            Ok(())
        }

        /// `xmlFAReduceEpsilonTransitions`, iteratively, in the same order.
        fn reduce(&mut self, from: usize, to: usize) -> Result<(), OutOfBudget> {
            if !self.enter(from, to) {
                return Ok(());
            }
            let mut stack: Vec<(usize, usize)> = vec![(to, 0)];
            while let Some(&(x, i)) = stack.last() {
                if i >= self.len(x) {
                    stack.pop();
                    continue;
                }
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                self.spend(0)?;
                let t = self.st(x).trans[i];
                if t.to < 0 {
                    continue;
                }
                if t.atom == EPS {
                    if t.to as usize != from && self.enter(from, t.to as usize) {
                        stack.push((t.to as usize, 0));
                    }
                } else {
                    self.add(from as u32, t.atom, t.name, t.to as u32)?;
                }
            }
            Ok(())
        }

        fn enter(&mut self, from: usize, to: usize) -> bool {
            let Some(s) = self.states[to].as_mut() else { return false };
            if s.mark == MARK_START || s.mark == MARK_VISITED {
                return false;
            }
            s.mark = MARK_VISITED;
            if s.ty == Ty::Final {
                self.st(from).ty = Ty::Final;
            }
            true
        }

        /// `xmlFAFinishReduceEpsilonTransitions`, iteratively.
        fn finish_reduce(&mut self, to: usize) {
            let mut stack = vec![to];
            while let Some(x) = stack.pop() {
                let Some(s) = self.states[x].as_mut() else { continue };
                if s.mark == MARK_START || s.mark == MARK_NORMAL {
                    continue;
                }
                s.mark = MARK_NORMAL;
                for t in &s.trans {
                    if t.to >= 0 && t.atom == EPS {
                        stack.push(t.to as usize);
                    }
                }
            }
        }

        /// `xmlFAEliminateEpsilonTransitions`.
        fn eliminate(&mut self) -> Result<(), OutOfBudget> {
            self.eliminate_simple()?;
            for s in self.states.iter_mut() {
                if s.as_ref().is_some_and(|s| s.ty == Ty::Unreach) {
                    *s = None;
                }
            }
            let mut has_epsilon = false;
            for sn in (0..self.states.len()).rev() {
                let Some(s) = self.states[sn].as_mut() else { continue };
                if s.trans.is_empty() && s.ty != Ty::Final {
                    s.ty = Ty::Sink;
                }
                let mut tn = 0;
                while tn < self.len(sn) {
                    let t = self.st(sn).trans[tn];
                    if t.atom == EPS && t.to >= 0 {
                        if t.to as usize == sn {
                            self.st(sn).trans[tn].to = -1;
                        } else {
                            has_epsilon = true;
                            self.st(sn).trans[tn].to = -2;
                            self.st(sn).mark = MARK_START;
                            self.reduce(sn, t.to as usize)?;
                            self.finish_reduce(t.to as usize);
                            self.st(sn).mark = MARK_NORMAL;
                        }
                    }
                    tn += 1;
                }
            }
            if has_epsilon {
                for s in self.states.iter_mut().flatten() {
                    for t in s.trans.iter_mut() {
                        if t.atom == EPS && t.to >= 0 {
                            t.to = -1;
                        }
                    }
                }
            }
            // Drop the states the start state cannot reach.
            let mut reached = vec![false; self.states.len()];
            let mut stack = vec![0usize];
            reached[0] = true;
            while let Some(x) = stack.pop() {
                let Some(s) = self.states[x].as_ref() else { continue };
                for t in &s.trans {
                    if t.to >= 0 && t.atom != EPS {
                        let to = t.to as usize;
                        if self.states[to].is_some() && !reached[to] {
                            reached[to] = true;
                            stack.push(to);
                        }
                    }
                }
            }
            for (i, s) in self.states.iter_mut().enumerate() {
                if !reached[i] {
                    *s = None;
                }
            }
            Ok(())
        }

        /// `xmlFAComputesDeterminism` for a DTD automaton: there are no
        /// counters, and no live epsilon is left once eliminated.
        fn deterministic(&mut self) -> Result<bool, OutOfBudget> {
            for sn in 0..self.states.len() {
                let n = self.len(sn);
                if self.states[sn].is_none() || n < 2 {
                    continue;
                }
                self.spend(n * n / 2)?;
                let s = self.st(sn);
                for a in 0..n {
                    let t1 = s.trans[a];
                    if t1.atom == EPS || t1.to < 0 {
                        continue;
                    }
                    for b in 0..a {
                        let t2 = s.trans[b];
                        if t2.to >= 0 && t2.atom != EPS && t1.to == t2.to && t1.name == t2.name {
                            s.trans[b].to = -1;
                        }
                    }
                }
            }
            for s in self.states.iter().flatten() {
                let n = s.trans.len();
                for a in 0..n {
                    let t1 = s.trans[a];
                    if t1.atom == EPS || t1.to < 0 {
                        continue;
                    }
                    for b in 0..a {
                        let t2 = s.trans[b];
                        if t2.to >= 0 && t2.atom != EPS && t1.name == t2.name {
                            return Ok(false);
                        }
                    }
                }
            }
            Ok(true)
        }
    }

    /// libxml2's verdict, or `None` when the work budget ran out.
    pub(super) fn libxml2_deterministic(t: &Tree) -> Option<bool> {
        let mut am = Am {
            states: Vec::new(),
            atoms: 0,
            work: 0,
        };
        let start = am.push();
        am.st(start as usize).ty = Ty::Start;
        let mut run = || -> Result<bool, OutOfBudget> {
            let mut state = start;
            am.build(t, t.root, &mut state)?;
            am.st(state as usize).ty = Ty::Final;
            am.eliminate()?;
            am.deterministic()
        };
        run().ok()
    }
}

/// A compiled children content model.
pub(crate) struct ContentModel {
    /// Element name to name id.
    ids: HashMap<String, u32>,
    /// The positions that can come first, as (name id, position), sorted.
    start: Vec<(u32, u32)>,
    /// Per position, the positions that can follow it, sorted the same way.
    follow: Vec<Vec<(u32, u32)>>,
    /// Per position, whether the children may end after it.
    accept: Vec<bool>,
    /// Whether no children at all match.
    nullable: bool,
    /// libxml2's printed form of the model, when it is not deterministic.
    nondeterministic: Option<String>,
}

impl ContentModel {
    /// Compile `spec`, the text of an `ElementDecl::Children`.
    pub(crate) fn compile(spec: &str) -> Result<ContentModel, String> {
        let t = parse(spec)?;
        // Positions, numbered in document order of the model.
        let mut pos_of = vec![NONE; t.nodes.len()];
        let mut pos_name = Vec::new();
        {
            let mut stack = vec![t.root];
            while let Some(n) = stack.pop() {
                let node = t.nodes[n as usize];
                match node.kind {
                    Kind::Element(id) => {
                        pos_of[n as usize] = pos_name.len() as u32;
                        pos_name.push(id);
                    }
                    Kind::Seq | Kind::Or => {
                        stack.push(node.c2);
                        stack.push(node.c1);
                    }
                }
            }
        }
        let mut follow: Vec<Vec<u32>> = vec![Vec::new(); pos_name.len()];
        let mut entries = 0usize;
        let mut add = |follow: &mut Vec<Vec<u32>>, from: &[u32], to: &[u32]| -> Result<(), String> {
            entries += from.len().saturating_mul(to.len());
            if entries > MAX_FOLLOW_ENTRIES {
                return Err(format!("content model {spec} is too large to validate against"));
            }
            for &p in from {
                follow[p as usize].extend_from_slice(to);
            }
            Ok(())
        };
        struct Info {
            nullable: bool,
            first: Vec<u32>,
            last: Vec<u32>,
        }
        // Union of two disjoint position sets, copying the smaller.
        fn join(mut a: Vec<u32>, mut b: Vec<u32>) -> Vec<u32> {
            if a.len() < b.len() {
                std::mem::swap(&mut a, &mut b);
            }
            a.extend_from_slice(&b);
            a
        }
        let mut info: Vec<Option<Info>> = (0..t.nodes.len()).map(|_| None).collect();
        let mut stack = vec![(t.root, false)];
        while let Some((n, children_done)) = stack.pop() {
            let node = t.nodes[n as usize];
            let mut i = match node.kind {
                Kind::Element(_) => {
                    let p = pos_of[n as usize];
                    Info {
                        nullable: false,
                        first: vec![p],
                        last: vec![p],
                    }
                }
                Kind::Seq | Kind::Or if !children_done => {
                    stack.push((n, true));
                    stack.push((node.c2, false));
                    stack.push((node.c1, false));
                    continue;
                }
                Kind::Seq => {
                    let a = info[node.c1 as usize].take().expect("child computed");
                    let b = info[node.c2 as usize].take().expect("child computed");
                    add(&mut follow, &a.last, &b.first)?;
                    let first = if a.nullable { join(a.first, b.first) } else { a.first };
                    let last = if b.nullable { join(b.last, a.last) } else { b.last };
                    Info {
                        nullable: a.nullable && b.nullable,
                        first,
                        last,
                    }
                }
                Kind::Or => {
                    let a = info[node.c1 as usize].take().expect("child computed");
                    let b = info[node.c2 as usize].take().expect("child computed");
                    Info {
                        nullable: a.nullable || b.nullable,
                        first: join(a.first, b.first),
                        last: join(a.last, b.last),
                    }
                }
            };
            if matches!(node.occ, Occ::Mult | Occ::Plus) {
                add(&mut follow, &i.last, &i.first)?;
            }
            if matches!(node.occ, Occ::Opt | Occ::Mult) {
                i.nullable = true;
            }
            info[n as usize] = Some(i);
        }
        let root = info[t.root as usize].take().expect("root computed");
        let mut accept = vec![false; pos_name.len()];
        for &p in &root.last {
            accept[p as usize] = true;
        }
        let keyed = |v: &[u32]| -> (Vec<(u32, u32)>, bool) {
            let mut k: Vec<(u32, u32)> = v.iter().map(|&p| (pos_name[p as usize], p)).collect();
            k.sort_unstable();
            k.dedup();
            let ambiguous = k.windows(2).any(|w| w[0].0 == w[1].0);
            (k, ambiguous)
        };
        let (start, mut ambiguous) = keyed(&root.first);
        let follow: Vec<Vec<(u32, u32)>> = follow
            .into_iter()
            .map(|f| {
                let (k, a) = keyed(&f);
                ambiguous |= a;
                k
            })
            .collect();
        // When the Glushkov sets are unambiguous, libxml2's automaton is
        // deterministic too, so its slower check runs only on an ambiguity.
        let nondeterministic =
            (ambiguous && det::libxml2_deterministic(&t) == Some(false)).then(|| print(&t));
        let ids = t
            .names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();
        Ok(ContentModel {
            ids,
            start,
            follow,
            accept,
            nullable: root.nullable,
            nondeterministic,
        })
    }

    /// libxml2's printed form of the model, if the model is not deterministic.
    pub(crate) fn nondeterministic(&self) -> Option<&str> {
        self.nondeterministic.as_deref()
    }

    /// Whether `kids`, the element children's qualified names in order, match.
    ///
    /// Works for any model: the set of current positions stays a single
    /// position when the model is deterministic, and is deduplicated when it
    /// is not, so the cost is bounded by children × positions either way.
    pub(crate) fn matches<S: AsRef<str>>(&self, kids: &[S]) -> bool {
        let mut cur: Vec<u32> = Vec::new();
        let mut next: Vec<u32> = Vec::new();
        let mut at_start = true;
        for k in kids {
            let Some(&id) = self.ids.get(k.as_ref()) else {
                return false;
            };
            next.clear();
            let mut take = |set: &[(u32, u32)]| {
                let lo = set.partition_point(|e| e.0 < id);
                next.extend(set[lo..].iter().take_while(|e| e.0 == id).map(|e| e.1));
            };
            if at_start {
                take(&self.start);
                at_start = false;
            } else {
                for &p in &cur {
                    take(&self.follow[p as usize]);
                }
            }
            if next.is_empty() {
                return false;
            }
            if next.len() > 1 {
                next.sort_unstable();
                next.dedup();
            }
            std::mem::swap(&mut cur, &mut next);
        }
        if at_start {
            self.nullable
        } else {
            cur.iter().any(|&p| self.accept[p as usize])
        }
    }

    #[cfg(test)]
    fn positions(&self) -> usize {
        self.accept.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printed(spec: &str) -> String {
        print(&parse(spec).unwrap())
    }

    fn nd(spec: &str) -> Option<String> {
        ContentModel::compile(spec).unwrap().nondeterministic().map(str::to_string)
    }

    fn m(spec: &str, kids: &str) -> bool {
        let kids: Vec<&str> = kids.split_whitespace().collect();
        ContentModel::compile(spec).unwrap().matches(&kids)
    }

    #[test]
    fn prints_as_libxml2_prints() {
        // Every expected string is xmllint 2.15.3's own output.
        assert_eq!(printed("(a|(a,a))*"), "(a | (a , a))*");
        assert_eq!(printed("(a,(b,c))"), "(a , b , c)");
        assert_eq!(printed("(a+)+"), "(a)+");
        assert_eq!(printed("(((a)))"), "(a)");
        assert_eq!(printed("((a?),a?)*"), "(a? , a?)*");
        assert_eq!(printed("((a|b),(c|d))+"), "((a | b) , (c | d))+");
        assert_eq!(printed("((a,b)|(a,c))"), "((a , b) | (a , c))");
        assert_eq!(printed("(a , (b|c)* , b)"), "(a , (b | c)* , b)");
        assert_eq!(printed("((a?,b?)+,a)"), "((a? , b?)+ , a)");
        assert_eq!(printed("(a|b|a)"), "(a | b | a)");
    }

    #[test]
    fn determinism_matches_libxml2() {
        // xmllint 2.15.3 --valid verdicts on each model.
        for s in [
            "(a|(a,a))*", "(a|a)", "((a,b)|(a,c))", "(a?,a)", "(a?,a?)", "(a*,a)",
            "((a|b)*,a)", "((a,b)*,a)", "(a,(b|c)*,b)", "((a?),a?)*", "(a|b|a)",
            "((a?,b?)+,a)", "((a,b)?,a)",
        ] {
            assert!(nd(s).is_some(), "{s} should not be deterministic");
        }
        for s in [
            "(a*)*", "((a,b)|(c,d))", "(a,a?)", "(a+)+", "((a,b?)*)", "(a,b)?",
            "((a|b),(a|b)?)", "(((a)))", "(a,(a|b))", "((a?,b?)*)",
        ] {
            assert!(nd(s).is_none(), "{s} should be deterministic");
        }
    }

    #[test]
    fn matching() {
        assert!(m("(a*)*", ""));
        assert!(m("(a*)*", "a a a"));
        assert!(!m("(a*)*", "a b"));
        assert!(m("(a,b)?", ""));
        assert!(m("(a,b)?", "a b"));
        assert!(!m("(a,b)?", "a"));
        assert!(m("(a,(b|c)*,d)", "a b c b d"));
        assert!(!m("(a,(b|c)*,d)", "a b c b"));
        assert!(m("(a+)", "a a"));
        assert!(!m("(a+)", ""));
        assert!(m("((a,b)|(c,d))", "c d"));
        assert!(!m("((a,b)|(c,d))", "a d"));
        // Non-deterministic models still match correctly.
        assert!(m("(a|(a,a))*", "a a a"));
        assert!(m("((a,b)|(a,c))", "a c"));
        assert!(!m("((a,b)|(a,c))", "a d"));
        // Names outside ASCII are names.
        assert!(m("(\u{e9}l\u{e9}ment, b)", "\u{e9}l\u{e9}ment b"));
    }

    #[test]
    fn wide_and_deep_models_do_not_recurse() {
        let wide = format!("({})", (0..20_000).map(|i| format!("e{i}")).collect::<Vec<_>>().join(" , "));
        let cm = ContentModel::compile(&wide).unwrap();
        assert_eq!(cm.positions(), 20_000);
        assert!(cm.nondeterministic().is_none());
        let kids: Vec<String> = (0..20_000).map(|i| format!("e{i}")).collect();
        assert!(cm.matches(&kids));
        let deep = format!("{}a{}", "(".repeat(250), ")".repeat(250));
        assert!(ContentModel::compile(&deep).unwrap().matches(&["a"]));
        let too_deep = format!("{}a{}", "(".repeat(300), ")".repeat(300));
        assert!(ContentModel::compile(&too_deep).is_err());
        // A long printed form is cut the way libxml2 cuts it.
        let long = format!("({})", (0..2000).map(|i| format!("e{i}?")).collect::<Vec<_>>().join(" , "));
        let p = printed(&long);
        // xmllint 2.15.3 ends this model's message with "e631? , e632? ...)".
        assert!(p.len() < PRINT_SIZE && p.ends_with("e631? , e632? ...)"), "{}", &p[p.len() - 20..]);
    }

    #[test]
    fn malformed_models_are_errors_not_panics() {
        for s in ["", "a", "(", "()", "(a,)", "(a|b,c)", "(a))", "(a b)", "(,a)", "(a)x"] {
            assert!(ContentModel::compile(s).is_err(), "{s:?}");
        }
    }
}
