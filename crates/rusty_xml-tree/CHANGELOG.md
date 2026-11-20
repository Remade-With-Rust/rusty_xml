# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-tree-v0.10.3...rusty_xml-tree-v0.10.4) - 2026-11-20

### Other

- *(parser)* per-parse setup without allocations ([#80](https://github.com/Remade-With-Rust/rusty_xml/pull/80))

## [0.10.2](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-tree-v0.10.1...rusty_xml-tree-v0.10.2) - 2026-11-18

### Other

- push-parse length hint ([#75](https://github.com/Remade-With-Rust/rusty_xml/pull/75)); one tree walk, made cheaper ([#76](https://github.com/Remade-With-Rust/rusty_xml/pull/76))

## [0.10.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-tree-v0.10.0...rusty_xml-tree-v0.10.1) - 2026-11-16

### Added

- one settings type, interruptible and encoding-aware DTD/XInclude/reader entries (#70, #71)

### Other

- Merge branch 'b/70-entries' into release/0.10.1

## [0.10.0](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-tree-v0.9.5...rusty_xml-tree-v0.10.0) - 2026-11-15

### Changed

- `Node::name` and `Node::content` are now `CompactString` (re-exported as `rusty_xml::CompactString`) instead of `String`, and `XmlDoc::alloc` takes `impl Into<CompactString>`. Reading them is unchanged: they deref to `&str` and compare with `str` and `String`, and `XmlDoc::name()` / `content()` still return `&str`. To migrate, assign with `.into()` (`node.content = s.into()`) and call `.to_string()` where an owned `String` is needed. Short names and text are stored inline: on 197 real pages, parsing is about 20% faster, dropping a tree about 45% faster and the trees hold 18% less memory (#62).

### Added

- *(tree)* [**breaking**] keep short node names and text inline with compact_str ([#62](https://github.com/Remade-With-Rust/rusty_xml/pull/62))

## [0.9.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-tree-v0.9.0...rusty_xml-tree-v0.9.1) - 2026-11-14

### Added

- *(tree)* add the Interrupt trait for cooperative cancellation ([#39](https://github.com/Remade-With-Rust/rusty_xml/pull/39))

### Fixed

- *(parser)* report the error a recovering parse survived; pin #44 and #45
- *(tree)* read node content without recursing per level ([#41](https://github.com/Remade-With-Rust/rusty_xml/pull/41))

## [0.9.0](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-tree-v0.8.5...rusty_xml-tree-v0.9.0) - 2026-11-13

### Other

- *(tree)* [**breaking**] shrink Node from 184 to 88 bytes ([#23](https://github.com/Remade-With-Rust/rusty_xml/pull/23))

## [0.8.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-tree-v0.8.4...rusty_xml-tree-v0.8.5) - 2026-11-13

### Fixed

- *(html)* doctype, unterminated comment and root rows from #16

## [0.8.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-tree-v0.8.3...rusty_xml-tree-v0.8.4) - 2026-11-12

### Fixed

- *(html)* build libxml2 2.15's trees (HTMLparser.c port)
