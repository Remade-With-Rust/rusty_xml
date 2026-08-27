//! Local XML Catalogs (OASIS). No network. `XML_PARSE_NO_SYS_CATALOG` respected by callers.

use rusty_xml_tree::{NodeKind, XmlDoc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CAT_NS: &str = "urn:oasis:names:tc:entity:xmlns:xml:catalog";

#[derive(Clone, Debug, Default)]
pub struct XmlCatalog {
    pub public: HashMap<String, String>,
    pub system: HashMap<String, String>,
    pub uri: HashMap<String, String>,
    pub rewrite_system: Vec<(String, String)>,
    pub rewrite_uri: Vec<(String, String)>,
    pub next_catalog: Vec<PathBuf>,
}

impl XmlCatalog {
    /// `xmlLoadCatalog`.
    #[doc(alias = "xmlLoadCatalog")]
    pub fn xml_load_catalog(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let doc = crate::xml_read_memory(&bytes, path.to_str(), None, crate::default_parse_options())
            .map_err(|e| e.to_string())?;
        let mut cat = XmlCatalog::default();
        walk(&doc, doc.xml_doc_get_root_element(), &mut cat, path.parent().unwrap_or(path));
        Ok(cat)
    }

    /// `xmlCatalogResolve`.
    #[doc(alias = "xmlCatalogResolve")]
    pub fn xml_catalog_resolve(&self, public_id: Option<&str>, system_id: Option<&str>) -> Option<String> {
        if let Some(p) = public_id {
            if let Some(u) = self.public.get(p) {
                return Some(u.clone());
            }
        }
        if let Some(s) = system_id {
            if let Some(u) = self.system.get(s) {
                return Some(u.clone());
            }
            for (prefix, replace) in &self.rewrite_system {
                if s.starts_with(prefix) {
                    return Some(format!("{}{}", replace, &s[prefix.len()..]));
                }
            }
        }
        None
    }

    /// `xmlCatalogResolveURI`.
    #[doc(alias = "xmlCatalogResolveURI")]
    pub fn xml_catalog_resolve_uri(&self, uri: &str) -> Option<String> {
        if let Some(u) = self.uri.get(uri) {
            return Some(u.clone());
        }
        for (prefix, replace) in &self.rewrite_uri {
            if uri.starts_with(prefix) {
                return Some(format!("{}{}", replace, &uri[prefix.len()..]));
            }
        }
        None
    }
}

fn walk(doc: &XmlDoc, node: Option<rusty_xml_tree::NodeId>, cat: &mut XmlCatalog, base: &Path) {
    let Some(id) = node else { return };
    if doc.kind(id) == NodeKind::Element {
        let name = doc.name(id);
        let ns = doc.ns_uri(id);
        let in_cat = ns == Some(CAT_NS) || ns.is_none();
        if in_cat {
            match name {
                "public" => {
                    if let (Some(idv), Some(uri)) = (doc.xml_get_prop(id, "publicId"), doc.xml_get_prop(id, "uri")) {
                        cat.public.insert(idv, resolve_uri(base, &uri));
                    }
                }
                "system" => {
                    if let (Some(idv), Some(uri)) = (doc.xml_get_prop(id, "systemId"), doc.xml_get_prop(id, "uri")) {
                        cat.system.insert(idv, resolve_uri(base, &uri));
                    }
                }
                "uri" => {
                    if let (Some(name), Some(uri)) = (doc.xml_get_prop(id, "name"), doc.xml_get_prop(id, "uri")) {
                        cat.uri.insert(name, resolve_uri(base, &uri));
                    }
                }
                "rewriteSystem" => {
                    if let (Some(p), Some(r)) = (
                        doc.xml_get_prop(id, "systemIdStartString"),
                        doc.xml_get_prop(id, "rewritePrefix"),
                    ) {
                        cat.rewrite_system.push((p, resolve_uri(base, &r)));
                    }
                }
                "rewriteURI" => {
                    if let (Some(p), Some(r)) = (
                        doc.xml_get_prop(id, "uriStartString"),
                        doc.xml_get_prop(id, "rewritePrefix"),
                    ) {
                        cat.rewrite_uri.push((p, resolve_uri(base, &r)));
                    }
                }
                "nextCatalog" => {
                    if let Some(c) = doc.xml_get_prop(id, "catalog") {
                        cat.next_catalog.push(base.join(c));
                    }
                }
                _ => {}
            }
        }
        let mut ch = doc.first_child(id);
        while let Some(c) = ch {
            walk(doc, Some(c), cat, base);
            ch = doc.next_sibling(c);
        }
    }
}

fn resolve_uri(base: &Path, uri: &str) -> String {
    if uri.contains("://") {
        uri.to_string()
    } else {
        base.join(uri).to_string_lossy().into_owned()
    }
}

/// `xmlInitializeCatalog` — no-op (no process-global catalog).
#[doc(alias = "xmlInitializeCatalog")]
pub fn xml_initialize_catalog() {}

/// `xmlCatalogCleanup` — no-op.
#[doc(alias = "xmlCatalogCleanup")]
pub fn xml_catalog_cleanup() {}
