# C-only baseline board (M0)

Pinned oracle: oracle/PIN.

| workload | file | bytes | repeat | N | cpu_med_ms | cpu_min_ms | null_med_ms | ns/byte | MB/s |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| parse-noout | slashdot.xml | 3675 | 10000 | 3 | 1281.25 | 1250 | 1265.625 | 34.864 | 27.354 |
| parse-noout | title.xml | 63 | 10000 | 3 | 593.75 | 562.5 | 593.75 | 942.46 | 1.012 |
| stream-noout | slashdot.xml | 3675 | 10000 | 3 | 1109.375 | 1078.125 | 1125 | 30.187 | 31.592 |
| stream-noout | title.xml | 63 | 10000 | 3 | 500 | 484.375 | 484.375 | 793.651 | 1.202 |

METHOD pinned=yes core=4 (not 0) priority=High interleaved=ABBA metric=CPU-time N=3 inner=xmllint --repeat x3 (=10000 iterations) null=C-vs-C work=bytes*repeat+exit=0 us_arm=False
