# Changelog

All notable changes to this project are documented here. The format is loosely
based on [Keep a Changelog](https://keepachangelog.com/); this project uses
[Semantic Versioning](https://semver.org/).

## [0.3.0]

A correctness release. 0.2.0 was fast; this one measures how right it is, for
the first time, against something other than our own corpus.

The headline is a number we did not have before: **65.0% of the W3C XML
Conformance Test Suite**, against pinned libxml2's **79.9%** on the same 2039
scored cases. That gap is the honest statement of where this parser stands.
It is not a drop-in replacement for libxml2 yet, and the suite says so.

### Security

- **A 32 GB allocation from 32 bytes of DTD.** `<!ELEMENT doc (a & b)?>` uses
  SGML's "and" connector. `&` is not a name character, so the content-model
  reader's `take_name` returned `""` and the position never advanced -- the
  loop appended an empty token forever until the process died. Any document
  carrying a DTD could do this to anything that validates. Found by the
  conformance suite on its first run, before it scored a single case.
- **Three process aborts on deep documents.** Saving a 2000-deep tree
  overflowed the stack, *inside* our own `MAX_DEPTH` of 5000, and
  canonicalization overflowed on both the inclusive and exclusive paths. The
  writer and C14N are both driven by explicit stacks now; nothing on the parse,
  save, format, XPath, stream or canonicalize path recurses per level.

### Fixed

- **The internal DTD subset parser was a lenient scanner.** It consumed
  everything up to the next `>` and shrugged: `<!ENTITY foo"text">` with no
  space, `PUBLIC` with one literal instead of two, `(foo,bar)` as an
  enumeration, an unquoted default that silently became an empty string, `NAME`
  as an attribute type, SGML `-- comments --` inside a declaration. All
  accepted. Declaration syntax is checked now, which is most of the +9.5 points
  on not-well-formed cases.
- **Any error in the internal subset was discarded.** It was parsed with
  `unwrap_or_default()`, so a malformed DTD became an empty one: every entity
  and ATTLIST default it declared vanished, and the failure surfaced later as a
  bogus "entity not defined" blaming the reference instead of the declaration.
- **Four literals were never character-validated.** Character data was checked;
  attribute values, CDATA, entity values and ATTLIST defaults were not. A C0
  control byte in any of them was accepted and the writer quietly substituted
  U+FFFD on the way out, so a document came back holding a character it never
  contained -- and an ATTLIST default propagated that into every element that
  took the default.
- **The DOCTYPE was never serialized.** A read-modify-write silently dropped
  every entity, ATTLIST default and notation the document declared.
- **A character reference split the text run it was in.** `&#65; &#66;` became
  three text nodes, the middle whitespace-only, so `XML_PARSE_NOBLANKS` deleted
  it and `A B` came back as `AB`. Silent text loss.
- **The HTML parser never nested anything.** Every generic element was given
  the body as its parent, so fifty nested `<div>` came out as fifty empty
  siblings -- tree depth 4 where C measured 53. A block element also now closes
  an open `<p>`, as it does in C.
- **HTML had no serializer.** HTML documents went out through the XML writer:
  an XML declaration where C writes the doctype, `<br/>` where C writes `<br>`,
  and `&#xFFFD;` for control characters C passes through. Re-parsed, a trailing
  slash is not an end tag, so content after a `<br/>` became its *child* -- the
  tree moved on every save-and-reparse cycle.
- **`--format` echoed the source indentation back** instead of reformatting,
  because it did not imply `noblanks` as xmllint's does. 1699 lines differed
  from C on the 300 KB corpus. Indentation also now caps at 60 columns, as C's
  does; ours grew without limit and diverged at level 31.
- Streaming defects at chunk boundaries: multi-byte characters and CRLF split
  across chunks, a BOM shifting every offset, and non-UTF-8 documents handed
  raw to a UTF-8 parser.
- XHTML empty elements serialize as `<br />`, with the space.

### Added

- **A conformance runner** (`cargo run -p rusty_xml-bench --bin xmlconf`). The
  suite is fetched by `scripts/fetch-xmlconf.ps1`, never vendored. `--oracle`
  runs every case through the pinned C build too, because a pass rate on its
  own means nothing when libxml2 does not score 100% either.
- **A fuzzer** (`--bin fuzz`), seeded and clock-free so a failure replays from
  its seed alone. Four invariants: nothing panics, chunked parsing equals whole
  parsing at every chunk size, and both XML and HTML round trips are fixed
  points. It found the text-splitting bug, the HTML round-trip instability, and
  a char-boundary panic in code that had not yet been committed.
- The corpus went from 7 files to 16: SOAP, RSS, XHTML, an internal subset that
  declares things, namespace rebinding and undeclaring, Unicode across the
  planes, mixed content, deep nesting, attribute-heavy SVG. Four defects turned
  up on first contact with it.
- 2125 HTML5 named character references, and twelve more single-byte encodings
  (windows-1250/1251/1253-1258, KOI8-R, KOI8-U, IBM866, macintosh).

### Changed

- **Documents that used to parse may now be rejected.** That is the point of
  this release: the malformed-declaration forms listed above were all accepted
  before. `XML_PARSE_RECOVER` still tolerates them.
- `xml_parse_chunk` is genuinely incremental. Peak buffered bytes on a 308 KB
  document fell from 308,576 to 12.
- The internal DTD subset is serialized **verbatim**. libxml2 re-serializes it
  from its parsed form, reordering declarations and respacing content models.
  Preserving the document's own bytes round-trips better, and it is the one
  deliberate byte-identity divergence from C.

### Known gaps

- 65.0% conformance against libxml2's 79.9%. The remaining not-well-formed
  failures are concentrated in name-character classification (productions
  84-89) and content-model validation.
- No XML 1.1, no external entity loading, no gzip, no CJK multi-byte encodings
  (Shift_JIS, EUC-JP, GBK, Big5).
- HTML output does not re-escape non-ASCII as named entities (`&nbsp;`,
  `&copy;`) the way C does.

## [0.2.0]

A performance release. Parsing a document is **65x faster** than 0.1.0 on real
content, and `rxmlint` now parses faster than pinned libxml2 `xmllint` on the
same files. Every step was gated byte-identical against the C oracle.

### Fixed

- **Parsing was quadratic in document length.** `peek_char` ran
  `std::str::from_utf8` over the *whole remaining buffer* to decode one
  character, so a parse re-validated the tail once per character. A 302 KB
  document validated 58.9 GB of input. A UTF-8 scalar is at most four bytes, so
  only those are checked now. This alone was ~55x on a 308 KB file.
- **DTD-defaulted attributes serialised in a different order on every run** of
  the same binary, because the declaration map is a `HashMap` with a randomly
  seeded hasher. This crate ships C14N for XML-DSig, where a signature over a
  non-reproducible serialisation is worthless. The defaults are now sorted.
- **The tree parse was recording a SAX event log and discarding it.**
  `xml_read_memory` ran a `SaxRecorder`, deep-copying the name, prefix, URI,
  namespace list and every attribute of every element into a `Vec` that was then
  dropped. It was over half of all allocations on an attribute-heavy document.

### Changed

- Allocations per parse fell **72%** on a 627 KB attribute-heavy document
  (450,079 -> 126,039) and **64%** on a 308 KB text document, with allocated
  bytes down 76% and 79%. Attribute nodes take owned strings; QNames are split
  once and moved rather than copied; character data and attribute values are
  taken in runs; the end tag is compared in place; nodes whose name is implied
  by their kind store no `String`; the node arena is sized from measured
  density.
- Starting a parse session costs **5 allocations instead of 8** (`<a/>`), and a
  63-byte document 10 instead of 15 with half the bytes. `sniff_encoding_decl`
  had been building a 1 KB `String` *and* a lowercased copy of it on every
  single parse.
- `peek_char` is no longer called at all on an all-ASCII document; every caller
  reaches a byte lane. The multi-byte decoder is unchanged and still gated.
- **`rxmlint` and the bench crate now ship `rusty_alloc`** through the
  `rusty_xml-alloc` seam. The library still declares no `#[global_allocator]`
  and does not depend on it -- an embedding application keeps that choice.

### Added

- `XmlDoc::add_attr_owned`, `XmlDoc::alloc_unnamed`,
  `XmlDoc::with_node_capacity`, `XmlDoc::reserve_nodes`.
- `NullSax`, a handler that discards every callback, for tree-only parses.
- `corpora/big-300k.xml` and `corpora/big-attr.xml` so the published board is
  reproducible. The shipped corpus was too small to measure: every row of the
  0.1 board was flagged `DURATION_SHORT` and dominated by process startup.

### Breaking

- A node whose name is implied by its kind (`#text`, `#comment`,
  `#cdata-section`, `#document`) now stores an empty `String`.
  `XmlDoc::name()` is unchanged and still returns the canonical name; code
  reading the `pub name` field **directly** will see `""`.

### Notes

- Entity handling still diverges from C and is the next correctness work: we
  substitute general entities where libxml2 keeps reference nodes without
  `XML_PARSE_NOENT`, and a nested reference is emitted as escaped literal text.
- libxml2 splits `characters` callbacks at multi-byte boundaries; we coalesce.

## [0.1.0]

First public release on [crates.io](https://crates.io/crates/rusty_xml).

### Added

- **Parser:** UTF-8 well-formed XML 1.0, 15 built-in 8-bit encodings (no iconv),
  push (`xml_parse_chunk`), IO callbacks, local OASIS catalogs. SAX traces gated
  line-for-line against pinned `xmllint --sax` (libxml2 v2.15.3).
- **Tree / save / writer / reader:** arena DOM, `xmlsave`, `xmlTextWriter`,
  `xmlTextReader`. `parse(write(parse(x)))` standing gate.
- **XPath 1.0** compile + eval; `rxmlint --xpath` prints xmllint form.
- **DTD** internal subset + default attributes, `xml_validate_document`.
- **C14N 1.0** and exclusive canonicalization.
- **HTML** parser (implied html/head/body) and working subsets of RelaxNG, XML
  Schema, and Schematron.
- **`rxmlint`** CLI (never `xmllint`) with `--noout`, `--sax`, `--stream`,
  `--xpath`, `--c14n`, `--html`, `--push`, `--dtdvalid`, `--relaxng`,
  `--schema`, `--schematron`, `--repeat`.
- Safe defaults: `XML_PARSE_NONET | XML_PARSE_NO_XXE` always.

### Notes

- Every published crate is `#![forbid(unsafe_code)]`. No `libxml2-sys`.
- Gzip (`XML_PARSE_UNZIP`) is not wired. Japanese encodings that need iconv in
  C are unsupported here too.
- C ABI (`rusty_xml-c-abi`) is a stub until M8.
- Speed vs pinned `xmllint` is the M7 campaign — the 0.1 session board is in
  `bench/SIDE-BY-SIDE.md` and is not a publish claim.
