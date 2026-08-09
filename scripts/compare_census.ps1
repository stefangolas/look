# Per-model rendered/lost comparison across two census dirs.
param(
    [string]$BaseDir,
    [string]$CurDir
)

$base = @{}; $cur = @{}
foreach ($f in Get-ChildItem $BaseDir -Filter *.census.txt) {
    $first = Get-Content $f.FullName | Where-Object { $_ -match 'faces declared' } | Select-Object -First 1
    if ($first -match '(\d+) faces declared, (\d+) rendered, (\d+) lost') {
        $base[$f.BaseName] = [pscustomobject]@{ declared=[int]$matches[1]; rendered=[int]$matches[2]; lost=[int]$matches[3] }
    }
}
foreach ($f in Get-ChildItem $CurDir -Filter *.census.txt) {
    $first = Get-Content $f.FullName | Where-Object { $_ -match 'faces declared' } | Select-Object -First 1
    if ($first -match '(\d+) faces declared, (\d+) rendered, (\d+) lost') {
        $cur[$f.BaseName] = [pscustomobject]@{ declared=[int]$matches[1]; rendered=[int]$matches[2]; lost=[int]$matches[3] }
    }
}
Write-Output ("model               baseDecl baseRend baseLost | curDecl curRend curLost | dRend  dLost")
foreach ($k in ($base.Keys | Sort-Object)) {
    if (-not $cur.ContainsKey($k)) { continue }
    $b = $base[$k]; $c = $cur[$k]
    $dR = $c.rendered - $b.rendered; $dL = $c.lost - $b.lost
    Write-Output ("{0}  {1,7} {2,8} {3,8} | {4,7} {5,8} {6,8} | {7,6} {8,6}" -f $k, $b.declared, $b.rendered, $b.lost, $c.declared, $c.rendered, $c.lost, $dR, $dL)
}
