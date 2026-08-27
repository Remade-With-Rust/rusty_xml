# Changelog

All notable changes to this project are documented here. The format is loosely
based on [Keep a Changelog](https://keepachangelog.com/); this project uses
[Semantic Versioning](https://semver.org/).

## [0.2.0]

A performance release. Parsing a document is **65x faster** than 0.1.0 on real
content, and `rxmlint` now parses faster than pinned libxml2 `xmllint` on the
same files. Every step was gated byte-identical against the C oracle.

### Fixed

- **Parsing was quadratic in document length.** `peek_char` ran
  `std::str::from_utf8` over the *whole remaining buffer* to decode one
  character, so a parse re-validated the tail once per character. A 302 KB
  document validated 58.9 GB of input. A UTF-8 scalar is at most four bytes, so
  only those are checked now. This alone was ~55x on a 308 KB file.
- **DTD-defaulted attributes serialised in a different order on every run** of
  the same binary, because the declaration map is a `HashMap` with a randomly
  seeded hasher. This crate ships C14N for XML-DSig, where a signature over a
  non-reproducible serialisation is worthless. The defaults are now sorted.
- **The tree parse was recording a SAX event log and discarding it.**
  `xml_read_memory` ran a `SaxRecorder`, deep-copying the name, prefix, URI,
  namespace list and every attribute of every element into a `Vec` that was then
  dropped. It was over half of all allocations on an attribute-heavy document.

### Changed

- Allocations per parse fell **72%** on a 627 KB attribute-heavy document
  (450,079 -> 126,039) and **64%** on a 308 KB text document, with allocated
  bytes down 76% and 79%. Attribute nodes take owned strings; QNames are split
  once and moved rather than copied; character data and attribute values are
  taken in runs; the end tag is compared in place; nodes whose name is implied
  by their kind store no `String`; the node arena is sized from measured
  density.
- Starting a parse session costs **5 allocations instead of 8** (`<a/>`), and a
  63-byte document 10 instead of 15 with half the bytes. `sniff_encoding_decl`
  had been building a 1 KB `String` *and* a lowercased copy of it on every
  single parse.
- `peek_char` is no longer called at all on an all-ASCII document; every caller
  reaches a byte lane. The multi-byte decoder is unchanged and still gated.
- **`rxmlint` and the bench crate now ship `rusty_alloc`** through the
  `rusty_xml-alloc` seam. The library still declares no `#[global_allocator]`
  and does not depend on it -- an embedding application keeps that choice.

### Added

- `XmlDoc::add_attr_owned`, `XmlDoc::alloc_unnamed`,
  `XmlDoc::with_node_capacity`, `XmlDoc::reserve_nodes`.
- `NullSax`, a handler that discards every callback, for tree-only parses.
- `corpora/big-300k.xml` and `corpora/big-attr.xml` so the published board is
  reproducible. The shipped corpus was too small to measure: every row of the
  0.1 board was flagged `DURATION_SHORT` and dominated by process startup.

### Breaking

- A node whose name is implied by its kind (`#text`, `#comment`,
  `#cdata-section`, `#document`) now stores an empty `String`.
  `XmlDoc::name()` is unchanged and still returns the canonical name; code
  reading the `pub name` field **directly** will see `""`.

### Notes

- Entity handling still diverges from C and is the next correctness work: we
  substitute general entities where libxml2 keeps reference nodes without
  `XML_PARSE_NOENT`, and a nested reference is emitted as escaped literal text.
- libxml2 splits `characters` callbacks at multi-byte boundaries; we coalesce.

## [0.1.0]

First public release on [crates.io](https://crates.io/crates/rusty_xml).

### Added

- **Parser:** UTF-8 well-formed XML 1.0, 15 built-in 8-bit encodings (no iconv),
  push (`xml_parse_chunk`), IO callbacks, local OASIS catalogs. SAX traces gated
  line-for-line against pinned `xmllint --sax` (libxml2 v2.15.3).
- **Tree / save / writer / reader:** arena DOM, `xmlsave`, `xmlTextWriter`,
  `xmlTextReader`. `parse(write(parse(x)))` standing gate.
- **XPath 1.0** compile + eval; `rxmlint --xpath` prints xmllint form.
- **DTD** internal subset + default attributes, `xml_validate_document`.
- **C14N 1.0** and exclusive canonicalization.
- **HTML** parser (implied html/head/body) and working subsets of RelaxNG, XML
  Schema, and Schematron.
- **`rxmlint`** CLI (never `xmllint`) with `--noout`, `--sax`, `--stream`,
  `--xpath`, `--c14n`, `--html`, `--push`, `--dtdvalid`, `--relaxng`,
  `--schema`, `--schematron`, `--repeat`.
- Safe defaults: `XML_PARSE_NONET | XML_PARSE_NO_XXE` always.

### Notes

- Every published crate is `#![forbid(unsafe_code)]`. No `libxml2-sys`.
- Gzip (`XML_PARSE_UNZIP`) is not wired. Japanese encodings that need iconv in
  C are unsupported here too.
- C ABI (`rusty_xml-c-abi`) is a stub until M8.
- Speed vs pinned `xmllint` is the M7 campaign — the 0.1 session board is in
  `bench/SIDE-BY-SIDE.md` and is not a publish claim.
