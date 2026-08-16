# Dispatch one packet to a warm slot's worker (S2: opencode run, deepseek by
# default). One packet, one process, one context reset - the architecture's
# isolation unit, not agent discipline.
#
# This script only launches the worker and captures its event stream; it does
# not judge the result. verify.ps1 is the only acceptance authority (S5).
#
# Usage: run-packet.ps1 -Slot 0 -Packet loop/packets/BG-S0-002.md [-Model ...] [-DryRun]
param(
    [Parameter(Mandatory)] [int]$Slot,
    [Parameter(Mandatory)] [string]$Packet,
    [string]$Model = 'deepseek/deepseek-v4-flash',
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

$repoRoot  = Split-Path $PSScriptRoot -Parent
$slotRoot  = Join-Path $repoRoot "loop\slots\$Slot"
$wt        = Join-Path $slotRoot "wt"
$targetDir = Join-Path $slotRoot "target"
$eventsLog = Join-Path $slotRoot "events.jsonl"

if (-not (Test-Path $wt)) {
    throw "slot $Slot has no worktree at $wt; run new-slot.ps1 -Slot $Slot -Branch NAME first"
}
if (-not (Test-Path $Packet)) {
    throw "packet not found: $Packet"
}

# The packet is passed as the literal prompt text, per S2 - the worker never
# reads PACKETS.jsonl or the build spec, only this one file (S3a).
$packetText = Get-Content -Path $Packet -Raw

if ($DryRun) {
    Write-Host "DRY RUN -- would execute:"
    Write-Host ("  opencode run --dir `"{0}`" -m {1} --format json --auto (contents of {2})" -f $wt, $Model, $Packet)
    Write-Host ("  (CARGO_TARGET_DIR={0}, CARGO_INCREMENTAL=0)" -f $targetDir)
    Write-Host ("  event stream teed to: {0}" -f $eventsLog)
    exit 0
}

# CARGO_INCREMENTAL=0 / CARGO_TARGET_DIR so any cargo invocations the worker
# makes land in this slot's sticky target dir, per S7 rule 1/2.
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_TARGET_DIR = $targetDir

Write-Host ("Running packet '{0}' in slot {1} (model={2})..." -f $Packet, $Slot, $Model)

# --format json gives the machine-readable event stream the orchestrator's
# S3(b) context-budget supervisor reads (cumulative tokens, turn count) and
# that the ledger records. Tee rather than redirect so a human watching the
# console still sees it live.
opencode run --dir $wt -m $Model --format json --auto $packetText 2>&1 |
    Tee-Object -FilePath $eventsLog

exit $LASTEXITCODE
