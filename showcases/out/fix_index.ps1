$p = 'C:\Users\stefa\look\docs\defects\DEFECT_INDEX.md'
$t = [System.IO.File]::ReadAllText($p)
$pairs = @(
  @('| [ORI-FRAME-HANDEDNESS-001](ORI-FRAME-HANDEDNESS-001.md) | ArchitecturalUp frame law returns a left-handed frame | Mechanism established |',
    '| [ORI-FRAME-HANDEDNESS-001](ORI-FRAME-HANDEDNESS-001.md) | ArchitecturalUp frame law returns a left-handed frame | Closed (10a1d13) |'),
  @('| [ORI-FRAME-ORTHONORMALITY-GATE-001](ORI-FRAME-ORTHONORMALITY-GATE-001.md) | FrameLaw dispatch bypasses Frame3''s orthonormality gate | Mechanism established |',
    '| [ORI-FRAME-ORTHONORMALITY-GATE-001](ORI-FRAME-ORTHONORMALITY-GATE-001.md) | FrameLaw dispatch bypasses Frame3''s orthonormality gate | Closed (10a1d13) |'),
  @('| [SEM-FACET-SCALE-ZERO-001](SEM-FACET-SCALE-ZERO-001.md) | Facet backend accepts a through-zero Scale law the BREP backend refuses | Mechanism established |',
    '| [SEM-FACET-SCALE-ZERO-001](SEM-FACET-SCALE-ZERO-001.md) | Facet backend accepts a through-zero Scale law the BREP backend refuses | Closed (10a1d13) |'),
  @('| [SEM-FACET-CORRESPONDENCE-TRUNCATION-001](SEM-FACET-CORRESPONDENCE-TRUNCATION-001.md) | Facet backend silently zip-truncates a mismatched LinearCorrespondence | Mechanism established |',
    '| [SEM-FACET-CORRESPONDENCE-TRUNCATION-001](SEM-FACET-CORRESPONDENCE-TRUNCATION-001.md) | Facet backend silently zip-truncates a mismatched LinearCorrespondence | Closed (10a1d13) |'),
  @('| [NUM-INTERPOLE-OVERSHOOT-001](NUM-INTERPOLE-OVERSHOOT-001.md) | try_interpole produces catastrophically oscillating interpolants at moderate data counts | Mechanism established (root-cause attribution asserted) |',
    '| [NUM-INTERPOLE-OVERSHOOT-001](NUM-INTERPOLE-OVERSHOOT-001.md) | try_interpole produces catastrophically oscillating interpolants at moderate data counts | Closed for the standing API (10a1d13); certified solve = CC-001 |')
)
foreach ($pair in $pairs) {
  $t = $t.Replace($pair[0], $pair[1])
}
[System.IO.File]::WriteAllText($p, $t)
Get-ChildItem 'C:\Users\stefa\look\docs\defects\DEFECT_INDEX.md' | Select-String -Pattern '10a1d13' | ForEach-Object {
  Write-Output ($_.LineNumber.ToString() + ': ' + $_.Line.Substring(0, [Math]::Min(110, $_.Line.Length)))
}
