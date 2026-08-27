# codec-measurement harness: pinned xmllint (C) vs rxmlint (us).
# Never links libxml2. Default is C-only (M0 board). Use -Both for side-by-side.
#
# Method line is printed every run. Numbers without it are not evidence.

[CmdletBinding()]
param(
    [int] $N = 6,
    [int] $RepeatFlags = 3,
    [switch] $Publish,
    [switch] $Both,
    [switch] $COnly,
    [string] $Core = "4",
    [int] $MinBytes = 0,
    [string[]] $Workloads = @("parse-noout", "stream-noout")
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
if ($Publish) { $N = 20 }

$Oracle = $env:RUSTY_XML_ORACLE
if (-not $Oracle) {
    $Oracle = Join-Path $Root "oracle\bin\xmllint.exe"
}
if (-not (Test-Path $Oracle)) {
    throw "Pinned oracle missing: $Oracle (run scripts/fetch-oracle.ps1)"
}

$Us = Join-Path $Root "target\release\rxmlint.exe"
$RunUs = $Both -and -not $COnly
if ($RunUs -and -not (Test-Path $Us)) {
    throw "rxmlint missing: $Us (cargo build -p rusty_xml-cli --release)"
}

$Files = @(
    (Join-Path $Root "corpora\slashdot.xml"),
    (Join-Path $Root "corpora\android-lite.xml"),
    (Join-Path $Root "corpora\svg-lite.xml"),
    (Join-Path $Root "corpora\atom-lite.xml"),
    (Join-Path $Root "corpora\title.xml")
) | Where-Object { (Test-Path $_) -and ((Get-Item $_).Length -ge $MinBytes) }
$Files = @($Files)

if (-not $Files) {
    throw "no corpora files matched MinBytes=$MinBytes"
}

function Get-CpuMs([string] $Exe, [string[]] $Argv) {
    $outFile = Join-Path $env:TEMP ("rusty_xml-bench-out-" + [guid]::NewGuid().ToString() + ".txt")
    $errFile = Join-Path $env:TEMP ("rusty_xml-bench-err-" + [guid]::NewGuid().ToString() + ".txt")
    $p = Start-Process -FilePath $Exe -ArgumentList $Argv -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $outFile -RedirectStandardError $errFile
    $null = $p.Handle
    try { $p.ProcessorAffinity = [IntPtr]([int]$Core) } catch { }
    try { $p.PriorityClass = "High" } catch { }
    $p.WaitForExit()
    $errTxt = ""
    if (Test-Path $errFile) {
        $errTxt = Get-Content -Raw $errFile -ErrorAction SilentlyContinue
    }
    Remove-Item $outFile, $errFile -ErrorAction SilentlyContinue
    return @{
        CpuMs = $p.TotalProcessorTime.TotalMilliseconds
        Exit  = $p.ExitCode
        Err   = $errTxt
    }
}

function Median([double[]] $xs) {
    $s = $xs | Sort-Object
    $s[[int]([math]::Floor(($s.Count - 1) / 2))]
}

function Pair-Z([int] $wins, [int] $nPairs) {
    if ($nPairs -le 0) { return 0 }
    ($wins - ($nPairs / 2.0)) / (0.5 * [math]::Sqrt($nPairs))
}

function Get-Argv([string] $wl, [string] $file, [string[]] $repeatArgs) {
    $base = @("--noout") + $repeatArgs
    if ($wl -eq "stream-noout") { $base = @("--stream", "--noout") + $repeatArgs }
    elseif ($wl -eq "sax-noout") { $base = @("--sax", "--noout") + $repeatArgs }
    return $base + @($file)
}

# xmllint --repeat is a flag: first sets 100, each extra multiplies by 10.
$repeatArgs = @()
for ($k = 0; $k -lt $RepeatFlags; $k++) { $repeatArgs += "--repeat" }
$inner = 100
for ($k = 1; $k -lt $RepeatFlags; $k++) { $inner *= 10 }

$oracleItem = Get-Item $Oracle
Write-Host "oracle: $Oracle"
Write-Host ("oracle mtime={0:o} bytes={1}" -f $oracleItem.LastWriteTimeUtc, $oracleItem.Length)
& $Oracle --version
Write-Host ""

if ($RunUs) {
    $usItem = Get-Item $Us
    Write-Host "us: $Us"
    Write-Host ("us mtime={0:o} bytes={1}" -f $usItem.LastWriteTimeUtc, $usItem.Length)
    $usBytes = [System.IO.File]::ReadAllBytes($Us)
    $usAscii = [System.Text.Encoding]::ASCII.GetString($usBytes)
    if ($usAscii -notmatch "rxmlint-repeat-flag-v1") {
        throw "rxmlint is stale (missing --repeat marker rxmlint-repeat-flag-v1). Rebuild: cargo build -p rusty_xml-cli --release"
    }
    Write-Host "us --repeat marker: rxmlint-repeat-flag-v1 (fresh)"
    Write-Host ""
}

$nl = [Environment]::NewLine
$pipe = [char]124
$outDir = Join-Path $Root "bench"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

if ($RunUs) {
    Write-Host "=== rusty_xml us vs C (session) ==="
    Write-Host ""

    Write-Host "-- work counts (us --bench-counts, single parse; C has no census) --"
    $work = @{}
    foreach ($file in $Files) {
        $leaf = Split-Path $file -Leaf
        $cParse = Get-CpuMs $Us @("--bench-counts", "--noout", $file)
        $cStream = Get-CpuMs $Us @("--stream", "--bench-counts", "--noout", $file)
        $cSax = Get-CpuMs $Us @("--sax", "--bench-counts", "--noout", $file)
        if ($cParse.Exit -ne 0) { throw "us parse-noout failed on $leaf : $($cParse.Err)" }
        $work[$leaf] = @{
            parse  = ($cParse.Err | Out-String).Trim()
            stream = ($cStream.Err | Out-String).Trim()
            sax    = ($cSax.Err | Out-String).Trim()
        }
        Write-Host ("  {0} parse:  {1}" -f $leaf, $work[$leaf].parse)
        Write-Host ("  {0} stream: {1}" -f $leaf, $work[$leaf].stream)
        Write-Host ("  {0} sax:    {1}" -f $leaf, $work[$leaf].sax)
    }
    Write-Host ""

    # Null arm this session: C vs C on the first file / first workload.
    $nullFile = $Files[0]
    $nullWl = $Workloads[0]
    $nullArgv = Get-Argv $nullWl $nullFile $repeatArgs
    $nullA = New-Object System.Collections.Generic.List[double]
    $nullB = New-Object System.Collections.Generic.List[double]
    $nullWins = 0
    $nullPairs = 0
    $leadingNew = $true
    $nullN = [math]::Min($N, 6)
    Write-Host ("-- null arm C vs C  wl={0} file={1} N={2} --" -f $nullWl, (Split-Path $nullFile -Leaf), $nullN)
    for ($i = 0; $i -lt $nullN; $i++) {
        if ($leadingNew) {
            $a = Get-CpuMs $Oracle $nullArgv
            $b = Get-CpuMs $Oracle $nullArgv
            $b2 = Get-CpuMs $Oracle $nullArgv
            $a2 = Get-CpuMs $Oracle $nullArgv
        } else {
            $b = Get-CpuMs $Oracle $nullArgv
            $a = Get-CpuMs $Oracle $nullArgv
            $a2 = Get-CpuMs $Oracle $nullArgv
            $b2 = Get-CpuMs $Oracle $nullArgv
        }
        foreach ($pair in @(@($a, $b), @($a2, $b2))) {
            $nullA.Add($pair[0].CpuMs)
            $nullB.Add($pair[1].CpuMs)
            $nullPairs++
            if ($pair[0].CpuMs -lt $pair[1].CpuMs) { $nullWins++ }
        }
        $leadingNew = -not $leadingNew
    }
    $nullMedA = [double](Median $nullA.ToArray())
    $nullMedB = [double](Median $nullB.ToArray())
    $nullFloorPct = 0
    if ($nullMedB -gt 0) { $nullFloorPct = [math]::Abs($nullMedA - $nullMedB) / $nullMedB * 100.0 }
    $nullZ = Pair-Z $nullWins $nullPairs
    Write-Host ("null C-vs-C medA={0:N3}ms medB={1:N3}ms floor={2:N2}% wins={3}/{4} z={5:N2}" -f `
        $nullMedA, $nullMedB, $nullFloorPct, $nullWins, $nullPairs, $nullZ)
    Write-Host ""

    $results = @()
    foreach ($wl in $Workloads) {
        foreach ($file in $Files) {
            $bytes = (Get-Item $file).Length
            $argv = Get-Argv $wl $file $repeatArgs
            $samplesUs = New-Object System.Collections.Generic.List[double]
            $samplesC = New-Object System.Collections.Generic.List[double]
            $exits = New-Object System.Collections.Generic.List[int]
            $winsUs = 0
            $nPairs = 0
            $leadingNew = $true
            for ($i = 0; $i -lt $N; $i++) {
                if ($leadingNew) {
                    $u = Get-CpuMs $Us $argv
                    $c = Get-CpuMs $Oracle $argv
                    $c2 = Get-CpuMs $Oracle $argv
                    $u2 = Get-CpuMs $Us $argv
                } else {
                    $c = Get-CpuMs $Oracle $argv
                    $u = Get-CpuMs $Us $argv
                    $u2 = Get-CpuMs $Us $argv
                    $c2 = Get-CpuMs $Oracle $argv
                }
                foreach ($pair in @(@($u, $c), @($u2, $c2))) {
                    if ($pair[0].Exit -ne 0 -or $pair[1].Exit -ne 0) {
                        throw "non-zero exit on $wl $(Split-Path $file -Leaf) us=$($pair[0].Exit) c=$($pair[1].Exit) userr=$($pair[0].Err) cerr=$($pair[1].Err)"
                    }
                    $samplesUs.Add($pair[0].CpuMs)
                    $samplesC.Add($pair[1].CpuMs)
                    $nPairs++
                    if ($pair[0].CpuMs -lt $pair[1].CpuMs) { $winsUs++ }
                    $exits.Add($pair[0].Exit)
                    $exits.Add($pair[1].Exit)
                }
                $leadingNew = -not $leadingNew
            }
            $medUs = [double](Median $samplesUs.ToArray())
            $medC = [double](Median $samplesC.ToArray())
            $minUs = ($samplesUs | Measure-Object -Minimum).Minimum
            $minC = ($samplesC | Measure-Object -Minimum).Minimum
            $z = Pair-Z $winsUs $nPairs
            $ratio = 0
            if ($medC -gt 0) { $ratio = $medUs / $medC }
            $durWarn = ""
            $faster = [math]::Min($medUs, $medC)
            if ($faster -lt 15000) {
                $durWarn = "DURATION_SHORT"
            }
            $nsUs = 0; $nsC = 0; $mbsUs = 0; $mbsC = 0
            if ($medUs -ge 1.0) {
                $nsUs = ($medUs * 1e6) / ([double]$bytes * $inner)
                $mbsUs = ($bytes * $inner / 1MB) / ($medUs / 1000.0)
            }
            if ($medC -ge 1.0) {
                $nsC = ($medC * 1e6) / ([double]$bytes * $inner)
                $mbsC = ($bytes * $inner / 1MB) / ($medC / 1000.0)
            }
            $leaf = Split-Path $file -Leaf
            $workLine = ""
            if ($wl -eq "stream-noout") { $workLine = $work[$leaf].stream }
            elseif ($wl -eq "sax-noout") { $workLine = $work[$leaf].sax }
            else { $workLine = $work[$leaf].parse }
            $row = [pscustomobject]@{
                workload    = $wl
                file        = $leaf
                bytes       = $bytes
                repeat      = $inner
                n           = $N
                npairs      = $nPairs
                us_med_ms   = [math]::Round($medUs, 3)
                us_min_ms   = [math]::Round($minUs, 3)
                c_med_ms    = [math]::Round($medC, 3)
                c_min_ms    = [math]::Round($minC, 3)
                us_over_c   = [math]::Round($ratio, 3)
                wins_us     = $winsUs
                z           = [math]::Round($z, 2)
                us_ns_byte  = [math]::Round($nsUs, 3)
                c_ns_byte   = [math]::Round($nsC, 3)
                us_MBs      = [math]::Round($mbsUs, 3)
                c_MBs       = [math]::Round($mbsC, 3)
                work        = $workLine
                note        = $durWarn
            }
            $results += $row
            Write-Host ("{0,-14} {1,-18} bytes={2} inner={3} us_med={4}ms min={5}ms  C_med={6}ms min={7}ms  us/C={8}  wins(us)={9}/{10} z={11}  us={12} MB/s C={13} MB/s  {14}  work={15}" -f `
                $row.workload, $row.file, $row.bytes, $row.repeat, $row.us_med_ms, $row.us_min_ms, $row.c_med_ms, $row.c_min_ms, $row.us_over_c, $row.wins_us, $row.npairs, $row.z, $row.us_MBs, $row.c_MBs, $row.note, $row.work)
        }
    }

    $method = ("METHOD pinned=yes core={0} (not 0) priority=High interleaved=ABBA metric=CPU-time N={1} pairs={2} inner=--repeat x{3} (={4} iterations) null=C-vs-C floor={5:N2}% ({6}/{7} z={8:N2}) work=bytes*repeat+exit=0 + us --bench-counts us_arm=True publish={9} oracle_mtime={10:o} us_mtime={11:o}" -f `
        $Core, $N, ($N * 2), $RepeatFlags, $inner, $nullFloorPct, $nullWins, $nullPairs, $nullZ, [bool]$Publish, $oracleItem.LastWriteTimeUtc, $usItem.LastWriteTimeUtc)
    Write-Host ""
    Write-Host $method

    $board = Join-Path $Root "bench\SIDE-BY-SIDE.md"
    $parts = New-Object System.Collections.Generic.List[string]
    $parts.Add("# us vs C (libxml2) session board")
    $parts.Add("")
    $parts.Add("Pinned oracle: oracle/PIN. Not a publish claim (N=$N; publish is N>=20).")
    $parts.Add("")
    $parts.Add(("Null arm C-vs-C on {0} {1}: medA={2}ms medB={3}ms floor={4:N2}% wins={5}/{6} z={7:N2}" -f $nullWl, (Split-Path $nullFile -Leaf), ([math]::Round($nullMedA,3)), ([math]::Round($nullMedB,3)), $nullFloorPct, $nullWins, $nullPairs, [math]::Round($nullZ,2)))
    $parts.Add("")
    $col = @("workload", "file", "bytes", "inner", "N", "us_med_ms", "us_min_ms", "C_med_ms", "C_min_ms", "us/C", "wins_us", "z", "us_MB/s", "C_MB/s", "note")
    $parts.Add(($pipe.ToString() + " " + ($col -join (" " + $pipe + " ")) + " " + $pipe))
    $align = @("---", "---", "---:", "---:", "---:", "---:", "---:", "---:", "---:", "---:", "---:", "---:", "---:", "---:", "---")
    $parts.Add(($pipe.ToString() + ($align -join $pipe) + $pipe))
    foreach ($r in $results) {
        $vals = @($r.workload, $r.file, $r.bytes, $r.repeat, $r.n, $r.us_med_ms, $r.us_min_ms, $r.c_med_ms, $r.c_min_ms, $r.us_over_c, ("{0}/{1}" -f $r.wins_us, $r.npairs), $r.z, $r.us_MBs, $r.c_MBs, $r.note)
        $parts.Add(($pipe.ToString() + " " + ($vals -join (" " + $pipe + " ")) + " " + $pipe))
    }
    $parts.Add("")
    $parts.Add("Work counts (us `--bench-counts`, one parse; C reports no census; both arms exit 0 on the timed argv):")
    $parts.Add("")
    foreach ($r in $results) {
        $parts.Add(("- ``{0}`` ``{1}``: {2}" -f $r.workload, $r.file, $r.work))
    }
    $parts.Add("")
    $parts.Add($method)
    $parts.Add("")
    $parts.Add("C default parse flags are `XML_PARSE_COMPACT | XML_PARSE_BIG_LINES` plus `xmlCtxtReadFile` per `--repeat` (Windows pin has no mmap). us is `xml_read_memory` / `xml_reader_for_memory` on one `fs::read`, with `XML_PARSE_NONET | XML_PARSE_NO_XXE`. That is CLI-vs-CLI, not a kernel A/B.")
    [System.IO.File]::WriteAllText($board, [string]::Join($nl, $parts.ToArray()) + $nl)
    Write-Host "wrote $board"
    return
}

Write-Host "=== rusty_xml M0 C-only board ==="
Write-Host ""

$results = @()
foreach ($wl in $Workloads) {
    foreach ($file in $Files) {
        $bytes = (Get-Item $file).Length
        $argv = Get-Argv $wl $file $repeatArgs
        $samplesA = New-Object System.Collections.Generic.List[double]
        $samplesB = New-Object System.Collections.Generic.List[double]
        $exits = New-Object System.Collections.Generic.List[int]
        $leadingNew = $true
        for ($i = 0; $i -lt $N; $i++) {
            if ($leadingNew) {
                $a = Get-CpuMs $Oracle $argv
                $b = Get-CpuMs $Oracle $argv
                $b2 = Get-CpuMs $Oracle $argv
                $a2 = Get-CpuMs $Oracle $argv
                $samplesA.Add($a.CpuMs); $samplesA.Add($a2.CpuMs)
                $samplesB.Add($b.CpuMs); $samplesB.Add($b2.CpuMs)
                $exits.Add($a.Exit); $exits.Add($b.Exit); $exits.Add($b2.Exit); $exits.Add($a2.Exit)
            } else {
                $b = Get-CpuMs $Oracle $argv
                $a = Get-CpuMs $Oracle $argv
                $a2 = Get-CpuMs $Oracle $argv
                $b2 = Get-CpuMs $Oracle $argv
                $samplesA.Add($a.CpuMs); $samplesA.Add($a2.CpuMs)
                $samplesB.Add($b.CpuMs); $samplesB.Add($b2.CpuMs)
                $exits.Add($b.Exit); $exits.Add($a.Exit); $exits.Add($a2.Exit); $exits.Add($b2.Exit)
            }
            $leadingNew = -not $leadingNew
        }
        $badExit = $exits | Where-Object { $_ -ne 0 }
        if ($badExit) { throw "non-zero exit on $wl ${file}: $badExit" }
        $medA = [double](Median $samplesA.ToArray())
        $medB = [double](Median $samplesB.ToArray())
        $minA = ($samplesA | Measure-Object -Minimum).Minimum
        if ($medA -lt 1.0) {
            Write-Host "BELOW_TIMER_FLOOR $wl $(Split-Path $file -Leaf) cpu_med=$medA ms (Windows clock ~15.6ms). Increase -RepeatFlags."
            $nsPerByteA = 0
            $mbs = 0
        } else {
            $nsPerByteA = ($medA * 1e6) / ([double]$bytes * $inner)
            $mbs = ($bytes * $inner / 1MB) / ($medA / 1000.0)
        }
        $row = [pscustomobject]@{
            workload   = $wl
            file       = Split-Path $file -Leaf
            bytes      = $bytes
            repeat     = $inner
            n          = $N
            cpu_ms_med = [math]::Round($medA, 3)
            cpu_ms_min = [math]::Round($minA, 3)
            null_med   = [math]::Round($medB, 3)
            ns_per_byte= [math]::Round($nsPerByteA, 3)
            MB_s       = [math]::Round($mbs, 3)
        }
        $results += $row
        Write-Host ("{0,-14} {1,-16} bytes={2} repeat={3} cpu_med={4}ms min={5}ms null_med={6}ms {7} ns/byte {8} MB/s" -f `
            $row.workload, $row.file, $row.bytes, $row.repeat, $row.cpu_ms_med, $row.cpu_ms_min, $row.null_med, $row.ns_per_byte, $row.MB_s)
    }
}

Write-Host ""
Write-Host ("METHOD pinned=yes core={0} (not 0) priority=High interleaved=ABBA metric=CPU-time N={1} inner=xmllint --repeat x{2} (={3} iterations) null=C-vs-C work=bytes*repeat+exit=0 us_arm={4} publish={5}" -f `
    $Core, $N, $RepeatFlags, $inner, $RunUs, [bool]$Publish)

$board = Join-Path $Root "bench\C-ONLY-BOARD.md"
$parts = New-Object System.Collections.Generic.List[string]
$parts.Add("# C-only baseline board (M0)")
$parts.Add("")
$parts.Add("Pinned oracle: oracle/PIN.")
$parts.Add("")
$col = @("workload", "file", "bytes", "repeat", "N", "cpu_med_ms", "cpu_min_ms", "null_med_ms", "ns/byte", "MB/s")
$parts.Add(($pipe.ToString() + " " + ($col -join (" " + $pipe + " ")) + " " + $pipe))
$align = @("---", "---", "---:", "---:", "---:", "---:", "---:", "---:", "---:", "---:")
$parts.Add(($pipe.ToString() + ($align -join $pipe) + $pipe))
foreach ($r in $results) {
    $vals = @($r.workload, $r.file, $r.bytes, $r.repeat, $r.n, $r.cpu_ms_med, $r.cpu_ms_min, $r.null_med, $r.ns_per_byte, $r.MB_s)
    $parts.Add(($pipe.ToString() + " " + ($vals -join (" " + $pipe + " ")) + " " + $pipe))
}
$parts.Add("")
$parts.Add(("METHOD pinned=yes core=" + $Core + " (not 0) priority=High interleaved=ABBA metric=CPU-time N=" + $N + " inner=xmllint --repeat x" + $RepeatFlags + " (=" + $inner + " iterations) null=C-vs-C work=bytes*repeat+exit=0 us_arm=" + $RunUs))
[System.IO.File]::WriteAllText($board, [string]::Join($nl, $parts.ToArray()) + $nl)
Write-Host "wrote $board"
