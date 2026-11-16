//! `xmlParserErrors` discriminants kept identical to libxml2 `xmlerror.h`.

use std::fmt;

pub const XML_ERR_OK: i32 = 0;
pub const XML_ERR_INTERNAL_ERROR: i32 = 1;
pub const XML_ERR_NO_MEMORY: i32 = 2;
pub const XML_ERR_DOCUMENT_START: i32 = 3;
pub const XML_ERR_DOCUMENT_EMPTY: i32 = 4;
pub const XML_ERR_DOCUMENT_END: i32 = 5;
pub const XML_ERR_INVALID_HEX_CHARREF: i32 = 6;
pub const XML_ERR_INVALID_DEC_CHARREF: i32 = 7;
pub const XML_ERR_INVALID_CHARREF: i32 = 8;
pub const XML_ERR_INVALID_CHAR: i32 = 9;
pub const XML_ERR_ENTITYREF_NO_NAME: i32 = 22;
pub const XML_ERR_ENTITYREF_SEMICOL_MISSING: i32 = 23;
pub const XML_ERR_UNDECLARED_ENTITY: i32 = 26;
pub const XML_ERR_UNSUPPORTED_ENCODING: i32 = 32;
pub const XML_ERR_LT_IN_ATTRIBUTE: i32 = 38;
pub const XML_ERR_ATTRIBUTE_NOT_STARTED: i32 = 39;
pub const XML_ERR_ATTRIBUTE_WITHOUT_VALUE: i32 = 41;
pub const XML_ERR_ATTRIBUTE_REDEFINED: i32 = 42;
pub const XML_ERR_LITERAL_NOT_FINISHED: i32 = 44;
pub const XML_ERR_COMMENT_NOT_FINISHED: i32 = 45;
pub const XML_ERR_PI_NOT_FINISHED: i32 = 47;
pub const XML_ERR_XMLDECL_NOT_FINISHED: i32 = 57;
/// An IGNORE section is not closed.
pub const XML_ERR_CONDSEC_NOT_FINISHED: i32 = 59;
/// The external subset has content that is not a declaration, or an
/// INCLUDE section that is not closed.
pub const XML_ERR_EXT_SUBSET_NOT_FINISHED: i32 = 60;
pub const XML_ERR_MISPLACED_CDATA_END: i32 = 62;
pub const XML_ERR_CDATA_NOT_FINISHED: i32 = 63;
pub const XML_ERR_RESERVED_XML_NAME: i32 = 64;
pub const XML_ERR_SPACE_REQUIRED: i32 = 65;
pub const XML_ERR_NAME_REQUIRED: i32 = 68;
pub const XML_ERR_GT_REQUIRED: i32 = 73;
pub const XML_ERR_LT_REQUIRED: i32 = 72;
pub const XML_ERR_EQUAL_REQUIRED: i32 = 75;
pub const XML_ERR_TAG_NAME_MISMATCH: i32 = 76;
pub const XML_ERR_TAG_NOT_FINISHED: i32 = 77;
pub const XML_ERR_ENCODING_NAME: i32 = 79;
pub const XML_ERR_HYPHEN_IN_COMMENT: i32 = 80;
/// Bytes not valid in the encoding in force (#70).
pub const XML_ERR_INVALID_ENCODING: i32 = 81;
/// A conditional section keyword is not followed by `[`.
pub const XML_ERR_CONDSEC_INVALID: i32 = 83;
pub const XML_ERR_EXTRA_CONTENT: i32 = 86;
pub const XML_ERR_ENTITY_LOOP: i32 = 89;
/// A text declaration without its required `encoding` (#70).
pub const XML_ERR_MISSING_ENCODING: i32 = 101;
/// A conditional section keyword is neither INCLUDE nor IGNORE.
pub const XML_ERR_CONDSEC_INVALID_KEYWORD: i32 = 95;
/// A SAX handler, [`crate::xml_stop_parser`] or an [`rusty_xml_tree::Interrupt`] stopped the parse.
pub const XML_ERR_USER_STOP: i32 = 111;
/// A resource limit was exceeded, such as the entity amplification factor.
pub const XML_ERR_RESOURCE_LIMIT: i32 = 114;
pub const XML_WAR_NS_URI_RELATIVE: i32 = 100;
pub const XML_NS_ERR_XML_NAMESPACE: i32 = 200;
pub const XML_NS_ERR_UNDEFINED_NAMESPACE: i32 = 201;
pub const XML_NS_ERR_QNAME: i32 = 202;
pub const XML_NS_ERR_ATTRIBUTE_REDEFINED: i32 = 203;

/// Parser / tree error with C discriminant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlError {
    pub code: i32,
    pub message: String,
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: error code {}: {}",
            self.line, self.col, self.code, self.message
        )
    }
}

impl std::error::Error for XmlError {}

impl XmlError {
    pub fn new(code: i32, message: impl Into<String>, line: u32, col: u32) -> Self {
        Self {
            code,
            message: message.into(),
            line,
            col,
        }
    }
}

/// The error every interruptible entry reports once its interrupt fired:
/// libxml2's `XML_ERR_USER_STOP` (#71: one constructor, not one per entry).
pub(crate) fn interrupted_error() -> XmlError {
    XmlError::new(XML_ERR_USER_STOP, "Parser interrupted", 0, 0)
}

/// Record a fatal error on a tree a parse hands back anyway (recover mode,
/// the HTML depth cap): one line, shared by both parsers (#71).
pub(crate) fn record_fatal(doc: &mut rusty_xml_tree::XmlDoc, e: &XmlError) {
    doc.warnings.push(e.to_string());
}
