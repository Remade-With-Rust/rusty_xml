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
| sizing the node arena (one-shot, push, push told its length, HTML) and shrinking it at the end | `rusty_xml-parser` `arena.rs` |
| walking a tree in document order without recursion | `rusty_xml-tree`: `XmlDoc::walk` |

Exceptions, each with a comment at the loop saying why: C14N (`c14n.rs`, a
namespace scope per element) and the writer (`rusty_xml-writer`, indent and
newline between children) keep their own stack, and XPath's document-order
walk (`walk_doc_order`, attributes between a node and its children) keeps its
own loop, measured 17% cheaper than `walk` on the path every XPath sort,
union and `id()` takes.

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

## When sources disagree

Code, comments, docs and this file must say the same thing. When they do not,
do not pick one silently: fix the wrong one in the same change, or say in the
pull request which is right and why.

## Before you finish

- Search for what the change makes false: comments, docs and this file that
  state the old rule, number or name (grep for it), and the other paths listed
  above.
- Measure what it could make worse, not only what it targets: peak and held
  memory, and parse and read time, on the same inputs before and after.
  Report every row that got worse.
- Run `cargo test --workspace --exclude rusty_xml-bench`, which includes
  `entry_gates.rs`.
- A shared helper added or moved gets its row in the table above in the same
  change. A deliberate second copy gets a one-line comment naming why, and a
  line under Exceptions.
- Public API changes are additive unless the issue accepts a break.
- A rule or layout fact stated in this file changes: update it in the same
  change. This file stays short; detail goes in `docs/` or the README.

## Where things are

The crate map is in the README, "Architecture". Besides it: tests of public
behaviour are in `crates/rusty_xml/tests` (`gates.rs` for conformance gates,
`limits_*.rs` for limits, `entry_gates.rs` for the table over every entry
point); `docs/plan/` holds the plan and the libxml2 API census; `oracle/PIN`
pins xmllint; `corpora/` holds the inputs. Adding, moving or removing a crate
updates the README "Architecture" in the same change, in both copies
(`README.md` and `crates/rusty_xml/README.md`; CI diffs them).
