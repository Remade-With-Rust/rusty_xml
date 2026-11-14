//! DTD validation, C14N, RelaxNG, XML Schema, Schematron.

#![forbid(unsafe_code)]

mod c14n;
mod content;
mod interrupt;
mod rng;
mod xsd;
mod schematron;

use content::ContentModel;
use interrupt::{NoPoll, Poll};
use rusty_xml_tree::{
    AttrDefault, ElementDecl, Interrupt, InterruptPoll, NodeId, NodeKind, XmlDoc, XmlDtd,
};
use std::collections::HashMap;

pub use c14n::*;
pub use interrupt::ValidError;
pub use rng::*;
pub use xsd::*;
pub use schematron::*;

/// `xmlValidateDocument` against the document's attached DTD.
#[doc(alias = "xmlValidateDocument")]
pub fn xml_validate_document(doc: &XmlDoc) -> Result<(), String> {
    let dtd = doc.dtd.as_ref().ok_or("no DTD")?;
    xml_validate_dtd(doc, dtd)
}

/// [`xml_validate_document`], stopping with [`ValidError::Interrupted`] once
/// `interrupt` fires. It is polled before the walk and every few hundred
/// elements.
pub fn xml_validate_document_interruptible(
    doc: &XmlDoc,
    interrupt: &dyn Interrupt,
) -> Result<(), ValidError> {
    let dtd = doc.dtd.as_ref().ok_or("no DTD")?;
    xml_validate_dtd_interruptible(doc, dtd, interrupt)
}

/// `xmlValidateDtd`.
#[doc(alias = "xmlValidateDtd")]
pub fn xml_validate_dtd(doc: &XmlDoc, dtd: &XmlDtd) -> Result<(), String> {
    validate_dtd(doc, dtd, &mut NoPoll).map_err(ValidError::into_message)
}

/// [`xml_validate_dtd`], stopping with [`ValidError::Interrupted`] once
/// `interrupt` fires.
pub fn xml_validate_dtd_interruptible(
    doc: &XmlDoc,
    dtd: &XmlDtd,
    interrupt: &dyn Interrupt,
) -> Result<(), ValidError> {
    validate_dtd(doc, dtd, &mut InterruptPoll::new(interrupt))
}

fn validate_dtd<P: Poll>(doc: &XmlDoc, dtd: &XmlDtd, poll: &mut P) -> Result<(), ValidError> {
    poll.check()?;
    let root = check_declarations(doc, dtd)?;
    // Walk iteratively: validation used to recurse per element, which is the
    // same stack cliff the parser, the writer and C14N all had.
    let mut ids: HashMap<String, ()> = Default::default();
    let mut idrefs: Vec<String> = Vec::new();
    // Each element type's content model is compiled once, on first use, and
    // shared by every element of that type.
    let mut models: HashMap<String, Result<ContentModel, String>> = HashMap::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        poll.tick()?;
        validate_element(doc, id, dtd, &mut ids, &mut idrefs, &mut models)?;
        let mut c = doc.last_child(id);
        while let Some(x) = c {
            if doc.kind(x) == NodeKind::Element {
                stack.push(x);
            }
            c = doc.prev_sibling(x);
        }
    }
    // IDREF VC: every referenced ID must be declared somewhere in the
    // document. This was never checked at all.
    for r in &idrefs {
        if !ids.contains_key(r) {
            return Err(format!("IDREF attribute references unknown ID \"{r}\"").into());
        }
    }
    Ok(())
}

/// The constraints on the declarations themselves; returns the root element.
fn check_declarations(doc: &XmlDoc, dtd: &XmlDtd) -> Result<NodeId, String> {
    let root = doc
        .xml_doc_get_root_element()
        .ok_or("document has no root")?;
    if let Some(n) = &dtd.name {
        // The DOCTYPE names a QName, so `<!DOCTYPE xml:foo>` must be compared
        // against `xml:foo` and not against the local part alone.
        if doc.qname(root) != *n {
            return Err(format!("root element {} does not match DOCTYPE {n}", doc.qname(root)));
        }
    }
    // Constraints on the DECLARATIONS themselves, before any instance of them
    // is looked at. None of these was checked, so a declaration could promise
    // something no document could satisfy.
    // An entity the parse could not resolve is a validity error when the
    // subset was incomplete -- it stopped being a well-formedness one, but it
    // did not stop being an error.
    if let Some(e) = doc.undeclared_entity_refs.first() {
        return Err(format!("Entity '{e}' not defined"));
    }
    // "Notation Declared": a notation named by an NDATA annotation or by a
    // NOTATION attribute type has to have been declared. We were parsing
    // <!NOTATION> and throwing the name away, so nothing could tell.
    for n in &dtd.ndata_notations {
        if !dtd.notations.contains(n) {
            return Err(format!("Notation {n} is not declared"));
        }
    }
    for ((elem, aname), ad) in &dtd.attributes {
        if ad.att_type != "NOTATION" {
            continue;
        }
        for n in &ad.enumerated {
            if !dtd.notations.contains(n) {
                return Err(format!(
                    "Notation {n} in attribute {aname} of {elem} is not declared"
                ));
            }
        }
    }
    // "Unique Element Type Declaration": no element type may be declared more
    // than once. A HashMap cannot say so on its own, so the parser records it.
    if let Some(dup) = dtd.duplicate_elements.first() {
        return Err(format!("Redefinition of element {dup}"));
    }
    // "No Duplicate Types": a mixed content model may not name the same
    // element twice.
    for (elem, decl) in &dtd.elements {
        if let ElementDecl::Mixed(names) = decl {
            let mut seen = std::collections::HashSet::new();
            for n in names {
                if !seen.insert(n) {
                    return Err(format!(
                        "Definition of {elem} has duplicate references of {n}"
                    ));
                }
            }
        }
    }
    for ((elem, aname), ad) in &dtd.attributes {
        // "No Duplicate Tokens": an enumeration may not repeat a value.
        let mut seen = std::collections::HashSet::new();
        for e in &ad.enumerated {
            if !seen.insert(e) {
                return Err(format!(
                    "attribute {aname} of {elem}: enumeration value token {e} duplicated"
                ));
            }
        }
        // "ID Attribute Default": an ID attribute must be #IMPLIED or
        // #REQUIRED -- it cannot carry a default value, since two elements
        // taking the default would share an ID.
        if ad.att_type == "ID"
            && matches!(ad.default, AttrDefault::Value | AttrDefault::Fixed)
        {
            return Err(format!(
                "ID attribute {aname} of {elem} is not valid, must be #IMPLIED or #REQUIRED"
            ));
        }
        // "No Notation on Empty Element": an element declared EMPTY cannot
        // carry a NOTATION attribute, since it can never have content for the
        // notation to describe.
        if ad.att_type == "NOTATION" && matches!(dtd.elements.get(elem), Some(ElementDecl::Empty)) {
            return Err(format!(
                "NOTATION attribute type declared for EMPTY element {elem}"
            ));
        }
        // "Attribute Default Value Syntactically Correct": the default has to
        // satisfy the type it is declared with.
        let Some(def) = ad.default_value.as_deref() else {
            continue;
        };
        let ok = match ad.att_type.as_str() {
            "ID" | "IDREF" | "ENTITY" => is_name(def),
            "IDREFS" | "ENTITIES" => {
                def.split_space().next().is_some()
                    && def.split_space().all(is_name)
            }
            "NMTOKEN" => is_nmtoken(def),
            "NMTOKENS" => {
                def.split_space().next().is_some()
                    && def.split_space().all(is_nmtoken)
            }
            _ => true,
        };
        if !ok {
            return Err(format!("invalid default value for attribute {aname} of {elem}"));
        }
        if !ad.enumerated.is_empty() && !ad.enumerated.iter().any(|e| e == def) {
            return Err(format!("invalid default value for attribute {aname} of {elem}"));
        }
    }
    Ok(root)
}

/// Split a list-typed attribute value on #x20, and only on #x20.
///
/// The tokenized list types are `Nmtoken (#x20 Nmtoken)*`. Literal tab and
/// newline become spaces during attribute-value normalization, but a character
/// reference contributes its character unchanged -- so `abc&#9;xyz` is ONE
/// token holding a tab, and not a valid Nmtoken at all. Splitting on any
/// whitespace turned it into two tokens that both looked fine.
trait SplitSpace {
    fn split_space(&self) -> std::iter::Filter<std::str::Split<'_, char>, fn(&&str) -> bool>;
}

impl SplitSpace for str {
    fn split_space(&self) -> std::iter::Filter<std::str::Split<'_, char>, fn(&&str) -> bool> {
        fn non_empty(s: &&str) -> bool {
            !s.is_empty()
        }
        self.split(' ').filter(non_empty as fn(&&str) -> bool)
    }
}

/// A Name, as the ID / IDREF validity constraints require.
fn is_name(v: &str) -> bool {
    let mut cs = v.chars();
    match cs.next() {
        Some(c) if rusty_xml_parser::chvalid::xml_is_name_start_char(c as u32, false) => {}
        _ => return false,
    }
    cs.all(|c| rusty_xml_parser::chvalid::xml_is_name_char(c as u32, false))
}

/// An Nmtoken: like a Name but with no restriction on the first character.
fn is_nmtoken(v: &str) -> bool {
    !v.is_empty()
        && v.chars()
            .all(|c| rusty_xml_parser::chvalid::xml_is_name_char(c as u32, false))
}

fn validate_element(
    doc: &XmlDoc,
    id: NodeId,
    dtd: &XmlDtd,
    ids: &mut HashMap<String, ()>,
    idrefs: &mut Vec<String>,
    models: &mut HashMap<String, Result<ContentModel, String>>,
) -> Result<(), String> {
    if doc.kind(id) != NodeKind::Element {
        return Ok(());
    }
    // Declarations name QNames, so an element declared `<!ELEMENT xml:foo>` is
    // looked up as `xml:foo`. Using the local part alone reported a declared
    // element as undeclared.
    let name = doc.qname(id);
    // "Element Valid": an element with no declaration is invalid, and nothing
    // said so. A DTD that declares nothing at all is not a validating DTD, so
    // only complain when there are declarations to be missing from.
    // Not gated on the DTD declaring anything: validating against a subset
    // that declares no elements should fail for every element, which is what C
    // reports. Requiring a non-empty map let a DTD of pure entity declarations
    // validate any document at all.
    if !dtd.elements.contains_key(&name) {
        return Err(format!("No declaration for element {name}"));
    }
    if let Some(decl) = dtd.elements.get(&name) {
        match decl {
            ElementDecl::Empty => {
                // An entity reference is content even when it expands to
                // nothing: `<foo>&empty;</foo>` leaves no node behind, so the
                // child list alone cannot see it.
                if doc.first_child(id).is_some()
                    || doc.elements_with_entity_refs.contains(&id)
                {
                    return Err(format!("element {name} must be EMPTY"));
                }
            }
            ElementDecl::Any => {}
            ElementDecl::Mixed(_) => {
                let mut c = doc.first_child(id);
                while let Some(x) = c {
                    match doc.kind(x) {
                        NodeKind::Element => {
                            if let ElementDecl::Mixed(allowed) = decl {
                                if !allowed.is_empty() && !allowed.iter().any(|n| *n == doc.qname(x)) {
                                    return Err(format!("element {} not allowed in mixed {name}", doc.qname(x)));
                                }
                            }
                        }
                        NodeKind::Text | NodeKind::CData | NodeKind::Comment | NodeKind::Pi => {}
                        _ => {}
                    }
                    c = doc.next_sibling(x);
                }
            }
            ElementDecl::Children(spec) => {
                let kids: Vec<String> = {
                    let mut v = Vec::new();
                    let mut c = doc.first_child(id);
                    while let Some(x) = c {
                        if doc.kind(x) == NodeKind::Element {
                            v.push(doc.qname(x));
                        } else if doc.kind(x) == NodeKind::Text
                            && (!doc.xml_is_blank_node(x) || doc.reference_text.contains(&x))
                        {
                            // Whitespace that arrived as `&#32;` is character
                            // data, not the ignorable indentation beside it,
                            // and only the parser can tell them apart.
                            return Err(format!("character data not allowed in {name}"));
                        } else if doc.kind(x) == NodeKind::CData {
                            // A CDATA section is character data whatever is in
                            // it. Whitespace inside one is never the ignorable
                            // kind, so an empty `<![CDATA[]]>` still breaks an
                            // element-only content model -- and we were only
                            // looking at Text nodes.
                            return Err(format!("character data not allowed in {name}"));
                        }
                        c = doc.next_sibling(x);
                    }
                    v
                };
                if !models.contains_key(&name) {
                    models.insert(name.clone(), ContentModel::compile(spec));
                }
                let model = models[&name].as_ref().map_err(String::clone)?;
                // libxml2 refuses a model that is not deterministic (XML 1.0
                // Appendix E) when it first validates an element of that type.
                if let Some(printed) = model.nondeterministic() {
                    return Err(format!(
                        "Content model of {name} is not deterministic: {printed}"
                    ));
                }
                if !model.matches(&kids) {
                    return Err(format!("content of {name} does not match {spec}"));
                }
            }
        }
    }
    for ((elem, aname), ad) in &dtd.attributes {
        if elem != &name {
            continue;
        }
        // An ATTLIST declares a QName, so `xml:lang` is looked up as
        // `xml:lang`. xml_get_prop is xmlGetProp -- it matches unprefixed
        // attributes only -- so every prefixed declared attribute looked
        // absent, and a #REQUIRED one was reported missing on a document that
        // plainly had it.
        let have = {
            let mut found = None;
            let mut a = doc.first_attr(id);
            while let Some(x) = a {
                if doc.qname(x) == *aname {
                    found = Some(doc.content(x).to_string());
                    break;
                }
                a = doc.next_sibling(x);
            }
            found
        };
        match ad.default {
            AttrDefault::Required if have.is_none() => {
                return Err(format!("attribute {aname} of {name} is required"));
            }
            AttrDefault::Fixed => {
                if let (Some(v), Some(fix)) = (&have, &ad.default_value) {
                    if v != fix {
                        return Err(format!("attribute {aname} must be {fix}"));
                    }
                }
            }
            _ => {}
        }
        if let Some(v) = &have {
            if !ad.enumerated.is_empty() && !ad.enumerated.iter().any(|e| e == v) {
                return Err(format!("attribute {aname} value not in enumeration"));
            }
            // The tokenized types carry validity constraints on their VALUES,
            // and not one of them was enforced -- the ID branch said
            // "uniqueness checked loosely", which meant not at all.
            match ad.att_type.as_str() {
                "ID" | "IDREF" => {
                    // Namespaces in XML erratum NE05: an ID or IDREF value is
                    // an NCName, so a colon in one is a validity error. This
                    // is stricter than libxml2, which does not check it -- but
                    // it is a VALIDITY constraint, so it costs nothing at
                    // parse time and refuses no document anyone can read.
                    if v.contains(':') {
                        return Err(format!(
                            "Value {v} for attribute {aname} of {name} is not an NCName"
                        ));
                    }
                    if !is_name(v) {
                        return Err(format!(
                            "Syntax of value for attribute {aname} of {name} is not valid"
                        ));
                    }
                    if ad.att_type == "ID" {
                        if ids.insert(v.clone(), ()).is_some() {
                            return Err(format!("ID {v} already defined"));
                        }
                    } else {
                        idrefs.push(v.clone());
                    }
                }
                "IDREFS" => {
                    let mut any = false;
                    for part in v.split_space() {
                        any = true;
                        if part.contains(':') {
                            return Err(format!(
                                "Value {part} for attribute {aname} of {name} is not an NCName"
                            ));
                        }
                        if !is_name(part) {
                            return Err(format!(
                                "Syntax of value for attribute {aname} of {name} is not valid"
                            ));
                        }
                        idrefs.push(part.to_string());
                    }
                    if !any {
                        return Err(format!(
                            "Syntax of value for attribute {aname} of {name} is not valid"
                        ));
                    }
                }
                "NMTOKEN" => {
                    if !is_nmtoken(v) {
                        return Err(format!(
                            "Syntax of value for attribute {aname} of {name} is not valid"
                        ));
                    }
                }
                "NMTOKENS" => {
                    if v.split_space().next().is_none()
                        || !v.split_space().all(is_nmtoken)
                    {
                        return Err(format!(
                            "Syntax of value for attribute {aname} of {name} is not valid"
                        ));
                    }
                }
                "ENTITY" | "ENTITIES" => {
                    for part in v.split_space() {
                        if !is_name(part) {
                            return Err(format!(
                                "Syntax of value for attribute {aname} of {name} is not valid"
                            ));
                        }
                        if !dtd.unparsed_entities.contains(part) {
                            return Err(format!(
                                "ENTITY attribute {aname} references an unknown entity \"{part}\""
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // "Attribute Value Type": every attribute an element carries must be
    // declared for that element type. Nothing checked, so `xml:space` on an
    // element whose ATTLIST never mentions it was fine by us.
    // A namespace declaration is an attribute as far as validity is concerned:
    // it has to be declared, and a #FIXED default still binds its value. We
    // keep ns declarations off the attribute chain, so nothing looked at them
    // at all.
    for (pre, uri) in doc.ns_defs(id) {
        let q = match pre {
            Some(p) => format!("xmlns:{p}"),
            None => "xmlns".to_string(),
        };
        match dtd.attributes.get(&(name.clone(), q.clone())) {
            None => return Err(format!("No declaration for attribute {q} of element {name}")),
            Some(ad) => {
                if ad.default == AttrDefault::Fixed {
                    if let Some(fix) = ad.default_value.as_deref() {
                        if uri != fix {
                            return Err(format!(
                                "Value for attribute {q} of {name} is different from default {fix}"
                            ));
                        }
                    }
                }
            }
        }
    }
    {
        let mut a = doc.first_attr(id);
        while let Some(x) = a {
            let q = doc.qname(x);
            if !dtd.attributes.contains_key(&(name.clone(), q.clone())) {
                return Err(format!("No declaration for attribute {q} of element {name}"));
            }
            a = doc.next_sibling(x);
        }
    }
    // "One ID per Element Type": an element type may carry at most one ID
    // attribute, however the declarations are spread across ATTLISTs.
    let id_attrs = dtd
        .attributes
        .iter()
        .filter(|((e, _), ad)| e == &name && ad.att_type == "ID")
        .count();
    if id_attrs > 1 {
        return Err(format!("Element {name} has {id_attrs} ID attributes"));
    }
    Ok(())
}
