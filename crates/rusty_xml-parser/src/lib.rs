//! Well-formed XML parser (libxml2 `parser.h` semantics). No C.

#![forbid(unsafe_code)]

mod chvalid_tables;
pub mod chvalid;
mod error;
mod encoding_tables;
mod encoding_tables_more;
mod encoding;
mod parse;
mod catalog;
mod html_entities;
mod html;
mod xinclude;
mod dtd;

pub use error::*;
pub use chvalid::*;
pub use encoding::*;
pub use parse::*;
pub use catalog::*;
pub use html::*;
pub use html_entities::{decode_html_text, html_entity};
pub use xinclude::*;
pub use dtd::*;
