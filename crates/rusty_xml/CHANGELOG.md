# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-v0.9.0...rusty_xml-v0.9.1) - 2026-11-14

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
