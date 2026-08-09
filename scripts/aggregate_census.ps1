# Aggregate face_census census.txt files from a directory.
param(
    [string]$Dir
)

$declared = 0; $rendered = 0; $lost = 0
$rejInt = 0; $rejAmb = 0; $fail = 0
$files = Get-ChildItem "$Dir" -Filter *.census.txt | Sort-Object Name
foreach ($f in $files) {
    $first = Get-Content $f.FullName | Where-Object { $_ -match 'faces declared' } | Select-Object -First 1
    if (-not $first) { continue }
    if ($first -match '(\d+) faces declared, (\d+) rendered, (\d+) lost') {
        $declared += [int]$matches[1]; $rendered += [int]$matches[2]; $lost += [int]$matches[3]
    }
    $rej = Get-Content $f.FullName | Where-Object { $_ -match 'rejected_intrinsic=' } | Select-Object -First 1
    if ($rej -match 'rejected_intrinsic=(\d+) rejected_ambiguous=(\d+) failed_renderable_or_unknown=(\d+)') {
        $rejInt += [int]$matches[1]; $rejAmb += [int]$matches[2]; $fail += [int]$matches[3]
    }
}
Write-Output "models=$($files.Count)"
Write-Output "declared=$declared"
Write-Output "rendered=$rendered"
Write-Output "lost=$lost"
Write-Output "rejected_intrinsic=$rejInt"
Write-Output "rejected_ambiguous=$rejAmb"
Write-Output "failed_renderable_or_unknown=$fail"
Write-Output "sum check: rendered+lost=$($rendered+$lost) (should equal declared)"
Write-Output "bucket check: rejInt+rejAmb+fail=$($rejInt+$rejAmb+$fail) (should equal lost)"
