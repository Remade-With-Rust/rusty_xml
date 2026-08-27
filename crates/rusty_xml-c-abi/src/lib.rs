//! Optional cdylib exporting libxml2 C names — stub until mission M8.
//!
//! This crate must not become a default feature of `rusty_xml`.

#![forbid(unsafe_code)]

/// C ABI is not implemented in M0–M2.
pub const C_ABI_MISSION: &str = "M8";
