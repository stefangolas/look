param(
    [Parameter(Mandatory = $true)]
    [string]$Model,
    [string]$Name = 'large-scene',
    [string]$Look = (Join-Path $PSScriptRoot '..\target\release\look.exe'),
    [string]$F3D = 'C:\Program Files\F3D\bin\f3d-console.exe',
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\target\bench\large-scene'),
    [int]$Iterations = 3,
    [int]$Width = 512,
    [int]$Height = 512,
    [int]$F3DMaxSizeMiB = 4096,
    [switch]$SkipConditioning
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

$modelPath = [IO.Path]::GetFullPath($Model)
$lookExecutable = [IO.Path]::GetFullPath($Look)
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
if (-not (Test-Path -LiteralPath $modelPath)) {
    throw "Missing benchmark model: $modelPath"
}
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
$outputs = @{
    look = Join-Path $outputRoot "$Name-look.png"
    f3d = Join-Path $outputRoot "$Name-f3d.png"
}
$arguments = @{
    look = @(
        'render', $modelPath,
        '--view', 'iso',
        '--camera', 'orthographic',
        '--resolution', "${Width}x${Height}",
        '--material-mode', 'source',
        '--preset', 'f3d-match',
        '--background', '#252525',
        '--output', $outputs.look,
        '--json'
    )
    f3d = @(
        $modelPath,
        '--no-config',
        '--output', $outputs.f3d,
        '--resolution', "$Width,$Height",
        # look places the iso camera at +1,+1,+1 looking back at the target,
        # so F3D needs the opposite vector as its viewing direction.
        '--camera-direction=-1,-1,-1',
        '--camera-orthographic',
        '--anti-aliasing=none',
        '--ambient-occlusion=0',
        '--tone-mapping=0',
        '--background-color', '#252525',
        '--light-intensity', '1',
        "--max-size=$F3DMaxSizeMiB"
    )
}
$executables = @{ look = $lookExecutable; f3d = $F3D }

if (-not $SkipConditioning) {
    foreach ($tool in @('look', 'f3d')) {
        [void](Invoke-TimedProcess $executables[$tool] $arguments[$tool] $outputs[$tool])
    }
}

$samples = @{ look = @(); f3d = @() }
for ($iteration = 0; $iteration -lt $Iterations; $iteration++) {
    $order = if ($iteration % 2 -eq 0) { @('look', 'f3d') } else { @('f3d', 'look') }
    foreach ($tool in $order) {
        $samples[$tool] += Invoke-TimedProcess $executables[$tool] $arguments[$tool] $outputs[$tool]
    }
}

$lookDistribution = Get-Distribution $samples.look
$f3dDistribution = Get-Distribution $samples.f3d
$report = [ordered]@{
    generated_at = [DateTimeOffset]::UtcNow.ToString('O')
    classification = 'local physical machine; fresh process; alternating order'
    conditioning = if ($SkipConditioning) { 'performed before harness invocation' } else { 'one unmeasured launch per renderer' }
    configuration = [ordered]@{
        model = $modelPath
        bytes = (Get-Item -LiteralPath $modelPath).Length
        material = 'glTF source PBR'
        camera = 'isometric orthographic automatic fit'
        resolution = @($Width, $Height)
        antialiasing = 'disabled'
        background = '#252525'
        iterations = $Iterations
        f3d_max_size_mib = $F3DMaxSizeMiB
        f3d_user_config = 'disabled'
    }
    look = $lookDistribution
    f3d = $f3dDistribution
    f3d_over_look = [math]::Round($f3dDistribution.median_ms / $lookDistribution.median_ms, 3)
    outputs = $outputs
}
$reportPath = Join-Path $outputRoot 'benchmark.json'
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding utf8
$report | ConvertTo-Json -Depth 8
