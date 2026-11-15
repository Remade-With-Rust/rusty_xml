# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.3](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.9.2...rusty_xml-v0.9.3) - 2026-11-15

### Performance

- SAX with `XML_PARSE_NO_TREE` now streams: no element node is built, so memory stays at the input size (354 MB 3MF model part: peak working set 1,461 MB -> 342 MB, time 7.25-8.58 s -> 1.90-2.02 s). Names are read in place and attribute buffers are pooled, which also makes ordinary tree parsing 15.5% cheaper in instructions on the XML benchmark. Output is unchanged (#52).

### Other

- *(parser)* stream SAX without a tree, read names in place ([#52](https://github.com/Remade-With-Rust/rusty_xml/pull/52))

## [0.9.2](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.9.1...rusty_xml-v0.9.2) - 2026-11-14

### Changed

- HTML parsing now stops at libxml2's nesting cap: an element deeper than 256 (2048 under `XML_PARSE_HUGE`) is dropped, the parse halts with the tree built so far, and `XmlDoc::warnings` records `XML_ERR_RESOURCE_LIMIT`. This matches `xmllint --html` and changes output only for documents nested past 256. `html_read_memory_with` with `HtmlParseSettings::with_max_depth(-1)` restores 0.9.1's uncapped parse (#48).

### Other

- *(html)* size the node arena from the markup, not the length ([#49](https://github.com/Remade-With-Rust/rusty_xml/pull/49))

## [0.9.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.9.0...rusty_xml-v0.9.1) - 2026-11-14

### Changed

- `xml_xpath_cmp_nodes` now returns libxml2's `xmlXPathCmpNodes` values: 1 when the first node precedes the second, -1 when it follows, 0 for the same node, -2 for a node outside the document or nodes in different trees. 0.9.0 returned the opposite sign. Callers that sort with it, or test `== 1` for "before", should check their order (#38).

### Added

- *(tree)* add the Interrupt trait for cooperative cancellation ([#39](https://github.com/Remade-With-Rust/rusty_xml/pull/39))

### Fixed

- *(parser)* report the error a recovering parse survived; pin #44 and #45
- *(parser)* close the unbounded paths found in the #41 review
- *(parser)* bound entity amplification as libxml2 does, keep push errors, add stop and cancel controls (#32, #35, #39, #40)

### Other

- state the new depth, amplification, content-model, XPath and cancellation guarantees
- Merge branch 'fix/valid-36-39' into int/issues-32-41
- Merge branch 'fix/xpath-34-37-38-40' into int/issues-32-41
- *(html)* keep the deep-stack test's stack deep

## [0.9.0](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.8.5...rusty_xml-v0.9.0) - 2026-11-13

### Fixed

- *(html)* decide the input encoding as libxml2 does, honouring <meta charset> ([#27](https://github.com/Remade-With-Rust/rusty_xml/pull/27))
- *(html)* write CR, TAB and LF raw in HTML output, as libxml2 does

### Other

- Merge pull request #20 from vinhch/feat/html-fixtures-16

## [0.8.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.8.4...rusty_xml-v0.8.5) - 2026-11-13

### Fixed

- *(html)* doctype, unterminated comment and root rows from #16

### Other

- *(fuzz)* require HTML stability from the second round trip, as libxml2 does
- sync the crate README with the root README

## [0.8.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.8.3...rusty_xml-v0.8.4) - 2026-11-12

### Other

- sync the crate README with the root README

## [0.8.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.8.0...rusty_xml-v0.8.1) - 2026-08-29

### Other

- an In the wild block above the headline ([#5](https://github.com/Remade-With-Rust/rusty_xml/pull/5))
