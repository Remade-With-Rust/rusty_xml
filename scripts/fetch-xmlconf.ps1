# Fetch the W3C XML Conformance Test Suite. Never vendored into the repo.
#
# 2593 cases that measure well-formedness and validity against the spec rather
# than against our own corpus. Run it with:
#
#     cargo run --release -p rusty_xml-bench --bin xmlconf -- --oracle
#
# --oracle also runs every case through the pinned libxml2 build, because a
# pass rate on its own means nothing: libxml2 does not score 100% either, and
# what matters is the difference.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$Dest = Join-Path $Root "oracle\xmlconf"
$Archive = Join-Path $env:TEMP "xmlts20130923.tar.gz"

# The 2013-09-23 drop is the last published revision of the suite.
$Url = "https://www.w3.org/XML/Test/xmlts20130923.tar.gz"

if (Test-Path (Join-Path $Dest "xmlconf\xmlconf.xml")) {
    Write-Host "xmlconf already present at $Dest"
    exit 0
}

New-Item -ItemType Directory -Force -Path $Dest | Out-Null
Write-Host "downloading $Url"
Invoke-WebRequest -Uri $Url -OutFile $Archive

# tar ships with Windows 10 1803 and later.
Write-Host "extracting to $Dest"
tar xzf $Archive -C $Dest
if ($LASTEXITCODE -ne 0) { throw "tar failed with $LASTEXITCODE" }

$catalog = Join-Path $Dest "xmlconf\xmlconf.xml"
if (-not (Test-Path $catalog)) { throw "extraction did not produce $catalog" }

Write-Host "xmlconf ready: $catalog"
