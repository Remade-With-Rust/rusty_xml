# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
