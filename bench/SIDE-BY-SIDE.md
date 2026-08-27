# us vs C (libxml2) — publish board

Pinned oracle: `oracle/PIN` (libxml2 **v2.15.3**, SHA in that file).
`us/C` **< 1 means rusty_xml is faster**.

| workload | file | bytes | inner | us MB/s | C MB/s | **us/C** | wins | z | null floor |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| parse-noout | `big-attr.xml` | 627,420 | 100 | **83.25** | 37.18 | **0.447x** | 20/20 | +4.47 | 0.95% |
| parse-noout | `big-1m.xml` | 1,270,526 | 100 | **115.74** | 74.56 | **0.644x** | 20/20 | +4.47 | 0.00% |
| parse-noout | `big-300k.xml` | 308,576 | 100 | **125.56** | 78.47 | **0.625x** | 20/20 | +4.47 | 3.85% |
| parse-noout | `slashdot.xml` | 3,675 | 10,000 | **118.05** | 50.98 | **0.432x** | 20/20 | +4.47 | 6.52% |

METHOD pinned=yes core=4 (not 0) priority=High metric=CPU-time interleaved=ABBA
N=20 pairs null=C-vs-C per row inner=--repeat work=bytes*inner, both arms exit 0
us=`rxmlint` 0.2.0 (links rusty_alloc) C=pinned `xmllint` v2.15.3 (system allocator)

## What these numbers do and do not say

**As shipped.** `rxmlint` links `rusty_alloc`; `xmllint` uses the system
allocator. That is the comparison a user experiences, and it is the table above.

**Same allocator**, both on the system allocator — the parser alone, pinned
CPU-time, N=11: `big-attr` **0.80x**, `big-300k` **1.07x**. About half the
margin above is the allocator, which C could also adopt. Quoting only the
as-shipped ratio as a parser result would be wrong.

**The `slashdot.xml` row is flattered.** C runs `xmlCtxtReadFile` per
`--repeat` (the Windows pin has no mmap) while we do one `fs::read` then
`xml_read_memory`. On a 3.6 KB file that per-iteration open-and-read is a
double-digit percentage of C's 68.75 us/parse. On the large rows it is ~1%
and does not move the verdict. Read the large files as the real result.

**A 0.00%% null floor is CPU-time quantisation** (15.625 ms), not precision.
The effects here are 55-131%% and every row is 20/20 at z=+4.47.

**Flags differ.** C defaults to `XML_PARSE_COMPACT | XML_PARSE_BIG_LINES`;
we force `XML_PARSE_NONET | XML_PARSE_NO_XXE`. This is CLI-vs-CLI, not a
kernel A/B.
