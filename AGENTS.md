# Working in this repository

Read this before changing code, whether you are a person or an agent.

## One implementation per job

The HTML and XML parsers have separate grammars and shared plumbing. The
plumbing lives once; never copy it into one parser.

| job | the one place |
|---|---|
| settings for a long call (interrupt, depth cap, amplification) | `rusty_xml-tree` `settings.rs`: `ParseSettings`, `ParseSettings::depth_cap` |
| interrupt polling | `rusty_xml-tree` `interrupt.rs`: `InterruptPoll` (the HTML parser calls it on a byte cadence, the XML parser on ticks) |
| the "interrupted" error, recording a fatal error on a returned tree | `rusty_xml-parser` `error.rs`: `interrupted_error`, `record_fatal` |
| bytes to UTF-8 (documents, both parsers' declared encodings, external DTDs, XInclude text) | `rusty_xml-parser` `encoding.rs`: `decode_input`, `decode_text` |
| walking a tree in document order without recursion | `rusty_xml-tree`: `XmlDoc::walk` |

Both parsers live in `rusty_xml-parser` (`parse.rs` for XML, `html.rs` for
HTML; DTDs in `dtd.rs`, XInclude in `xinclude.rs`). Before writing a helper,
search the workspace for the same job, the other parser first. A constant
with the same meaning is defined once.

## A change on one path is checked on the others

The paths are: the HTML parser, the XML parser, push (`xml_parse_chunk`), SAX
(`xml_sax_parse_memory`, with and without `XML_PARSE_NO_TREE`), the reader,
`xml_parse_dtd`, and XInclude. A fix, limit or gate added on one of them is
checked on the rest: apply it there, or say in the pull request why it does
not apply. Most past bugs were a fix that reached one copy (#70, #71).

## Gates run on every entry point

A new gate goes in `crates/rusty_xml/tests/entry_gates.rs`, which runs each
case against every entry point it applies to. A row that skips one names the
reason.

## Evidence

- Performance claims are deterministic instruction counts (callgrind) or
  timings with the method stated; output must stay identical.
- Behaviour that libxml2 has is checked against the pinned `xmllint`
  (`oracle/PIN`).
- Every crate keeps `#![forbid(unsafe_code)]`.
