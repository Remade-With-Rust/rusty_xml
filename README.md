### In The Wild with 6,024 Active Installs
> [RAG Converter](https://ragconverter.com) uses `rusty_xml` to read Word, PowerPoint and Excel, without LibreOffice.
> It makes personal and work files AI-readable without them leaving the machine:
> the whole conversion runs as WebAssembly in the browser tab, with nothing
> uploaded and nothing to install.

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

| | libxml2 (C) | **rusty_xml (Rust)** |
|---|---|---|
| C/C++ in the dependency tree | all of it | **none** — no `libxml2-sys`, no iconv, no zlib-sys |
| `unsafe` in the published crates | extensive | **0** — `#![forbid(unsafe_code)]` (one pinned dependency, [`compact_str`](https://crates.io/crates/compact_str), has audited internal `unsafe`) |
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
That board was measured on an earlier release. 0.9.3 parses the same kind of
documents with 15.5% fewer instructions (callgrind, 8 real files, identical
output), so it understates the gap today.

**Streaming SAX** (`xml_sax_parse_memory` with `XML_PARSE_NO_TREE`) builds no
tree at all, so memory stays at the input size. A generated 3MF model part of
138 MB with 3M elements, where every element is visited and every vertex's `x`
attribute is parsed. Windows, release, four alternating rounds:

| parser | time | peak working set |
|---|---:|---:|
| quick-xml 0.41 (`read_event_into`) | 0.32–0.33 s | 135 MB |
| rusty_xml 0.9.2 | 2.04–2.10 s | 434 MB |
| **rusty_xml 0.9.3** | **0.57–0.60 s** | **136 MB** |

rusty_xml still checks what quick-xml leaves to the caller: name characters,
namespace bindings, duplicate attributes and attribute-value normalization.

**Many small documents** (the XML parts of a DOCX, XLSX or PPTX package):
`xml_sax_parse_memory` and `xml_read_memory` start each parse from the buffers
the thread's last parse grew, up to 64 KiB, so a SAX parse of a PPTX part makes
3 or 4 allocations. One `XmlSaxParser`, calling `parse_memory` per part, keeps
every buffer, with no limit, for as long as you keep the parser. Events and
results are the same either way (#80).

---

### Conformance — where we actually stand

Speed is the easy half. Here is the hard half, measured against the **W3C XML
Conformance Test Suite** rather than against our own corpus, with the pinned C
build scored on the identical cases:

| | rusty_xml 0.8.0 | libxml2 2.15.3 |
|---|---|---|
| **Total** (2036 scored cases) | **99.9%** | 95.9% |
| valid documents accepted (601) | **100.0%** | 100.0% |
| invalid documents rejected (175) | **100.0%** | 53.7% |
| well-formedness (`not-wf`, 1260) | **99.8%** | 99.8% |

**2034 of 2036.** Every valid document accepted, every invalid one rejected,
and one more not-well-formed case caught than libxml2.

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
- **0 `unsafe`** in all twelve crates. Node names and text use
  `compact_str` (pinned to an exact version), which keeps short strings inline
  and has internal `unsafe` covered by its fuzzing, Miri and Kani proofs. It
  passed `cargo audit`, `cargo deny` and a checksum scan before adoption.
- Nothing on the parse, save, format, XPath, stream, canonicalize, XInclude,
  DTD, XSD or Schematron path recurses per level of nesting, so document depth
  cannot exhaust the stack. RelaxNG matching backtracks, so it does recurse,
  and it refuses documents nested past 320–640 levels (depending on the
  grammar) instead of overflowing.
- Untrusted input cannot drive unbounded work:
  - **Entity expansion** is bounded as libxml2 bounds it. Past 1,000,000
    bytes, expansion may not exceed 5× the input consumed, for general and
    parameter entities alike (`XmlParseSettings::max_amplification`).
  - **DTD content models** match in one pass, in time linear in the number of
    children, and non-deterministic models are reported with libxml2's
    message.
  - **XPath** sorts by a document-order index, so steps are not quadratic, and
    `XmlXPathContext::set_op_limit` caps the work, as libxml2's `opLimit` does.
  - **RelaxNG** remembers what each pattern did on each element once it has
    had to backtrack, so a failing document cannot make alternatives that
    reach the same definition retry the same subtree exponentially, and a
    run of siblings is matched in linear time.
  - **DTD validation** looks attribute declarations up through an index
    and shares identical follow sets, so a large DTD (TEI, DocBook) does
    not multiply the cost of every element.
  - **HTML nesting** stops at libxml2's cap, 256 levels (2048 under
    `XML_PARSE_HUGE`), so code that walks the tree recursively is safe on any
    page. `HtmlParseSettings::max_depth` raises or removes it.
- **Long-running calls can be cancelled.** Parsing, XPath, the validators,
  C14N and save have `*_interruptible` variants that take an `Interrupt`: an
  `AtomicBool`, a closure or `NeverInterrupt`. DTD loading, XInclude and the
  reader take one through `ParseSettings` (`*_with`), which also carries the
  depth cap and amplification limit for both parsers. A SAX handler can end
  a parse with `stop_requested`.
- A seeded fuzzer (`--bin fuzz`) holds four invariants: no panics, chunked
  parsing equals whole parsing at every chunk size, the XML round trip is a
  fixed point, and the HTML round trip reaches one after a second pass, as
  libxml2 2.15's does.

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
rusty_xml = "0.10"
```

MSRV is **1.85**. The library never sets `#[global_allocator]`.

The published crates (MIT OR Apache-2.0). Each version is the workspace
`version` that crate's `Cargo.toml` inherits (`version.workspace = true`):

| Crate | Version | Role | Docs |
|---|---|---|---|
| [`rusty_xml`](https://crates.io/crates/rusty_xml) | 0.10.1 | **the facade — depend on this** | [docs.rs](https://docs.rs/rusty_xml) |
| [`rusty_xml-parser`](https://crates.io/crates/rusty_xml-parser) | 0.10.1 | well-formed parse, encodings, push, catalogs, HTML | [docs.rs](https://docs.rs/rusty_xml-parser) |
| [`rusty_xml-tree`](https://crates.io/crates/rusty_xml-tree) | 0.10.1 | arena DOM | [docs.rs](https://docs.rs/rusty_xml-tree) |
| [`rusty_xml-sax`](https://crates.io/crates/rusty_xml-sax) | 0.10.1 | SAX2 recorder + xmllint-debug dump | [docs.rs](https://docs.rs/rusty_xml-sax) |
| [`rusty_xml-reader`](https://crates.io/crates/rusty_xml-reader) | 0.10.1 | `xmlTextReader` | [docs.rs](https://docs.rs/rusty_xml-reader) |
| [`rusty_xml-writer`](https://crates.io/crates/rusty_xml-writer) | 0.10.1 | `xmlsave` + `xmlTextWriter` | [docs.rs](https://docs.rs/rusty_xml-writer) |
| [`rusty_xml-xpath`](https://crates.io/crates/rusty_xml-xpath) | 0.10.1 | XPath 1.0 | [docs.rs](https://docs.rs/rusty_xml-xpath) |
| [`rusty_xml-valid`](https://crates.io/crates/rusty_xml-valid) | 0.10.1 | DTD, C14N, RelaxNG, XSD, Schematron | [docs.rs](https://docs.rs/rusty_xml-valid) |
| [`rusty_xml-cli`](https://crates.io/crates/rusty_xml-cli) | 0.10.1 | `rxmlint` binary | — |
| [`rusty_xml-alloc`](https://crates.io/crates/rusty_xml-alloc) | 0.10.1 | allocator seam **for binaries only** — the library never uses it | — |

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
