# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-writer-v0.10.4...rusty_xml-writer-v0.10.5) - 2026-11-20

### Fixed

- ship LICENSE-MIT, LICENSE-APACHE and NOTICE.md in every published crate

## [0.10.2](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-writer-v0.10.1...rusty_xml-writer-v0.10.2) - 2026-11-18

### Other

- push-parse length hint ([#75](https://github.com/Remade-With-Rust/rusty_xml/pull/75)); one tree walk, made cheaper ([#76](https://github.com/Remade-With-Rust/rusty_xml/pull/76))

## [0.9.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-writer-v0.9.0...rusty_xml-writer-v0.9.1) - 2026-11-14

### Added

- *(writer)* add xml_save_doc_interruptible ([#39](https://github.com/Remade-With-Rust/rusty_xml/pull/39))

### Other

- *(writer)* poll through an Option so the save loop is compiled once

## [0.9.0](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-writer-v0.8.5...rusty_xml-writer-v0.9.0) - 2026-11-13

### Fixed

- *(html)* write CR, TAB and LF raw in HTML output, as libxml2 does

## [0.8.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-writer-v0.8.4...rusty_xml-writer-v0.8.5) - 2026-11-13

### Fixed

- *(html)* doctype, unterminated comment and root rows from #16

## [0.8.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-writer-v0.8.3...rusty_xml-writer-v0.8.4) - 2026-11-12

### Fixed

- *(html)* build libxml2 2.15's trees (HTMLparser.c port)
