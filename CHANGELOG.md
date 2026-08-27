# Changelog

All notable changes to this project are documented here. The format is loosely
based on [Keep a Changelog](https://keepachangelog.com/); this project uses
[Semantic Versioning](https://semver.org/).

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
