# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.5](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-valid-v0.9.4...rusty_xml-valid-v0.9.5) - 2026-11-15

### Performance

- *(valid)* index attribute declarations per validation, share identical follow sets, and stop Schematron inside one evaluation when interrupted: TEI-sized DTDs validate 100x faster, 64 large content models use 20 MiB instead of 2 GiB ([#58](https://github.com/Remade-With-Rust/rusty_xml/issues/58))

### Fixed

- *(valid)* RelaxNG sequences keep order, choices reconsider, siblings linear (#57, #58)

### Other

- Merge branch 'batch/57-rng' into release/0.9.5

## [0.9.4](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-valid-v0.9.3...rusty_xml-valid-v0.9.4) - 2026-11-15

### Fixed

- *(valid)* memoise RelaxNG matching once it backtracks ([#56](https://github.com/Remade-With-Rust/rusty_xml/pull/56))

## [0.9.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-valid-v0.9.0...rusty_xml-valid-v0.9.1) - 2026-11-14

### Added

- *(valid)* interruptible RelaxNG, XSD, Schematron and C14N, with bounded stack and time (#39, #41)

### Fixed

- *(valid)* match DTD content models in linear time and report non-deterministic ones ([#36](https://github.com/Remade-With-Rust/rusty_xml/pull/36))
