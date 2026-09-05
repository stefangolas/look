$p = 'C:\Users\stefa\look\docs\defects\NUM-INTERPOLE-OVERSHOOT-001.md'
$t = [System.IO.File]::ReadAllText($p)
$broken = '```nClosed for the standing API (CC-DEF-BREP-FIXES, commit 10a1d13: SW admission + BOUND_FACTOR; certified path remains CC-001''s business)n```n'
$fixed = '```' + "`n" + 'Closed for the standing API (CC-DEF-BREP-FIXES, commit 10a1d13: SW admission + BOUND_FACTOR; certified path remains CC-001 business)' + "`n" + '```'
$t = $t.Replace($broken, $fixed)
[System.IO.File]::WriteAllText($p, $t)
Get-Content $p -TotalCount 14 | Select-Object -Last 4
