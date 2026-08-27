//! Allocator seam for rusty_xml **binaries**.
//!
//! The published library never declares `#[global_allocator]` and never depends
//! on this crate: a library that picks the allocator forces that choice on every
//! consumer, and two such libraries cannot be linked together. The choice and
//! its version pin live here, in one place; the `#[global_allocator]` attribute
//! itself belongs to each deliverable.
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOC: rusty_xml_alloc::Allocator = rusty_xml_alloc::NEW;
//! ```

#![forbid(unsafe_code)]
#![no_std]

/// The allocator every rusty_xml deliverable ships with: `rusty_alloc`, the
/// pure-Rust remake of mimalloc. No C, and it builds for `wasm32`.
pub use rusty_alloc_api::RustyAlloc as Allocator;

/// A ready-made instance for the `#[global_allocator]` static.
pub const NEW: Allocator = Allocator;

/// The seam's identity, kept for the workspace layout check.
pub const SEAM: &str = "rusty_xml-alloc";
