# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.10.3...rusty_xml-parser-v0.10.4) - 2026-11-20

### Performance

- `xml_sax_parse_memory` and `xml_read_memory` start each parse from the buffers the thread's last one-shot parse grew, as an `XmlSaxParser` does. A thread keeps at most 64 KiB (12 KiB after the 202 XML parts of four PPTX files); a parse that grows more frees its buffers as before.
- Namespace declarations reuse their frames and prefix and URI buffers. The per-parse setup no longer allocates for encoding names, the XML declaration's version and standalone values, or a no-tree arena that was shrunk back.
- A SAX parse of a real PPTX part now makes 3 or 4 allocations, fresh or reused (0.10.3: 43 to 85 fresh, 17 to 48 reused).
- Windows, system allocator, 2,000 real PPTX parts, pinned ABBA (20 pairs): a fresh `xml_sax_parse_memory` per part is 8.6% faster (54.4 to 49.9 ms) and a reused `XmlSaxParser` 2.2% faster. Reuse is now worth 0.2% over a fresh parse, down from 7.3%.
- callgrind Ir on the same parts: 2.75% fewer instructions fresh, 0.92% fewer reused. Tree and HTML parsing are flat (−0.08%, −0.05%). Output is identical to 0.10.3 ([#80](https://github.com/Remade-With-Rust/rusty_xml/issues/80), [#85](https://github.com/Remade-With-Rust/rusty_xml/pull/85)).

## [0.10.3](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.10.2...rusty_xml-parser-v0.10.3) - 2026-11-20

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

- *(parser)* count a name's colons in the walk, without a branch ([#80](https://github.com/Remade-With-Rust/rusty_xml/pull/80))
- *(parser)* pack a namespace prefix key without memcpy ([#80](https://github.com/Remade-With-Rust/rusty_xml/pull/80))
- *(parser)* test for "/>" with two byte compares ([#80](https://github.com/Remade-With-Rust/rusty_xml/pull/80))
- *(parser)* no out-of-line drop of an empty Vec per element ([#80](https://github.com/Remade-With-Rust/rusty_xml/pull/80))
- *(parser)* name scan walks the ASCII run as a slice, colons found after ([#80](https://github.com/Remade-With-Rust/rusty_xml/pull/80))
- *(parser)* keyed namespace lookup; an element that declares nothing only enters a level ([#80](https://github.com/Remade-With-Rust/rusty_xml/pull/80))
- *(parser)* resolve each element's namespace once; no scope frame for elements that declare nothing ([#80](https://github.com/Remade-With-Rust/rusty_xml/pull/80))

## [0.10.2](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.10.1...rusty_xml-parser-v0.10.2) - 2026-11-18

### Other

- push-parse length hint ([#75](https://github.com/Remade-With-Rust/rusty_xml/pull/75)); one tree walk, made cheaper ([#76](https://github.com/Remade-With-Rust/rusty_xml/pull/76))
- correct the 0.10 install line and the 0.9.3 SAX numbers

## [0.10.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.10.0...rusty_xml-parser-v0.10.1) - 2026-11-16

### Performance

- *(parser)* SAX: the end-tag QName is a `CompactString` with a pool of reused heap buffers, and a prefixed attribute's URI is copied into the pooled slot's buffer instead of a fresh `String`: per element, `<e></e>` 1,344 -> 1,261 instructions and 3 prefixed attributes 4,590 -> 4,217; on Windows 96.8 -> 68.6 ns and 314 -> 234 ns ([#69](https://github.com/Remade-With-Rust/rusty_xml/issues/69))
- *(parser)* XML reserves its node arena at 3/2 nodes per `<` or `=` instead of one per 10 bytes (a 3.5 MB sparse document peaked at 32.8 MiB of commit, now 5.8 MiB), and the push result is shrunk like the one-shot one (a 4.8 MB document pushed held 71.2 MiB, now 50.7 MiB) ([#70](https://github.com/Remade-With-Rust/rusty_xml/issues/70))
- *(parser)* one arena policy, one duplicate-attribute check and one character-reference parse shared by the HTML and XML parsers ([#71](https://github.com/Remade-With-Rust/rusty_xml/issues/71))

### Fixed

- *(parser)* a character reference to a non-`Char` inside an entity is refused in attribute values and in no-tree content (error 9), as it already was in content and as libxml2 does ([#70](https://github.com/Remade-With-Rust/rusty_xml/issues/70))
- *(xinclude)* copied attributes keep their prefix and namespace, and copying is linear (50,000 attributes took 3.7 s)

### Changed

- `xml_parse_dtd` reads a text declaration and honours its `encoding` (it used to fail with error 65), and refuses invalid UTF-8 when nothing declares another encoding (error 81) instead of substituting U+FFFD, as libxml2 does
- XInclude `parse="text"` honours its `encoding` attribute and refuses invalid UTF-8 without one

### Added

- one settings type, interruptible and encoding-aware DTD/XInclude/reader entries (#70, #71)

### Other

- Merge branch 'b/70-entries' into release/0.10.1

## [0.10.0](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.9.5...rusty_xml-parser-v0.10.0) - 2026-11-15

### Added

- *(tree)* [**breaking**] keep short node names and text inline with compact_str ([#62](https://github.com/Remade-With-Rust/rusty_xml/pull/62))

## [0.9.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.9.4...rusty_xml-parser-v0.9.5) - 2026-11-15

### Performance

- *(html)* reserve 17/8 nodes per `<`, not 2: parse instructions -5.9% on dense fragments and -3.0% on real pages, and a page that just overflowed the reserve peaks at 8.7 MiB of commit instead of 19.1 ([#62](https://github.com/Remade-With-Rust/rusty_xml/issues/62))

### Fixed

- *(dtd)* conditional sections, PE values expanded once, first PE binds ([#58](https://github.com/Remade-With-Rust/rusty_xml/pull/58))

### Other

- Merge branch 'batch/58-dtd' into release/0.9.5

## [0.9.3](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.9.2...rusty_xml-parser-v0.9.3) - 2026-11-15

### Performance

- SAX with `XML_PARSE_NO_TREE` now streams: no element node is built, so memory stays at the input size (138 MB 3MF model part, 3M elements, measured in [#53](https://github.com/Remade-With-Rust/rusty_xml/pull/53): peak working set 434 MB -> 136 MB, time 2.04-2.10 s -> 0.57-0.60 s). Names are read in place and attribute buffers are pooled, which also makes ordinary tree parsing 15.5% cheaper in instructions on the XML benchmark. Output is unchanged (#52).

### Other

- *(parser)* stream SAX without a tree, read names in place ([#52](https://github.com/Remade-With-Rust/rusty_xml/pull/52))

## [0.9.2](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.9.1...rusty_xml-parser-v0.9.2) - 2026-11-14

### Changed

- HTML parsing now stops at libxml2's nesting cap: an element deeper than 256 (2048 under `XML_PARSE_HUGE`) is dropped, the parse halts with the tree built so far, and `XmlDoc::warnings` records `XML_ERR_RESOURCE_LIMIT`. This matches `xmllint --html` and changes output only for documents nested past 256. `html_read_memory_with` with `HtmlParseSettings::with_max_depth(-1)` restores 0.9.1's uncapped parse (#48).

### Added

- *(html)* libxml2's nesting cap, 256 by default, adjustable ([#48](https://github.com/Remade-With-Rust/rusty_xml/pull/48))

### Other

- *(html)* size the node arena from the markup, not the length ([#49](https://github.com/Remade-With-Rust/rusty_xml/pull/49))

## [0.9.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.9.0...rusty_xml-parser-v0.9.1) - 2026-11-14

### Fixed

- *(parser)* report the error a recovering parse survived; pin #44 and #45
- *(parser)* close the unbounded paths found in the #41 review
- *(html)* index the open-element stack once it is deep
- *(parser)* bound entity amplification as libxml2 does, keep push errors, add stop and cancel controls (#32, #35, #39, #40)
- *(xinclude)* walk and copy the tree without recursing ([#34](https://github.com/Remade-With-Rust/rusty_xml/pull/34))
- *(dtd)* check the entity graph for loops in linear time
- *(parser)* delete the dead read.rs and correct the depth-limit comments ([#33](https://github.com/Remade-With-Rust/rusty_xml/pull/33))

### Other

- *(html)* poll the interrupt by input position and keep deep-stack upkeep cold

## [0.9.0](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.8.5...rusty_xml-parser-v0.9.0) - 2026-11-13

### Fixed

- *(html)* decide the input encoding as libxml2 does, honouring <meta charset> ([#27](https://github.com/Remade-With-Rust/rusty_xml/pull/27))
- deduplicate attributes through a set past 32 per tag ([#21](https://github.com/Remade-With-Rust/rusty_xml/pull/21))

### Other

- *(html)* validate once and stop the encoding scan as early as it can
- *(html)* reserve the node arena from the input length and shrink it after the parse ([#22](https://github.com/Remade-With-Rust/rusty_xml/pull/22))
- *(tree)* [**breaking**] shrink Node from 184 to 88 bytes ([#23](https://github.com/Remade-With-Rust/rusty_xml/pull/23))
- *(xml)* build the duplicate-attribute sets only past 32 attributes
- *(html)* keep the ordinary tag's attribute path at its old cost

## [0.8.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.8.4...rusty_xml-parser-v0.8.5) - 2026-11-13

### Fixed

- *(html)* doctype, unterminated comment and root rows from #16
- *(html)* keep decode_html's 16-byte skip on a char boundary

## [0.8.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-parser-v0.8.3...rusty_xml-parser-v0.8.4) - 2026-11-12

### Fixed

- *(html)* build libxml2 2.15's trees (HTMLparser.c port)
