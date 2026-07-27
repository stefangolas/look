param(
    [string]$V3 = (Join-Path $PSScriptRoot '..\target\release\v3.exe'),
    [string]$F3D = 'C:\Program Files\F3D\bin\f3d-console.exe',
    [string]$ModelDirectory = (Join-Path $PSScriptRoot '..\target\bench\models'),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\target\bench\pbr'),
    [string[]]$Models = @('Box', 'DamagedHelmet', 'Avocado', 'BoomBox', 'MetalRoughSpheres', 'NormalTangentTest'),
    [int]$Iterations = 7,
    [int]$Width = 512,
    [int]$Height = 512
)

$ErrorActionPreference = 'Stop'

function Invoke-TimedProcess(
    [string]$Executable,
    [string[]]$Arguments,
    [string]$ExpectedOutput
) {
    if (Test-Path -LiteralPath $ExpectedOutput) {
        Remove-Item -LiteralPath $ExpectedOutput -Force
    }
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Executable
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.Arguments = ($Arguments | ForEach-Object {
        '"' + $_.Replace('"', '\"') + '"'
    }) -join ' '
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $clock = [Diagnostics.Stopwatch]::StartNew()
    [void]$process.Start()
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    $clock.Stop()
    if ($process.ExitCode -ne 0) {
        throw "$Executable exited $($process.ExitCode): $($stderr.Result)"
    }
    if (-not (Test-Path -LiteralPath $ExpectedOutput)) {
        throw "$Executable did not create $ExpectedOutput. stdout: $($stdout.Result) stderr: $($stderr.Result)"
    }
    [math]::Round($clock.Elapsed.TotalMilliseconds, 3)
}

function Get-Distribution([double[]]$Samples) {
    $ordered = @($Samples | Sort-Object)
    $middle = [int][math]::Floor($ordered.Count / 2)
    [ordered]@{
        samples_ms = $Samples
        min_ms = $ordered[0]
        median_ms = $ordered[$middle]
        max_ms = $ordered[-1]
    }
}

$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
$v3Executable = [IO.Path]::GetFullPath($V3)

$appliedDpi = 96
if ($env:OS -eq 'Windows_NT') {
    $dpi = Get-ItemProperty -LiteralPath 'HKCU:\Control Panel\Desktop\WindowMetrics' -Name AppliedDPI -ErrorAction SilentlyContinue
    if ($null -ne $dpi -and $dpi.AppliedDPI -gt 0) {
        $appliedDpi = [int]$dpi.AppliedDPI
    }
}
$f3dWidth = [int][math]::Ceiling($Width * 96 / $appliedDpi)
$f3dHeight = [int][math]::Ceiling($Height * 96 / $appliedDpi)

$results = @()
foreach ($name in $Models) {
    $model = [IO.Path]::GetFullPath((Join-Path $ModelDirectory "$name.glb"))
    if (-not (Test-Path -LiteralPath $model)) {
        throw "Missing benchmark model: $model"
    }
    $outputs = @{
        v3 = Join-Path $outputRoot "$name-v3.png"
        f3d = Join-Path $outputRoot "$name-f3d.png"
    }
    $arguments = @{
        v3 = @(
            'render', $model,
            '--view', 'front',
            '--camera', 'orthographic',
            '--resolution', "${Width}x${Height}",
            '--material-mode', 'source',
            '--background', '#252525',
            '--ambient', '0.35',
            '--light-direction=-1,-2,-3',
            '--light-intensity', '0.85',
            '--output', $outputs.v3,
            '--json'
        )
        f3d = @(
            $model,
            '--output', $outputs.f3d,
            '--resolution', "$f3dWidth,$f3dHeight",
            '--camera-direction=-Z',
            '--camera-orthographic',
            '--anti-aliasing=none',
            '--ambient-occlusion=0',
            '--tone-mapping=0',
            '--background-color', '#252525',
            '--light-intensity', '1'
        )
    }
    $executables = @{ v3 = $v3Executable; f3d = $F3D }

    # One untimed launch per tool initializes OS file and driver caches. Each
    # measured invocation remains a new process. Alternating first position
    # prevents one renderer from systematically inheriting a warmer GPU.
    foreach ($tool in @('v3', 'f3d')) {
        [void](Invoke-TimedProcess $executables[$tool] $arguments[$tool] $outputs[$tool])
    }

    $samples = @{ v3 = @(); f3d = @() }
    for ($iteration = 0; $iteration -lt $Iterations; $iteration++) {
        $order = if ($iteration % 2 -eq 0) { @('v3', 'f3d') } else { @('f3d', 'v3') }
        foreach ($tool in $order) {
            $samples[$tool] += Invoke-TimedProcess $executables[$tool] $arguments[$tool] $outputs[$tool]
        }
    }

    $v3Distribution = Get-Distribution $samples.v3
    $f3dDistribution = Get-Distribution $samples.f3d
    $results += [ordered]@{
        model = "$name.glb"
        bytes = (Get-Item -LiteralPath $model).Length
        v3 = $v3Distribution
        f3d = $f3dDistribution
        f3d_over_v3 = [math]::Round($f3dDistribution.median_ms / $v3Distribution.median_ms, 3)
        outputs = $outputs
    }
}

$report = [ordered]@{
    generated_at = [DateTimeOffset]::UtcNow.ToString('O')
    classification = 'local physical machine; fresh process; driver warm; alternating order'
    configuration = [ordered]@{
        material = 'glTF source PBR'
        camera = 'front orthographic automatic fit'
        resolution = @($Width, $Height)
        antialiasing = 'disabled'
        background = '#252525'
        iterations = $Iterations
        f3d_logical_resolution = @($f3dWidth, $f3dHeight)
        windows_applied_dpi = $appliedDpi
    }
    results = $results
}
$reportPath = Join-Path $outputRoot 'benchmark.json'
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding utf8
$report | ConvertTo-Json -Depth 8
