# us vs C (libxml2) session board

Pinned oracle: oracle/PIN. Not a publish claim (N=8; publish is N>=20).

Null arm C-vs-C on parse-noout slashdot.xml: medA=734.375ms medB=703.125ms floor=4.44% wins=3/12 z=-1.73

| workload | file | bytes | inner | N | us_med_ms | us_min_ms | C_med_ms | C_min_ms | us/C | wins_us | z | us_MB/s | C_MB/s | note |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| parse-noout | slashdot.xml | 3675 | 10000 | 8 | 4156.25 | 3890.625 | 765.625 | 656.25 | 5.429 | 0/16 | -4 | 8.432 | 45.776 | DURATION_SHORT |
| parse-noout | android-lite.xml | 41 | 10000 | 8 | 31.25 | 15.625 | 406.25 | 359.375 | 0.077 | 16/16 | 4 | 12.512 | 0.962 | DURATION_SHORT |
| parse-noout | svg-lite.xml | 119 | 10000 | 8 | 109.375 | 78.125 | 421.875 | 390.625 | 0.259 | 16/16 | 4 | 10.376 | 2.69 | DURATION_SHORT |
| parse-noout | atom-lite.xml | 217 | 10000 | 8 | 125 | 109.375 | 484.375 | 390.625 | 0.258 | 16/16 | 4 | 16.556 | 4.272 | DURATION_SHORT |
| parse-noout | title.xml | 63 | 10000 | 8 | 31.25 | 15.625 | 406.25 | 375 | 0.077 | 16/16 | 4 | 19.226 | 1.479 | DURATION_SHORT |
| stream-noout | slashdot.xml | 3675 | 10000 | 8 | 4718.75 | 4187.5 | 796.875 | 609.375 | 5.922 | 0/16 | -4 | 7.427 | 43.981 | DURATION_SHORT |
| stream-noout | android-lite.xml | 41 | 10000 | 8 | 31.25 | 15.625 | 359.375 | 312.5 | 0.087 | 16/16 | 4 | 12.512 | 1.088 | DURATION_SHORT |
| stream-noout | svg-lite.xml | 119 | 10000 | 8 | 156.25 | 125 | 515.625 | 468.75 | 0.303 | 16/16 | 4 | 7.263 | 2.201 | DURATION_SHORT |
| stream-noout | atom-lite.xml | 217 | 10000 | 8 | 156.25 | 125 | 531.25 | 406.25 | 0.294 | 16/16 | 4 | 13.245 | 3.895 | DURATION_SHORT |
| stream-noout | title.xml | 63 | 10000 | 8 | 31.25 | 31.25 | 421.875 | 375 | 0.074 | 16/16 | 4 | 19.226 | 1.424 | DURATION_SHORT |

Work counts (us --bench-counts, one parse; C reports no census; both arms exit 0 on the timed argv):

- `parse-noout` `slashdot.xml`: bytes=3675 elements=101
- `parse-noout` `android-lite.xml`: bytes=41 elements=1
- `parse-noout` `svg-lite.xml`: bytes=119 elements=2
- `parse-noout` `atom-lite.xml`: bytes=217 elements=4
- `parse-noout` `title.xml`: bytes=63 elements=1
- `stream-noout` `slashdot.xml`: bytes=3675 reader_ticks=403
- `stream-noout` `android-lite.xml`: bytes=41 reader_ticks=1
- `stream-noout` `svg-lite.xml`: bytes=119 reader_ticks=5
- `stream-noout` `atom-lite.xml`: bytes=217 reader_ticks=15
- `stream-noout` `title.xml`: bytes=63 reader_ticks=3

METHOD pinned=yes core=4 (not 0) priority=High interleaved=ABBA metric=CPU-time N=8 pairs=16 inner=--repeat x3 (=10000 iterations) null=C-vs-C floor=4.44% (3/12 z=-1.73) work=bytes*repeat+exit=0 + us --bench-counts us_arm=True publish=False oracle_mtime=2026-08-27T04:39:45.3204412Z us_mtime=2026-08-27T15:36:18.5218781Z

C default parse flags are XML_PARSE_COMPACT | XML_PARSE_BIG_LINES plus xmlCtxtReadFile per --repeat (Windows pin has no mmap). us is xml_read_memory / xml_reader_for_memory on one s::read, with XML_PARSE_NONET | XML_PARSE_NO_XXE. That is CLI-vs-CLI, not a kernel A/B.
