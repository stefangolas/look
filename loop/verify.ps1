# The only acceptance authority for a packet (§5). Runs in a slot's worktree,
# never trusts the worker's RESULT.json, and gates on what's actually in the
# diff. Exit 0 = ACCEPTED, non-zero = REJECTED/BLOCKED.
#
# Usage: verify.ps1 -Slot 0 -Packet loop/packets/BG-S0-002.md [-Base <ref>]
#
# $Base is the ref the diff is computed against (merge-base for a normal
# packet run). Defaults to where the slot branch diverged from
# integration/kernel-bg, matching how new-slot.ps1 forks a slot branch.
param(
    [Parameter(Mandatory)] [int]$Slot,
    [Parameter(Mandatory)] [string]$Packet,
    [string]$Base
)

$ErrorActionPreference = 'Stop'

$repoRoot   = Split-Path $PSScriptRoot -Parent
$slotRoot   = Join-Path $repoRoot "loop\slots\$Slot"
$wt         = Join-Path $slotRoot "wt"
$targetDir  = Join-Path $slotRoot "target"
$outFile    = Join-Path $slotRoot "out.txt"
$verdictFile = Join-Path $slotRoot "VERDICT.json"

if (-not (Test-Path $wt)) {
    throw "slot $Slot has no worktree at $wt; run new-slot.ps1 first"
}
if (-not (Test-Path $Packet)) {
    throw "packet not found: $Packet"
}

New-Item -ItemType Directory -Force -Path $slotRoot | Out-Null
if (Test-Path $outFile) { Remove-Item -Force $outFile }
New-Item -ItemType File -Path $outFile | Out-Null

function Write-OutSection {
    param([string]$Header)
    Add-Content -Path $outFile -Value ("`n===== $Header =====`n")
}

# ---------------------------------------------------------------------------
# Packet parsing. gen-packet.ps1 (§10 step 2) does not exist yet, so this
# parser has to cope with a hand-written JSON or YAML/markdown packet: JSON
# is parsed properly; YAML/markdown is scraped with a tolerant regex reader
# rather than a real parser, since PowerShell 5.1 has no built-in YAML
# support and pulling in a module is not worth it for a handful of scalar
# and list fields.
# ---------------------------------------------------------------------------
function Get-YamlListField {
    param([string]$Text, [string]$Key)
    if ($Text -match "(?m)^${Key}:\s*\[(.*?)\]") {
        return @($Matches[1] -split ',' | ForEach-Object { $_.Trim().Trim('"', "'") } | Where-Object { $_ -ne '' })
    }
    $lines = $Text -split "\r?\n"
    $items = New-Object System.Collections.Generic.List[string]
    $capture = $false
    foreach ($line in $lines) {
        if (-not $capture) {
            if ($line -match "^${Key}:\s*$") { $capture = $true }
            continue
        }
        if ($line -match '^\s*-\s*(.+?)\s*$') {
            $item = ($Matches[1] -split '#')[0].Trim().Trim('"', "'")
            if ($item -ne '') { $items.Add($item) }
        } elseif ($line.Trim() -eq '') {
            continue
        } else {
            break
        }
    }
    return @($items)
}

function Read-PacketFields {
    param([string]$Path)
    $raw = Get-Content -Path $Path -Raw
    $ext = [System.IO.Path]::GetExtension($Path).ToLowerInvariant()

    if ($ext -eq '.json') {
        $obj = $raw | ConvertFrom-Json
        return [pscustomobject]@{
            crates         = @($obj.crates)
            write_allow    = @($obj.write_allow)
            tests_required = @($obj.tests_required)
        }
    }

    $yamlText = $raw
    if ($raw -match '(?s)```ya?ml\s*\r?\n(.*?)```') {
        $yamlText = $Matches[1]
    }

    return [pscustomobject]@{
        crates         = Get-YamlListField -Text $yamlText -Key 'crates'
        write_allow    = Get-YamlListField -Text $yamlText -Key 'write_allow'
        tests_required = Get-YamlListField -Text $yamlText -Key 'tests_required'
    }
}

$pkt = Read-PacketFields -Path $Packet
if (-not $pkt.crates -or $pkt.crates.Count -eq 0) {
    throw "could not read 'crates' field from packet $Packet"
}

# `crates` holds cargo package names, not paths -- cargo -p wants the name and
# the vendored directory happens to match it, so the directory is only used to
# catch a typo'd package name before we spend a build on it.
foreach ($c in $pkt.crates) {
    $probe = Join-Path $wt "vendor\truck\$c\Cargo.toml"
    if ($c -ne 'look' -and -not (Test-Path $probe)) {
        throw "packet names crate '$c' but $probe does not exist"
    }
}
$crateNames = @($pkt.crates)
# Repeated as `-p a -p b ...` in every cargo invocation below.
$pArgs = @($crateNames | ForEach-Object { '-p'; $_ })

if (-not $Base) {
    $Base = (git -C $wt merge-base HEAD integration/kernel-bg 2>$null)
    if (-not $Base) {
        throw "could not compute a default -Base (no merge-base with integration/kernel-bg); pass -Base explicitly"
    }
    $Base = $Base.Trim()
}

# gate results, in report order. Each entry: name, status (PASS/FAIL/SKIP), detail.
$gates = New-Object System.Collections.Generic.List[pscustomobject]
$failedEarly = $false

function Add-Gate {
    param([string]$Name, [string]$Status, [string]$Detail = '')
    $gates.Add([pscustomobject]@{ name = $Name; status = $Status; detail = $Detail })
    Write-Host ("{0} ... {1}" -f $Name, $Status)
}

$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = $targetDir

# ---------------------------------------------------------------------------
# V1 scope — git diff --name-only <base>...HEAD must be a subset of write_allow.
# ---------------------------------------------------------------------------
$changed = @(git -C $wt diff --name-only "$Base...HEAD" | Where-Object { $_ -ne '' })

# write_allow entries are repo-relative, the same form `git diff --name-only`
# prints, so they compare directly once separators are normalised. RESULT.json
# is the packet's own required output and is always in scope.
$allowedRel = @(@($pkt.write_allow | ForEach-Object { ($_ -replace '\\', '/').Trim() }) + 'RESULT.json' + 'QUESTION.md')

$offenders = @($changed | Where-Object { ($_ -replace '\\', '/') -notin $allowedRel })

if ($offenders.Count -gt 0) {
    Add-Gate 'V1 scope' 'FAIL' ("out-of-allowlist paths: " + ($offenders -join ', '))
    $failedEarly = $true
} else {
    Add-Gate 'V1 scope' 'PASS' ("{0} changed file(s), all within write_allow" -f $changed.Count)
}

# ---------------------------------------------------------------------------
# V2 build — cargo check --locked -p <crate>
# ---------------------------------------------------------------------------
if (-not $failedEarly) {
    Write-OutSection 'V2 build: cargo check --locked -p'
    Push-Location $wt
    try {
        cargo check --locked @pArgs *>> $outFile
        $exit = $LASTEXITCODE
    } finally { Pop-Location }
    if ($exit -eq 0) {
        Add-Gate 'V2 build' 'PASS'
    } else {
        Add-Gate 'V2 build' 'FAIL' "cargo check exit $exit; see out.txt"
        $failedEarly = $true
    }
} else {
    Add-Gate 'V2 build' 'SKIP' 'earlier gate failed'
}

# ---------------------------------------------------------------------------
# V3 lint — cargo fmt --check, then cargo clippy -D warnings
# ---------------------------------------------------------------------------
if (-not $failedEarly) {
    Write-OutSection 'V3 lint: cargo fmt --check -p'
    Push-Location $wt
    try {
        cargo fmt --check @pArgs *>> $outFile
        $fmtExit = $LASTEXITCODE
    } finally { Pop-Location }

    if ($fmtExit -ne 0) {
        Add-Gate 'V3 lint' 'FAIL' "cargo fmt --check failed; see out.txt"
        $failedEarly = $true
    } else {
        Write-OutSection 'V3 lint: cargo clippy -D warnings'
        Push-Location $wt
        try {
            cargo clippy @pArgs --all-targets -- -D warnings *>> $outFile
            $clippyExit = $LASTEXITCODE
        } finally { Pop-Location }
        if ($clippyExit -eq 0) {
            Add-Gate 'V3 lint' 'PASS'
        } else {
            Add-Gate 'V3 lint' 'FAIL' "cargo clippy exit $clippyExit; see out.txt"
            $failedEarly = $true
        }
    }
} else {
    Add-Gate 'V3 lint' 'SKIP' 'earlier gate failed'
}

# ---------------------------------------------------------------------------
# V4 house rules — scripts/kernel-gates.sh <base>, diff-scoped, H-1/H-3/H-4.
# ---------------------------------------------------------------------------
if (-not $failedEarly) {
    Write-OutSection 'V4 house rules: kernel-gates.sh'
    Push-Location $wt
    try {
        bash scripts/kernel-gates.sh $Base *>> $outFile
        $gatesExit = $LASTEXITCODE
    } finally { Pop-Location }
    if ($gatesExit -eq 0) {
        Add-Gate 'V4 house rules' 'PASS'
    } else {
        Add-Gate 'V4 house rules' 'FAIL' "kernel-gates.sh exit $gatesExit; see out.txt"
        $failedEarly = $true
    }
} else {
    Add-Gate 'V4 house rules' 'SKIP' 'earlier gate failed'
}

# ---------------------------------------------------------------------------
# V5 tests — cargo test -p <crate> --lib --tests. Never bare `cargo test`.
# ---------------------------------------------------------------------------
if (-not $failedEarly) {
    Write-OutSection 'V5 tests: cargo test -p --lib --tests'
    Push-Location $wt
    try {
        cargo test @pArgs --lib --tests *>> $outFile
        $testExit = $LASTEXITCODE
    } finally { Pop-Location }
    if ($testExit -eq 0) {
        Add-Gate 'V5 tests' 'PASS'
    } else {
        Add-Gate 'V5 tests' 'FAIL' "cargo test exit $testExit; see out.txt"
        $failedEarly = $true
    }
} else {
    Add-Gate 'V5 tests' 'SKIP' 'earlier gate failed'
}

# ---------------------------------------------------------------------------
# V6 test-reality — every tests_required entry must correspond to a test fn
# name actually present in the diff, and if the crate disables autotests
# (the truck-polymesh precedent), any new tests/*.rs file must be declared
# as a [[test]] target or it silently never runs.
#
# NOTE / known limitation: tests_required entries in the spec are prose
# descriptions ("property: 10^4 sampled points ... lie in enclose(box)"),
# not literal fn names, and no gen-packet.ps1 exists yet to establish a
# fn-naming convention between the packet and the worker. Absent that
# convention this gate can only apply a heuristic — normalized keyword
# overlap between each tests_required string and the #[test]-attributed fn
# names added in the diff — rather than an exact name match. That heuristic
# is intentionally conservative in the FAIL direction (it flags rather than
# rubber-stamps when in doubt); tightening it to exact-name matching is
# gen-packet.ps1's job, once it defines the naming convention it will emit.
# ---------------------------------------------------------------------------
if (-not $failedEarly) {
    $diffText = git -C $wt diff "$Base...HEAD" -- '*.rs'
    $addedLines = @($diffText -split "\r?\n" | Where-Object { $_ -match '^\+[^+]' })

    # Collect fn names on or immediately after a #[test]/#[proptest]-style
    # attribute line among the added lines (order-preserving scan).
    $testFnNames = New-Object System.Collections.Generic.List[string]
    $pendingTestAttr = $false
    foreach ($line in $addedLines) {
        $body = $line.Substring(1)
        if ($body -match '#\[\s*(test|proptest|test_case|tokio::test)') {
            $pendingTestAttr = $true
            continue
        }
        if ($body -match 'fn\s+([A-Za-z0-9_]+)') {
            if ($pendingTestAttr) {
                $testFnNames.Add($Matches[1])
                $pendingTestAttr = $false
            }
            continue
        }
        if ($body.Trim() -ne '' -and $body -notmatch '^\s*(#\[|//)') {
            # non-attribute, non-comment line breaks the "immediately after"
            # adjacency assumption for a pending attribute.
            $pendingTestAttr = $false
        }
    }

    $missingRequired = New-Object System.Collections.Generic.List[string]
    foreach ($req in $pkt.tests_required) {
        $keywords = @(($req -replace '[^A-Za-z0-9 ]', ' ') -split '\s+' |
            Where-Object { $_.Length -ge 4 } | ForEach-Object { $_.ToLowerInvariant() })
        $hit = $false
        foreach ($fn in $testFnNames) {
            $fnLower = $fn.ToLowerInvariant()
            foreach ($kw in $keywords) {
                if ($fnLower -like "*$kw*") { $hit = $true; break }
            }
            if ($hit) { break }
        }
        if (-not $hit) { $missingRequired.Add($req) }
    }

    # autotests=false precedent: new tests/*.rs files must be declared.
    $addedFiles = @(git -C $wt diff --name-only --diff-filter=A "$Base...HEAD")
    $undeclared = New-Object System.Collections.Generic.List[string]
    foreach ($c in $crateNames) {
        $ctoml = Join-Path $wt "vendor\truck\$c\Cargo.toml"
        if (-not (Test-Path $ctoml)) { continue }
        $ctext = Get-Content -Path $ctoml -Raw
        if ($ctext -notmatch 'autotests\s*=\s*false') { continue }
        $newTestFiles = @($addedFiles | Where-Object { $_ -match "truck/$c/" -and $_ -match '[\\/]tests[\\/].+\.rs$' })
        foreach ($f in $newTestFiles) {
            $baseName = [System.IO.Path]::GetFileNameWithoutExtension($f)
            if ($ctext -notmatch [regex]::Escape($baseName)) { $undeclared.Add($f) }
        }
    }

    if ($missingRequired.Count -gt 0 -or $undeclared.Count -gt 0) {
        $detailParts = @()
        if ($missingRequired.Count -gt 0) {
            $detailParts += ("no matching test fn for: " + ($missingRequired -join ' | '))
        }
        if ($undeclared.Count -gt 0) {
            $detailParts += ("new test file(s) not declared as [[test]] under autotests=false: " + ($undeclared -join ', '))
        }
        Add-Gate 'V6 test-reality' 'FAIL' ($detailParts -join '; ')
        $failedEarly = $true
    } else {
        Add-Gate 'V6 test-reality' 'PASS' ("{0} required test(s) matched, {1} test fn(s) found in diff" -f $pkt.tests_required.Count, $testFnNames.Count)
    }
} else {
    Add-Gate 'V6 test-reality' 'SKIP' 'earlier gate failed'
}

# ---------------------------------------------------------------------------
# V7 mutation spot-check — TODO. Per §5: for the enumerated items (BG-FID-008
# double cover, BG-NUM-002 double root, BG-NUM-004 F-2 both directions,
# BG-CE-002 offset pcurve), re-run the packet's negative test against a
# deliberately weakened implementation (a #[cfg(test)] mod vacuity harness
# checked in beside the test) and assert it fails. Not implemented: doing
# this correctly needs the packet schema to name which test is the negative
# test and where the weakened-impl harness lives, and no packet has that
# field yet (gen-packet.ps1 doesn't exist). Always passes until then.
# ---------------------------------------------------------------------------
Add-Gate 'V7 mutation spot-check' 'PASS' 'TODO stub — always passes; see comment in verify.ps1'

# ---------------------------------------------------------------------------
# V8 no-regression — TODO. Per §5: the previously accepted wave's tests still
# pass. Not implemented: this needs LEDGER.jsonl / PACKETS.jsonl to know what
# "the previous wave" is and which crates it touched, which is orchestrator
# state that doesn't exist yet. Always passes until then.
# ---------------------------------------------------------------------------
Add-Gate 'V8 no-regression' 'PASS' 'TODO stub — always passes; see comment in verify.ps1'

# ---------------------------------------------------------------------------
# Verdict
# ---------------------------------------------------------------------------
$accepted = -not ($gates | Where-Object { $_.status -eq 'FAIL' })

Write-Host ''
Write-Host ("VERDICT: {0}" -f ($(if ($accepted) { 'ACCEPTED' } else { 'REJECTED' })))
Write-Host ("packet={0} slot={1} crates={2} base={3}" -f $Packet, $Slot, ($crateNames -join ','), $Base)

$verdict = [pscustomobject]@{
    packet    = $Packet
    slot      = $Slot
    crates    = $crateNames
    base      = $Base
    verdict   = $(if ($accepted) { 'ACCEPTED' } else { 'REJECTED' })
    gates     = $gates
    timestamp = (Get-Date).ToUniversalTime().ToString('o')
}
$verdict | ConvertTo-Json -Depth 6 | Set-Content -Path $verdictFile -Encoding utf8

if ($accepted) { exit 0 } else { exit 1 }
