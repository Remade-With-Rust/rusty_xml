# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-xpath-v0.9.0...rusty_xml-xpath-v0.9.1) - 2026-11-14

### Changed

- `xml_xpath_cmp_nodes` now returns libxml2's `xmlXPathCmpNodes` values: 1 when the first node precedes the second, -1 when it follows, 0 for the same node, -2 for a node outside the document or nodes in different trees. 0.9.0 returned the opposite sign. Callers that sort with it, or test `== 1` for "before", should check their order (#38).

### Fixed

- *(xpath)* bound XPath work on deep and large documents (#34, #37, #38, #39, #40)
