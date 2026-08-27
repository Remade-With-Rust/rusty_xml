//! Allocator seam for rusty_xml **binaries**.
//!
//! The published library never declares `#[global_allocator]` and never depends
//! on `rusty_alloc`. Deliverables (`rxmlint`, the bench) may wire rusty_alloc
//! through this crate later. Today it is a documented no-op so the workspace
//! layout matches `docs/plan/rusty_xml.md` §5.

#![forbid(unsafe_code)]

/// Placeholder so the crate is not empty.
pub const SEAM: &str = "rusty_xml-alloc";
