$files = @(
  'ORI-FRAME-HANDEDNESS-001.md',
  'ORI-FRAME-ORTHONORMALITY-GATE-001.md',
  'SEM-FACET-SCALE-ZERO-001.md',
  'SEM-FACET-CORRESPONDENCE-TRUNCATION-001.md'
)
foreach ($f in $files) {
  $p = Join-Path 'C:\Users\stefa\look\docs\defects' $f
  $t = [System.IO.File]::ReadAllText($p)
  $broken = '```nClosed (CC-DEF-BREP-FIXES, commit 10a1d13)n```'
  $fixed = '```' + "`n" + 'Closed (CC-DEF-BREP-FIXES, commit 10a1d13)' + "`n" + '```'
  $t = $t.Replace($broken, $fixed)
  [System.IO.File]::WriteAllText($p, $t)
}
$p = 'C:\Users\stefa\look\docs\defects\NUM-INTERPOLE-OVERSHOOT-001.md'
$t = [System.IO.File]::ReadAllText($p)
$old = 'Mechanism established (measurements demonstrated; root cause attribution to the no-pivot solve asserted)'
$new = 'Closed for the standing API (CC-DEF-BREP-FIXES, commit 10a1d13: SW admission + BOUND_FACTOR post-check; the certified solve remains CC-001 business)'
$t = $t.Replace($old, $new)
[System.IO.File]::WriteAllText($p, $t)
Get-ChildItem 'C:\Users\stefa\look\docs\defects\*.md' | Select-String -Pattern 'Closed \(' | ForEach-Object {
  Write-Output ($_.Filename + ' :: ' + $_.Line.Trim())
}
