# rusty_xml

[![crates.io](https://img.shields.io/crates/v/rusty_xml?logo=rust)](https://crates.io/crates/rusty_xml)
[![docs.rs](https://img.shields.io/docsrs/rusty_xml?logo=docsdotrs)](https://docs.rs/rusty_xml)
[![CI](https://github.com/Remade-With-Rust/rusty_xml/actions/workflows/ci.yml/badge.svg)](https://github.com/Remade-With-Rust/rusty_xml/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Remade With Rust](https://img.shields.io/badge/Remade%20With-Rust-000?logo=rust&logoColor=fff)](https://github.com/Remade-With-Rust)
[![By Mata Network](https://img.shields.io/badge/by-Mata%20Network-5b2be0)](https://www.mata.network)

> **rusty_xml** is a ground-up, pure-**Rust** remake of
> [libxml2](https://gitlab.gnome.org/GNOME/libxml2): well-formed XML 1.0 parse,
> arena DOM, SAX2, pull reader, writer/save, XPath 1.0, DTD, C14N, HTML, and
> working subsets of RelaxNG / XSD / Schematron. `#![forbid(unsafe_code)]` on
> every published crate, **no C**, no `libxml2-sys`, no copyleft. Defaults are
> **`XML_PARSE_NONET | XML_PARSE_NO_XXE`** — the safe posture libxml2's own
> README says the C library does not have.

Part of **[Remade With Rust](https://github.com/Remade-With-Rust)** by
**[Mata Network](https://www.mata.network/)** — the XML toolkit for the stack
that already ships
**[rusty_zstd](https://crates.io/crates/rusty_zstd)**,
**[rusty_h264](https://crates.io/crates/rusty_h264)**, and
**[remade_ffmpeg_rs](https://github.com/Remade-With-Rust/remade_ffmpeg_rs)**.
[Jump to the ecosystem ↓](#the-remade-with-rust-ecosystem)

---

## The headline

A pure-**safe-Rust** XML 1.0 toolkit that is a **reimplementation**, not a
wrapper, with libxml2 function names as `#[doc(alias)]` and **safe defaults**
C historically got wrong:

- **Parse:** UTF-8 well-formed documents, 15 built-in 8-bit encodings (no
  iconv), push (`xmlParseChunk`), IO callbacks, local OASIS catalogs. SAX
  traces are gated line-for-line against pinned `xmllint --sax`.
- **Tree / save / writer / reader:** arena DOM, `xmlsave` (including
  `XML_SAVE_NO_EMPTY` / `NO_DECL`), `xmlTextWriter`, `xmlTextReader`. Empty
  elements are one reader `Element`, no extra `EndElement`.
  `parse(write(parse(x)))` is a standing gate.
- **XPath 1.0** compile + eval; `rxmlint --xpath` prints xmllint form.
- **Validate / canonicalize:** DTD internal subset + default attributes,
  `xmlValidateDocument`, C14N 1.0 and exclusive, XInclude via a caller loader.
- **HTML** (`HTMLparser.c` grammar, implied html/head/body) and working
  subsets of RelaxNG, XML Schema, and Schematron.
- **CLI is `rxmlint`**, never `xmllint`. Same flag language so a bench script
  can swap argv[0].
- **The C oracle is an external process.** We never link libxml2. Pin:
  libxml2 **v2.15.3** (`oracle/PIN`).

| | libxml2 (C) | **rusty_xml (Rust)** |
|---|---|---|
| C/C++ in the dependency tree | all of it | **none** — no `libxml2-sys`, no iconv, no zlib-sys |
| `unsafe` in the published crates | extensive | **0** — `#![forbid(unsafe_code)]` |
| License | MIT | **MIT OR Apache-2.0** |
| Defaults on untrusted input | libxml2 README: *not recommended* | **`NONET \| NO_XXE`**, always |
| CLI name | `xmllint` | **`rxmlint`** (does not shadow C) |
| Network entity loads | historically on | **off** unless you pass flags (and the parser still ORs `NONET`) |

### Performance — faster than libxml2

Paired board against pinned **libxml2 v2.15.3** `xmllint`. Pinned to one core
(not core 0), High priority, **CPU time**, arms **ABBA**-interleaved, **N=20**
pairs, C-vs-C null arm per row. `us/C` **< 1 means rusty_xml is faster**. Raw
rows and the full method line: [`bench/SIDE-BY-SIDE.md`](bench/SIDE-BY-SIDE.md).

| workload (parse to DOM, discard) | rusty_xml | libxml2 | us/C | wins |
|---|---:|---:|---:|---:|
| `big-attr.xml` — 627 KB, 48k attributes | **83.3 MB/s** | 37.2 MB/s | **0.45×** | 20/20, z = +4.47 |
| `big-300k.xml` — 308 KB, text-heavy | **125.6 MB/s** | 78.5 MB/s | **0.63×** | 20/20, z = +4.47 |
| `big-1m.xml` — 1.27 MB, real content | **115.7 MB/s** | 74.6 MB/s | **0.64×** | 20/20, z = +4.47 |

**1.6× to 2.2× faster than the C library, on every file, in every pair.**

<sub>**Two numbers, because one alone would mislead.** The table is
**as shipped**: `rxmlint` links [`rusty_alloc`](https://crates.io/crates/rusty_alloc)
(pure-Rust mimalloc) and `xmllint` uses the system allocator — that is what you
actually run. **Same allocator**, both on the system allocator, the parser
alone: `big-attr` **0.80×**, `big-300k` **1.07×**. Roughly half the margin is
the allocator, and C could adopt one too. Also: C runs `xmlCtxtReadFile` per
`--repeat` (the Windows pin has no mmap) while we do one `fs::read` then
`xml_read_memory` — about 1% at these sizes, but it inflates C on small files,
so the large rows are the honest result. Flags differ (C defaults to
`XML_PARSE_COMPACT \| XML_PARSE_BIG_LINES`; we force `NONET \| NO_XXE`). This
is CLI-vs-CLI, not a kernel A/B. Correctness is gated byte-identical against
the same pinned oracle throughout.</sub>

---

### Conformance — where we actually stand

Speed is the easy half. Here is the hard half, measured against the **W3C XML
Conformance Test Suite** rather than against our own corpus, with the pinned C
build scored on the identical cases:

| | rusty_xml 0.5.0 | libxml2 2.15.3 |
|---|---|---|
| **Total** (2039 scored cases) | **97.8%** | 94.8% |
| well-formedness (`not-wf`, 1263) | **98.6%** | 98.0% |
| invalid documents rejected (175) | **89.7%** | 53.7% |
| valid documents accepted (601) | 98.7% | 100.0% |

Ahead of the thing we are replacing on every category but one: C still accepts
every valid document and we refuse eight of six hundred.

**A correction.** 0.4.0 published "79.9%, level with libxml2" and that number
was measured wrong. The suite marks 313 cases `EDITION="1 2 3 4"` -- they test
the name rules of XML 1.0 *before* the 5th edition, which is not the language
either implementation parses by default. libxml2's own `runxmlconf.c` reads
that attribute and parses those cases with `XML_PARSE_OLD10`; our runner
ignored it and scored a 5th-edition parser against 4th-edition expectations,
counting 313 non-failures as failures for both sides. Correcting the runner
moved the real figures to 90.0% and 94.8% -- we were behind, not level -- and
exposed that `XML_PARSE_OLD10` never reached the DTD parser at all.

0.2.0 never ran the suite. The first run, in 0.3.0, crashed before scoring a
single case -- a 32 GB allocation reachable from thirty-two bytes of DTD -- and
then scored 59.1%. Everything since came from reading failures rather than
guessing: an internal subset parser that was a scanner rather than a parser,
four literals never character-validated, a validator whose ID branch said
"uniqueness checked loosely" and meant not at all, and an entity whose
replacement text was inserted as escaped text instead of the markup it was.

Run it yourself — the suite is fetched, never vendored:

```sh
pwsh scripts/fetch-xmlconf.ps1
cargo run --release -p rusty_xml-bench --bin xmlconf -- --oracle
```

What we *do* gate hard, and what those numbers cost nothing:

- **62 of 64** byte-identical comparisons against pinned `xmllint` over 16
  corpora x {plain, `--format`, `--c14n`, `--exc-c14n`}. The two exceptions are
  one deliberate divergence: we serialize the internal DTD subset verbatim
  where C re-serializes and reorders it.
- **0 `unsafe`** in all twelve crates.
- Nothing on the parse, save, format, XPath, stream or canonicalize path
  recurses per level of nesting, so document depth cannot exhaust the stack.
- A seeded fuzzer (`--bin fuzz`) holds four invariants: no panics, chunked
  parsing equals whole parsing at every chunk size, and both XML and HTML round
  trips are fixed points.

If you need full libxml2 conformance today, use libxml2. If you need a
memory-safe XML toolkit that is faster than C, has no C in its dependency tree,
and tells you exactly which cases it gets wrong, this is it.

---

## What is this?

`rusty_xml` is libxml2 remade in Rust. Unlike
[`libxml2-sys`](https://crates.io/crates/libxml2-sys) / `quick-xml` /
`roxmltree` — bindings or different grammars — there is **no C in the
dependency tree** here and the public names match C (`xmlReadMemory` →
`xml_read_memory`, documented with `#[doc(alias)]`).

libxml2's own README says it is **not recommended for untrusted data**. That
is the defect this remake exists to close: semantic identity under matched
options, **safe defaults** (no network, no XXE, bounded amplification).

It is a reimplementation of the algorithms, not a fork. The C sources are
neither distributed nor linked; a pinned `xmllint` is used only as an
external-process oracle (`scripts/fetch-oracle.ps1`).

`cargo-deny` enforces the promise: no `*-sys` crate, no copyleft, and no
`libxml2-sys` anywhere in the graph.

## The Remade With Rust ecosystem

<!-- ORG BOILERPLATE — keep identical across repos -->

**Remade With Rust** is an initiative by **[Mata Network](https://www.mata.network/)**
to rebuild essential C and C++ tools in Rust — for the memory safety, the
predictable performance, and the freedom of a permissive license. Each project
is a reimplementation, not a fork: same wire protocols and file formats, new
code you can actually depend on.

We build the core to production grade and open-source it so the community can
extend it. No copyleft. No surprises. Just the tools we rely on, made faster and
safer.

| Project | What it is |
|---|---|
| 🎬 **[remade_ffmpeg_rs](https://github.com/Remade-With-Rust/remade_ffmpeg_rs)** | **Our FFmpeg alternative.** Drop-in `ffmpeg` and `ffprobe` binaries — demux → decode → filter → encode → mux, rebuilt as composable Rust crates with **zero GPL/LGPL**. Apache-2.0. |
| 🧠 **[FFAI](https://github.com/Remade-With-Rust/FFAI)** | **Our sister project: media *for* AI.** "The AI media toolkit, remade with rust." Embedded ASR + TTS (**Mercury**), OCR (**Carmenta**) and vision-language captioning (**Argus**) behind an ffmpeg-style, swap-by-name architecture — no Python, no CUDA. MIT OR Apache-2.0. |
| 🌐 **[Mata Network](https://www.mata.network/)** | **The home page.** *"Stop sacrificing your privacy for convenience."* Sovereign, self-hostable privacy infrastructure — wallet & identity, password manager, contact manager, and a browser extension that stops information leaking as you browse. Remade With Rust is its open-source arm. |

→ All projects: **[github.com/Remade-With-Rust](https://github.com/Remade-With-Rust)**

<!-- /ORG BOILERPLATE -->

## Install

One crate — `rusty_xml` — is the public facade; it re-exports parser, tree,
SAX, reader, writer, XPath, and validation. Add it with:

```sh
cargo add rusty_xml
```

or in `Cargo.toml`:

```toml
[dependencies]
rusty_xml = "0.5"
```

MSRV is **1.85**. The library never sets `#[global_allocator]`.

The published crates (all `0.1`, MIT OR Apache-2.0):

| Crate | Role | Docs |
|---|---|---|
| [`rusty_xml`](https://crates.io/crates/rusty_xml) | **the facade — depend on this** | [docs.rs](https://docs.rs/rusty_xml) |
| [`rusty_xml-parser`](https://crates.io/crates/rusty_xml-parser) | well-formed parse, encodings, push, catalogs, HTML | [docs.rs](https://docs.rs/rusty_xml-parser) |
| [`rusty_xml-tree`](https://crates.io/crates/rusty_xml-tree) | arena DOM | [docs.rs](https://docs.rs/rusty_xml-tree) |
| [`rusty_xml-sax`](https://crates.io/crates/rusty_xml-sax) | SAX2 recorder + xmllint-debug dump | [docs.rs](https://docs.rs/rusty_xml-sax) |
| [`rusty_xml-reader`](https://crates.io/crates/rusty_xml-reader) | `xmlTextReader` | [docs.rs](https://docs.rs/rusty_xml-reader) |
| [`rusty_xml-writer`](https://crates.io/crates/rusty_xml-writer) | `xmlsave` + `xmlTextWriter` | [docs.rs](https://docs.rs/rusty_xml-writer) |
| [`rusty_xml-xpath`](https://crates.io/crates/rusty_xml-xpath) | XPath 1.0 | [docs.rs](https://docs.rs/rusty_xml-xpath) |
| [`rusty_xml-valid`](https://crates.io/crates/rusty_xml-valid) | DTD, C14N, RelaxNG, XSD, Schematron | [docs.rs](https://docs.rs/rusty_xml-valid) |
| [`rusty_xml-cli`](https://crates.io/crates/rusty_xml-cli) | `rxmlint` binary | — |
| [`rusty_xml-alloc`](https://crates.io/crates/rusty_xml-alloc) | allocator seam **for binaries only** — the library never uses it | — |

Not published: `rusty_xml-bench` (oracle harness), `rusty_xml-c-abi` (M8 stub).

**The library picks no allocator.** `rusty_xml-alloc` pins
[`rusty_alloc`](https://crates.io/crates/rusty_alloc) for `rxmlint`; the
published library declares no `#[global_allocator]` and does not depend on it,
so an embedding application keeps that choice — and gets the same win for free
if it already ships `rusty_alloc`.

**Dropping it into a downstream tool:** depend on the facade. Call
`xml_read_memory` / `xml_reader_for_memory` / `xml_xpath_eval`. Do not add
`libxml2-sys`. Safe defaults are already on; you do not opt into `NONET`.

## Quick start

```rust
use rusty_xml::{default_parse_options, xml_read_memory, xml_save_doc};

fn main() -> Result<(), rusty_xml::XmlError> {
    let xml = br#"<root><item id="1">hi</item></root>"#;
    let doc = xml_read_memory(xml, None, None, default_parse_options())?;
    let bytes = xml_save_doc(&doc, 0);
    assert!(std::str::from_utf8(&bytes).unwrap().contains("<item"));
    Ok(())
}
```

XPath 1.0 on the same tree:

```rust
use rusty_xml::{
    default_parse_options, xml_read_memory, xml_xpath_eval, XmlXPathContext, XPathObject,
};

fn main() -> Result<(), rusty_xml::XmlError> {
    let doc = xml_read_memory(
        br#"<root><item>a</item><item>b</item></root>"#,
        None,
        None,
        default_parse_options(),
    )?;
    let ctx = XmlXPathContext::xml_xpath_new_context(&doc);
    match xml_xpath_eval("count(//item)", &ctx).unwrap() {
        XPathObject::Number(n) => assert_eq!(n, 2.0),
        other => panic!("expected number, got {other:?}"),
    }
    Ok(())
}
```

Pull reader:

```rust
use rusty_xml::{default_parse_options, xml_reader_for_memory};

fn main() -> Result<(), rusty_xml::XmlError> {
    let mut r = xml_reader_for_memory(
        br#"<a><b/></a>"#,
        None,
        None,
        default_parse_options(),
    )?;
    let mut ticks = 0u32;
    while r.read() == 1 {
        ticks += 1;
    }
    assert!(ticks >= 2);
    Ok(())
}
```

Command-line (never installs as `xmllint`):

```sh
cargo install rusty_xml-cli
rxmlint --noout file.xml
rxmlint --sax --noout file.xml
rxmlint --stream --noout file.xml
rxmlint --xpath "//item" file.xml
rxmlint --c14n file.xml
```

## Architecture

The workspace mirrors libxml2's headers, not its build:

```text
crates/
  rusty_xml           public facade  ← depend on this
  rusty_xml-parser    parser.h — well-formed, encodings, push, catalogs, HTML
  rusty_xml-tree      tree.h — arena DOM
  rusty_xml-sax       SAX2.h — recorder + xmllint-debug dump
  rusty_xml-reader    xmlreader.h
  rusty_xml-writer    xmlsave.h + xmlwriter.h
  rusty_xml-xpath     xpath.h
  rusty_xml-valid     valid.h, c14n, RelaxNG / XSD / Schematron subsets
  rusty_xml-cli       rxmlint (not xmllint)
  rusty_xml-c-abi     optional cdylib, stub until M8. Not published.
  rusty_xml-bench     shells out to pinned xmllint. Never links libxml2.
  rusty_xml-alloc     rusty_alloc seam for binaries only. Library never uses it.
bench/                pinned oracle-vs-us timing harness (pinvs.ps1)
oracle/PIN            libxml2 v2.15.3 pin (binary is gitignored)
```

## Platform support

| Platform | Status |
|---|---|
| Windows (x86-64) | ✅ builds + tests |
| Linux | ✅ builds + tests |
| macOS | ✅ builds + tests |
| `wasm32-unknown-unknown` | ✅ library `cargo check` in CI |

No C toolchain, no iconv, no nasm. Gzip (`XML_PARSE_UNZIP`) is not wired yet
(a `1f 8b` buffer is an error). ISO-2022-JP / Shift_JIS / EUC-JP are
unsupported, matching libxml2 built **without** iconv.

## Roadmap

- [x] **M0** — pin oracle (libxml2 v2.15.3), C-only board, workspace skeleton
- [x] **M1** — character classes, UTF-8 well-formed parse, SAX-exact vs `xmllint --sax`
- [x] **M2** — tree mutation, `xmlsave`, `xmlTextWriter`, `xmlTextReader`, round-trip
- [x] **M3** — encodings without iconv, push parser, IO callbacks, local catalogs
- [x] **M4** — XPath 1.0 compile + eval, `rxmlint --xpath`
- [x] **M5** — DTD validation, C14N 1.0 + exclusive, XInclude (loader-gated)
- [x] **M6** — HTML parser, RelaxNG / XSD / Schematron working subsets
- [x] **M7** — performance campaign vs pinned `xmllint`: faster than C on every
      corpus file, N=20, 20/20 pairs ([`bench/SIDE-BY-SIDE.md`](bench/SIDE-BY-SIDE.md))
- [ ] Optional gzip (`miniz_oxide` / `XML_PARSE_UNZIP`)
- [x] **M8** — W3C conformance suite wired up and scored against the C oracle
      (65.0% vs libxml2 79.9%); seeded fuzzer; corpus widened 7 -> 16 files
- [x] **M9** — close the conformance gap: 65.0% -> 97.8% on the W3C suite,
      ahead of libxml2's 94.8% on the same 2039 cases
- [ ] C ABI `cdylib` (`XMLPUBFUN` names) + hardening audit
- [ ] Optional gzip (`miniz_oxide` / `XML_PARSE_UNZIP`)
- [ ] XML 1.1, external entity loading, CJK multi-byte encodings

Plan: [`docs/plan/rusty_xml.md`](docs/plan/rusty_xml.md).

## License

**MIT OR Apache-2.0**, at your option — see [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE). No GPL/LGPL and no C anywhere in the
dependency tree, CI-enforced with `cargo-deny`. The C `xmllint` binary used
as a measurement oracle is neither distributed here nor linked; see
[NOTICE.md](NOTICE.md).

## About Mata Network

<!-- ORG BOILERPLATE — keep identical across repos -->

**[Mata Network](https://www.mata.network/)** builds sovereign, self-hostable
privacy infrastructure — *"stop sacrificing your privacy for convenience"*:
wallet & identity, a password manager, a contact manager, and a browser
extension that stops your information leaking as you browse.

**Remade With Rust** is our open-source home for the permissively-licensed
building blocks that work depends on — including
[remade_ffmpeg_rs](https://github.com/Remade-With-Rust/remade_ffmpeg_rs) (the
FFmpeg alternative) and [FFAI](https://github.com/Remade-With-Rust/FFAI) (the
AI media toolkit).

→ **[www.mata.network](https://www.mata.network/)**

<!-- /ORG BOILERPLATE -->
