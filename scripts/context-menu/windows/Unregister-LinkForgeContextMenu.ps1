$keys = @(
    "HKCU\Software\Classes\*\shell\LinkForge.PickSource",
    "HKCU\Software\Classes\*\shell\LinkForge.Symlink",
    "HKCU\Software\Classes\*\shell\LinkForge.Hardlink",
    "HKCU\Software\Classes\*\shell\LinkForge.LinkCount",
    "HKCU\Software\Classes\*\shell\LinkForge.Siblings",
    "HKCU\Software\Classes\Directory\shell\LinkForge.PickSource",
    "HKCU\Software\Classes\Directory\shell\LinkForge.DropSymlink",
    "HKCU\Software\Classes\Directory\shell\LinkForge.DropHardlink",
    "HKCU\Software\Classes\Directory\shell\LinkForge.Symlink",
    "HKCU\Software\Classes\Directory\shell\LinkForge.Siblings",
    "HKCU\Software\Classes\Directory\shell\LinkForge.ScanGroups",
    "HKCU\Software\Classes\Directory\shell\LinkForge.CloneTree",
    "HKCU\Software\Classes\Directory\Background\shell\LinkForge.DropSymlink",
    "HKCU\Software\Classes\Directory\Background\shell\LinkForge.DropHardlink"
)

foreach ($key in $keys) {
    & reg.exe query $key *> $null
    if ($LASTEXITCODE -eq 0) {
        & reg.exe delete $key /f | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to remove $key"
        }
    }
}

Write-Host "Removed LinkForge context menu entries."
