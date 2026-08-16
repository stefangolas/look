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
    [int]$StallMinutes = 12,
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

# The packet is copied into the worktree and the prompt points at it, rather
# than being passed as the prompt itself (S2). A packet is ~9 KB and the
# launcher is a .cmd shim, whose command line dies at 8191 characters with
# "The command line is too long" -- which arrives as an empty event stream, not
# as an error the worker can report. Handing over a file also leaves the slot
# holding an exact record of what its worker was given.
#
# The worker still reads only this one file: not PACKETS.jsonl, not the build
# spec (S3a).
$packetCopy = Join-Path $wt 'PACKET.md'
Copy-Item -Path $Packet -Destination $packetCopy -Force
$packetText = "Read the file PACKET.md in the root of this repository and carry out the work packet it describes, exactly and completely. It is self-contained: do not read any other specification file. Follow its stop conditions, and finish by writing RESULT.json as it instructs."

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
# that the ledger records.
#
# Launched detached with the stream redirected to a file rather than piped,
# because a pipe gives us no handle to kill: BG-S0-002's first run stopped
# emitting events mid-step and sat there for 45 minutes on a hung API call,
# holding a slot and its write set while producing nothing. Growth of the event
# log is the liveness signal -- an idle worker and a working one look identical
# from CPU time, since both are mostly waiting on the model.
if (Test-Path $eventsLog) { Remove-Item -Force $eventsLog }
$errLog = Join-Path $slotRoot "worker.err"

# `opencode` on this machine is an npm shim (opencode.ps1 / opencode.cmd), not
# an exe, so Start-Process cannot launch it directly -- it reports "%1 is not a
# valid Win32 application". Prefer the .cmd shim, which is a real process
# Windows can start and whose exit code propagates.
$launcher = (Get-Command 'opencode.cmd' -ErrorAction SilentlyContinue).Source
if (-not $launcher) {
    $launcher = (Get-Command 'opencode.exe' -ErrorAction SilentlyContinue).Source
}
if (-not $launcher) {
    throw "cannot find an opencode.cmd or opencode.exe to launch (Get-Command opencode resolves to a .ps1 shim, which Start-Process cannot run)"
}

$proc = Start-Process -FilePath $launcher -PassThru -WindowStyle Hidden `
    -ArgumentList @('run', '--dir', $wt, '-m', $Model, '--format', 'json', '--auto', $packetText) `
    -RedirectStandardOutput $eventsLog -RedirectStandardError $errLog

# Fire and forget. A worker runs for tens of minutes; anything that waits on it
# is a long-lived process of its own, and when that waiter was killed it took
# the worker with it mid-run. The orchestrator polls slot-status.ps1 instead --
# short calls, no parent to lose -- and that is also where the stall check
# lives now.
Set-Content -Path (Join-Path $slotRoot 'worker.pid') -Value $proc.Id -Encoding ascii
Set-Content -Path (Join-Path $slotRoot 'worker.packet') -Value $Packet -Encoding ascii
Write-Host ("started pid {0}; poll with loop\slot-status.ps1" -f $proc.Id)
exit 0
