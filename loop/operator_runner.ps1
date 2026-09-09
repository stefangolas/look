# The loop operator runner: spawns a fresh operator agent every 20 minutes,
# kills it at 15 (the timeout), logs everything. Singleton-guarded.
#
# The operator is a CONTROL-FLOW agent per loop/OPERATOR_CHARTER.md.
# Model: glm-5.3-flash. If the provider 402s, the cycle logs it and the
# next cycle retries - balance is human-owned.

$ErrorActionPreference = 'Continue'
$env:PYTHONIOENCODING = 'utf-8'
$env:PATH = "C:\Users\stefa\look\loop\cargoq;C:\Users\stefa\AppData\Local\Python\pythoncore-3.14-64;C:\Program Files\PyManager\runtime;" + $env:PATH
Set-Location C:\Users\stefa\look

$LOG = "C:\Users\stefa\look\loop\operator.log"

function Log([string]$msg) {
    $ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
    Add-Content -LiteralPath $LOG -Value "[$ts] $msg" -ErrorAction Continue
}

# Startup banner proves the runner is alive and writable
Log "RUNNER STARTED (pid $PID) - cycle every 20 min, agent timeout 15 min"

$PIDFILE = "C:\Users\stefa\look\loop\operator_runner.pid"

while ($true) {
    try {
        # Singleton guard via PID file: skip only if a LIVE runner holds it
        $held = if (Test-Path $PIDFILE) { Get-Content $PIDFILE -ErrorAction SilentlyContinue } else { $null }
        if ($held -and ($held -ne "$PID") -and (Get-Process -Id $held -ErrorAction SilentlyContinue)) {
            Log "GUARD: runner pid $held alive - skipping cycle"
            Start-Sleep -Seconds 1200
            continue
        }
        Set-Content -LiteralPath $PIDFILE -Value "$PID"

        Log "CYCLE START - spawning operator agent (deepseek-v4-flash, timeout 15 min)"

        # Spawn the agent via -EncodedCommand (no quoting issues): the child
        # reads the charter from file itself, appends the execute directive,
        # and tees all output to the log. NOTE: glm-5.3-flash's zai route
        # 500s ("Unexpected server error") - deepseek is the operator model.
        $childCmd = "opencode.cmd run -m deepseek/deepseek-v4-flash ((Get-Content 'C:\Users\stefa\look\loop\OPERATOR_CHARTER.md' -Raw) + [char]10 + 'EXECUTE THIS CHARTER NOW. One pass: run the procedure steps 1-7 in order, act strictly within the MAY list, log everything, then exit.') *>> 'C:\Users\stefa\look\loop\operator.log'"
        $b64 = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($childCmd))
        $child = Start-Process -FilePath "powershell.exe" `
            -ArgumentList @('-NoProfile', '-EncodedCommand', $b64) `
            -WorkingDirectory "C:\Users\stefa\look" -WindowStyle Hidden -PassThru

        Log "agent spawned (child pid $($child.Id))"
        $exited = $child.WaitForExit(900000)   # 15 minutes
        if (-not $exited) {
            taskkill /PID $child.Id /T /F 2>&1 | Out-Null
            Log "TIMEOUT: operator killed at 15 min (as designed)"
        } else {
            Log "agent exited code $($child.ExitCode)"
        }
    } catch {
        Log "CYCLE ERROR: $($_.Exception.Message)"
    }

    # Next cycle at the 20-minute mark
    Start-Sleep -Seconds 1200
}
