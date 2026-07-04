param(
    [string] $Version = "0.1.0",
    [string] $OutputDir = "target/release-assets/windows",
    [switch] $SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "../..")
$targetRoot = Join-Path $repoRoot "target"
$outputPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDir))
$targetPath = [System.IO.Path]::GetFullPath($targetRoot)

if (-not $outputPath.StartsWith($targetPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "OutputDir must resolve inside the repository target directory: $outputPath"
}

$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [Environment]::GetEnvironmentVariable("Path", "User")

function Invoke-Native {
    param(
        [string] $FilePath,
        [string[]] $Arguments
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE."
    }
}

function Copy-ReleaseFile {
    param(
        [string] $Source,
        [string] $Destination
    )

    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        throw "Missing release input: $Source"
    }

    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

if ($Version -notmatch '^(?<major>0|[1-9]\d*)\.(?<minor>0|[1-9]\d*)\.(?<patch>0|[1-9]\d*)$') {
    throw "Windows release assets require a plain major.minor.patch version, got '$Version'."
}

$windowsVersion = "$($Matches.major).$($Matches.minor).$($Matches.patch).0"
$cargo = (Get-Command cargo -ErrorAction Stop).Source
$makensis = (Get-Command makensis -ErrorAction Stop).Source

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        Invoke-Native $cargo @("build", "-p", "linkforge-cli", "--release")
        Invoke-Native $cargo @("build", "-p", "linkforge-gui", "--release")
        Invoke-Native $cargo @("build", "-p", "linkforge-context-menu-windows", "--target", "x86_64-pc-windows-msvc", "--release")
    }

    if (Test-Path -LiteralPath $outputPath) {
        Remove-Item -LiteralPath $outputPath -Recurse -Force
    }

    $stageDir = Join-Path $outputPath "staging"
    $supportDir = Join-Path $stageDir "installer"
    $contextMenuDir = Join-Path $stageDir "context-menu"
    $iconsDir = Join-Path $stageDir "icons"
    New-Item -ItemType Directory -Path $supportDir, $contextMenuDir, $iconsDir -Force | Out-Null

    Copy-ReleaseFile (Join-Path $repoRoot "target/release/linkforge.exe") (Join-Path $stageDir "linkforge.exe")
    Copy-ReleaseFile (Join-Path $repoRoot "target/release/linkforge-gui.exe") (Join-Path $stageDir "linkforge-gui.exe")
    Copy-ReleaseFile (Join-Path $repoRoot "target/x86_64-pc-windows-msvc/release/linkforge_context_menu_windows.dll") (Join-Path $stageDir "linkforge_context_menu_windows.dll")
    Copy-ReleaseFile (Join-Path $repoRoot "crates/linkforge-gui/icons/icon.ico") (Join-Path $iconsDir "icon.ico")
    Copy-ReleaseFile (Join-Path $repoRoot "scripts/context-menu/windows/modern/Register-LinkForgeModernContextMenu.ps1") (Join-Path $contextMenuDir "Register-LinkForgeModernContextMenu.ps1")
    Copy-ReleaseFile (Join-Path $repoRoot "scripts/context-menu/windows/modern/Unregister-LinkForgeModernContextMenu.ps1") (Join-Path $contextMenuDir "Unregister-LinkForgeModernContextMenu.ps1")

    $pathHelper = @'
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Add", "Remove")]
    [string] $Action,

    [Parameter(Mandatory = $true)]
    [string] $InstallDir
)

$ErrorActionPreference = "Stop"
$normalizedInstallDir = [System.IO.Path]::GetFullPath($InstallDir.TrimEnd("\"))
$current = [Environment]::GetEnvironmentVariable("Path", "User")
$parts = @()

if (-not [string]::IsNullOrWhiteSpace($current)) {
    foreach ($part in ($current -split ";")) {
        if ([string]::IsNullOrWhiteSpace($part)) {
            continue
        }

        $trimmed = $part.Trim()
        try {
            $normalizedPart = [System.IO.Path]::GetFullPath($trimmed.TrimEnd("\"))
        } catch {
            $normalizedPart = $trimmed
        }

        if ($normalizedPart -ine $normalizedInstallDir) {
            $parts += $trimmed
        }
    }
}

if ($Action -eq "Add") {
    $parts += $normalizedInstallDir
}

[Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
'@

    Set-Content -LiteralPath (Join-Path $supportDir "Update-LinkForgeUserPath.ps1") -Value $pathHelper -Encoding UTF8

    $registerInstalledContextMenu = @'
$ErrorActionPreference = "Stop"
$installDir = Split-Path -Parent $PSScriptRoot

& (Join-Path $installDir "context-menu/Register-LinkForgeModernContextMenu.ps1") `
    -Configuration Release `
    -GuiExePath (Join-Path $installDir "linkforge-gui.exe") `
    -ShellExtDllPath (Join-Path $installDir "linkforge_context_menu_windows.dll") `
    -StagingDir (Join-Path $installDir "context-menu-staging")
'@

    $unregisterInstalledContextMenu = @'
$ErrorActionPreference = "Stop"
$installDir = Split-Path -Parent $PSScriptRoot

& (Join-Path $installDir "context-menu/Unregister-LinkForgeModernContextMenu.ps1")
'@

    Set-Content -LiteralPath (Join-Path $supportDir "Register-LinkForgeInstalledContextMenu.ps1") -Value $registerInstalledContextMenu -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $supportDir "Unregister-LinkForgeInstalledContextMenu.ps1") -Value $unregisterInstalledContextMenu -Encoding UTF8

    $installerPath = Join-Path $outputPath "LinkForge_${Version}_x64-setup.exe"
    $nsisScriptPath = Join-Path $outputPath "LinkForgeInstaller.nsi"
    $stageNsisPath = $stageDir.Replace("\", "\\")
    $installerNsisPath = $installerPath.Replace("\", "\\")

    $nsisScript = @"
!include "MUI2.nsh"
!include "LogicLib.nsh"

!define PRODUCT_NAME "LinkForge"
!define PRODUCT_PUBLISHER "MorningFrog"
!define PRODUCT_VERSION "$Version"
!define PRODUCT_VERSION_WIN "$windowsVersion"
!define PRODUCT_HOMEPAGE "https://github.com/MorningFrog/linkforge"

Name "`${PRODUCT_NAME}"
OutFile "$installerNsisPath"
InstallDir "`$LOCALAPPDATA\Programs\LinkForge"
RequestExecutionLevel user
SetCompressor /SOLID lzma

VIProductVersion "`${PRODUCT_VERSION_WIN}"
VIAddVersionKey /LANG=1033 "ProductName" "`${PRODUCT_NAME}"
VIAddVersionKey /LANG=1033 "CompanyName" "`${PRODUCT_PUBLISHER}"
VIAddVersionKey /LANG=1033 "FileDescription" "LinkForge installer"
VIAddVersionKey /LANG=1033 "FileVersion" "`${PRODUCT_VERSION}"
VIAddVersionKey /LANG=1033 "ProductVersion" "`${PRODUCT_VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "`${PRODUCT_PUBLISHER}"

!define MUI_ICON "$stageNsisPath\\icons\\icon.ico"
!define MUI_UNICON "$stageNsisPath\\icons\\icon.ico"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Function RunPowerShell
  Pop `$0
  Pop `$1
  DetailPrint `$1
  StrCpy `$3 "`$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
  IfFileExists "`$3" +2 0
  StrCpy `$3 "`$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe"
  ExecWait '"`$3" -NoLogo -NoProfile -ExecutionPolicy Bypass `$0' `$2
  `$`{If`}` `$2 != 0
    SetErrorLevel `$2
    Abort "`$1 failed with exit code `$2"
  `$`{EndIf`}
FunctionEnd

Function un.RunPowerShell
  Pop `$0
  Pop `$1
  DetailPrint `$1
  StrCpy `$3 "`$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
  IfFileExists "`$3" +2 0
  StrCpy `$3 "`$WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe"
  ExecWait '"`$3" -NoLogo -NoProfile -ExecutionPolicy Bypass `$0' `$2
  `$`{If`}` `$2 != 0
    SetErrorLevel `$2
    Abort "`$1 failed with exit code `$2"
  `$`{EndIf`}
FunctionEnd

Section "Install"
  SetRegView 64
  SetOutPath "`$INSTDIR"
  File /r "$stageNsisPath\\*"

  WriteUninstaller "`$INSTDIR\Uninstall.exe"
  CreateDirectory "`$SMPROGRAMS\LinkForge"
  CreateShortCut "`$SMPROGRAMS\LinkForge\LinkForge.lnk" "`$INSTDIR\linkforge-gui.exe"

  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "DisplayName" "LinkForge"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "Publisher" "`${PRODUCT_PUBLISHER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "DisplayVersion" "`${PRODUCT_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "InstallLocation" "`$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "DisplayIcon" "`$INSTDIR\linkforge-gui.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "URLInfoAbout" "`${PRODUCT_HOMEPAGE}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "UninstallString" '"`$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "QuietUninstallString" '"`$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge" "NoRepair" 1

  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\linkforge.exe" "" "`$INSTDIR\linkforge.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\linkforge-gui.exe" "" "`$INSTDIR\linkforge-gui.exe"

  Push "Adding LinkForge to the user PATH"
  Push "-File `$\"`$INSTDIR\installer\Update-LinkForgeUserPath.ps1`$\" -Action Add -InstallDir `$\"`$INSTDIR`$\""
  Call RunPowerShell

  Push "Registering LinkForge Windows 11 context menu"
  Push "-File `$\"`$INSTDIR\installer\Register-LinkForgeInstalledContextMenu.ps1`$\""
  Call RunPowerShell
SectionEnd

Section "Uninstall"
  SetRegView 64
  Push "Unregistering LinkForge Windows 11 context menu"
  Push "-File `$\"`$INSTDIR\installer\Unregister-LinkForgeInstalledContextMenu.ps1`$\""
  Call un.RunPowerShell

  Push "Removing LinkForge from the user PATH"
  Push "-File `$\"`$INSTDIR\installer\Update-LinkForgeUserPath.ps1`$\" -Action Remove -InstallDir `$\"`$INSTDIR`$\""
  Call un.RunPowerShell

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\linkforge.exe"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\linkforge-gui.exe"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinkForge"

  Delete "`$SMPROGRAMS\LinkForge\LinkForge.lnk"
  RMDir "`$SMPROGRAMS\LinkForge"
  RMDir /r "`$INSTDIR\context-menu-staging"
  RMDir /r "`$INSTDIR"
SectionEnd
"@

    Set-Content -LiteralPath $nsisScriptPath -Value $nsisScript -Encoding UTF8
    Invoke-Native $makensis @($nsisScriptPath)

    if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
        throw "NSIS did not create installer: $installerPath"
    }

    Get-FileHash -Algorithm SHA256 -LiteralPath $installerPath |
        Format-List |
        Out-String |
        Set-Content -LiteralPath (Join-Path $outputPath "windows-installer-sha256.txt") -Encoding UTF8

    Write-Host "Built Windows installer: $installerPath"
} finally {
    Pop-Location
}
