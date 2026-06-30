$ErrorActionPreference = "Stop"

$package = Get-AppxPackage -Name "LinkForge.ContextMenu" -ErrorAction SilentlyContinue
if ($null -eq $package) {
    Write-Host "LinkForge Windows 11 context menu package is not registered."
    exit 0
}

Remove-AppxPackage -Package $package.PackageFullName
Write-Host "Removed LinkForge Windows 11 context menu package."
