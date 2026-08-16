# Create or reset a sticky slot: one git worktree + one CARGO_TARGET_DIR,
# reused across many packets (§1 "sticky slots"). The point of stickiness is
# that the ~240-crate dependency build is paid once per slot, not once per
# packet, so this script's job is to get a slot from "doesn't exist" or
# "left dirty by a previous packet" to "clean checkout of $Branch, warm
# target dir" in one idempotent call.
#
# Usage: new-slot.ps1 -Slot 0 -Branch packet/BG-S0-002
param(
    [Parameter(Mandatory)] [int]$Slot,
    [Parameter(Mandatory)] [string]$Branch,
    [double]$MinFreeGB = 8
)

$ErrorActionPreference = 'Stop'

$repoRoot  = Split-Path $PSScriptRoot -Parent
$slotRoot  = Join-Path $repoRoot "loop\slots\$Slot"
$wt        = Join-Path $slotRoot "wt"
$targetDir = Join-Path $slotRoot "target"

# §7 rule 5: refuse, don't flag. A slot warm build is the single largest
# single write burst in the loop (target/quick-scale, ~2.5 GB), so check the
# floor before touching disk at all rather than failing partway through.
$freeGB = (Get-PSDrive C).Free / 1GB
if ($freeGB -lt $MinFreeGB) {
    Write-Error ("new-slot: {0:N1} GB free on C:, below the {1:N1} GB floor. " -f $freeGB, $MinFreeGB `
        + "Run the janitor (see docs §7.2) before creating or warming a slot.")
    exit 1
}

# The branch a fresh or reset slot forks from. In normal use this is whatever
# branch the orchestrator itself is on (integration/kernel-bg); parameterizing
# it as "current HEAD of the repo you invoked this from" keeps the script
# usable from a detached-HEAD CI checkout too.
$baseRef = (git -C $repoRoot rev-parse --abbrev-ref HEAD).Trim()
if (-not $baseRef -or $baseRef -eq 'HEAD') {
    $baseRef = (git -C $repoRoot rev-parse HEAD).Trim()
}

New-Item -ItemType Directory -Force -Path $slotRoot | Out-Null

$isExistingWorktree = $false
if (Test-Path $wt) {
    # Confirm it's actually a live git worktree and not just a stray
    # directory (e.g. left behind by a manual rm that missed .git/worktrees).
    git -C $wt rev-parse --is-inside-work-tree *> $null
    if ($LASTEXITCODE -eq 0) {
        $isExistingWorktree = $true
    }
}

if ($isExistingWorktree) {
    # Idempotent path: reuse the worktree and target dir, just repoint the
    # branch. -B creates the branch if it doesn't exist and force-resets it
    # to $baseRef if it does — this is the "reset rather than error" contract.
    Write-Host "Slot $Slot worktree exists at $wt; resetting branch $Branch to $baseRef"
    git -C $wt checkout -B $Branch $baseRef
    if ($LASTEXITCODE -ne 0) { throw "git checkout -B $Branch failed in $wt" }
    git -C $wt reset --hard $baseRef
    if ($LASTEXITCODE -ne 0) { throw "git reset --hard $baseRef failed in $wt" }
    # Drop whatever the previous packet left uncommitted, but never the
    # target dir — it lives outside $wt (CARGO_TARGET_DIR points elsewhere)
    # so it is never at risk here; -e is defensive in case that ever changes.
    git -C $wt clean -fdx -e target
} else {
    if (Test-Path $wt) {
        throw "loop/slots/$Slot/wt exists but is not a git worktree; remove it manually before retrying."
    }
    $branchExists = [bool](git -C $repoRoot branch --list $Branch)
    if ($branchExists) {
        Write-Host "Branch $Branch already exists; attaching worktree and resetting to $baseRef"
        git -C $repoRoot worktree add $wt $Branch
        if ($LASTEXITCODE -ne 0) { throw "git worktree add $wt $Branch failed" }
        git -C $wt reset --hard $baseRef
    } else {
        Write-Host "Creating worktree $wt on new branch $Branch from $baseRef"
        git -C $repoRoot worktree add -b $Branch $wt $baseRef
        if ($LASTEXITCODE -ne 0) { throw "git worktree add -b $Branch $wt $baseRef failed" }
    }
}

New-Item -ItemType Directory -Force -Path $targetDir | Out-Null

# CARGO_INCREMENTAL=0 per §7 rule 1 — incremental state buys nothing in a
# one-packet-per-process world and was 389 MB dead weight in the last audit.
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = $targetDir

Write-Host "Warming slot $Slot ($wt), target=$targetDir ..."
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Push-Location $wt
try {
    cargo check --workspace --all-targets
    $exitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
$sw.Stop()

if ($exitCode -ne 0) {
    Write-Error "warm build failed (cargo check --workspace --all-targets exit $exitCode) in slot $Slot"
    exit $exitCode
}

$targetSizeGB = (Get-ChildItem $targetDir -Recurse -Force -ErrorAction SilentlyContinue |
    Measure-Object -Property Length -Sum).Sum / 1GB

Write-Host ("Warm build: {0:N1} min" -f $sw.Elapsed.TotalMinutes)
Write-Host ("Target dir size: {0:N2} GB" -f $targetSizeGB)
Write-Host ("Free disk after warm: {0:N1} GB" -f ((Get-PSDrive C).Free / 1GB))
