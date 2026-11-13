# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
