# Full per-face transition analysis between two ledger dirs (ABC corpus).
param(
    [string]$BaseDir,
    [string]$CurDir
)

$base = @{}; $cur = @{}
foreach ($f in Get-ChildItem $BaseDir -Filter *.ledger.txt) {
    $baseName = $f.BaseName
    foreach ($line in [System.IO.File]::ReadLines($f.FullName)) {
        if ($line -match '^FACE\t' -and $line -match 'source_face_id=([^\s\t]+)') {
            $id = $Matches[1]
            $rendered = if ($line -match 'rendered=([01])') { [int]$Matches[1] } else { 0 }
            $kind = if ($line -match 'surface_kind=([a-z0-9_]+)') { $Matches[1] } else { '' }
            $reason = if ($line -match 'reason=(\S+)') { $Matches[1] } else { '' }
            $base[("${baseName}:${id}")] = [pscustomobject]@{ rendered=$rendered; kind=$kind; reason=$reason }
        }
    }
}
foreach ($f in Get-ChildItem $CurDir -Filter *.ledger.txt) {
    $baseName = $f.BaseName
    foreach ($line in [System.IO.File]::ReadLines($f.FullName)) {
        if ($line -match '^FACE\t' -and $line -match 'source_face_id=([^\s\t]+)') {
            $id = $Matches[1]
            $rendered = if ($line -match 'rendered=([01])') { [int]$Matches[1] } else { 0 }
            $kind = if ($line -match 'surface_kind=([a-z0-9_]+)') { $Matches[1] } else { '' }
            $reason = if ($line -match 'reason=(\S+)') { $Matches[1] } else { '' }
            $cur[("${baseName}:${id}")] = [pscustomobject]@{ rendered=$rendered; kind=$kind; reason=$reason }
        }
    }
}
Write-Output "base faces=$($base.Count) cur faces=$($cur.Count)"

$lost = @{}; $gained = @{}
foreach ($k in $base.Keys) {
    if ($cur.ContainsKey($k)) {
        if ($base[$k].rendered -eq 1 -and $cur[$k].rendered -eq 0) { $lost[$k] = $true }
        elseif ($base[$k].rendered -eq 0 -and $cur[$k].rendered -eq 1) { $gained[$k] = $true }
    }
}
Write-Output "rendered->lost: $($lost.Count)"
Write-Output "lost->rendered: $($gained.Count)"

Write-Output "=== lost by surface kind ==="
$lost.Keys | ForEach-Object { $base[$_].kind } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize

Write-Output "=== lost by current reason ==="
$lost.Keys | ForEach-Object { $cur[$_].reason } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize

Write-Output "=== lost by old reason ==="
$lost.Keys | ForEach-Object { $base[$_].reason } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize

Write-Output "=== gained by surface kind ==="
$gained.Keys | ForEach-Object { $cur[$_].kind } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize

Write-Output "=== lost by model ==="
$lost.Keys | ForEach-Object { ($_ -split ':')[0] } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize

Write-Output "=== gained by model ==="
$gained.Keys | ForEach-Object { ($_ -split ':')[0] } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize
