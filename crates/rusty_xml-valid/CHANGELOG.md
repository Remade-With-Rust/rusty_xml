# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.1](https://github.com/Remade-With-Rust/rusty_xml/compare/rusty_xml-valid-v0.9.0...rusty_xml-valid-v0.9.1) - 2026-11-14

### Added

- *(valid)* interruptible RelaxNG, XSD, Schematron and C14N, with bounded stack and time (#39, #41)

### Fixed

- *(valid)* match DTD content models in linear time and report non-deterministic ones ([#36](https://github.com/Remade-With-Rust/rusty_xml/pull/36))
