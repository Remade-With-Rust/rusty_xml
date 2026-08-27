# us vs C (libxml2) session board

Pinned oracle: oracle/PIN. Not a publish claim (N=6; publish is N>=20).

Null arm C-vs-C on parse-noout C: medA=17109.375ms medB=19062.5ms floor=10.25% wins=9/12 z=1.73

| workload | file | bytes | inner | N | us_med_ms | us_min_ms | C_med_ms | C_min_ms | us/C | wins_us | z | us_MB/s | C_MB/s | note |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| parse-noout | slashdot.xml | 3675 | 100000 | 6 | 57406.25 | 42078.125 | 10437.5 | 7843.75 | 5.5 | 0/12 | -3.46 | 6.105 | 33.578 | DURATION_SHORT |
| stream-noout | slashdot.xml | 3675 | 100000 | 6 | 61187.5 | 39906.25 | 10578.125 | 6625 | 5.784 | 0/12 | -3.46 | 5.728 | 33.132 | DURATION_SHORT |

Work counts (us --bench-counts, one parse; C reports no census; both arms exit 0 on the timed argv):

- `parse-noout` `slashdot.xml`: bytes=3675 elements=101
- `stream-noout` `slashdot.xml`: bytes=3675 reader_ticks=403

METHOD pinned=yes core=4 (not 0) priority=High interleaved=ABBA metric=CPU-time N=6 pairs=12 inner=--repeat x4 (=100000 iterations) null=C-vs-C floor=10.25% (9/12 z=1.73) work=bytes*repeat+exit=0 + us --bench-counts us_arm=True publish=False oracle_mtime=2026-08-27T04:39:45.3204412Z us_mtime=2026-08-27T12:35:47.7187005Z

C default parse flags are XML_PARSE_COMPACT | XML_PARSE_BIG_LINES plus xmlCtxtReadFile per --repeat (Windows pin has no mmap). us is xml_read_memory / xml_reader_for_memory on one s::read, with XML_PARSE_NONET | XML_PARSE_NO_XXE. That is CLI-vs-CLI, not a kernel A/B.
