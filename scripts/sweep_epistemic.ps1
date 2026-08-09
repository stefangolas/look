# Run spline_edge_epistemic_compact over all baseline->current rendered->lost faces
# and aggregate edge/face classifications by model and surface family.
param(
    [string]$BaseDir = "C:\Users\stefa\look-corpus\planar-c\abc",
    [string]$CurDir = "C:\Users\stefa\look-corpus\final-472",
    [string]$AbcRoot = "C:\Users\stefa\look-corpus\abc",
    [string]$Bin = "C:\Users\stefa\look\target\release\examples\spline_edge_epistemic_compact.exe"
)

# 1. Compute the rendered->lost face id per model from ledgers.
$base = @{}; $cur = @{}
foreach ($f in Get-ChildItem $BaseDir -Filter *.ledger.txt) {
    $baseName = $f.BaseName
    foreach ($line in [System.IO.File]::ReadLines($f.FullName)) {
        if ($line -match '^FACE\t' -and $line -match 'source_face_id=([^\s\t]+)') {
            $id = $Matches[1]
            $rendered = if ($line -match 'rendered=([01])') { [int]$Matches[1] } else { 0 }
            $base[("${baseName}:${id}")] = $rendered
        }
    }
}
foreach ($f in Get-ChildItem $CurDir -Filter *.ledger.txt) {
    $baseName = $f.BaseName
    foreach ($line in [System.IO.File]::ReadLines($f.FullName)) {
        if ($line -match '^FACE\t' -and $line -match 'source_face_id=([^\s\t]+)') {
            $id = $Matches[1]
            $rendered = if ($line -match 'rendered=([01])') { [int]$Matches[1] } else { 0 }
            $cur[("${baseName}:${id}")] = $rendered
        }
    }
}
$lostByModel = @{}
foreach ($k in $base.Keys) {
    if ($cur.ContainsKey($k) -and $base[$k] -eq 1 -and $cur[$k] -eq 0) {
        $model = ($k -split ':')[0] -replace '\.ledger$', ''
        $face = ($k -split ':')[1] -replace '^#', ''
        if (-not $lostByModel.ContainsKey($model)) { $lostByModel[$model] = New-Object System.Collections.ArrayList }
        [void]$lostByModel[$model].Add([int]$face)
    }
}

$edgeClasses = @{}
$faceClassCount = @{}
$global:outLines = New-Object System.Collections.ArrayList

foreach ($model in ($lostByModel.Keys | Sort-Object)) {
    $ids = $lostByModel[$model]
    $step = Get-ChildItem "$AbcRoot\$model" -Filter *.step | Select-Object -First 1
    if (-not $step) { Write-Output "no step for $model"; continue }
    # chunk ids to avoid oversized command lines
    $chunks = @()
    for ($i = 0; $i -lt $ids.Count; $i += 30) {
        $chunk = $ids[$i..([Math]::Min($i + 29, $ids.Count - 1))] -join ','
        $chunks += $chunk
    }
    $tmp = "C:\Users\stefa\AppData\Local\Temp\opencode\epi_sweep_$model.txt"
    "" | Set-Content $tmp
    foreach ($chunk in $chunks) {
        & $Bin $step.FullName --faces $chunk 2>&1 | Add-Content $tmp
    }
    $lines = Get-Content $tmp
    foreach ($l in $lines) {
        if ($l -match '^FACE\t') {
            if ($l -match 'family=([^\t]+)\tbounds=(\d+)') {
                $fam = $matches[1]
                $b = $matches[2]
                if (-not $faceClassCount.ContainsKey("$model`t$fam")) { $faceClassCount["$model`t$fam"] = @{} }
                $facesWithB = if ($faceClassCount["$model`t$fam"].ContainsKey("faces")) { $faceClassCount["$model`t$fam"]["faces"] } else { 0 }
                $faceClassCount["$model`t$fam"]["faces"] = $facesWithB + 1
            }
        } elseif ($l -match '^EDGE\t') {
            if ($l -match 'class=([A-Za-z-]+)') {
                $cls = $matches[1]
                $modelOf = if ($l -match 'face=(\d+)') { $model } else { $model }
                $key = "$model`t$cls"
                if (-not $edgeClasses.ContainsKey($key)) { $edgeClasses[$key] = 0 }
                $edgeClasses[$key]++
            }
        }
    }
    [void]$global:outLines.Add("MODEL $model lost=$($ids.Count)")
}

Write-Output "=== EDGE CLASSIFICATION by model ==="
$edgeClasses.Keys | Sort-Object | ForEach-Object {
    Write-Output "$_`t$($edgeClasses[$_])"
}
Write-Output "=== FACE counts by model/family ==="
$faceClassCount.Keys | Sort-Object | ForEach-Object {
    Write-Output "$_`tfaces=$($faceClassCount[$_]['faces'])"
}
