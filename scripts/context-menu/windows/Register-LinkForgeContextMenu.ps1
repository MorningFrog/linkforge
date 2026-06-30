param(
    [Parameter(Mandatory = $true)]
    [string] $ExePath
)

$resolvedExe = (Resolve-Path -LiteralPath $ExePath).Path

function Add-LinkForgeVerb {
    param(
        [Parameter(Mandatory = $true)]
        [string] $BaseKey,
        [Parameter(Mandatory = $true)]
        [string] $Name,
        [Parameter(Mandatory = $true)]
        [string] $Label,
        [Parameter(Mandatory = $true)]
        [string] $Action
    )
    Add-LinkForgeVerbWithTarget $BaseKey $Name $Label $Action "%1"
}

function Add-LinkForgeVerbWithTarget {
    param(
        [Parameter(Mandatory = $true)]
        [string] $BaseKey,
        [Parameter(Mandatory = $true)]
        [string] $Name,
        [Parameter(Mandatory = $true)]
        [string] $Label,
        [Parameter(Mandatory = $true)]
        [string] $Action,
        [Parameter(Mandatory = $true)]
        [string] $TargetToken
    )

    $verbKey = "$BaseKey\$Name"
    $commandKey = "$verbKey\command"
    $command = "`"$resolvedExe`" --context-action $Action --paths `"$TargetToken`""

    & reg.exe add $verbKey /ve /d $Label /f | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to register $verbKey"
    }

    & reg.exe add $commandKey /ve /d $command /f | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to register $commandKey"
    }
}

$fileBase = "HKCU\Software\Classes\*\shell"
$directoryBase = "HKCU\Software\Classes\Directory\shell"
$directoryBackgroundBase = "HKCU\Software\Classes\Directory\Background\shell"

Add-LinkForgeVerb $fileBase "LinkForge.PickSource" "LinkForge: Pick Link Source" "pick-source"
Add-LinkForgeVerb $fileBase "LinkForge.Symlink" "LinkForge: Create Symbolic Link..." "symlink"
Add-LinkForgeVerb $fileBase "LinkForge.Hardlink" "LinkForge: Create Hard Link..." "hardlink"
Add-LinkForgeVerb $fileBase "LinkForge.LinkCount" "LinkForge: Show Link Count" "link-count"
Add-LinkForgeVerb $fileBase "LinkForge.Siblings" "LinkForge: Find Hard Link Siblings..." "siblings"

Add-LinkForgeVerb $directoryBase "LinkForge.PickSource" "LinkForge: Pick Link Source" "pick-source"
Add-LinkForgeVerb $directoryBase "LinkForge.DropSymlink" "LinkForge: Create Symlink from Picked Source" "drop-symlink"
Add-LinkForgeVerb $directoryBase "LinkForge.DropHardlink" "LinkForge: Create Hard Link from Picked Source" "drop-hardlink"
Add-LinkForgeVerb $directoryBase "LinkForge.Symlink" "LinkForge: Create Symbolic Link..." "symlink"
Add-LinkForgeVerb $directoryBase "LinkForge.Siblings" "LinkForge: Find Hard Link Siblings..." "siblings"
Add-LinkForgeVerb $directoryBase "LinkForge.ScanGroups" "LinkForge: Scan Hard Link Groups" "scan-groups"
Add-LinkForgeVerb $directoryBase "LinkForge.CloneTree" "LinkForge: Clone Tree Preserving Hard Links..." "clone-tree"

Add-LinkForgeVerbWithTarget $directoryBackgroundBase "LinkForge.DropSymlink" "LinkForge: Create Symlink from Picked Source" "drop-symlink" "%V"
Add-LinkForgeVerbWithTarget $directoryBackgroundBase "LinkForge.DropHardlink" "LinkForge: Create Hard Link from Picked Source" "drop-hardlink" "%V"

Write-Host "Registered LinkForge classic context menu entries for $resolvedExe"
Write-Host "On Windows 11 these entries appear under 'Show more options'. Use the linkforge-context-menu-windows crate through scripts/context-menu/windows/modern/Register-LinkForgeModernContextMenu.ps1 for the top-level Windows 11 menu."
