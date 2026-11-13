# HTML parser: libxml2 conformance at fewer instructions (issue #12)

Issue #12 reported nine ways `html_read_memory` built a different tree from
libxml2's `HTMLparser.c`. All nine are fixed by porting libxml2 2.15.3's
tokenizer and tree construction. Each fix was chosen from measured alternatives
by deterministic instruction count, so the conforming parser is also the
cheaper one.

## Instrument

- **Count:** callgrind `Ir`, inclusive of `html_read_memory` only (parse; no
  file I/O, no tree walk). Exact: the same binary re-run gives the same count
  to the instruction, and every round measured `final` first and last.
- **Two corpora:**
  - `frags`: the issue's 15 cases plus 10 synthetic stress files, parsed 20×
    (117,678 bytes).
  - `pages`: 197 real pages from rustdoc, mdBook, asciidoc (git) and Sphinx
    (Python), parsed once (8,177,955 bytes; 17 of them are CRLF).
- **Anchors:** element, text, attribute and comment counts, text bytes, and an
  FNV fingerprint of the whole tree. A variant that claims the same output
  must reproduce `final`'s fingerprint exactly, and every variant below did.
- **Correctness:** every corpus file serialized by pinned
  `oracle/bin/xmllint.exe --html` and by `rxmlint --html`, then compared.

## Result

| | frags parse Ir | pages parse Ir | oracle-identical files |
|---|---:|---:|---|
| before (0.8.3) | 412,967,819 | 514,091,494 | 1/25, 11/197 |
| after | 299,459,656 | 318,331,084 | 22/25, 194/197 |
| change | **−27.5%** | **−38.1%** | |

The 3 + 3 files that still differ do so only in serialization, not in the
tree: libxml2's HTML writer emits non-ASCII as named or numeric references,
leaves `<` and newlines raw inside attribute values, and percent-escapes
`href`/`src`. Those writer differences are separate from #12.

## Per problem: the winner and what it beat

Each percentage is parse Ir of the alternative relative to the winner
(frags / pages); positive means the alternative costs more. Fix-off rows show
what the old wrong behaviour cost in the same codebase.

| # | Problem | Winning solution | Alternative | frags | pages |
|---|---|---|---|---:|---:|
| 1 | Implied end tags, `section` closing `p`, `frame` void, late `title`/`meta` | libxml2's name stack, with tags interned to `u8` and compile-time bitset tables (`htmlStartClose` as one `u128` row per tag; priority and flags as byte tables) | libxml2's own shape: `bsearch` over (old, new) name pairs, `strcmp` scan for priority, `bsearch` for element rows | +23.1% | +15.1% |
| 1 | (tag interning) | collision-free perfect hash: one hash, one load, one compare | binary search over the 101 names | +10.7% | +13.1% |
| 2 | Script/style/title/textarea content | libxml2 data modes, memchr between `<`s, script escape states | fix off: content parsed as markup | +3.0% | +0.6% |
| 2 | (raw scan) | memchr to the next `<` | byte loop | −0.2% | +3.0% |
| 3 | `&name` without `;` | only the 106 legacy names (oracle-verified) may omit it; attribute rule (no alnum or `=` after); no allocation per reference | the obvious patch: shrink over all 2125 names, then filter | +10.1% | 0.0% |
| 3 | (decode scan) | 16-byte probe after each reference, then memchr | byte loop / memchr only | −0.4% / +1.5% | +0.6% / +0.03% |
| 3 | (fix cost) | | fix off: old decoder | +3.3% | −0.2% |
| 4 | First duplicate wins; valueless attrs have no value | dedupe against the parsed list, then `add_attr` with no scan; bare attrs recorded in `XmlDoc::html_bare_attrs` | probe the element's attributes in the arena | +0.3% | +0.1% |
| 4 | (fix cost) | | fix off: `xml_set_prop`, last wins, value = name | +1.0% | +0.9% |
| 5 | CR / CRLF | one vectorized pass over the input finds CR and NUL together; normalize only if a CR exists; skip every per-run NUL scan if no NUL exists | a memchr gate for CR, plus a NUL memchr per text run and attribute value | +2.0% | +6.0% |
| 5 | (fix cost) | | fix off: no normalization | +0.1% | −1.0% (the 17 CRLF pages' real work) |
| 6 | `<!foo>`, `<?pi>`, `</ 3>` | bogus comments, as libxml2 makes them | fix off: dropped | −2.3% (8,040 comment nodes libxml2 creates) | 0.0% |
| — | Input path (enabling) | borrow valid UTF-8, validate once | copy, then lossy re-validation | +6.5% | +21.9% |

### Refuted, with the number

- Forcing `expand` out of line with `#[inline(never)]` gave a byte-identical
  count on both corpora (303,882,736 / 318,422,339): LLVM already kept it out.
- Folding the end-tag priority loop into `stack[at+1..].iter().any(..)` cost
  +0.6% on pages (slice-iterator setup on every end tag). The index loop stays.
- Two splits went opposite ways on the two corpora: the raw-text scan (byte
  loop wins on short synthetic scripts, memchr on real ones) and the entity
  scan (byte loop wins on entity-dense text). The real-pages winner was kept
  for the first. For the second, a hybrid beat both pure arms on the two
  corpora combined.

## Follow-up: duplicate-attribute checks (issue #21)

Both parsers compared each attribute with every earlier one on its tag, so one
tag with 100,000 attributes took 12 s as XML and 7 s as HTML. Past 32
attributes each now uses a hash set; up to 32 the pairwise scan stays.

Same instrument, plus `rxmlint --noout` under callgrind on 8 real XML files
from libxml2's test tree (whole-program Ir, which varies by about 3K between
runs of one binary). Base is 0.8.5; tree fingerprints are identical.

| Workload | 0.8.5 | With the fix | Change |
|---|---|---|---|
| HTML fragments, parse Ir | 297,996,642 | 297,873,922 | −0.04% |
| HTML real pages, parse Ir | 316,481,131 | 315,625,915 | −0.27% |
| XML, 8 real files, total Ir | 189,911,929 | 187,789,086 | −1.12% |

Where the change comes from, on the real pages:

- The length test in front of the pairwise scan costs about 3 Ir per element
  (+0.09%). It runs before the attribute buffer is taken; the first shape,
  which handed the taken buffer to the out-of-line path, cost +0.18%.
- `Parser::name` compiled in two shapes, 1.5M Ir apart, and which one came out
  changed with edits elsewhere in the crate, including edits to the XML
  parser. In the dear shape the lowercasing collect is inlined. Moving it to
  `fold_name` with `#[inline(never)]` fixes the shape (−0.38%): `name` then
  cost the same 26,740,292 Ir in four builds.
- `is_boolean_attr` gained a second caller and stopped being inlined.
  `#[inline(always)]` restores it (−0.05%).
- XML: the namespace-expansion set was built for every element, seeding a
  hasher each time. Building both sets only past 32 attributes is the −1.12%.

Function-level counts carry the attribution, because totals also move with
libc `memcmp`'s own instruction count: on the fragments it rose by 0.17% of
the parse count between two builds at an unchanged number of calls.

## Next lever (outside #12)

Node creation is now the largest cost: about 500 Ir per element or text node,
mostly moving the 184-byte `Node` into the arena by `memcpy`. That cost is in
`rusty_xml-tree` and shared with the XML parser.
