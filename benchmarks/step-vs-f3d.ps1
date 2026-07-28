param(
    [string]$Look,
    [string]$F3D = 'C:\Program Files\F3D\bin\f3d-console.exe',
    [string]$Nist,
    [int]$Iterations = 5
)

# Fresh-process STEP render, look against F3D. F3D reads STEP through its
# bundled OpenCASCADE plugin, so this compares whole pipelines: parse,
# tessellate, and render, as a user experiences them from a CLI.
$models = @(
    "nist_ctc_01_asme1_rd.stp",
    "nist_ftc_06_asme1_rd.stp",
    "nist_ctc_03_asme1_ap242-e2.stp",
    "nist_ctc_02_asme1_ap242-e2.stp",
    "nist_ftc_09_asme1_ap242-e1.stp",
    "nist_stc_09_asme1_ap242-e3.stp"
)

# Invoked directly rather than through Start-Process: passing pre-quoted paths
# in an argument array hands the quotes to the process as literal characters.
function Measure-Look([string]$Exe, [string]$Model, [string]$Out, [int]$Count) {
    $samples = @()
    for ($i = 0; $i -lt $Count; $i++) {
        if (Test-Path -LiteralPath $Out) { Remove-Item -LiteralPath $Out -Force }
        $clock = [Diagnostics.Stopwatch]::StartNew()
        & $Exe render $Model --view front --camera orthographic --resolution 512x512 `
            --preset f3d-match --background "#252525" --output $Out > $null 2>&1
        $code = $LASTEXITCODE
        $clock.Stop()
        if ($code -ne 0 -or -not (Test-Path -LiteralPath $Out)) { return $null }
        $samples += $clock.Elapsed.TotalMilliseconds
    }
    ($samples | Sort-Object)[[int]($samples.Count / 2)]
}

function Measure-F3D([string]$Exe, [string]$Model, [string]$Out, [int]$Count) {
    $samples = @()
    for ($i = 0; $i -lt $Count; $i++) {
        if (Test-Path -LiteralPath $Out) { Remove-Item -LiteralPath $Out -Force }
        $clock = [Diagnostics.Stopwatch]::StartNew()
        & $Exe $Model --no-config --force-reader=STEP --output $Out --resolution 512,512 `
            "--camera-direction=-Z" --camera-orthographic --anti-aliasing=none `
            --ambient-occlusion=0 --tone-mapping=0 --background-color "#252525" > $null 2>&1
        $code = $LASTEXITCODE
        $clock.Stop()
        if ($code -ne 0 -or -not (Test-Path -LiteralPath $Out)) { return $null }
        $samples += $clock.Elapsed.TotalMilliseconds
    }
    ($samples | Sort-Object)[[int]($samples.Count / 2)]
}

$lookOut = Join-Path $env:TEMP "svf-look.png"
$f3dOut = Join-Path $env:TEMP "svf-f3d.png"

"{0,-24} {1,7} {2,11} {3,11} {4,9}" -f "model", "MB", "look", "F3D 3.5", "F3D/look"
foreach ($name in $models) {
    $file = Get-ChildItem $Nist -Recurse -Filter $name | Select-Object -First 1
    if (-not $file) { continue }
    $short = $name.Replace("nist_", "").Replace("_asme1", "").Replace(".stp", "")

    # One unmeasured launch each so caches and drivers are warm.
    Measure-Look $Look $file.FullName $lookOut 1 | Out-Null
    Measure-F3D $F3D $file.FullName $f3dOut 1 | Out-Null

    $lookMs = Measure-Look $Look $file.FullName $lookOut $Iterations
    $f3dMs = Measure-F3D $F3D $file.FullName $f3dOut $Iterations

    if ($null -eq $lookMs) { "{0,-24} look FAILED" -f $short; continue }
    if ($null -eq $f3dMs) {
        "{0,-24} {1,7:N2} {2,11:N1} {3,11} {4,9}" -f $short, ($file.Length / 1MB), $lookMs, "FAILED", "-"
        continue
    }
    "{0,-24} {1,7:N2} {2,11:N1} {3,11:N1} {4,8:N2}x" -f `
        $short, ($file.Length / 1MB), $lookMs, $f3dMs, ($f3dMs / $lookMs)
}
