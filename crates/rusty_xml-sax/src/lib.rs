//! SAX2 events and an xmllint `--sax` debug dump.
//!
//! Structured equality is the gate. The debug printer exists so a line-for-line
//! diff against pinned `xmllint --sax` is possible; C's `%.4s` attribute quirk
//! (reads past the value into the input) is reproduced when `value_input_off` is set.

#![forbid(unsafe_code)]

/// One SAX2 callback as recorded for the event-exact gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaxEvent {
    SetDocumentLocator,
    StartDocument,
    EndDocument,
    StartElementNs {
        local: String,
        prefix: Option<String>,
        uri: Option<String>,
        namespaces: Vec<(Option<String>, String)>,
        attributes: Vec<SaxAttr>,
        nb_defaulted: i32,
    },
    EndElementNs {
        local: String,
        prefix: Option<String>,
        uri: Option<String>,
    },
    Characters {
        data: String,
    },
    CData {
        data: String,
    },
    Comment(String),
    ProcessingInstruction {
        target: String,
        data: Option<String>,
    },
    Warning(String),
    Error(String),
}

/// Attribute as delivered to `startElementNs`.
#[derive(Clone, Debug)]
pub struct SaxAttr {
    pub local: String,
    pub prefix: Option<String>,
    pub uri: Option<String>,
    pub value: String,
    /// Byte offset in the original input of the first value character (after the quote).
    /// Used only to reproduce xmllint's `%.4s` debug print.
    pub value_input_off: Option<usize>,
}

impl PartialEq for SaxAttr {
    fn eq(&self, other: &Self) -> bool {
        self.local == other.local
            && self.prefix == other.prefix
            && self.uri == other.uri
            && self.value == other.value
    }
}

impl Eq for SaxAttr {}

/// SAX2 handler. Default methods are no-ops so a recorder can override a subset.
pub trait SaxHandler {
    fn set_document_locator(&mut self) {}
    fn start_document(&mut self) {}
    fn end_document(&mut self) {}
    fn start_element_ns(
        &mut self,
        local: &str,
        prefix: Option<&str>,
        uri: Option<&str>,
        namespaces: &[(Option<String>, String)],
        attributes: &[SaxAttr],
        nb_defaulted: i32,
    ) {
        let _ = (local, prefix, uri, namespaces, attributes, nb_defaulted);
    }
    fn end_element_ns(&mut self, local: &str, prefix: Option<&str>, uri: Option<&str>) {
        let _ = (local, prefix, uri);
    }
    fn characters(&mut self, data: &str) {
        let _ = data;
    }
    fn cdata_block(&mut self, data: &str) {
        let _ = data;
    }
    fn comment(&mut self, data: &str) {
        let _ = data;
    }
    fn processing_instruction(&mut self, target: &str, data: Option<&str>) {
        let _ = (target, data);
    }
    fn warning(&mut self, msg: &str) {
        let _ = msg;
    }
    fn error(&mut self, msg: &str) {
        let _ = msg;
    }
}

/// A handler that discards every callback, using the trait's default bodies.
///
/// Building a tree does not need the SAX event stream, but the tree entry
/// points used [`SaxRecorder`], which deep-copies the local name, prefix, URI,
/// namespace list and *every attribute* of every element into a log that is
/// then dropped. On a 627 KB document that was over half of all allocations.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullSax;

impl SaxHandler for NullSax {}

/// Records every callback for the event-exact gate.
#[derive(Clone, Debug, Default)]
pub struct SaxRecorder {
    pub events: Vec<SaxEvent>,
}

impl SaxHandler for SaxRecorder {
    fn set_document_locator(&mut self) {
        self.events.push(SaxEvent::SetDocumentLocator);
    }
    fn start_document(&mut self) {
        self.events.push(SaxEvent::StartDocument);
    }
    fn end_document(&mut self) {
        self.events.push(SaxEvent::EndDocument);
    }
    fn start_element_ns(
        &mut self,
        local: &str,
        prefix: Option<&str>,
        uri: Option<&str>,
        namespaces: &[(Option<String>, String)],
        attributes: &[SaxAttr],
        nb_defaulted: i32,
    ) {
        self.events.push(SaxEvent::StartElementNs {
            local: local.to_string(),
            prefix: prefix.map(str::to_string),
            uri: uri.map(str::to_string),
            namespaces: namespaces.to_vec(),
            attributes: attributes.to_vec(),
            nb_defaulted,
        });
    }
    fn end_element_ns(&mut self, local: &str, prefix: Option<&str>, uri: Option<&str>) {
        self.events.push(SaxEvent::EndElementNs {
            local: local.to_string(),
            prefix: prefix.map(str::to_string),
            uri: uri.map(str::to_string),
        });
    }
    fn characters(&mut self, data: &str) {
        self.events.push(SaxEvent::Characters {
            data: data.to_string(),
        });
    }
    fn cdata_block(&mut self, data: &str) {
        self.events.push(SaxEvent::CData {
            data: data.to_string(),
        });
    }
    fn comment(&mut self, data: &str) {
        self.events.push(SaxEvent::Comment(data.to_string()));
    }
    fn processing_instruction(&mut self, target: &str, data: Option<&str>) {
        self.events.push(SaxEvent::ProcessingInstruction {
            target: target.to_string(),
            data: data.map(str::to_string),
        });
    }
    fn warning(&mut self, msg: &str) {
        self.events.push(SaxEvent::Warning(msg.to_string()));
    }
    fn error(&mut self, msg: &str) {
        self.events.push(SaxEvent::Error(msg.to_string()));
    }
}

impl SaxRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Line-for-line dump matching `oracle/src/xmllint.c` debug SAX handlers.
    pub fn to_xmllint_debug(&self, input: &[u8]) -> String {
        let mut out = String::new();
        for ev in &self.events {
            out.push_str(&event_to_xmllint_debug(ev, input));
        }
        out
    }
}

fn trunc_bytes(s: &str, n: usize) -> String {
    let b = s.as_bytes();
    let k = b.len().min(n);
    String::from_utf8_lossy(&b[..k]).into_owned()
}

fn opt_name(p: &Option<String>) -> String {
    match p {
        None => "NULL".into(),
        Some(s) => s.clone(),
    }
}

fn opt_uri(u: &Option<String>) -> String {
    match u {
        None => "NULL".into(),
        Some(s) => format!("'{s}'"),
    }
}

/// Format one event the way pinned xmllint `--sax` prints it.
pub fn event_to_xmllint_debug(ev: &SaxEvent, input: &[u8]) -> String {
    match ev {
        SaxEvent::SetDocumentLocator => "SAX.setDocumentLocator()\n".into(),
        SaxEvent::StartDocument => "SAX.startDocument()\n".into(),
        SaxEvent::EndDocument => "SAX.endDocument()\n".into(),
        SaxEvent::StartElementNs {
            local,
            prefix,
            uri,
            namespaces,
            attributes,
            nb_defaulted,
        } => {
            let mut s = format!(
                "SAX.startElementNs({local}, {}, {}, {}",
                opt_name(prefix),
                opt_uri(uri),
                namespaces.len()
            );
            for (pre, href) in namespaces {
                s.push_str(", xmlns");
                if let Some(p) = pre {
                    s.push(':');
                    s.push_str(p);
                }
                s.push_str(&format!("='{href}'"));
            }
            s.push_str(&format!(", {}, {nb_defaulted}", attributes.len()));
            for a in attributes {
                if let Some(p) = &a.prefix {
                    s.push_str(&format!(", {p}:{}='", a.local));
                } else {
                    s.push_str(&format!(", {}='", a.local));
                }
                let four = if let Some(off) = a.value_input_off {
                    let end = (off + 4).min(input.len());
                    if off < input.len() {
                        String::from_utf8_lossy(&input[off..end]).into_owned()
                    } else {
                        trunc_bytes(&a.value, 4)
                    }
                } else {
                    trunc_bytes(&a.value, 4)
                };
                s.push_str(&format!("{four}...', {}", a.value.len()));
            }
            s.push_str(")\n");
            s
        }
        SaxEvent::EndElementNs {
            local,
            prefix,
            uri,
        } => {
            if uri.is_none() {
                format!("SAX.endElementNs({local}, {}, NULL)\n", opt_name(prefix))
            } else {
                format!(
                    "SAX.endElementNs({local}, {}, {})\n",
                    opt_name(prefix),
                    opt_uri(uri)
                )
            }
        }
        SaxEvent::Characters { data } => {
            format!("SAX.characters({}, {})\n", trunc_bytes(data, 30), data.len())
        }
        SaxEvent::CData { data } => {
            format!("SAX.pcdata({}, {})\n", trunc_bytes(data, 20), data.len())
        }
        SaxEvent::Comment(c) => format!("SAX.comment({c})\n"),
        SaxEvent::ProcessingInstruction { target, data } => match data {
            Some(d) => format!("SAX.processingInstruction({target}, {d})\n"),
            None => format!("SAX.processingInstruction({target}, NULL)\n"),
        },
        SaxEvent::Warning(m) => format!("SAX.warning: {m}"),
        SaxEvent::Error(m) => format!("SAX.error: {m}"),
    }
}

/// `xmlSAX2InitDefaultSAXHandler` is a no-op beyond constructing a recorder.
#[doc(alias = "xmlSAX2InitDefaultSAXHandler")]
pub fn xml_sax2_init_default_sax_handler() -> SaxRecorder {
    SaxRecorder::new()
}

/// `xmlSAXVersion` — we speak SAX2.
#[doc(alias = "xmlSAXVersion")]
pub fn xml_sax_version() -> i32 {
    2
}
