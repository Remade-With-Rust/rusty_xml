//! Arena DOM matching libxml2 `tree.h` ownership: the document owns every node.
//! Handles are indices, not parent+child `&mut` pairs.

#![forbid(unsafe_code)]

use std::collections::HashMap;

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
    /// Element name → content model.
    pub elements: HashMap<String, ElementDecl>,
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



#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub name: String,
    pub prefix: Option<String>,
    pub ns_uri: Option<String>,
    pub content: String,
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub first_attr: Option<NodeId>,
    pub last_attr: Option<NodeId>,
    /// Namespace declarations on this element (`xmlns` / `xmlns:prefix`), in source order.
    pub ns_defs: Vec<(Option<String>, String)>,
}

impl Node {
    fn new(kind: NodeKind, name: String) -> Self {
        Self {
            kind,
            name,
            prefix: None,
            ns_uri: None,
            content: String::new(),
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            first_attr: None,
            last_attr: None,
            ns_defs: Vec::new(),
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
        let mut nodes = Vec::new();
        nodes.push(Node::new(NodeKind::Document, "#document".into()));
        Self {
            nodes,
            version: version.unwrap_or("1.0").to_string(),
            encoding: None,
            standalone: None,
            root: None,
            dtd: None,
        }
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
        self.nodes.reserve(n.min(1 << 20));
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
        self.node(id).prefix.as_deref()
    }

    pub fn ns_uri(&self, id: NodeId) -> Option<&str> {
        self.node(id).ns_uri.as_deref()
    }

    pub fn content(&self, id: NodeId) -> &str {
        &self.node(id).content
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).parent
    }

    pub fn first_child(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).first_child
    }

    pub fn last_child(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).last_child
    }

    pub fn next_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).next_sibling
    }

    pub fn prev_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).prev_sibling
    }

    pub fn first_attr(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).first_attr
    }

    pub fn ns_defs(&self, id: NodeId) -> &[(Option<String>, String)] {
        &self.node(id).ns_defs
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
        self.node_mut(id).ns_uri = ns_uri.map(str::to_string);
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
        self.node_mut(child).parent = Some(parent);
        let last = self.node(parent).last_child;
        if let Some(l) = last {
            self.node_mut(l).next_sibling = Some(child);
            self.node_mut(child).prev_sibling = Some(l);
        } else {
            self.node_mut(parent).first_child = Some(child);
        }
        self.node_mut(parent).last_child = Some(child);
        if parent == NodeId::DOCUMENT && self.kind(child) == NodeKind::Element {
            self.root = Some(child);
        }
    }

    /// `xmlAddNextSibling`.
    #[doc(alias = "xmlAddNextSibling")]
    pub fn xml_add_next_sibling(&mut self, cur: NodeId, elem: NodeId) {
        self.xml_unlink_node(elem);
        let parent = self.node(cur).parent;
        let next = self.node(cur).next_sibling;
        self.node_mut(elem).parent = parent;
        self.node_mut(elem).prev_sibling = Some(cur);
        self.node_mut(elem).next_sibling = next;
        self.node_mut(cur).next_sibling = Some(elem);
        if let Some(n) = next {
            self.node_mut(n).prev_sibling = Some(elem);
        } else if let Some(p) = parent {
            self.node_mut(p).last_child = Some(elem);
        }
    }

    /// `xmlAddPrevSibling`.
    #[doc(alias = "xmlAddPrevSibling")]
    pub fn xml_add_prev_sibling(&mut self, cur: NodeId, elem: NodeId) {
        self.xml_unlink_node(elem);
        let parent = self.node(cur).parent;
        let prev = self.node(cur).prev_sibling;
        self.node_mut(elem).parent = parent;
        self.node_mut(elem).next_sibling = Some(cur);
        self.node_mut(elem).prev_sibling = prev;
        self.node_mut(cur).prev_sibling = Some(elem);
        if let Some(p) = prev {
            self.node_mut(p).next_sibling = Some(elem);
        } else if let Some(par) = parent {
            self.node_mut(par).first_child = Some(elem);
        }
    }

    /// `xmlUnlinkNode`.
    #[doc(alias = "xmlUnlinkNode")]
    pub fn xml_unlink_node(&mut self, id: NodeId) {
        if id == NodeId::DOCUMENT {
            return;
        }
        let parent = self.node(id).parent;
        let prev = self.node(id).prev_sibling;
        let next = self.node(id).next_sibling;
        if let Some(p) = prev {
            self.node_mut(p).next_sibling = next;
        }
        if let Some(n) = next {
            self.node_mut(n).prev_sibling = prev;
        }
        if let Some(par) = parent {
            if self.node(par).first_child == Some(id) {
                self.node_mut(par).first_child = next;
            }
            if self.node(par).last_child == Some(id) {
                self.node_mut(par).last_child = prev;
            }
        }
        if self.root == Some(id) {
            self.root = None;
        }
        self.node_mut(id).parent = None;
        self.node_mut(id).prev_sibling = None;
        self.node_mut(id).next_sibling = None;
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
        self.node_mut(id).prefix = prefix;
        self.node_mut(id).content = value;
        self.node_mut(id).parent = Some(elem);
        let last = self.node(elem).last_attr;
        if let Some(l) = last {
            self.node_mut(l).next_sibling = Some(id);
            self.node_mut(id).prev_sibling = Some(l);
        } else {
            self.node_mut(elem).first_attr = Some(id);
        }
        self.node_mut(elem).last_attr = Some(id);
        id
    }

    pub fn add_attr(&mut self, elem: NodeId, name: &str, prefix: Option<&str>, value: &str) -> NodeId {
        let id = self.alloc(NodeKind::Attribute, name);
        self.node_mut(id).prefix = prefix.map(str::to_string);
        self.node_mut(id).content = value.to_string();
        self.node_mut(id).parent = Some(elem);
        let last = self.node(elem).last_attr;
        if let Some(l) = last {
            self.node_mut(l).next_sibling = Some(id);
            self.node_mut(id).prev_sibling = Some(l);
        } else {
            self.node_mut(elem).first_attr = Some(id);
        }
        self.node_mut(elem).last_attr = Some(id);
        id
    }

    pub fn push_ns_def(&mut self, elem: NodeId, prefix: Option<String>, uri: String) {
        self.node_mut(elem).ns_defs.push((prefix, uri));
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
                    self.node_mut(p).next_sibling = next;
                } else {
                    self.node_mut(node).first_attr = next;
                }
                if next.is_none() {
                    self.node_mut(node).last_attr = prev;
                }
                if let Some(n) = next {
                    self.node_mut(n).prev_sibling = prev;
                }
                self.node_mut(id).parent = None;
                self.node_mut(id).prev_sibling = None;
                self.node_mut(id).next_sibling = None;
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

    fn collect_text(&self, id: NodeId, out: &mut String) {
        let mut c = self.first_child(id);
        while let Some(ch) = c {
            match self.kind(ch) {
                NodeKind::Text | NodeKind::CData => out.push_str(self.content(ch)),
                NodeKind::Element => self.collect_text(ch, out),
                _ => {}
            }
            c = self.next_sibling(ch);
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
        self.node_mut(node).ns_uri = href.map(str::to_string);
        self.node_mut(node).prefix = prefix.map(str::to_string);
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
