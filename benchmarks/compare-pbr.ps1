param(
    [string]$Look = (Join-Path $PSScriptRoot '..\target\release\look.exe'),
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
$lookExecutable = [IO.Path]::GetFullPath($Look)

$f3dWidth = $Width
$f3dHeight = $Height

$results = @()
foreach ($name in $Models) {
    $model = [IO.Path]::GetFullPath((Join-Path $ModelDirectory "$name.glb"))
    if (-not (Test-Path -LiteralPath $model)) {
        throw "Missing benchmark model: $model"
    }
    $outputs = @{
        look = Join-Path $outputRoot "$name-look.png"
        f3d = Join-Path $outputRoot "$name-f3d.png"
    }
    $arguments = @{
        look = @(
            'render', $model,
            '--view', 'front',
            '--camera', 'orthographic',
            '--resolution', "${Width}x${Height}",
            '--material-mode', 'source',
            '--preset', 'f3d-match',
            '--background', '#252525',
            '--output', $outputs.look,
            '--json'
        )
        f3d = @(
            $model,
            '--no-config',
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
    $executables = @{ look = $lookExecutable; f3d = $F3D }

    # One untimed launch per tool initializes OS file and driver caches. Each
    # measured invocation remains a new process. Alternating first position
    # prevents one renderer from systematically inheriting a warmer GPU.
    foreach ($tool in @('look', 'f3d')) {
        [void](Invoke-TimedProcess $executables[$tool] $arguments[$tool] $outputs[$tool])
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
    $results += [ordered]@{
        model = "$name.glb"
        bytes = (Get-Item -LiteralPath $model).Length
        look = $lookDistribution
        f3d = $f3dDistribution
        f3d_over_look = [math]::Round($f3dDistribution.median_ms / $lookDistribution.median_ms, 3)
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
        f3d_user_config = 'disabled'
    }
    results = $results
}
$reportPath = Join-Path $outputRoot 'benchmark.json'
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding utf8
$report | ConvertTo-Json -Depth 8
