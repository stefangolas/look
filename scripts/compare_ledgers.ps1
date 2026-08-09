# Compare two 00007705 face_census ledgers by source_face_id.
param(
    [string]$Base,
    [string]$Cur
)

$script:baseMap = @{}
$script:curMap = @{}

foreach ($line in Get-Content $Base) {
    if ($line -match '^FACE\t' -and $line -match 'source_face_id=([^\s\t]+)') {
        $id = $Matches[1]
        $script:baseMap[$id] = $line
    }
}
foreach ($line in Get-Content $Cur) {
    if ($line -match '^FACE\t' -and $line -match 'source_face_id=([^\s\t]+)') {
        $id = $Matches[1]
        $script:curMap[$id] = $line
    }
}

Write-Output "baseline faces: $($script:baseMap.Count), current faces: $($script:curMap.Count)"

function Get-Field {
    param([string]$Line, [string]$Name)
    if ($Line -match "$Name=([^\s\t]+)") { return $Matches[1] }
    return ''
}

$lost = New-Object System.Collections.ArrayList
$gained = New-Object System.Collections.ArrayList
foreach ($k in $script:baseMap.Keys) {
    if ($script:curMap.ContainsKey($k)) {
        $br = (Get-Field $script:baseMap[$k] 'rendered')
        $cr = (Get-Field $script:curMap[$k] 'rendered')
        if ($br -eq '1' -and $cr -eq '0') { [void]$lost.Add($k) }
        elseif ($br -eq '0' -and $cr -eq '1') { [void]$gained.Add($k) }
    }
}
Write-Output "rendered->lost: $($lost.Count)"
Write-Output "lost->rendered: $($gained.Count)"

Write-Output "--- lost by kind (baseline) ---"
$lost | ForEach-Object { (Get-Field $script:baseMap[$_] 'surface_kind') } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize

Write-Output "--- gained by kind (current) ---"
$gained | ForEach-Object { (Get-Field $script:curMap[$_] 'surface_kind') } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize

Write-Output "--- lost by reason (current) ---"
$lost | ForEach-Object { (Get-Field $script:curMap[$_] 'reason') } | Group-Object | Sort-Object Count -Descending |
    Select-Object Count, Name | Format-Table -AutoSize

$lostIds = ($lost | Sort-Object) -join ','
$gainedIds = ($gained | Sort-Object) -join ','
Write-Output "LOST_IDS=$lostIds"
Write-Output "GAINED_IDS=$gainedIds"
