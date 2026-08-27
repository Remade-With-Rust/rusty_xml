# Fetch and build the pinned libxml2 xmllint oracle. Never linked into rusty_xml.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$Src = Join-Path $Root "oracle\src"
$Build = Join-Path $Root "oracle\build"
$Bin = Join-Path $Root "oracle\bin"
$Tag = "v2.15.3"
$ExpectSha = "c94eb0210183b9d7cb43f8e7fddc6be55843ef49"

New-Item -ItemType Directory -Force -Path (Join-Path $Root "oracle") | Out-Null

if (-not (Test-Path (Join-Path $Src ".git"))) {
    git clone --depth 1 --branch $Tag https://github.com/GNOME/libxml2.git $Src
}

Push-Location $Src
try {
    $have = (git rev-parse HEAD).Trim()
    if ($have -ne $ExpectSha) {
        Write-Warning "oracle SHA $have != pin $ExpectSha (tag $Tag)"
    }
} finally {
    Pop-Location
}

New-Item -ItemType Directory -Force -Path $Build | Out-Null
cmake -S $Src -B $Build -G "Visual Studio 17 2022" -A x64 `
    -DBUILD_SHARED_LIBS=OFF `
    -DLIBXML2_WITH_ICONV=OFF `
    -DLIBXML2_WITH_ZLIB=OFF `
    -DLIBXML2_WITH_PYTHON=OFF `
    -DLIBXML2_WITH_HTTP=OFF `
    -DLIBXML2_WITH_TESTS=OFF `
    -DLIBXML2_WITH_PROGRAMS=ON `
    -DLIBXML2_WITH_MODULES=OFF
if ($LASTEXITCODE -ne 0) { throw "cmake configure failed" }

cmake --build $Build --config Release --target xmllint
if ($LASTEXITCODE -ne 0) { throw "cmake build failed" }

New-Item -ItemType Directory -Force -Path $Bin | Out-Null
$built = Join-Path $Build "Release\xmllint.exe"
if (-not (Test-Path $built)) {
    $built = Join-Path $Build "xmllint.exe"
}
Copy-Item $built (Join-Path $Bin "xmllint.exe") -Force
& (Join-Path $Bin "xmllint.exe") --version
