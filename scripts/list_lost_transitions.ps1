# For each rendered->lost face, report surface kind + current reason + model.
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

# current-lost faces that were baseline-rendered, by kind+reason+model
$lost = @{}
foreach ($k in $base.Keys) {
    if ($cur.ContainsKey($k) -and $base[$k].rendered -eq 1 -and $cur[$k].rendered -eq 0) { $lost[$k] = $true }
}
$lost.Keys | ForEach-Object {
    $model = ($_ -split ':')[0]
    $face = ($_ -split ':')[1]
    $b = $base[$_]; $c = $cur[$_]
    "{0} | # {1,-8} | kind={2,-9} | old_reason={3,-24} | new_reason={4}" -f $model, $face, $b.kind, $b.reason, $c.reason
} | Sort-Object
