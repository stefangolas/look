# One short call that says what every slot is doing. The orchestrator polls
# this instead of waiting on a worker: a packet runs for tens of minutes, and
# any process that waits that long is itself something that can be killed --
# when one was, it took its worker down mid-run.
#
# Liveness is the growth of the event log, not CPU time: a worker waiting on the
# model and a worker whose API call will never return look identical from the
# outside. -StallMinutes without a byte means hung.
#
# Usage: slot-status.ps1 [-StallMinutes 12] [-KillStalled]
param(
    [int]$StallMinutes = 12,
    [switch]$KillStalled
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
$slotsDir = Join-Path $repoRoot 'loop\slots'

if (-not (Test-Path $slotsDir)) {
    Write-Host "no slots yet"
    exit 0
}

foreach ($slot in (Get-ChildItem $slotsDir -Directory | Sort-Object Name)) {
    $eventsLog = Join-Path $slot.FullName 'events.jsonl'
    $pidFile   = Join-Path $slot.FullName 'worker.pid'
    $pktFile   = Join-Path $slot.FullName 'worker.packet'
    $wt        = Join-Path $slot.FullName 'wt'

    $packet = if (Test-Path $pktFile) { (Get-Content $pktFile -Raw).Trim() } else { '-' }

    $workerPid = $null
    if (Test-Path $pidFile) { $workerPid = [int](Get-Content $pidFile -Raw).Trim() }
    $alive = $false
    if ($workerPid) {
        $alive = [bool](Get-Process -Id $workerPid -ErrorAction SilentlyContinue)
    }

    $size = 0; $ageMin = $null
    if (Test-Path $eventsLog) {
        $item = Get-Item $eventsLog
        $size = $item.Length
        $ageMin = [math]::Round(((Get-Date) - $item.LastWriteTime).TotalMinutes, 1)
    }

    $result = if (Test-Path (Join-Path $wt 'RESULT.json')) { 'RESULT.json' }
              elseif (Test-Path (Join-Path $wt 'QUESTION.md')) { 'QUESTION.md' }
              else { '-' }

    $dirty = 0
    if (Test-Path $wt) {
        $dirty = @(git -C $wt status --porcelain | Where-Object { $_ -ne '' }).Count
    }

    $state = if ($alive -and $ageMin -ne $null -and $ageMin -gt $StallMinutes) { 'STALLED' }
             elseif ($alive) { 'RUNNING' }
             elseif ($result -ne '-') { 'FINISHED' }
             else { 'IDLE' }

    Write-Host ("slot {0,-3} {1,-9} packet={2,-28} pid={3,-7} events={4,8} bytes, {5} min old  changed={6}  {7}" -f `
        $slot.Name, $state, (Split-Path $packet -Leaf), $(if ($alive) { $workerPid } else { '-' }), $size, $ageMin, $dirty, $result)

    if ($state -eq 'STALLED' -and $KillStalled) {
        Stop-Process -Id $workerPid -Force -ErrorAction SilentlyContinue
        Write-Host ("  killed pid {0} after {1} min of silence" -f $workerPid, $ageMin)
    }
}
