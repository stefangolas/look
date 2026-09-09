# The loop operator runner: spawns a fresh operator agent every 20 minutes,
# kills it at 15 (the timeout), logs everything. Singleton-guarded.
#
# The operator is a CONTROL-FLOW agent per loop/OPERATOR_CHARTER.md.
# Model: glm-5.3-flash (zai). If the provider 402s, the cycle logs it and
# the next cycle retries - balance is human-owned.

$ErrorActionPreference = 'Continue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$env:PYTHONIOENCODING = 'utf-8'
$env:PATH = "C:\Users\stefa\look\loop\cargoq;C:\Users\stefa\AppData\Local\Python\pythoncore-3.14-64;C:\Program Files\PyManager\runtime;" + $env:PATH
Set-Location C:\Users\stefa\look

$charter = Get-Content loop\OPERATOR_CHARTER.md -Raw

while ($true) {
    $ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'

    # Singleton guard: never spawn if an operator agent is somehow still alive
    $alive = Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" |
        Where-Object { $_.CommandLine -match 'operator_runner' -and $_.ProcessId -ne $PID }
    if ($alive) {
        Add-Content loop\operator.log "[$ts] GUARD: another runner instance alive ($($alive.ProcessId -join ',')) - skipping cycle"
        Start-Sleep -Seconds 1200
        continue
    }

    Add-Content loop\operator.log "`n========== OPERATOR CYCLE $ts =========="

    # Spawn the agent with the charter, hard timeout 15 minutes
    $proc = Start-Process -FilePath "cmd.exe" -ArgumentList "/c opencode run -m glm-5.3-flash `"$charter`" >> loop\operator.log 2>&1" `
        -WorkingDirectory "C:\Users\stefa\look" -WindowStyle Hidden -PassThru
    $exited = $proc.WaitForExit(900000)   # 15 minutes
    if (-not $exited) {
        taskkill /PID $proc.Id /T /F 2>&1 | Out-Null
        Add-Content loop\operator.log "[$(Get-Date -Format 'HH:mm:ss')] TIMEOUT: operator killed at 15 min (as designed)"
    } else {
        Add-Content loop\operator.log "[$(Get-Date -Format 'HH:mm:ss')] operator exited code $($proc.ExitCode)"
    }

    # Next cycle at the 20-minute mark
    Start-Sleep -Seconds 1200
}
