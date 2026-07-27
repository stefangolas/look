param(
    [string]$Version = $env:LOOK_VERSION,
    [string]$InstallDirectory = $env:LOOK_INSTALL_DIR
)
$ErrorActionPreference = 'Stop'
if (-not $Version) { $Version = 'latest' }
if (-not $InstallDirectory) { $InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\look\bin' }
$repository = if ($env:LOOK_REPOSITORY) { $env:LOOK_REPOSITORY } else { 'stefangolas/look' }
$architecture = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'aarch64' } else { 'x86_64' }
if ($Version -eq 'latest') {
    $Version = (Invoke-RestMethod "https://api.github.com/repos/$repository/releases/latest").tag_name
}
$asset = "look-$Version-windows-$architecture.zip"
$base = "https://github.com/$repository/releases/download/$Version"
$temporary = Join-Path ([IO.Path]::GetTempPath()) ("look-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $temporary | Out-Null
try {
    Invoke-WebRequest "$base/$asset" -OutFile (Join-Path $temporary $asset)
    Invoke-WebRequest "$base/$asset.sha256" -OutFile (Join-Path $temporary "$asset.sha256")
    $expected = ((Get-Content (Join-Path $temporary "$asset.sha256")) -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash (Join-Path $temporary $asset) -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "SHA-256 mismatch for $asset" }
    Expand-Archive (Join-Path $temporary $asset) -DestinationPath $temporary
    New-Item -ItemType Directory -Force $InstallDirectory | Out-Null
    Copy-Item (Join-Path $temporary "look-$Version-windows-$architecture\look.exe") (Join-Path $InstallDirectory 'look.exe') -Force
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (($userPath -split ';') -notcontains $InstallDirectory) {
        [Environment]::SetEnvironmentVariable('Path', (($userPath.TrimEnd(';') + ';' + $InstallDirectory).TrimStart(';')), 'User')
        Write-Host "Added $InstallDirectory to the user PATH; open a new terminal to use it."
    }
    Write-Host "look installed to $(Join-Path $InstallDirectory 'look.exe')"
} finally {
    if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
