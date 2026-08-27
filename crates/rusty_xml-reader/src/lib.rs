//! `xmlTextReader` pull parser. M2 walks a tree built by the UTF-8 parser.

#![forbid(unsafe_code)]

use rusty_xml_parser::{xml_read_memory, XmlError};
use rusty_xml_tree::{NodeId, NodeKind, XmlDoc};

/// libxml2 `xmlReaderTypes`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum ReaderType {
    None = 0,
    Element = 1,
    Attribute = 2,
    Text = 3,
    CData = 4,
    EntityReference = 5,
    Entity = 6,
    ProcessingInstruction = 7,
    Comment = 8,
    Document = 9,
    DocumentType = 10,
    DocumentFragment = 11,
    Notation = 12,
    Whitespace = 13,
    SignificantWhitespace = 14,
    EndElement = 15,
    EndEntity = 16,
    XmlDeclaration = 17,
}

#[derive(Clone, Copy)]
struct Cursor {
    id: NodeId,
    end: bool,
}

/// `xmlTextReader`.
pub struct XmlTextReader {
    doc: XmlDoc,
    cur: Option<Cursor>,
    started: bool,
    eof: bool,
    attr: Option<NodeId>,
}

impl XmlTextReader {
    fn kind_of(doc: &XmlDoc, id: NodeId, end: bool) -> ReaderType {
        if end {
            return ReaderType::EndElement;
        }
        match doc.kind(id) {
            NodeKind::Element => ReaderType::Element,
            NodeKind::Attribute => ReaderType::Attribute,
            NodeKind::Text => {
                if doc.xml_is_blank_node(id) {
                    ReaderType::SignificantWhitespace
                } else {
                    ReaderType::Text
                }
            }
            NodeKind::CData => ReaderType::CData,
            NodeKind::Pi => ReaderType::ProcessingInstruction,
            NodeKind::Comment => ReaderType::Comment,
            NodeKind::Document => ReaderType::Document,
            _ => ReaderType::None,
        }
    }

    /// `xmlReaderForMemory`.
    #[doc(alias = "xmlReaderForMemory")]
    pub fn xml_reader_for_memory(
        buffer: &[u8],
        url: Option<&str>,
        encoding: Option<&str>,
        options: i32,
    ) -> Result<Self, XmlError> {
        let doc = xml_read_memory(buffer, url, encoding, options)?;
        Ok(Self {
            doc,
            cur: None,
            started: false,
            eof: false,
            attr: None,
        })
    }

    /// `xmlReaderWalker`.
    #[doc(alias = "xmlReaderWalker")]
    pub fn xml_reader_walker(doc: XmlDoc) -> Self {
        Self {
            doc,
            cur: None,
            started: false,
            eof: false,
            attr: None,
        }
    }

    /// `xmlTextReaderRead`. Returns 1 (node), 0 (eof), matching C.
    #[doc(alias = "xmlTextReaderRead")]
    pub fn read(&mut self) -> i32 {
        self.attr = None;
        if self.eof {
            return 0;
        }
        if !self.started {
            self.started = true;
            if let Some(id) = self.doc.first_child(NodeId::DOCUMENT) {
                self.cur = Some(Cursor { id, end: false });
                return 1;
            }
            self.eof = true;
            return 0;
        }
        let Cursor { id, end } = match self.cur {
            Some(c) => c,
            None => {
                self.eof = true;
                return 0;
            }
        };
        if !end && self.doc.kind(id) == NodeKind::Element {
            if let Some(ch) = self.doc.first_child(id) {
                self.cur = Some(Cursor { id: ch, end: false });
                return 1;
            }
            // empty element: no EndElement
            return self.advance_after(id);
        }
        self.advance_after(id)
    }

    fn advance_after(&mut self, id: NodeId) -> i32 {
        if let Some(n) = self.doc.next_sibling(id) {
            self.cur = Some(Cursor { id: n, end: false });
            return 1;
        }
        match self.doc.parent(id) {
            Some(p) if p != NodeId::DOCUMENT && self.doc.kind(p) == NodeKind::Element => {
                self.cur = Some(Cursor { id: p, end: true });
                1
            }
            _ => {
                self.eof = true;
                self.cur = None;
                0
            }
        }
    }

    pub fn node_type(&self) -> ReaderType {
        match self.cur {
            Some(Cursor { id, end }) => Self::kind_of(&self.doc, id, end),
            None => ReaderType::None,
        }
    }

    pub fn depth(&self) -> i32 {
        let mut d = 0;
        if let Some(Cursor { mut id, .. }) = self.cur {
            while let Some(p) = self.doc.parent(id) {
                if p == NodeId::DOCUMENT {
                    break;
                }
                d += 1;
                id = p;
            }
        }
        d
    }

    pub fn is_empty_element(&self) -> bool {
        match self.cur {
            Some(Cursor { id, end: false }) if self.doc.kind(id) == NodeKind::Element => {
                self.doc.first_child(id).is_none()
            }
            _ => false,
        }
    }

    pub fn local_name(&self) -> Option<&str> {
        self.cur.map(|c| self.doc.name(c.id))
    }

    pub fn prefix(&self) -> Option<&str> {
        self.cur.and_then(|c| self.doc.prefix(c.id))
    }

    pub fn namespace_uri(&self) -> Option<&str> {
        self.cur.and_then(|c| self.doc.ns_uri(c.id))
    }

    pub fn value(&self) -> Option<&str> {
        if let Some(a) = self.attr {
            return Some(self.doc.content(a));
        }
        match self.cur {
            Some(Cursor { id, end: false }) => match self.doc.kind(id) {
                NodeKind::Text | NodeKind::CData | NodeKind::Comment | NodeKind::Pi => {
                    Some(self.doc.content(id))
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub fn name(&self) -> Option<String> {
        self.cur.map(|c| self.doc.qname(c.id))
    }

    pub fn attribute_count(&self) -> i32 {
        match self.cur {
            Some(Cursor { id, end: false }) if self.doc.kind(id) == NodeKind::Element => {
                self.doc.attrs(id).count() as i32
            }
            _ => 0,
        }
    }

    pub fn has_attributes(&self) -> bool {
        self.attribute_count() > 0
    }

    /// `xmlTextReaderGetAttribute`.
    #[doc(alias = "xmlTextReaderGetAttribute")]
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        let Cursor { id, end } = self.cur?;
        if end {
            return None;
        }
        self.doc.xml_get_prop(id, name)
    }

    /// `xmlTextReaderMoveToFirstAttribute`.
    #[doc(alias = "xmlTextReaderMoveToFirstAttribute")]
    pub fn move_to_first_attribute(&mut self) -> i32 {
        let Cursor { id, end } = match self.cur {
            Some(c) => c,
            None => return 0,
        };
        if end {
            return 0;
        }
        match self.doc.first_attr(id) {
            Some(a) => {
                self.attr = Some(a);
                1
            }
            None => 0,
        }
    }

    /// `xmlTextReaderMoveToNextAttribute`.
    #[doc(alias = "xmlTextReaderMoveToNextAttribute")]
    pub fn move_to_next_attribute(&mut self) -> i32 {
        match self.attr {
            Some(a) => match self.doc.next_sibling(a) {
                Some(n) => {
                    self.attr = Some(n);
                    1
                }
                None => 0,
            },
            None => self.move_to_first_attribute(),
        }
    }

    /// `xmlTextReaderMoveToElement`.
    #[doc(alias = "xmlTextReaderMoveToElement")]
    pub fn move_to_element(&mut self) -> i32 {
        if self.attr.is_some() {
            self.attr = None;
            1
        } else {
            0
        }
    }

    pub fn doc(&self) -> &XmlDoc {
        &self.doc
    }
}

/// `xmlReaderForMemory`.
#[doc(alias = "xmlReaderForMemory")]
pub fn xml_reader_for_memory(
    buffer: &[u8],
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlTextReader, XmlError> {
    XmlTextReader::xml_reader_for_memory(buffer, url, encoding, options)
}

/// `xmlReaderForDoc`.
#[doc(alias = "xmlReaderForDoc")]
pub fn xml_reader_for_doc(
    cur: &str,
    url: Option<&str>,
    encoding: Option<&str>,
    options: i32,
) -> Result<XmlTextReader, XmlError> {
    xml_reader_for_memory(cur.as_bytes(), url, encoding, options)
}
