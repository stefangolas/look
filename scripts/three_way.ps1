# Three-way transition: baseline(018bd469) -> P1(17ac0f15) -> current(472bfd34).
# Reports which transitions happen at each hop for a given model.
param(
    [string]$Model,
    [string]$BaseDir,
    [string]$P1Dir,
    [string]$CurDir
)

function Get-Map {
    param([string]$Path)
    $map = @{}
    if (-not (Test-Path $Path)) { return $map }
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ($line -match '^FACE\t' -and $line -match 'source_face_id=([^\s\t]+)') {
            $id = $Matches[1]
            $rendered = if ($line -match 'rendered=([01])') { [int]$Matches[1] } else { 0 }
            $kind = if ($line -match 'surface_kind=([a-z0-9_]+)') { $Matches[1] } else { '' }
            $reason = if ($line -match 'reason=(\S+)') { $Matches[1] } else { '' }
            $map[$id] = [pscustomobject]@{ rendered=$rendered; kind=$kind; reason=$reason }
        }
    }
    return $map
}

$base = Get-Map (Join-Path $BaseDir "$Model.ledger.txt")
$p1 = Get-Map (Join-Path $P1Dir "$Model.ledger.txt")
$cur = Get-Map (Join-Path $CurDir "$Model.ledger.txt")
Write-Output "model=$Model base=$($base.Count) p1=$($p1.Count) cur=$($cur.Count)"

$b2pL = @{}; $b2pG = @{}
foreach ($k in $base.Keys) {
    if ($p1.ContainsKey($k)) {
        if ($base[$k].rendered -eq 1 -and $p1[$k].rendered -eq 0) { $b2pL[$k] = $true }
        elseif ($base[$k].rendered -eq 0 -and $p1[$k].rendered -eq 1) { $b2pG[$k] = $true }
    }
}
$p2cL = @{}; $p2cG = @{}
foreach ($k in $p1.Keys) {
    if ($cur.ContainsKey($k)) {
        if ($p1[$k].rendered -eq 1 -and $cur[$k].rendered -eq 0) { $p2cL[$k] = $true }
        elseif ($p1[$k].rendered -eq 0 -and $cur[$k].rendered -eq 1) { $p2cG[$k] = $true }
    }
}
Write-Output "baseline->P1: lost=$($b2pL.Count) gained=$($b2pG.Count)"
Write-Output "P1->current: lost=$($p2cL.Count) gained=$($p2cG.Count)"

Write-Output "--- baseline->P1 lost by kind ---"
$b2pL.Keys | ForEach-Object { $base[$_].kind } | Group-Object | Sort-Object Count -Descending | Select-Object Count, Name | Format-Table -AutoSize
Write-Output "--- baseline->P1 gained by kind ---"
$b2pG.Keys | ForEach-Object { $p1[$_].kind } | Group-Object | Sort-Object Count -Descending | Select-Object Count, Name | Format-Table -AutoSize
Write-Output "--- P1->current lost (detail) ---"
$p2cL.Keys | ForEach-Object { "$_ kind=$($p1[$_].kind) p1=$($p1[$_].reason) cur=$($cur[$_].reason)" }
Write-Output "--- P1->current gained (detail) ---"
$p2cG.Keys | ForEach-Object { "$_ kind=$($p1[$_].kind) p1=$($p1[$_].reason) cur=$($cur[$_].reason)" }
