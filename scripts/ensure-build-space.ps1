# Ensure at least MinFreeGB of free disk on C: by removing regenerable build
# artifacts. Build targets are reproducible from the locked pins, so deleting
# them only costs rebuild time; corpus ledgers and JSONL are NOT touched.
param(
    [double]$MinFreeGB = 15,
    [double]$TargetFreeGB = 25
)

$drive = Get-PSDrive C
$freeGB = $drive.Free / 1GB

Write-Host ("Free disk: {0:N1} GB" -f $freeGB)

if ($freeGB -ge $MinFreeGB) {
    exit 0
}

$targets = @(
    "C:\Users\stefa\look\target",
    "C:\Users\stefa\truck-fork\target"
)

foreach ($target in $targets) {
    if (Test-Path $target) {
        $before = (Get-PSDrive C).Free
        Write-Host "Removing regenerable build artifacts: $target"
        Remove-Item -Recurse -Force $target -ErrorAction Stop
        $after = (Get-PSDrive C).Free
        Write-Host ("Recovered {0:N1} GB" -f (($after - $before) / 1GB))

        $freeGB = $after / 1GB
        if ($freeGB -ge $TargetFreeGB) {
            break
        }
    }
}

$freeGB = (Get-PSDrive C).Free / 1GB
Write-Host ("Free disk after cleanup: {0:N1} GB" -f $freeGB)

if ($freeGB -lt $MinFreeGB) {
    Write-Warning "Disk remains below ${MinFreeGB} GB. Avoid large workspace builds/corpus outputs."
    exit 2
}
