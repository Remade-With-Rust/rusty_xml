# rusty_xml — remake libxml2 in Rust

Status: M0–M6 landed (UTF-8 parse, tree/save/writer/reader, encodings/push/IO/catalogs, XPath 1.0, DTD/C14N/XInclude, HTML/RelaxNG/XSD/Schematron). M7 performance campaign next.

Source of truth for the C surface: [GNOME/libxml2](https://github.com/GNOME/libxml2) (read-only mirror of [gitlab.gnome.org/GNOME/libxml2](https://gitlab.gnome.org/GNOME/libxml2)). MIT licensed. We reimplement the algorithms. We do not fork, vendor, or link the C.

Part of [Remade With Rust](https://github.com/Remade-With-Rust) by Mata Network. Same contract as the house guidance / `rusty_h264` / the house guidance: **same wire protocol and public functions, new code you can actually depend on. No C. No copyleft. No `*-sys`.**

---

## Method

Every measurement and keep/revert in this repo follows one discipline, stated here so a
reader can audit any number without trusting us:

- **A pinned C oracle**, run as an external process, never linked. Version and binary hash
  in `oracle/PIN`.
- **Dual-direction gates.** SAX traces compared line-for-line with the oracle, plus a tree
  diff, plus `parse(write(parse(x)))`.
- **Byte-identical first.** A speed change that alters output is not a speed change.
- **Admissible numbers only.** Pinned to one core, High priority, CPU time, arms
  ABBA-interleaved, a null arm per row, paired win-rate with a z-score, and a method line
  printed with every figure. A number without its method line is not evidence.
- **Work-count parity.** Both arms must do the same work, compared as a count, not a time.
- **Counters before clocks.** A deterministic count of the work removed is immune to a busy
  machine; the clock confirms the sign.
- **Revert what does not pay**, and record which kind of revert it was — measured worse, or
  inside the noise.

XML is a **parser**, not a media codec, but the method is the same one we use for codecs: a
pinned oracle, dual-direction gates, work-count parity, and no speed claim without a method
line.

---

## 0. The one-line test

> Could a crate user parse untrusted XML on a machine with no C toolchain, get the same tree libxml2 would have built under matched options, and never take an entity-expansion or use-after-free CVE that C libxml2 is famous for?

If no, we are not done.

libxml2's own README says it is **not recommended for untrusted data**. That is the defect this remake exists to close. Semantic identity with C under matched options; **safe defaults** that C historically got wrong (no network, bounded amplification, external entities off unless asked).

---

## 1. What we are remaking (and what we are not)

**We are remaking** the XML 1.0 toolkit libxml2 actually is:

- Well-formed parse (document / memory / file / push / IO callbacks)
- DOM tree (`tree.h`)
- SAX2 callbacks (`SAX2.h`)
- Pull reader (`xmlreader.h`)
- Writer (`xmlwriter.h`) and save (`xmlsave.h`)
- Namespaces, entities (internal, bounded), encodings without iconv
- XPath 1.0, xml:id, catalogs (local), C14N, DTD validation
- HTML parser as a later mission (libxml2's `HTMLparser.c` is a separate grammar)
- CLI shaped like `xmllint`

**We are not:**

- A wrapper around libxml2 (`libxml` / `libxml2-sys` on crates.io)
- A wrap of `quick-xml` / `roxmltree` / `xml-rs` with a libxml2-shaped façade
- A vendor of GNOME C into the crate
- An iconv or zlib-sys build
- A network client. `nanohttp` / `nanoftp` stay unimplemented. Resource loaders are caller-supplied. Default is `XML_PARSE_NONET` behaviour even when C still has a legacy on-switch

**Pin, do not chase master.** M0 records `xmllint --version` and the git tag. The bench crate shells out to that binary and **never links libxml2**, exactly as `rusty_zstd-bench` never links libzstd.

---

## 2. The house remake pattern (copy this, do not invent a third)

This is how the other Remade-With-Rust programs did the mapping. rusty_xml does the same three layers.

| Layer | rusty_zstd | rusty_alloc | rusty_h264 | rusty_xml |
|---|---|---|---|---|
| C oracle | facebook/zstd v1.5.7 as an **external process** | mimalloc v2.4.5 behaviour tests | Cisco `h264dec` / ffmpeg | **pinned `xmllint` + libxml2 test vectors**, external process |
| Semantic twin | `compress` / `decompress` / `Compressor::stream` = `ZSTD_compress` / `ZSTD_decompressStream` | ~150 of ~157 `mi_*` entry points, semantics-for-semantics | `Encoder` / `Decoder` over Annex-B | **every current `XMLPUBFUN` that is not `XML_DEPRECATED`**, semantics-for-semantics |
| C ABI | roadmap: optional `cdylib` so C callers relink | `include/rusty_mimalloc.h` | not the product | **optional `cdylib` exporting the C names** (`xmlReadMemory`, …) |
| Workspace | lib / cli / bench / alloc-seam | core / api / default | common / encoder / decoder / façade / cli / accel | see §5 |
| Dual gate | C↔us both directions, every commit | allocation semantics + double-free abort | bit-exact recon vs oracle | **event-exact parse + byte-exact save under matched flags** |
| CLI aliases | `rzstd` / `unzstd` / `zstdcat` / `zstdmt` | n/a | `rusty_h264-cli` | `rxmlint` (never shadow `xmllint` on PATH) |

**Naming law, same as zstd vs `ZSTD_*`:**

- Rust façade: `snake_case` that is a mechanical transliteration of the C name (`xmlReadMemory` → `xml_read_memory`).
- Exact C identifiers live as `#[doc(alias = "xmlReadMemory")]` and, when the `c_abi` feature is on, as `#[no_mangle] extern "C" fn xmlReadMemory(...)`.
- Do not invent a second vocabulary (`parse_doc` vs `xml_read_memory`). One name, two spellings.

Deprecated C symbols (`xmlParseFile`, `xmlSAXParseMemory`, `xmlInitGlobals`, …) are **not** in the Rust façade. They are listed in the census as `deprecated → use <current>` so a porting guide exists, and they may appear on the `c_abi` cdylib later for ABI-compat if a real C consumer needs them. They are not bring-up work.

---

## 3. The gates (correctness before any speed number)

XML has no CABAC `dif/rng/cnt`. The analog of a symbol-level oracle is a **deterministic SAX event trace** plus a **canonical tree dump**.

### 3.1 Event-exact parse (the decoder gate)

For every fixture:

1. Run pinned `xmllint --sax --noout` (or an in-tree probe binary built from the oracle, never linked into us) and capture the SAX2 callback sequence: `startDocument`, `startElementNs` (local, prefix, URI, namespaces, attributes), `characters` (exact utf-8 bytes, including whitespace), `ignorableWhitespace`, `cdataBlock`, `processingInstruction`, `comment`, `endElementNs`, `endDocument`, error codes.
2. Run our parser with the **same** `xmlParserOption` bits.
3. Diff the traces line-for-line. First mismatch is the brick. Do not advance.

This is the measurement discipline: brick by brick, never trust a matching *final* tree over a wrong *intermediate* event.

**Work-count parity (measurement §4):** both arms must report the same `bytes_consumed`, `element_starts`, `text_nodes`, `attributes`. Divergent counts void the comparison.

### 3.2 Byte-exact save (the encoder gate)

`parse(write(x))` is **not** enough — a self-consistent writer can still be illegal. Dual gate, same as the measurement discipline:

1. We write with options `O` → C's `xmllint` / `xmlReadMemory` accepts it and the SAX trace matches.
2. C writes the same tree with the same `xmlSaveOption` bits → our bytes match C's bytes **or** we have a recorded, justified delta (attribute order is the usual one; C14N is the identity that must be byte-exact).

C14N 1.0 (`c14n.h`) is the standing byte-identity corpus. Non-C14N dump may differ on `'` vs `"` quoting only when the option bit says so; those bits are part of the fixture.

### 3.3 Conformance corpora (not synthetic-only)

| Corpus | Why it exists | Gate |
|---|---|---|
| W3C XML 1.0 test suite (`xmlconf`) | Well-formed / not-well-formed / valid / invalid | pass/fail identical to C, including error class |
| libxml2 `test/` + `result/` | The oracle's own expected dumps | byte-diff against `result/` |
| Namespaces 1.0 / xml:id | Prefix/URI identity | event-exact |
| XPath 1.0 (xmlconf + libxml2 xpath tests) | Node-set membership, not string dump order | sorted node-id lists match |
| Generated holdout | encodings, DTD internals, billion-laughs, huge names | we reject what C rejects; we also reject amplification C still allows if our default bound is tighter — **record the delta** |
| Real content | SVG, RSS/Atom, Android manifests, OOXML `[Content_Types].xml`, Maven POM, a Wikipedia dump slice | event-exact + timed (see §7) |

A fixture that never enters the path you changed cannot gate it (the measurement discipline fixture-coverage law). Count SAX event types per fixture before claiming a brick is proven.

### 3.4 Security gates that C does not have (on by default)

These are **defaults**, overridable, and they are the product:

| Attack | Default | Match C when |
|---|---|---|
| External general/parameter entities (XXE) | **off** | `XML_PARSE_DTDLOAD` / `XML_PARSE_NOENT` set *and* a resource loader is installed |
| Network I/O | **off** | never from the library; caller loader only |
| Entity amplification (billion laughs) | `xml_set_max_amplification` default matches modern libxml2 (document it at M0 from C source) | same option value |
| Quadratic blowup (`xml:space`, attribute defaults) | bounded | same |
| Recovery parse of garbage | off (`XML_PARSE_RECOVER` is opt-in, and even then must not panic) | flag set |

Fuzz: cargo-fuzz with committed seeds from xmlconf + mutated well-formed docs. Zero panics, zero hangs, bounded RSS. This is a standing CI job, not a one-shot.

---

## 4. Direct function map — current (non-deprecated) libxml2 → rusty_xml

Mechanical rule: **Rust name = C name with `xml` kept as prefix and CamelCase broken on acronyms** (`xmlCtxtGetSAXHandler` → `xml_ctxt_get_sax_handler`). Types: `xmlDoc *` → `XmlDoc`, `xmlChar *` owned → `XmlString` (interned when it came from the dict), borrowed → `&XmlStr`. Integer option bitmasks stay the C numeric values so a ported caller can pass `XML_PARSE_NONET | XML_PARSE_NOENT` unchanged.

M0 generates the full census from `include/libxml/*.h` (`XMLPUBFUN` not `XML_DEPRECATED`). The tables below are the **bring-up spine** — the functions every other remake would have listed first. Unlisted current symbols still belong in the census and still get a 1:1 Rust item; they are not optional.

### 4.1 Lifecycle / parser context (`parser.h`)

| libxml2 | rusty_xml | Notes |
|---|---|---|
| `xmlInitParser` | `xml_init_parser` | No-op in Rust (no process-global ctor). Kept so ports compile. Document that. |
| `xmlCleanupParser` | `xml_cleanup_parser` | No-op. Same. |
| `xmlNewParserCtxt` | `xml_new_parser_ctxt` | Returns `XmlParserCtxt` |
| `xmlNewSAXParserCtxt` | `xml_new_sax_parser_ctxt` | |
| `xmlFreeParserCtxt` | `xml_free_parser_ctxt` / `Drop` | Rust façade relies on `Drop`; C ABI keeps the free |
| `xmlCreateDocParserCtxt` | `xml_create_doc_parser_ctxt` | |
| `xmlCreatePushParserCtxt` | `xml_create_push_parser_ctxt` | Push parser |
| `xmlParseChunk` | `xml_parse_chunk` | |
| `xmlCreateIOParserCtxt` | `xml_create_io_parser_ctxt` | Callbacks, no std `Read` in the C ABI |
| `xmlReadDoc` | `xml_read_doc` | **Preferred** document parse |
| `xmlReadFile` | `xml_read_file` | |
| `xmlReadMemory` | `xml_read_memory` | Hot path for benches |
| `xmlReadFd` | `xml_read_fd` | |
| `xmlReadIO` | `xml_read_io` | |
| `xmlCtxtReadDoc` / `File` / `Memory` / `Fd` / `IO` | `xml_ctxt_read_*` | Reuse a context |
| `xmlCtxtUseOptions` / `xmlCtxtSetOptions` / `xmlCtxtGetOptions` | `xml_ctxt_{use,set,get}_options` | Bit-identical option values |
| `xmlCtxtReset` / `xmlCtxtResetPush` | `xml_ctxt_reset*` | |
| `xmlStopParser` | `xml_stop_parser` | |
| `xmlParseDocument` | `xml_parse_document` | Drive an already-loaded ctxt |
| `xmlParseInNodeContext` | `xml_parse_in_node_context` | |
| `xmlCtxtParseDtd` | `xml_ctxt_parse_dtd` | |
| `xmlParseDTD` | `xml_parse_dtd` | |
| `xmlCtxtValidateDocument` / `xmlCtxtValidateDtd` | `xml_ctxt_validate_*` | |
| `xmlCtxtGetDocument` | `xml_ctxt_get_document` | |
| `xmlCtxtGetStatus` | `xml_ctxt_get_status` | `xmlParserStatus` bits match |
| `xmlCtxtGetInputPosition` | `xml_ctxt_get_input_position` | |
| `xmlCtxtSetMaxAmplification` | `xml_ctxt_set_max_amplification` | Security-critical |
| `xmlCtxtGetLastError` | `xml_ctxt_get_last_error` | |
| `xmlSetMaxAmplification` | `xml_set_max_amplification` | |

SAX1 `xmlParseDoc` / `xmlParseFile` / `xmlParseMemory` are **deprecated**. Façade ports them to `xml_read_*` in the porting guide only.

### 4.2 Tree (`tree.h`) — identity of the DOM

| libxml2 | rusty_xml |
|---|---|
| `xmlNewDoc` | `xml_new_doc` |
| `xmlFreeDoc` | `xml_free_doc` / `Drop` |
| `xmlCopyDoc` | `xml_copy_doc` |
| `xmlNewNode` / `xmlNewNodeEatName` | `xml_new_node*` |
| `xmlNewDocNode` / `xmlNewDocRawNode` | `xml_new_doc_node*` |
| `xmlNewChild` / `xmlAddChild` / `xmlAddChildList` | `xml_new_child` / `xml_add_child*` |
| `xmlAddPrevSibling` / `xmlAddNextSibling` / `xmlAddSibling` | `xml_add_*_sibling` |
| `xmlUnlinkNode` / `xmlReplaceNode` | `xml_unlink_node` / `xml_replace_node` |
| `xmlFreeNode` / `xmlFreeNodeList` | `Drop` + C ABI |
| `xmlDocGetRootElement` / `xmlDocSetRootElement` | `xml_doc_get_root_element` / `xml_doc_set_root_element` |
| `xmlNodeGetContent` / `xmlNodeSetContent` | `xml_node_get_content` / `xml_node_set_content` |
| `xmlNodeGetLang` / `xmlNodeGetSpacePreserve` | `xml_node_get_lang` / `xml_node_get_space_preserve` |
| `xmlGetNsList` / `xmlSearchNs` / `xmlSearchNsByHref` | `xml_search_ns*` |
| `xmlNewNs` / `xmlSetNs` | `xml_new_ns` / `xml_set_ns` |
| `xmlHasProp` / `xmlHasNsProp` / `xmlGetProp` / `xmlGetNsProp` | `xml_has_prop` / `xml_get_prop*` |
| `xmlSetProp` / `xmlSetNsProp` / `xmlUnsetProp` / `xmlUnsetNsProp` | `xml_set_prop*` / `xml_unset_prop*` |
| `xmlGetID` / `xmlIsID` | `xml_get_id` / `xml_is_id` |
| `xmlDocDump` / `xmlDocDumpFormatMemory` / `xmlNodeDump` | routed through save (§4.5) |
| `xmlGetLineNo` | `xml_get_line_no` |
| `xmlIsBlankNode` | `xml_is_blank_node` |

Node types (`XML_ELEMENT_NODE`, `XML_TEXT_NODE`, …) keep the C discriminant numbers.

Ownership in Rust: `XmlDoc` owns the arena. `XmlNodeRef` / `XmlNodeMut` are handles (index or generational), **not** raw pointers with parent-and-child `&mut`. This is the the house guidance move that lets the C ABI still hand out `xmlNode *` as indices into the arena while the safe façade cannot form aliasing mutable references. The C pointer value is stable for the lifetime of the doc, matching libxml2.

### 4.3 SAX2 (`SAX2.h`)

| libxml2 | rusty_xml |
|---|---|
| `xmlSAX2InitDefaultSAXHandler` | `xml_sax2_init_default_sax_handler` |
| `xmlSAXVersion` | `xml_sax_version` |
| `xmlDefaultSAXHandlerInit` | not in façade (deprecated / global) |
| SAX2 callbacks on `xmlSAXHandler` | `Sax2Handler` trait + C function-pointer struct for `c_abi` |

Callback names and order match C: `internalSubset`, `isStandalone`, `hasInternalSubset`, `hasExternalSubset`, `resolveEntity`, `getEntity`, `entityDecl`, `notationDecl`, `attributeDecl`, `elementDecl`, `unparsedEntityDecl`, `setDocumentLocator`, `startDocument`, `endDocument`, `startElement` (SAX1, optional), `endElement`, `reference`, `characters`, `ignorableWhitespace`, `processingInstruction`, `comment`, `warning`, `error`, `fatalError`, `getParameterEntity`, `cdataBlock`, `externalSubset`, `startElementNs`, `endElementNs`, `serror`.

The instrumented oracle prints these. Our handler for the gate is a recorder, not an application.

### 4.4 Reader (`xmlreader.h`) — pull parser, 1:1

Constructors: `xmlNewTextReader`, `xmlNewTextReaderFilename`, `xmlFreeTextReader`, `xmlTextReaderSetup`, `xmlReaderForDoc` / `File` / `Memory` / `Fd` / `IO`, `xmlReaderWalker`, `xmlReaderNew*`.

Iterators: `xmlTextReaderRead`, `xmlTextReaderNext`, `xmlTextReaderNextSibling`, `xmlTextReaderReadString`, `xmlTextReaderReadInnerXml` / `OuterXml`, `xmlTextReaderReadAttributeValue`.

Node attributes: `xmlTextReaderNodeType`, `Depth`, `AttributeCount`, `HasAttributes`, `HasValue`, `IsEmptyElement`, `IsDefault`, `IsNamespaceDecl`, `QuoteChar`, `ReadState`, `ConstName` / `LocalName` / `Prefix` / `NamespaceUri` / `Value` / `BaseUri` / `XmlLang` / `Encoding` / `XmlVersion`, plus the allocating non-`Const` twins.

Attributes: `MoveToAttribute` / `No` / `Ns`, `MoveToFirstAttribute`, `MoveToNextAttribute`, `MoveToElement`, `GetAttribute*`, `LookupNamespace`.

Extensions: `xmlTextReaderExpand`, `Preserve`, `CurrentNode`, `CurrentDoc`, `ByteConsumed`, `SetParserProp` / `GetParserProp`, `SetMaxAmplification`, `GetLastError`, RelaxNG / Schema validate hooks once those missions land.

`xmlReaderTypes` discriminants stay 0..=17 matching C.

### 4.5 Writer + save (`xmlwriter.h`, `xmlsave.h`)

Writer: `xmlNewTextWriter`, `xmlNewTextWriterFilename`, `xmlNewTextWriterMemory`, `xmlNewTextWriterDoc`, `xmlNewTextWriterTree`, `xmlFreeTextWriter`, then the full `Start`/`End`/`Write` family for Document, Element, ElementNS, Attribute, AttributeNS, Comment, PI, CDATA, Raw, String, Base64, BinHex, DTD* — **including the `Format` / `VFormat` variants** on the C ABI. Rust façade exposes the non-varargs forms; `format!` at the call site replaces `printf`.

Save: `xmlSaveToFd` / `Filename` / `Buffer` / `IO`, `xmlSaveDoc`, `xmlSaveTree`, `xmlSaveFlush`, `xmlSaveClose`, `xmlSaveSetAttrEscape` / `xmlSaveSetEscape`. `xmlSaveOption` bits keep C values (`XML_SAVE_FORMAT`, `XML_SAVE_NO_DECL`, `XML_SAVE_NO_EMPTY`, `XML_SAVE_AS_XML`, `XML_SAVE_AS_HTML`, `XML_SAVE_XHTML`, `XML_SAVE_NONET`, `WSNONSIG`, …).

### 4.6 XPath 1.0 (`xpath.h`)

| libxml2 | rusty_xml |
|---|---|
| `xmlXPathNewContext` | `xml_xpath_new_context` |
| `xmlXPathFreeContext` | `Drop` |
| `xmlXPathEval` / `xmlXPathEvalExpression` | `xml_xpath_eval*` |
| `xmlXPathNodeEval` | `xml_xpath_node_eval` |
| `xmlXPathCompile` / `xmlXPathCtxtCompile` | `xml_xpath_compile*` |
| `xmlXPathCompiledEval` / `ToBoolean` | `xml_xpath_compiled_eval*` |
| `xmlXPathSetContextNode` | `xml_xpath_set_context_node` |
| `xmlXPathCastToBoolean` / `Number` / `String` | `xml_xpath_cast_to_*` |
| `xmlXPathOrderDocElems` | `xml_xpath_order_doc_elems` |
| `xmlXPathCmpNodes` | `xml_xpath_cmp_nodes` |
| `xmlXPathIsNaN` / `IsInf` | `xml_xpath_is_nan` / `xml_xpath_is_inf` |

`xmlXPathObjectType` discriminants stay 0/1/2/3/4/8/9. Node-set comparison in the gate is **by node identity** (document order after `xml_xpath_order_doc_elems`), never by pointer address.

### 4.7 Support modules (map, then implement as the parser needs them)

Each of these is a 1:1 header, not a "we'll use HashMap and call it done":

| Header | First functions | Notes |
|---|---|---|
| `encoding.h` | `xmlFindCharEncodingHandler`, `xmlParseCharEncoding`, `xmlCharEncInFunc` / `OutFunc` | **No iconv.** Tables + `encoding_rs` for Encoding Standard sets; remaining ISO-8859 / EBCDIC as explicit tables. `wasm32` clean |
| `entities.h` | `xmlAddDocEntity`, `xmlGetPredefinedEntity`, `xmlEncodeEntitiesReentrant` | Predefined `&lt; &gt; &amp; &apos; &quot;` first |
| `dict.h` | `xmlDictCreate`, `xmlDictLookup`, `xmlDictOwns` | Intern names; the big allocation lever |
| `hash.h` | `xmlHashCreate`, `xmlHashAdd` / `Lookup` / `Scan` | IDs, entities, SAX tables |
| `xmlstring.h` | `xmlStrdup`, `xmlStrlen`, `xmlStrEqual`, `xmlStrcasestr`, `xmlUTF8Strlen` | C ABI needs these; Rust uses `&[u8]` / `&str` |
| `chvalid.h` | `xmlIsBaseChar`, `xmlIsCombining`, `xmlIsDigit`, `xmlIsExtender`, `xmlIsChar` | Generated from the same Unicode productions C uses; **byte-identical class tables** |
| `uri.h` | `xmlParseURI`, `xmlBuildURI`, `xmlCanonicPath` | |
| `xmlIO.h` | `xmlParserInputBufferCreate*`, `xmlOutputBufferCreate*`, `xmlRegisterInputCallbacks` | File/memory/fd/IO; **no HTTP** |
| `xmlerror.h` | `xmlGetLastError`, `xmlResetError`, `xmlSetStructuredErrorFunc` | Error codes keep C `xmlParserErrors` numbers |
| `catalog.h` | `xmlLoadCatalog`, `xmlACatalogResolve` | Local files only |
| `c14n.h` | `xmlC14NDocDumpMemory`, `xmlC14NDocSave` | Byte-exact vs C |
| `valid.h` | `xmlValidateDocument`, `xmlValidateDtd`, `xmlAddID` | DTD |
| `xinclude.h` | `xmlXIncludeProcess` | Needs resource loader; default no net |
| `pattern.h` | `xmlPatterncompile`, `xmlPatternMatch` | |
| `xmlregexp.h` | `xmlRegexpCompile`, `xmlRegexpExec` | Used by schemas |
| `relaxng.h` / `xmlschemas.h` / `schematron.h` | later missions | |
| `HTMLparser.h` / `HTMLtree.h` | later mission | Separate grammar |
| `xmlmemory.h` | `xmlMalloc` / `xmlFree` hooks | C ABI only; Rust uses the global allocator of the **deliverable** |
| `threads.h` | `xmlNewMutex` etc. | Prefer Rust `Mutex`; C ABI shims if a port needs them |
| `xmlmodule.h` | skip | Dynamic modules are a C plugin ABI we do not ship |

### 4.8 CLI map (`xmllint` → `rxmlint`)

Never install as `xmllint`. Primary binary `rxmlint`, same flag language as C so a bench script can swap the argv[0].

| xmllint | rxmlint | Bench role |
|---|---|---|
| `--noout` | `--noout` | parse-only timing |
| `--stream` | `--stream` | reader path |
| `--sax` / `--sax1` | `--sax` | event-exact traces |
| `--xpath EXPR` | `--xpath` | XPath mission |
| `--c14n` / `--exc-c14n` | `--c14n*` | byte-exact save |
| `--encode ENC` | `--encode` | encoding tables |
| `--html` | `--html` | later |
| `--relaxng` / `--schema` / `--schematron` | same | later |
| `--dtdvalid` | `--dtdvalid` | DTD mission |
| `--push` | `--push` | push parser |
| `--memory` | `--memory` | |
| `--repeat N` | `--repeat` | C's own timing loop; we still use the pinned harness, not this, for published numbers |
| `-o FILE` | `-o` | |

`XMLLINT_INDENT` env var honoured, as C does. Method line is still printed by **our** bench, not by `--repeat`.

---

## 5. Workspace (day-one layout)

Matches the house stack guidance §2 and the house guidance's crate split.

```
rusty_xml/
├── Cargo.toml                 # workspace; pins once
├── deny.toml                  # no *-sys, no copyleft, no libxml2-sys
├── rustfmt.toml
├── docs/plan/rusty_xml.md     # this file
├── bench/                     # pinned A/B vs xmllint; never links libxml2
│   └── pinvs.ps1              # pinned timing harness
├── corpora/                   # xmlconf + real docs + generated holdout (git-lfs or fetch script)
├── oracle/                    # fetch-oracle.ps1; pin notes; NEVER compiled into the lib
├── crates/
│   ├── rusty_xml/             # PUBLISHED façade — depend on this
│   ├── rusty_xml-parser/      # well-formed parse, push, encodings, chvalid, entities, dict
│   ├── rusty_xml-tree/        # arena DOM, ns, ids
│   ├── rusty_xml-sax/         # SAX2 recorder + handler trait
│   ├── rusty_xml-reader/      # xmlTextReader
│   ├── rusty_xml-writer/      # xmlTextWriter + xmlsave
│   ├── rusty_xml-xpath/       # XPath 1.0
│   ├── rusty_xml-valid/       # DTD (later: RelaxNG, XSD)
│   ├── rusty_xml-c-abi/       # optional cdylib; exact C names; not a default feature of the lib
│   ├── rusty_xml-cli/         # rxmlint — DELIVERABLE
│   ├── rusty_xml-bench/       # campaign harness — not published
│   └── rusty_xml-alloc/       # rusty_alloc seam for binaries only
├── fuzz/                      # cargo-fuzz targets
└── scripts/fetch-oracle.ps1
```

**Allocator law:** `#[global_allocator]` lives in `rusty_xml-cli` / bench binaries via `rusty_xml-alloc`. The published library does not depend on `rusty_alloc-api`.

**MSRV:** 1.85, same as the house guidance. `no_std + alloc` for the parser+tree (no file I/O). `std` feature adds file/fd/IO and the CLI.

**License:** MIT OR Apache-2.0 (libxml2 is MIT; dual is the house default and is compatible). `NOTICE.md` records that the C oracle is neither distributed nor linked.

---

## 6. Missions

One brick per commit. A mission is done when its gate is green in CI, not when the functions exist.

### M0 — Oracle, census, bench skeleton (no parser yet)

- Fetch and pin libxml2 (`scripts/fetch-oracle.ps1`). Record version, git SHA, configure flags (`minimum` off; HTML/reader/writer/xpath/c14n on; http off; zlib off for the first pin so gzip is a later apples-to-apples).
- Build C `xmllint` **Release** (`-O2 -fno-semantic-interposition` as their README warns defaults are unoptimised — the measurement discipline §8, the reference's defaults are configuration).
- Generate `docs/plan/API-CENSUS.md`: every `XMLPUBFUN` grouped current / deprecated / module, with our planned Rust name.
- Stand up `rusty_xml-bench`:
  - Arms: `rxmlint` (us) and pinned `xmllint` (C).
  - ABBA, pin to one core, High priority, **CPU time**, N ≥ 20, null arm (C vs C).
  - Work counts: bytes in, nodes (or SAX events), exit status.
  - Method line printed every run.
  - Refuses to report if work counts diverge or a run is below timer resolution.
- Corpora fetch: xmlconf + a small real set (one SVG, one Atom, one Android manifest).
- **Baseline board:** C-only, parse `--noout` and stream `--noout`, MB/s and ns/byte. We have no "us" arm yet. This board is the ceiling we will be compared to. Do not quote a ratio until both arms exist.

Exit: census committed, C bench green, method line in the log, oracle version in `oracle/PIN`.

### M1 — Well-formed UTF-8 document parse (event-exact)

Bring-up order, the measurement discipline style:

1. Character classes (`chvalid`) byte-identical vs C tables on 0..=0x10FFFF sample + xmlconf.
2. XML decl + encoding decl (UTF-8 only in M1).
3. Elements, attributes, namespaces (`startElementNs` / `endElementNs`).
4. Text, CDATA, comments, PI, whitespace (`XML_PARSE_NOBLANKS` matched).
5. Predefined entities. Internal general entities **without** expansion bombs (amplification bound).
6. `xml_read_memory` / `xml_read_doc` / `xml_ctxt_read_memory`.
7. Errors: well-formedness codes match C's `xmlParserErrors` for the xmlconf "not-wf" set.

Do **not** implement DTDs, XInclude, HTML, or recovery in M1.

SAX trace vs C is the gate. Tree dump vs C `xmllint --shell` `dir` / our dump format is the second gate.

### M2 — Tree mutation + save + writer + reader

- Full §4.2 tree API used by real ports (`xml_new_child`, props, ns, unlink).
- `xmlsave` with `XML_SAVE_FORMAT` / `NO_DECL` / `NO_EMPTY`.
- `xmlTextWriter` document/element/attribute/text/comment/PI/CDATA.
- `xmlTextReader` walk event-exact vs C `--stream` on the M1 corpus.
- Round-trip gate: `parse(write(parse(x)))` event-exact; C14N not required yet.

### M3 — Encodings, push, IO, catalogs (local)

- UTF-16 / UTF-32 BOM, ISO-8859-1, the rest of `xmlParseCharEncoding` enums C supports without iconv. Table-driven. No `encoding_rs` if a charset isn't in the Encoding Standard — write the table.
- Push (`xml_parse_chunk`) and IO callbacks.
- Local catalog resolve.
- Optional `gzip` feature: **pure-Rust** inflate (`miniz_oxide` or a house deflate if one exists by then). Never `libz-sys`. Off by default so the M0 pin stays comparable.

### M4 — XPath 1.0

- Compile + eval. Axes, node tests, predicates, core library functions.
- Gate: libxml2 xpath tests + xmlconf. Node-sets compared in document order after `xml_xpath_order_doc_elems`.
- `rxmlint --xpath` matches C's printed form **or** a recorded canonicalization (C's string dump of a nodeset is whitespace-sensitive — freeze the format at M4 start from C).

### M5 — DTD validation + C14N + XInclude (loader-gated)

- Internal + external DTD **only** through a caller resource loader.
- C14N 1.0 / exclusive C14N byte-exact vs C `--c14n`.
- XInclude with the same loader rule.

### M6 — HTML parser, RelaxNG, XML Schema, Schematron (separate grammars)

Each is its own sub-mission with its own corpus. HTML is not "XML with recovery"; it is `HTMLparser.c`. Do not start M6 until M1–M5 CI is green.

### M7 — Performance campaign (only after M2 is event-exact)

the measurement discipline order, no shortcuts:

1. the measurement discipline stage profiler (`profile` feature, ZST when off): `GuessEncoding`, `ScanName`, `ScanCharData`, `LookupNs`, `Intern`, `AllocNode`, `SaxDispatch`, `TreeLink`, `SaveEscape`.
2. Decompose the residue until every line is named.
3. the measurement discipline (intern, charset LUT, don't re-scan).
4. the measurement discipline (reader buffer, save grow, tree clone).
5. Document-size sweep (the measurement discipline) before any layout rewrite.
6. SIMD only for proven hot scans (`memchr`-class `<`, `]]>`, UTF-8 validate) after `--emit asm` shows zero packed ops.

**Published speed claims** vs C use the M0 harness, real corpora, paired win-rate + z, and the method line. Null-arm floor re-run per session. Do not average files.

Exit bars (set after the M0 C board exists; do not invent a ratio now): parse `--noout` and stream `--noout` within a stated factor of C at matched options, **and** event-exact. Ratio is standing, not a keep/revert for a brick — bricks keep on counters + identity.

### M8 — C ABI cdylib + hardening

- `rusty_xml-c-abi` exports current `XMLPUBFUN` names.
- the hardening audit audit, cargo-deny, fuzz, Miri on the arena, `wasm32-unknown-unknown` build.

---

## 7. Benchmark specification (standing)

Copied from the measurement discipline and `rusty_zstd-bench`. If the harness and this section disagree, the harness is wrong — fix the harness.

### 7.1 Arms

| Arm | Binary | Notes |
|---|---|---|
| C | pinned `oracle/bin/xmllint` | Release, flags recorded in `oracle/PIN` |
| us | `target/release/rxmlint` | `cargo build -p rusty_xml-cli --release`; verify mtime / a build marker (the measurement discipline §10) |
| null | C vs C | Resolution floor, every session |

Never time a debug build. Never time with the profiler feature on and then quote it as the product.

### 7.2 Workloads (each is a row; never a mean-only headline)

| id | what | C argv | us argv | work count |
|---|---|---|---|---|
| `parse-noout` | DOM parse, discard | `--noout FILE` | `--noout FILE` | bytes, elements |
| `stream-noout` | reader | `--stream --noout FILE` | `--stream --noout FILE` | bytes, reader ticks |
| `sax-noout` | SAX | `--sax --noout FILE` | `--sax --noout FILE` | SAX events |
| `xpath` | eval | `--xpath EXPR --noout FILE` | same | nodes in the set |
| `c14n` | canonicalize | `--c14n FILE` | same | output bytes |
| `roundtrip` | parse+save | C dump vs us dump | same options | output bytes must match C14N; non-C14N recorded |

Same files, same options, same number of inner repeats so arm **durations** match (the measurement discipline §5). If C finishes in 0.4 s and we in 4 s, lengthen the inner loop until both walls are ≥ ~15 s.

### 7.3 How a number is taken

PowerShell shape (Windows house box); Linux/macOS equivalent in the same script with `taskset` / `nice`:

- `Start-Process` → cache `$p.Handle` **before** `WaitForExit` → affinity one core (not 0) → `PriorityClass = High` → read `TotalProcessorTime`.
- ABBA: leading arm alternates each round (`NEW OLD OLD NEW`).
- N ≥ 20 pairs for anything under ~5%; N ≥ 31 for a claimed win.
- Report median **and** minimum, paired win rate, `z = (wins − N/2) / (0.5·√N)`, null-arm floor.
- Print the method line: pinned? interleaved? CPU or wall? N? null floor? work counts both arms? binary mtime?

A number without that line is not evidence. A speed brick that is not faster on this harness is reverted (the measurement discipline). A brick below the null floor is kept only with a **counter** of work removed (the measurement discipline §15).

### 7.4 What we refuse to compare

- C built without `-O2` (their default) against our `--release`.
- gzip-on C vs gzip-off us.
- `--recover` vs well-formed-only.
- Their `--repeat` wall vs our CPU time.
- A mean over corpora (the per-file spread is the story, same as rusty_zstd's Silesia board).

---

## 8. Instrument the oracle (bring-up only)

When SAX traces are not enough to locate a divergence (the measurement discipline probe law):

- Keep a **patch** against the pinned libxml2 that `fprintf`s entering state (line, col, consumed, parser state enum, current element name) at `xmlParseDocument` checkpoints.
- Build that binary as `oracle/bin/xmllint-probe`. Never ship it. Never link it.
- Gate probes by position so a shared function does not record the wrong call.
- Force single-threaded, deterministic input. No catalog, no net.

The probe is ephemeral; the patch + a fixture that triggers it stay in-tree until that class of bug cannot recur.

---

## 9. Non-goals and explicit refusals

- **Do not** add `libxml2-sys`, `libxml`, `quick-xml`, `roxmltree` as the implementation. Using `memchr` from a pure-Rust crate is fine; using someone else's XML grammar is not this project.
- **Do not** enable network entity loads to "match old C".
- **Do not** claim "faster than libxml2" from a laptop wall-clock or a Criterion microbench of `xml_read_memory` on a 200-byte string.
- **Do not** `unwrap` on malformed input. Malformed XML is the user-reachable path.
- **Do not** put `#[global_allocator]` in the library.
- **Do not** start HTML or XSD because they are in the C folder listing. M1 is UTF-8 well-formed XML or it is nothing.

---

## 10. First week, in order

1. Land this file. Land `deny.toml` + workspace skeleton with empty façade crates that compile.
2. M0: pin oracle, census, C-only bench board, fetch xmlconf.
3. M1 brick 1: `chvalid` tables + a test that they match C on a scraped dump of `xmlIsChar` for `0..=0xFFFF`.
4. M1 brick 2: `xml_read_memory` of `<a/>` event-exact vs `xmllint --sax --noout`.
5. Expand xmlconf well-formed / not-wf until the board is a real parser.

No speed work in week one. The M0 C board is the only number that week is allowed to produce.
