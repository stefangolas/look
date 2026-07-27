param(
    [string]$V3 = (Join-Path $PSScriptRoot '..\target\release\v3.exe'),
    [string]$F3D = 'C:\Program Files\F3D\bin\f3d-console.exe',
    [string]$ModelDirectory = (Join-Path $PSScriptRoot '..\target\bench\models'),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\target\bench\outputs'),
    [int]$Iterations = 7,
    [int]$Warmups = 2,
    [int]$Width = 512,
    [int]$Height = 512
)

$ErrorActionPreference = 'Stop'

function Resolve-InWorkspace([string]$Path) {
    $workspace = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
    $resolved = [System.IO.Path]::GetFullPath($Path)
    if (-not $resolved.StartsWith($workspace, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes workspace: $resolved"
    }
    return $resolved
}

function Invoke-TimedProcess(
    [string]$Executable,
    [string[]]$Arguments,
    [string]$ExpectedOutput
) {
    if (Test-Path -LiteralPath $ExpectedOutput) {
        Remove-Item -LiteralPath $ExpectedOutput -Force
    }

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Executable
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    # Windows PowerShell 5 does not expose ProcessStartInfo.ArgumentList.
    # Quote every argument so paths remain valid on both Windows PowerShell
    # and PowerShell 7.
    $startInfo.Arguments = ($Arguments | ForEach-Object {
        '"' + $_.Replace('"', '\"') + '"'
    }) -join ' '

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
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
    return [math]::Round($clock.Elapsed.TotalMilliseconds, 3)
}

function Get-Distribution([double[]]$Samples) {
    $ordered = @($Samples | Sort-Object)
    $middle = [int][math]::Floor($ordered.Count / 2)
    $median = if ($ordered.Count % 2 -eq 0) {
        ($ordered[$middle - 1] + $ordered[$middle]) / 2
    } else {
        $ordered[$middle]
    }
    return [ordered]@{
        samples_ms = $Samples
        min_ms = [math]::Round($ordered[0], 3)
        median_ms = [math]::Round($median, 3)
        max_ms = [math]::Round($ordered[-1], 3)
    }
}

$outputRoot = Resolve-InWorkspace $OutputDirectory
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

# F3D's Windows output resolution is expressed in DPI-virtualized logical
# pixels. Compensate so both tools emit the same physical PNG dimensions.
$appliedDpi = 96
if ($env:OS -eq 'Windows_NT') {
    $dpiSetting = Get-ItemProperty -LiteralPath 'HKCU:\Control Panel\Desktop\WindowMetrics' -Name AppliedDPI -ErrorAction SilentlyContinue
    if ($null -ne $dpiSetting -and $dpiSetting.AppliedDPI -gt 0) {
        $appliedDpi = [int]$dpiSetting.AppliedDPI
    }
}
$f3dLogicalWidth = [int][math]::Ceiling($Width * 96 / $appliedDpi)
$f3dLogicalHeight = [int][math]::Ceiling($Height * 96 / $appliedDpi)

$models = Get-ChildItem -LiteralPath $ModelDirectory -Filter '*.glb' | Sort-Object Name
if ($models.Count -eq 0) {
    throw "No GLB models found in $ModelDirectory"
}

$tools = @(
    @{
        name = 'v3'
        executable = [System.IO.Path]::GetFullPath($V3)
        arguments = {
            param($model, $output)
            @(
                'render', $model,
                '--view', 'front',
                '--camera', 'orthographic',
                '--resolution', "${Width}x${Height}",
                '--background', '#252525',
                '--output', $output,
                '--json'
            )
        }
    },
    @{
        name = 'f3d'
        executable = $F3D
        arguments = {
            param($model, $output)
            @(
                $model,
                '--output', $output,
                '--resolution', "$f3dLogicalWidth,$f3dLogicalHeight",
                '--camera-direction=-Z',
                '--camera-orthographic',
                '--anti-aliasing=none',
                '--background-color', '#252525'
            )
        }
    }
)

$results = @()
foreach ($model in $models) {
    foreach ($tool in $tools) {
        $output = Join-Path $outputRoot "$($model.BaseName)-$($tool.name).png"
        $arguments = & $tool.arguments $model.FullName $output

        for ($index = 0; $index -lt $Warmups; $index++) {
            [void](Invoke-TimedProcess $tool.executable $arguments $output)
        }

        $samples = @()
        for ($index = 0; $index -lt $Iterations; $index++) {
            $samples += Invoke-TimedProcess $tool.executable $arguments $output
        }

        $results += [ordered]@{
            model = $model.Name
            bytes = $model.Length
            tool = $tool.name
            resolution = @($Width, $Height)
            process_mode = 'fresh_process'
            distribution = Get-Distribution $samples
            output = $output
        }
    }
}

$report = [ordered]@{
    generated_at = [DateTimeOffset]::UtcNow.ToString('O')
    machine = [ordered]@{
        os = [System.Environment]::OSVersion.VersionString
        processor_count = [System.Environment]::ProcessorCount
        execution_class = 'physical'
    }
    configuration = [ordered]@{
        iterations = $Iterations
        warmups = $Warmups
        resolution = @($Width, $Height)
        f3d_logical_resolution = @($f3dLogicalWidth, $f3dLogicalHeight)
        windows_applied_dpi = $appliedDpi
        camera = 'front orthographic, automatic fit'
        antialiasing = 'disabled'
        background = '#252525'
    }
    results = $results
}

$reportPath = Join-Path $outputRoot 'benchmark.json'
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding utf8
$report | ConvertTo-Json -Depth 8
