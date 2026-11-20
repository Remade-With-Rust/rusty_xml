rusty_xml
Copyright 2026 Mata Network

This crate reimplements the algorithms of libxml2 (https://gitlab.gnome.org/GNOME/libxml2),
which is MIT licensed. The C sources are neither distributed with nor linked into this
crate. A pinned `xmllint` binary is used only as an external-process oracle for tests
and benches (`scripts/fetch-oracle.ps1`, recorded in `oracle/PIN`).
