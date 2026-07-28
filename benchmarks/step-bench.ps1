param([string]$Look, [string]$Nist, [int]$Iterations = 5)

# Fresh-process timing for STEP: every launch pays parse, tessellation, GPU
# init, and render, which is what a user actually experiences from a CLI.
$models = @(
    @{ name = "ctc_01 (AP203)";   file = "nist_ctc_01_asme1_rd.stp" },
    @{ name = "ctc_03 (AP242)";   file = "nist_ctc_03_asme1_ap242-e2.stp" },
    @{ name = "ftc_06 (AP203)";   file = "nist_ftc_06_asme1_rd.stp" },
    @{ name = "ftc_09 (AP242)";   file = "nist_ftc_09_asme1_ap242-e1.stp" },
    @{ name = "ctc_02 (AP242)";   file = "nist_ctc_02_asme1_ap242-e2.stp" },
    @{ name = "stc_09 (AP242)";   file = "nist_stc_09_asme1_ap242-e3.stp" }
)

$out = Join-Path $env:TEMP "step-bench-out.png"
"{0,-18} {1,8} {2,10} {3,10} {4,10} {5,9}" -f "model", "MB", "median", "min", "max", "tris"

foreach ($model in $models) {
    $path = (Get-ChildItem $Nist -Recurse -Filter $model.file | Select-Object -First 1)
    if (-not $path) { "{0,-18} not found" -f $model.name; continue }

    # One unmeasured launch so the file cache and GPU driver are warm.
    & $Look render $path.FullName --resolution 512x512 --output $out > $null 2>&1

    $samples = @()
    $triangles = 0
    for ($i = 0; $i -lt $Iterations; $i++) {
        $clock = [Diagnostics.Stopwatch]::StartNew()
        $json = & $Look render $path.FullName --resolution 512x512 --output $out --json 2>$null
        $clock.Stop()
        $samples += $clock.Elapsed.TotalMilliseconds
        if ($triangles -eq 0 -and $json) {
            $parsed = ($json -join "`n") | ConvertFrom-Json
            $triangles = $parsed.scene.statistics.triangles
        }
    }
    $sorted = $samples | Sort-Object
    "{0,-18} {1,8:N2} {2,10:N1} {3,10:N1} {4,10:N1} {5,9}" -f `
        $model.name, ($path.Length / 1MB), $sorted[[int]($sorted.Count / 2)], $sorted[0],
        $sorted[-1], $triangles
}
