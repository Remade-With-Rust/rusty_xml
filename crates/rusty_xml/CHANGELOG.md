# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.6](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.10.5...rusty_xml-v0.10.6) - 2026-11-23

### Other

- bump rusty_alloc-api to 2.6.0 ([#90](https://github.com/Remade-With-Rust/rusty_xml/pull/90))

## [0.10.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.10.4...rusty_xml-v0.10.5) - 2026-11-20

### Fixed

- ship LICENSE-MIT, LICENSE-APACHE and NOTICE.md in every published crate

## [0.10.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.10.3...rusty_xml-v0.10.4) - 2026-11-20

### Performance

- `xml_sax_parse_memory` and `xml_read_memory` start each parse from the buffers the thread's last one-shot parse grew, as an `XmlSaxParser` does. A thread keeps at most 64 KiB (12 KiB after the 202 XML parts of four PPTX files); a parse that grows more frees its buffers as before.
- Namespace declarations reuse their frames and prefix and URI buffers. The per-parse setup no longer allocates for encoding names, the XML declaration's version and standalone values, or a no-tree arena that was shrunk back.
- A SAX parse of a real PPTX part now makes 3 or 4 allocations, fresh or reused (0.10.3: 43 to 85 fresh, 17 to 48 reused).
- Windows, system allocator, 2,000 real PPTX parts, pinned ABBA (20 pairs): a fresh `xml_sax_parse_memory` per part is 8.6% faster (54.4 to 49.9 ms) and a reused `XmlSaxParser` 2.2% faster. Reuse is now worth 0.2% over a fresh parse, down from 7.3%.
- callgrind Ir on the same parts: 2.75% fewer instructions fresh, 0.92% fewer reused. Tree and HTML parsing are flat (−0.08%, −0.05%). Output is identical to 0.10.3 ([#80](https://github.com/Remade-With-Rust/rusty_xml/issues/80), [#85](https://github.com/Remade-With-Rust/rusty_xml/pull/85)).

### Other

- Tests: one-shot parses on the thread's pools match parses on a new thread, including a parse made inside a handler callback; README on the per-thread buffers.

## [0.10.3](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.10.2...rusty_xml-v0.10.3) - 2026-11-20

### Added

- `XmlSaxParser`: a SAX parser kept from one document to the next, for readers of many small documents such as the XML parts of a DOCX, XLSX or PPTX package. It keeps only buffer capacity, so events, errors and the returned document are those of `xml_sax_parse_memory`, after a failed or stopped parse too ([#80](https://github.com/Remade-With-Rust/rusty_xml/issues/80), [#81](https://github.com/Remade-With-Rust/rusty_xml/pull/81)).

### Performance

- SAX parsing of namespace-heavy documents takes 24.4% fewer instructions on 2,000 real PPTX parts with `XML_PARSE_NO_TREE` (1,115.8M to 843.7M Ir, callgrind), and 25.9% fewer with one `XmlSaxParser` reused across them. Changes:
  - each element's namespace is looked up once instead of three times;
  - an element that declares nothing pushes no scope frame;
  - prefixes compare as packed keys;
  - names are scanned as a slice;
  - three per-element calls the compiler could not remove are gone (empty-Vec drops, `memcmp` for `/>`, `memcpy` for a prefix key).
- Tree parsing gets the same changes: 6.7% fewer instructions on the 8-file XML benchmark. Output is identical to 0.10.2 ([#80](https://github.com/Remade-With-Rust/rusty_xml/issues/80), [#81](https://github.com/Remade-With-Rust/rusty_xml/pull/81)).

### Other

- README: `XmlSaxParser` for many small documents

## [0.10.2](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.10.1...rusty_xml-v0.10.2) - 2026-11-18

### Other

- push-parse length hint ([#75](https://github.com/Remade-With-Rust/rusty_xml/pull/75)); one tree walk, made cheaper ([#76](https://github.com/Remade-With-Rust/rusty_xml/pull/76))
- correct the 0.10 install line and the 0.9.3 SAX numbers

## [0.10.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.10.0...rusty_xml-v0.10.1) - 2026-11-16

### Performance

- SAX allocates nothing per end tag or prefixed attribute in steady state: 6% to 8% fewer instructions per element, 20% to 29% faster on Windows; the XML arena is sized from the markup and push results are shrunk; every workload measured is 0.1% to 0.4% cheaper in instructions ([#69](https://github.com/Remade-With-Rust/rusty_xml/issues/69), [#70](https://github.com/Remade-With-Rust/rusty_xml/issues/70))

### Changed

- `xml_parse_dtd` honours text declarations and refuses undeclared invalid UTF-8; XInclude text honours `encoding`; character references to non-`Char` values inside entities are refused everywhere ([#70](https://github.com/Remade-With-Rust/rusty_xml/issues/70))

### Added

- one settings type, interruptible and encoding-aware DTD/XInclude/reader entries (#70, #71)

### Other

- *(readme)* DTD, XInclude and reader cancellation through ParseSettings (#70, #71)
- Merge branch 'b/70-entries' into release/0.10.1

## [0.10.0](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.9.5...rusty_xml-v0.10.0) - 2026-11-15

### Changed

- `Node::name` and `Node::content` are now `CompactString` (re-exported as `rusty_xml::CompactString`) instead of `String`, and `XmlDoc::alloc` takes `impl Into<CompactString>`. Reading them is unchanged: they deref to `&str` and compare with `str` and `String`, and `XmlDoc::name()` / `content()` still return `&str`. To migrate, assign with `.into()` (`node.content = s.into()`) and call `.to_string()` where an owned `String` is needed. Short names and text are stored inline: on 197 real pages, parsing is about 20% faster, dropping a tree about 45% faster and the trees hold 18% less memory (#62).

### Added

- *(tree)* [**breaking**] keep short node names and text inline with compact_str ([#62](https://github.com/Remade-With-Rust/rusty_xml/pull/62))

## [0.9.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.9.4...rusty_xml-v0.9.5) - 2026-11-15

### Fixed

- RelaxNG: sequences keep element order, a choice reconsiders an alternative that took too much, and sibling runs are linear; 11 of libxml2's test verdicts now agree with xmllint ([#57](https://github.com/Remade-With-Rust/rusty_xml/issues/57))
- DTD loading: INCLUDE/IGNORE sections, parameter entities expanded and charged once (TEI P5 loads), first declaration binds (DocBook 4.5 loads) ([#58](https://github.com/Remade-With-Rust/rusty_xml/issues/58))

### Performance

- DTD validation indexes attribute declarations and shares follow sets; Schematron can be interrupted inside one test ([#58](https://github.com/Remade-With-Rust/rusty_xml/issues/58))
- HTML arena reserve of 17/8 nodes per `<`: parse instructions -3.0% to -5.9% ([#62](https://github.com/Remade-With-Rust/rusty_xml/issues/62))

### Other

- *(readme)* RelaxNG sibling runs and DTD scaling bounds (#57, #58)

## [0.9.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.9.3...rusty_xml-v0.9.4) - 2026-11-15

### Fixed

- *(valid)* memoise RelaxNG matching once it backtracks ([#56](https://github.com/Remade-With-Rust/rusty_xml/pull/56))

## [0.9.3](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.9.2...rusty_xml-v0.9.3) - 2026-11-15

### Performance

- SAX with `XML_PARSE_NO_TREE` now streams: no element node is built, so memory stays at the input size (138 MB 3MF model part, 3M elements, measured in [#53](https://github.com/Remade-With-Rust/rusty_xml/pull/53): peak working set 434 MB -> 136 MB, time 2.04-2.10 s -> 0.57-0.60 s). Names are read in place and attribute buffers are pooled, which also makes ordinary tree parsing 15.5% cheaper in instructions on the XML benchmark. Output is unchanged (#52).

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
