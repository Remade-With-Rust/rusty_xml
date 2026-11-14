//! Arena DOM matching libxml2 `tree.h` ownership: the document owns every node.
//! Handles are indices, not parent+child `&mut` pairs.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::num::NonZeroU32;

mod interrupt;
pub use interrupt::{Interrupt, InterruptPoll, Interrupted, NeverInterrupt, INTERRUPT_POLL_EVERY};

/// libxml2 `xmlElementType` discriminants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum NodeKind {
    Element = 1,
    Attribute = 2,
    Text = 3,
    CData = 4,
    EntityRef = 5,
    Entity = 6,
    Pi = 7,
    Comment = 8,
    Document = 9,
    DocumentType = 10,
    DocumentFrag = 11,
    Notation = 12,
    HtmlDocument = 13,
    Dtd = 14,
    ElementDecl = 15,
    AttributeDecl = 16,
    EntityDecl = 17,
    Namespace = 18,
    XIncludeStart = 19,
    XIncludeEnd = 20,
}

/// Parsed DTD attached to a document (`xmlDtd`).
#[derive(Clone, Debug, Default)]
pub struct XmlDtd {
    pub name: Option<String>,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
    pub int_subset: Option<String>,
    /// General entity name → replacement text.
    pub entities: HashMap<String, String>,
    /// Parameter entity name → replacement.
    pub parameter_entities: HashMap<String, String>,
    /// Names of entities declared with an NDATA annotation. An attribute of
    /// type ENTITY must name one of these, which needs them kept apart from
    /// parsed entities rather than lumped in with them.
    pub unparsed_entities: std::collections::HashSet<String>,
    /// Notation names declared by `<!NOTATION>`. We parsed those declarations
    /// and threw the name away, so nothing could check that an NDATA
    /// annotation or a NOTATION attribute type names one that exists.
    pub notations: std::collections::HashSet<String>,
    /// Notation names referenced by an NDATA annotation, kept so the
    /// "Notation Declared" constraint can be checked once the whole subset is
    /// read rather than in declaration order.
    pub ndata_notations: Vec<String>,
    /// The internal subset used a parameter entity reference, so the set of
    /// entity declarations may be incomplete. XML 1.0 4.1 turns "Entity
    /// Declared" from a well-formedness constraint into a validity one in that
    /// case: an unresolvable entity must not kill the parse.
    pub has_parameter_entity_refs: bool,
    /// Namespace errors found while reading the subset -- a colon in an entity
    /// or notation name. Merged into the document's list.
    pub namespace_errors: Vec<String>,
    /// Element name → content model.
    pub elements: HashMap<String, ElementDecl>,
    /// Element types declared more than once. A map cannot represent that, and
    /// "Unique Element Type Declaration" is a validity constraint, so it has to
    /// be recorded as the declarations go by and reported at validation time
    /// rather than at parse time.
    pub duplicate_elements: Vec<String>,
    /// (element, attribute) → declaration.
    pub attributes: HashMap<(String, String), AttrDecl>,
}

#[derive(Clone, Debug)]
pub enum ElementDecl {
    Empty,
    Any,
    Mixed(Vec<String>),
    Children(String),
}

#[derive(Clone, Debug)]
pub struct AttrDecl {
    pub att_type: String,
    pub default: AttrDefault,
    pub default_value: Option<String>,
    pub enumerated: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttrDefault {
    Required,
    Implied,
    Fixed,
    Value,
}

/// Stable handle into an [`XmlDoc`] arena. Valid for the lifetime of the doc.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Document node is always slot 0.
    pub const DOCUMENT: NodeId = NodeId(0);

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A link between two nodes of one document, or none: a [`NodeId`] stored as
/// `id + 1` in a `NonZeroU32`, so the absent case costs no extra bytes. As
/// `Option<NodeId>` each of a node's seven links took 8 bytes; this takes 4.
/// [`XmlDoc::parent`], [`XmlDoc::first_child`] and the other accessors still
/// return `Option<NodeId>`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Link(Option<NonZeroU32>);

impl Link {
    /// No node.
    pub const NONE: Link = Link(None);

    /// The node linked to, if any.
    #[inline]
    pub fn get(self) -> Option<NodeId> {
        self.0.map(|n| NodeId(n.get() - 1))
    }

    #[inline]
    pub fn is_none(self) -> bool {
        self.0.is_none()
    }

    #[inline]
    pub fn is_some(self) -> bool {
        self.0.is_some()
    }
}

impl From<NodeId> for Link {
    #[inline]
    fn from(id: NodeId) -> Self {
        // The arena is capped far below u32::MAX nodes, so id + 1 cannot wrap.
        Link(NonZeroU32::new(id.0.wrapping_add(1)))
    }
}

impl From<Option<NodeId>> for Link {
    #[inline]
    fn from(id: Option<NodeId>) -> Self {
        id.map_or(Link::NONE, Link::from)
    }
}

impl std::fmt::Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

/// A string interned in one document: a namespace prefix or URI. A document
/// holds a handful of distinct ones, and storing each node's own copy cost a
/// 24-byte `Option<String>` per field per node plus an allocation per
/// namespaced node. Resolve it with [`XmlDoc::atom`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Atom(NonZeroU32);

/// Interned strings are found by a scan while there are at most this many,
/// and through a hash index after. Prefixes and URIs come from the input, so
/// the index keeps the standard library's keyed hasher.
const ATOM_SCAN_MAX: usize = 16;



#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub name: String,
    /// Interned in the owning document; read it with [`XmlDoc::prefix`].
    pub prefix: Option<Atom>,
    /// Interned in the owning document; read it with [`XmlDoc::ns_uri`].
    pub ns_uri: Option<Atom>,
    pub content: String,
    pub parent: Link,
    pub first_child: Link,
    pub last_child: Link,
    pub prev_sibling: Link,
    pub next_sibling: Link,
    pub first_attr: Link,
    pub last_attr: Link,
}

impl Node {
    fn new(kind: NodeKind, name: String) -> Self {
        Self {
            kind,
            name,
            prefix: None,
            ns_uri: None,
            content: String::new(),
            parent: Link::NONE,
            first_child: Link::NONE,
            last_child: Link::NONE,
            prev_sibling: Link::NONE,
            next_sibling: Link::NONE,
            first_attr: Link::NONE,
            last_attr: Link::NONE,
        }
    }
}

/// libxml2 `xmlDoc`.
#[derive(Clone, Debug)]
pub struct XmlDoc {
    nodes: Vec<Node>,
    /// XML version string; default `"1.0"`.
    pub version: String,
    /// Encoding name from the XML declaration, if any.
    pub encoding: Option<String>,
    /// `Some(true/false)` from `standalone`, `None` if omitted.
    pub standalone: Option<bool>,
    /// First element child of the document (cached; also discoverable by walk).
    root: Option<NodeId>,
    /// Internal / attached DTD, if any.
    pub dtd: Option<XmlDtd>,
    /// Entity references the parse could not resolve. They are kept as written
    /// rather than being fatal when the subset is incomplete, and the
    /// validator reports them.
    pub undeclared_entity_refs: Vec<String>,
    /// Text nodes whose content came from a character or entity reference.
    ///
    /// Such text is never ignorable whitespace: `<foo><foo/>&#32;<foo/></foo>`
    /// against an element-only content model is character data where none is
    /// allowed, and looking only at whether the text is blank cannot tell it
    /// from the indentation beside it.
    /// Namespace errors reported while parsing.
    ///
    /// A namespace violation is never a well-formedness error: libxml2 parses
    /// the document and logs one, and its own conformance harness scores the
    /// namespace tests by looking for exactly that -- the document must parse
    /// AND an error must have been reported. Rejecting instead would refuse
    /// input C reads, so they are recorded here for the caller to inspect.
    pub namespace_errors: Vec<String>,
    /// Problems worth telling a tree-parsing caller about: a declared
    /// encoding that contradicts the byte-order mark, and, under
    /// `XML_PARSE_RECOVER`, the fatal error that ended the parse (formatted
    /// as `XmlError` displays, `line:col: error code N: message`), so a
    /// truncated tree can be told from a whole one. The SAX error channel
    /// carries these too, but the default handler discards them.
    pub warnings: Vec<String>,
    pub reference_text: std::collections::HashSet<NodeId>,
    /// Elements whose content included an entity reference, including one that
    /// expanded to nothing. `<foo>&empty;</foo>` against EMPTY is content, and
    /// the tree has no node to show for it.
    pub elements_with_entity_refs: std::collections::HashSet<NodeId>,
    /// HTML attributes written without a value (`<input value>`), in creation
    /// order, so sorted. libxml2 gives such an attribute no children: its value
    /// reads as `""`, but it serializes as the bare name. Empty for XML.
    pub html_bare_attrs: Vec<NodeId>,
    /// Interned prefixes and URIs; [`Atom`] `n` is `atoms[n - 1]`.
    atoms: Vec<Box<str>>,
    /// Hash index over `atoms`, built once there are more than
    /// `ATOM_SCAN_MAX` of them.
    atom_index: HashMap<Box<str>, Atom>,
    /// Namespace declarations (`xmlns` / `xmlns:prefix`) per element, in
    /// source order, sorted by element. Few elements declare any, so a side
    /// table costs less than a field on every node.
    ns_defs: Vec<(NodeId, Vec<(Option<String>, String)>)>,
}

impl Default for XmlDoc {
    fn default() -> Self {
        Self::xml_new_doc(Some("1.0"))
    }
}

impl XmlDoc {
    /// `xmlNewDoc`.
    #[doc(alias = "xmlNewDoc")]
    pub fn xml_new_doc(version: Option<&str>) -> Self {
        Self::with_node_capacity(version, 1)
    }

    /// Build a document whose node arena is sized up front. Starting a parse
    /// used to allocate the arena, push into it, then reallocate on reserve --
    /// three trips for what one sized allocation does. The document node's
    /// name is implied by its kind, so it stores no String either.
    pub fn with_node_capacity(version: Option<&str>, cap: usize) -> Self {
        const MAX_ARENA_BYTES: usize = 32 << 20;
        let ceiling = MAX_ARENA_BYTES / std::mem::size_of::<Node>();
        // A floor of 4 keeps a small document from reallocating its way up
        // from a single slot, which is what Vec::new()+push used to give it.
        let mut nodes = Vec::with_capacity(cap.clamp(4, ceiling));
        nodes.push(Node::new(NodeKind::Document, String::new()));
        Self {
            nodes,
            version: version.unwrap_or("1.0").to_string(),
            encoding: None,
            standalone: None,
            root: None,
            dtd: None,
            undeclared_entity_refs: Vec::new(),
            namespace_errors: Vec::new(),
            warnings: Vec::new(),
            reference_text: Default::default(),
            elements_with_entity_refs: Default::default(),
            html_bare_attrs: Vec::new(),
            atoms: Vec::new(),
            atom_index: HashMap::new(),
            ns_defs: Vec::new(),
        }
    }

    /// The string an [`Atom`] of this document stands for.
    #[inline]
    pub fn atom(&self, a: Atom) -> &str {
        &self.atoms[a.0.get() as usize - 1]
    }

    /// Intern `s` in this document.
    pub fn intern(&mut self, s: &str) -> Atom {
        let hit = if self.atom_index.is_empty() {
            self.atoms
                .iter()
                .position(|a| **a == *s)
                .map(|i| Atom(NonZeroU32::new(i as u32 + 1).expect("i + 1 > 0")))
        } else {
            self.atom_index.get(s).copied()
        };
        if let Some(a) = hit {
            return a;
        }
        self.atoms.push(s.into());
        let a = Atom(NonZeroU32::new(self.atoms.len() as u32).expect("len > 0"));
        if self.atoms.len() > ATOM_SCAN_MAX {
            if self.atom_index.is_empty() {
                for (i, t) in self.atoms.iter().enumerate() {
                    let at = Atom(NonZeroU32::new(i as u32 + 1).expect("i + 1 > 0"));
                    self.atom_index.insert(t.clone(), at);
                }
            } else {
                self.atom_index.insert(s.into(), a);
            }
        }
        a
    }

    /// Set a node's namespace prefix.
    pub fn set_prefix(&mut self, id: NodeId, prefix: Option<&str>) {
        let a = prefix.map(|p| self.intern(p));
        self.node_mut(id).prefix = a;
    }

    /// Set a node's namespace URI.
    pub fn set_ns_uri(&mut self, id: NodeId, uri: Option<&str>) {
        let a = uri.map(|u| self.intern(u));
        self.node_mut(id).ns_uri = a;
    }

    /// Number of nodes the arena can hold without reallocating.
    pub fn node_capacity(&self) -> usize {
        self.nodes.capacity()
    }

    /// Release node-arena capacity no node uses. A parser reserves from the
    /// input length, which overshoots on markup-light documents; the HTML
    /// parser calls this once its tree is complete.
    pub fn shrink_nodes(&mut self) {
        self.nodes.shrink_to_fit();
    }

    /// Pre-size the node arena. XML runs about one node per 10-15 input bytes,
    /// so a parser that knows the document length can skip most of the arena's
    /// doubling-and-copy. Capped so a huge document cannot reserve wildly.
    pub fn reserve_nodes(&mut self, n: usize) {
        // Measured node density is ~1 per 10-12 input bytes. The previous cap
        // of 65_536 was below a 700 KB document's node count, so the arena
        // still doubled-and-copied its way up -- about 22 MB of memcpy on a
        // 627 KB file. Shrinking Node itself would need an API break for a
        // sub-1% effect; reserving correctly removes the same traffic for free.
        // Cap by memory, not node count: 1<<20 nodes is ~190 MB of arena, which
        // a large document would commit up front before parsing a byte.
        const MAX_ARENA_BYTES: usize = 32 << 20;
        let cap = MAX_ARENA_BYTES / std::mem::size_of::<Node>();
        self.nodes.reserve(n.min(cap));
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.index()]
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.index()]
    }

    pub fn kind(&self, id: NodeId) -> NodeKind {
        self.node(id).kind
    }

    pub fn name(&self, id: NodeId) -> &str {
        let n = self.node(id);
        if n.name.is_empty() {
            // Nodes whose name is fixed by their kind store no String at all;
            // allocating "#text" once per text node was a measurable share of
            // every parse. The canonical name is derived here instead.
            return match n.kind {
                NodeKind::Text => "#text",
                NodeKind::CData => "#cdata-section",
                NodeKind::Comment => "#comment",
                NodeKind::Document => "#document",
                _ => "",
            };
        }
        &n.name
    }

    pub fn prefix(&self, id: NodeId) -> Option<&str> {
        self.node(id).prefix.map(|a| self.atom(a))
    }

    pub fn ns_uri(&self, id: NodeId) -> Option<&str> {
        self.node(id).ns_uri.map(|a| self.atom(a))
    }

    pub fn content(&self, id: NodeId) -> &str {
        &self.node(id).content
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).parent.get()
    }

    pub fn first_child(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).first_child.get()
    }

    pub fn last_child(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).last_child.get()
    }

    pub fn next_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).next_sibling.get()
    }

    pub fn prev_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).prev_sibling.get()
    }

    pub fn first_attr(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).first_attr.get()
    }

    pub fn ns_defs(&self, id: NodeId) -> &[(Option<String>, String)] {
        match self.ns_defs.binary_search_by_key(&id.0, |(n, _)| n.0) {
            Ok(i) => &self.ns_defs[i].1,
            Err(_) => &[],
        }
    }

    pub fn last_attr(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).last_attr.get()
    }

    /// Allocate a node whose name is implied by its kind, storing no String.
    /// [`XmlDoc::name`] reports the canonical name for these.
    pub fn alloc_unnamed(&mut self, kind: NodeKind) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node::new(kind, String::new()));
        id
    }

    pub fn alloc(&mut self, kind: NodeKind, name: impl Into<String>) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node::new(kind, name.into()));
        id
    }

    /// `xmlDocGetRootElement`.
    #[doc(alias = "xmlDocGetRootElement")]
    pub fn xml_doc_get_root_element(&self) -> Option<NodeId> {
        if let Some(r) = self.root {
            return Some(r);
        }
        let mut c = self.first_child(NodeId::DOCUMENT);
        while let Some(id) = c {
            if self.kind(id) == NodeKind::Element {
                return Some(id);
            }
            c = self.next_sibling(id);
        }
        None
    }

    /// `xmlDocSetRootElement`. Returns the previous root, if any.
    #[doc(alias = "xmlDocSetRootElement")]
    pub fn xml_doc_set_root_element(&mut self, elem: NodeId) -> Option<NodeId> {
        let prev = self.xml_doc_get_root_element();
        if let Some(p) = prev {
            self.xml_unlink_node(p);
        }
        self.xml_add_child(NodeId::DOCUMENT, elem);
        self.root = Some(elem);
        prev
    }

    /// `xmlNewNode`.
    #[doc(alias = "xmlNewNode")]
    pub fn xml_new_node(&mut self, ns_uri: Option<&str>, name: &str) -> NodeId {
        let id = self.alloc(NodeKind::Element, name);
        self.set_ns_uri(id, ns_uri);
        id
    }

    /// `xmlNewDocNode`.
    #[doc(alias = "xmlNewDocNode")]
    pub fn xml_new_doc_node(
        &mut self,
        ns_uri: Option<&str>,
        name: &str,
        content: Option<&str>,
    ) -> NodeId {
        let id = self.xml_new_node(ns_uri, name);
        if let Some(c) = content {
            if !c.is_empty() {
                let t = self.alloc(NodeKind::Text, "#text");
                self.node_mut(t).content = c.to_string();
                self.xml_add_child(id, t);
            }
        }
        id
    }

    /// `xmlNewChild`.
    #[doc(alias = "xmlNewChild")]
    pub fn xml_new_child(
        &mut self,
        parent: NodeId,
        ns_uri: Option<&str>,
        name: &str,
        content: Option<&str>,
    ) -> NodeId {
        let id = self.xml_new_doc_node(ns_uri, name, content);
        self.xml_add_child(parent, id);
        id
    }

    /// `xmlAddChild`.
    #[doc(alias = "xmlAddChild")]
    pub fn xml_add_child(&mut self, parent: NodeId, child: NodeId) {
        if child == parent {
            return;
        }
        self.xml_unlink_node(child);
        self.node_mut(child).parent = Link::from(Some(parent));
        let last = self.node(parent).last_child.get();
        if let Some(l) = last {
            self.node_mut(l).next_sibling = Link::from(Some(child));
            self.node_mut(child).prev_sibling = Link::from(Some(l));
        } else {
            self.node_mut(parent).first_child = Link::from(Some(child));
        }
        self.node_mut(parent).last_child = Link::from(Some(child));
        // `xmlDocGetRootElement` is the FIRST element child of the document.
        // Caching every appended element made a second root -- which HTML
        // content after `</html>` creates, as in libxml2 -- take the root
        // over. Only fill an empty cache, and fill it with the first element.
        if parent == NodeId::DOCUMENT && self.root.is_none() && self.kind(child) == NodeKind::Element {
            self.root = self.xml_doc_get_root_element();
        }
    }

    /// `xmlAddNextSibling`.
    #[doc(alias = "xmlAddNextSibling")]
    pub fn xml_add_next_sibling(&mut self, cur: NodeId, elem: NodeId) {
        self.xml_unlink_node(elem);
        let parent = self.node(cur).parent.get();
        let next = self.node(cur).next_sibling.get();
        self.node_mut(elem).parent = Link::from(parent);
        self.node_mut(elem).prev_sibling = Link::from(Some(cur));
        self.node_mut(elem).next_sibling = Link::from(next);
        self.node_mut(cur).next_sibling = Link::from(Some(elem));
        if let Some(n) = next {
            self.node_mut(n).prev_sibling = Link::from(Some(elem));
        } else if let Some(p) = parent {
            self.node_mut(p).last_child = Link::from(Some(elem));
        }
    }

    /// `xmlAddPrevSibling`.
    #[doc(alias = "xmlAddPrevSibling")]
    pub fn xml_add_prev_sibling(&mut self, cur: NodeId, elem: NodeId) {
        self.xml_unlink_node(elem);
        let parent = self.node(cur).parent.get();
        let prev = self.node(cur).prev_sibling.get();
        self.node_mut(elem).parent = Link::from(parent);
        self.node_mut(elem).next_sibling = Link::from(Some(cur));
        self.node_mut(elem).prev_sibling = Link::from(prev);
        self.node_mut(cur).prev_sibling = Link::from(Some(elem));
        if let Some(p) = prev {
            self.node_mut(p).next_sibling = Link::from(Some(elem));
        } else if let Some(par) = parent {
            self.node_mut(par).first_child = Link::from(Some(elem));
        }
    }

    /// `xmlUnlinkNode`.
    #[doc(alias = "xmlUnlinkNode")]
    pub fn xml_unlink_node(&mut self, id: NodeId) {
        if id == NodeId::DOCUMENT {
            return;
        }
        let parent = self.node(id).parent.get();
        let prev = self.node(id).prev_sibling.get();
        let next = self.node(id).next_sibling.get();
        if let Some(p) = prev {
            self.node_mut(p).next_sibling = Link::from(next);
        }
        if let Some(n) = next {
            self.node_mut(n).prev_sibling = Link::from(prev);
        }
        if let Some(par) = parent {
            if self.node(par).first_child.get() == Some(id) {
                self.node_mut(par).first_child = Link::from(next);
            }
            if self.node(par).last_child.get() == Some(id) {
                self.node_mut(par).last_child = Link::from(prev);
            }
        }
        if self.root == Some(id) {
            self.root = None;
        }
        self.node_mut(id).parent = Link::from(None);
        self.node_mut(id).prev_sibling = Link::from(None);
        self.node_mut(id).next_sibling = Link::from(None);
    }

    /// `xmlReplaceNode`.
    #[doc(alias = "xmlReplaceNode")]
    pub fn xml_replace_node(&mut self, old: NodeId, new: NodeId) -> NodeId {
        self.xml_add_next_sibling(old, new);
        self.xml_unlink_node(old);
        new
    }

    /// As [`XmlDoc::add_attr`], but takes ownership. The borrowing form has to
    /// allocate a fresh String for the name, the prefix and the value, all of
    /// which the parser already owns.
    pub fn add_attr_owned(
        &mut self,
        elem: NodeId,
        name: String,
        prefix: Option<String>,
        value: String,
    ) -> NodeId {
        let id = self.alloc(NodeKind::Attribute, name);
        let prefix = prefix.map(|p| self.intern(&p));
        self.node_mut(id).prefix = prefix;
        self.node_mut(id).content = value;
        self.node_mut(id).parent = Link::from(Some(elem));
        let last = self.node(elem).last_attr.get();
        if let Some(l) = last {
            self.node_mut(l).next_sibling = Link::from(Some(id));
            self.node_mut(id).prev_sibling = Link::from(Some(l));
        } else {
            self.node_mut(elem).first_attr = Link::from(Some(id));
        }
        self.node_mut(elem).last_attr = Link::from(Some(id));
        id
    }

    pub fn add_attr(&mut self, elem: NodeId, name: &str, prefix: Option<&str>, value: &str) -> NodeId {
        let id = self.alloc(NodeKind::Attribute, name);
        self.set_prefix(id, prefix);
        self.node_mut(id).content = value.to_string();
        self.node_mut(id).parent = Link::from(Some(elem));
        let last = self.node(elem).last_attr.get();
        if let Some(l) = last {
            self.node_mut(l).next_sibling = Link::from(Some(id));
            self.node_mut(id).prev_sibling = Link::from(Some(l));
        } else {
            self.node_mut(elem).first_attr = Link::from(Some(id));
        }
        self.node_mut(elem).last_attr = Link::from(Some(id));
        id
    }

    pub fn push_ns_def(&mut self, elem: NodeId, prefix: Option<String>, uri: String) {
        // A parser declares on the element it just created, so the common
        // case appends to the last entry or after it.
        match self.ns_defs.last_mut() {
            Some((n, defs)) if *n == elem => defs.push((prefix, uri)),
            Some((n, _)) if n.0 > elem.0 => {
                match self.ns_defs.binary_search_by_key(&elem.0, |(n, _)| n.0) {
                    Ok(i) => self.ns_defs[i].1.push((prefix, uri)),
                    Err(i) => self.ns_defs.insert(i, (elem, vec![(prefix, uri)])),
                }
            }
            _ => self.ns_defs.push((elem, vec![(prefix, uri)])),
        }
    }

    /// `xmlSetProp`.
    #[doc(alias = "xmlSetProp")]
    pub fn xml_set_prop(&mut self, node: NodeId, name: &str, value: &str) -> NodeId {
        let mut a = self.first_attr(node);
        while let Some(id) = a {
            if self.node(id).prefix.is_none() && self.node(id).name == name {
                self.node_mut(id).content = value.to_string();
                return id;
            }
            a = self.next_sibling(id);
        }
        self.add_attr(node, name, None, value)
    }

    /// `xmlGetProp`.
    #[doc(alias = "xmlGetProp")]
    pub fn xml_get_prop(&self, node: NodeId, name: &str) -> Option<String> {
        let mut a = self.first_attr(node);
        while let Some(id) = a {
            if self.node(id).prefix.is_none() && self.node(id).name == name {
                return Some(self.node(id).content.clone());
            }
            a = self.next_sibling(id);
        }
        None
    }

    /// `xmlHasProp`.
    #[doc(alias = "xmlHasProp")]
    pub fn xml_has_prop(&self, node: NodeId, name: &str) -> bool {
        self.xml_get_prop(node, name).is_some()
    }

    /// `xmlUnsetProp`.
    #[doc(alias = "xmlUnsetProp")]
    pub fn xml_unset_prop(&mut self, node: NodeId, name: &str) -> bool {
        let mut a = self.first_attr(node);
        let mut prev: Option<NodeId> = None;
        while let Some(id) = a {
            let next = self.next_sibling(id);
            if self.node(id).prefix.is_none() && self.node(id).name == name {
                if let Some(p) = prev {
                    self.node_mut(p).next_sibling = Link::from(next);
                } else {
                    self.node_mut(node).first_attr = Link::from(next);
                }
                if next.is_none() {
                    self.node_mut(node).last_attr = Link::from(prev);
                }
                if let Some(n) = next {
                    self.node_mut(n).prev_sibling = Link::from(prev);
                }
                self.node_mut(id).parent = Link::from(None);
                self.node_mut(id).prev_sibling = Link::from(None);
                self.node_mut(id).next_sibling = Link::from(None);
                return true;
            }
            prev = Some(id);
            a = next;
        }
        false
    }

    /// `xmlNodeGetContent` — concatenate descendant text/CDATA.
    #[doc(alias = "xmlNodeGetContent")]
    pub fn xml_node_get_content(&self, id: NodeId) -> String {
        match self.kind(id) {
            NodeKind::Text | NodeKind::CData | NodeKind::Comment | NodeKind::Pi | NodeKind::Attribute => {
                self.content(id).to_string()
            }
            _ => {
                let mut out = String::new();
                self.collect_text(id, &mut out);
                out
            }
        }
    }

    /// Append the text and CDATA under `id`, descending into elements, in
    /// document order.
    ///
    /// Walked through the sibling and parent links rather than by recursion:
    /// the recursive form cost a stack frame per level, so reading the content
    /// of a deep document (the parser allows 5,000 levels, a million under
    /// `XML_PARSE_HUGE`) overflowed the stack, which aborts the process.
    fn collect_text(&self, id: NodeId, out: &mut String) {
        let mut c = self.first_child(id);
        while let Some(ch) = c {
            match self.kind(ch) {
                NodeKind::Text | NodeKind::CData => out.push_str(self.content(ch)),
                NodeKind::Element => {
                    if let Some(first) = self.first_child(ch) {
                        c = Some(first);
                        continue;
                    }
                }
                _ => {}
            }
            // The next node in document order: a sibling, or the sibling of
            // the nearest ancestor below `id` that has one.
            let mut n = ch;
            c = loop {
                if let Some(s) = self.next_sibling(n) {
                    break Some(s);
                }
                match self.parent(n) {
                    Some(p) if p != id => n = p,
                    _ => break None,
                }
            };
        }
    }

    /// `xmlNodeSetContent` — replace children with a single text node.
    #[doc(alias = "xmlNodeSetContent")]
    pub fn xml_node_set_content(&mut self, id: NodeId, content: &str) {
        match self.kind(id) {
            NodeKind::Text | NodeKind::CData | NodeKind::Comment | NodeKind::Pi | NodeKind::Attribute => {
                self.node_mut(id).content = content.to_string();
            }
            _ => {
                let mut c = self.first_child(id);
                while let Some(ch) = c {
                    let next = self.next_sibling(ch);
                    self.xml_unlink_node(ch);
                    c = next;
                }
                if !content.is_empty() {
                    let t = self.alloc(NodeKind::Text, "#text");
                    self.node_mut(t).content = content.to_string();
                    self.xml_add_child(id, t);
                }
            }
        }
    }

    /// `xmlIsBlankNode`.
    #[doc(alias = "xmlIsBlankNode")]
    pub fn xml_is_blank_node(&self, id: NodeId) -> bool {
        match self.kind(id) {
            NodeKind::Text | NodeKind::CData => self.content(id).chars().all(|c| {
                c == ' ' || c == '\t' || c == '\n' || c == '\r'
            }),
            _ => false,
        }
    }

    /// `xmlSearchNs` — walk ancestors for a prefix binding.
    #[doc(alias = "xmlSearchNs")]
    pub fn xml_search_ns(&self, node: NodeId, prefix: Option<&str>) -> Option<String> {
        if prefix == Some("xml") {
            return Some("http://www.w3.org/XML/1998/namespace".into());
        }
        if prefix == Some("xmlns") {
            return Some("http://www.w3.org/2000/xmlns/".into());
        }
        let mut cur = Some(node);
        while let Some(id) = cur {
            for (p, uri) in self.ns_defs(id) {
                if p.as_deref() == prefix {
                    return Some(uri.clone());
                }
            }
            cur = self.parent(id);
        }
        None
    }

    /// `xmlNewNs` — add a namespace declaration on an element.
    #[doc(alias = "xmlNewNs")]
    pub fn xml_new_ns(&mut self, node: NodeId, href: &str, prefix: Option<&str>) {
        self.push_ns_def(node, prefix.map(str::to_string), href.to_string());
    }

    /// `xmlSetNs`.
    #[doc(alias = "xmlSetNs")]
    pub fn xml_set_ns(&mut self, node: NodeId, href: Option<&str>, prefix: Option<&str>) {
        self.set_ns_uri(node, href);
        self.set_prefix(node, prefix);
    }

    /// `xmlCopyDoc` — deep copy.
    #[doc(alias = "xmlCopyDoc")]
    pub fn xml_copy_doc(&self) -> XmlDoc {
        self.clone()
    }

    pub fn qname(&self, id: NodeId) -> String {
        match self.prefix(id) {
            Some(p) => format!("{}:{}", p, self.name(id)),
            None => self.name(id).to_string(),
        }
    }

    pub fn children(&self, id: NodeId) -> NodeIter<'_> {
        NodeIter {
            doc: self,
            next: self.first_child(id),
        }
    }

    pub fn attrs(&self, id: NodeId) -> NodeIter<'_> {
        NodeIter {
            doc: self,
            next: self.first_attr(id),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

/// Sibling iterator.
pub struct NodeIter<'a> {
    doc: &'a XmlDoc,
    next: Option<NodeId>,
}

impl Iterator for NodeIter<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let n = self.next?;
        self.next = self.doc.next_sibling(n);
        Some(n)
    }
}

/// `xmlFreeDoc` is `Drop`.
#[doc(alias = "xmlFreeDoc")]
pub fn xml_free_doc(_doc: XmlDoc) {}

impl XmlDoc {
    /// Deep-copy every child of `src_parent` in `src` under `dst_parent` here.
    ///
    /// Needed because an entity whose replacement text contains markup has to
    /// become NODES, not the escaped text of that markup. `<!ENTITY e
    /// "<b>x</b>">` used in content produced the literal string `<b>x</b>` in
    /// the tree, so anything reading the document for structure got garbage
    /// and DTD validation saw character data where an element was declared.
    ///
    /// Iterative, like every other traversal here: the replacement is
    /// attacker-supplied and may be arbitrarily deep.
    pub fn xml_copy_children_from(
        &mut self,
        src: &XmlDoc,
        src_parent: NodeId,
        dst_parent: NodeId,
    ) {
        // (source node, destination parent), pushed so they pop in order.
        let mut stack: Vec<(NodeId, NodeId)> = Vec::new();
        let mut c = src.last_child(src_parent);
        while let Some(x) = c {
            stack.push((x, dst_parent));
            c = src.prev_sibling(x);
        }
        while let Some((s, parent)) = stack.pop() {
            let n = src.node(s);
            let copy = self.alloc(n.kind, n.name.clone());
            // Atoms belong to their document: re-intern through this one.
            self.set_prefix(copy, src.prefix(s));
            self.set_ns_uri(copy, src.ns_uri(s));
            self.node_mut(copy).content = n.content.clone();
            for (p, u) in src.ns_defs(s) {
                self.push_ns_def(copy, p.clone(), u.clone());
            }
            self.xml_add_child(parent, copy);
            // Carry the source's own judgement across rather than marking
            // everything: a literal space in an entity's replacement is still
            // ignorable whitespace, only a character reference written out in
            // that replacement is not.
            if src.reference_text.contains(&s) {
                self.reference_text.insert(copy);
            }
            // Attributes are a separate chain, not children.
            let mut a = src.first_attr(s);
            while let Some(x) = a {
                let an = src.node(x);
                let ac = self.add_attr(copy, &an.name, src.prefix(x), &an.content);
                self.set_ns_uri(ac, src.ns_uri(x));
                a = src.next_sibling(x);
            }
            let mut k = src.last_child(s);
            while let Some(x) = k {
                stack.push((x, copy));
                k = src.prev_sibling(x);
            }
        }
    }
}

#[cfg(test)]
mod layout {
    use super::*;

    /// Issue #23: a node was 184 bytes, two thirds of a parsed HTML tree's heap.
    /// Growing it again should be a decision, not an accident.
    #[test]
    fn node_stays_88_bytes_on_64_bit() {
        if std::mem::size_of::<usize>() == 8 {
            assert_eq!(std::mem::size_of::<Node>(), 88);
        }
        assert_eq!(std::mem::size_of::<Link>(), 4);
        assert_eq!(std::mem::size_of::<Option<Atom>>(), 4);
    }

    #[test]
    fn links_round_trip_and_atoms_are_shared() {
        let mut doc = XmlDoc::xml_new_doc(None);
        let a = doc.xml_new_node(Some("urn:x"), "a");
        let b = doc.xml_new_node(Some("urn:x"), "b");
        doc.xml_add_child(NodeId::DOCUMENT, a);
        doc.xml_add_child(a, b);
        assert_eq!(doc.parent(b), Some(a));
        assert_eq!(doc.parent(NodeId::DOCUMENT), None);
        assert_eq!(doc.first_child(NodeId::DOCUMENT), Some(a));
        assert_eq!(doc.node(a).ns_uri, doc.node(b).ns_uri);
        assert_eq!(doc.ns_uri(b), Some("urn:x"));
        // past the scan limit the hash index takes over and still dedupes
        let uris: Vec<String> = (0..40).map(|i| format!("urn:{i}")).collect();
        let first: Vec<Atom> = uris.iter().map(|u| doc.intern(u)).collect();
        let again: Vec<Atom> = uris.iter().map(|u| doc.intern(u)).collect();
        assert_eq!(first, again);
        assert_eq!(doc.atom(first[39]), "urn:39");
        assert_eq!(doc.intern("urn:x"), doc.node(a).ns_uri.unwrap());
    }

    #[test]
    fn ns_defs_side_table_keeps_order_per_element() {
        let mut doc = XmlDoc::xml_new_doc(None);
        let a = doc.xml_new_node(None, "a");
        let b = doc.xml_new_node(None, "b");
        doc.push_ns_def(b, Some("p".into()), "u1".into());
        doc.push_ns_def(a, None, "u0".into()); // out of creation order
        doc.push_ns_def(b, Some("q".into()), "u2".into());
        doc.push_ns_def(a, Some("r".into()), "u3".into());
        assert_eq!(doc.ns_defs(a), &[(None, "u0".to_string()), (Some("r".to_string()), "u3".to_string())]);
        assert_eq!(doc.ns_defs(b), &[(Some("p".to_string()), "u1".to_string()), (Some("q".to_string()), "u2".to_string())]);
        assert!(doc.ns_defs(NodeId::DOCUMENT).is_empty());
    }
}
