param(
    [ValidateSet("Debug", "Release")]
    [string] $Configuration = "Debug",
    [string] $GuiExePath,
    [string] $ShellExtDllPath,
    [string] $StagingDir = "target/linkforge-modern-context-menu",
    [switch] $PrepareOnly,
    [switch] $VerifyOnly,
    [switch] $SkipGuiCheck
)

$ErrorActionPreference = "Stop"

$PackageName = "LinkForge.ContextMenu"
$SparsePackageVersion = "0.1.0.0"
$StagedGuiExeName = "linkforge-gui.exe"
$StagedShellExtDllName = "linkforge_context_menu_windows.dll"

function Get-ProfileName {
    if ($Configuration -eq "Release") {
        return "release"
    }

    return "debug"
}

function Get-BuildSuffix {
    if ($Configuration -eq "Release") {
        return " --release"
    }

    return ""
}

function Get-DefaultGuiExePath {
    Join-Path (Join-Path "target" (Get-ProfileName)) $StagedGuiExeName
}

function Test-AppxSideLoadingEnabled {
    $key = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock"
    if (-not (Test-Path -LiteralPath $key)) {
        return $false
    }

    $settings = Get-ItemProperty -LiteralPath $key
    return ($settings.AllowDevelopmentWithoutDevLicense -eq 1) -or ($settings.AllowAllTrustedApps -eq 1)
}

function Get-ArtifactHelp {
    param(
        [string] $Kind
    )

    $suffix = Get-BuildSuffix
    if ($Kind -eq "GUI") {
        return "Build it with: cargo build -p linkforge-gui$suffix"
    }

    return "Build it with: cargo build -p linkforge-context-menu-windows --target x86_64-pc-windows-msvc$suffix"
}

function Resolve-Artifact {
    param(
        [string] $Path,
        [string] $Kind,
        [string] $ExpectedExtension,
        [switch] $Minimal
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "$Kind path is empty."
    }

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        $help = Get-ArtifactHelp -Kind $Kind
        throw @"
Missing $Kind artifact:
  $Path

$help
"@
    }

    $resolved = Resolve-Path -LiteralPath $Path
    $item = Get-Item -LiteralPath $resolved.Path

    if ($Minimal) {
        return $item.FullName
    }

    if ($item.Extension -ne $ExpectedExtension) {
        throw "$Kind artifact must be a $ExpectedExtension file: $($item.FullName)"
    }

    if ($item.Length -le 0) {
        throw "$Kind artifact is empty: $($item.FullName)"
    }

    return $item.FullName
}

function Get-StagingPath {
    $resolvedStagingDir = $StagingDir
    if ([System.IO.Path]::IsPathRooted($resolvedStagingDir)) {
        return $resolvedStagingDir
    }

    return Join-Path (Get-Location) $resolvedStagingDir
}

function Assert-StagedFile {
    param(
        [string] $Path,
        [string] $Kind,
        [string] $ExpectedExtension,
        [switch] $SkipStrictChecks
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Missing staged $Kind artifact: $Path"
    }

    if ($SkipStrictChecks) {
        return
    }

    $item = Get-Item -LiteralPath $Path
    if ($item.Extension -ne $ExpectedExtension) {
        throw "Staged $Kind artifact must be a $ExpectedExtension file: $($item.FullName)"
    }

    if ($item.Length -le 0) {
        throw "Staged $Kind artifact is empty: $($item.FullName)"
    }
}

function Show-RegistrationFailureHint {
    param(
        [System.Management.Automation.ErrorRecord] $ErrorRecord
    )

    $message = $ErrorRecord.Exception.Message
    if ($message -match "0x80073CFF") {
        Write-Host "Hint: enable Developer Mode or app sideloading, then rerun this script."
    } elseif ($message -match "0x80073D2E") {
        Write-Host "Hint: sparse packages registered with -ExternalLocation require AllowExternalContent=true in the manifest."
    } elseif ($message -match "0x80070057") {
        Write-Host "Hint: ensure the manifest uses a concrete resource language such as en-us."
    }
}

function Test-LinkForgeModernRegistration {
    param(
        [string] $Stage
    )

    $manifestPath = Join-Path $Stage "AppxManifest.xml"
    $stagedGuiExe = Join-Path $Stage $StagedGuiExeName
    $stagedShellExtDll = Join-Path $Stage $StagedShellExtDllName

    Assert-StagedFile -Path $stagedGuiExe -Kind "GUI" -ExpectedExtension ".exe" -SkipStrictChecks:$SkipGuiCheck
    Assert-StagedFile -Path $stagedShellExtDll -Kind "shell extension" -ExpectedExtension ".dll"

    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Missing staged Appx manifest: $manifestPath"
    }

    $package = Get-AppxPackage -Name $PackageName -ErrorAction SilentlyContinue
    if ($null -eq $package) {
        throw "LinkForge Windows 11 context menu package is not registered."
    }

    Write-Host "Verified LinkForge Windows 11 context menu package:"
    Write-Host "  Name: $($package.Name)"
    Write-Host "  Full name: $($package.PackageFullName)"
    Write-Host "  Staging dir: $Stage"
    Write-Host "  Manifest: $manifestPath"
    Write-Host "  GUI: $stagedGuiExe"
    Write-Host "  Shell extension: $stagedShellExtDll"
}

if ([string]::IsNullOrWhiteSpace($GuiExePath)) {
    $GuiExePath = Get-DefaultGuiExePath
}

if ([string]::IsNullOrWhiteSpace($ShellExtDllPath)) {
    $ShellExtDllPath = Join-Path (Join-Path (Join-Path "target" "x86_64-pc-windows-msvc") (Get-ProfileName)) $StagedShellExtDllName
}

$stage = Get-StagingPath

if ($VerifyOnly) {
    Test-LinkForgeModernRegistration -Stage $stage
    exit 0
}

if ($SkipGuiCheck) {
    Write-Host "Skipping strict GUI artifact checks because -SkipGuiCheck was provided."
    $guiExe = Resolve-Artifact -Path $GuiExePath -Kind "GUI" -ExpectedExtension ".exe" -Minimal
} else {
    $guiExe = Resolve-Artifact -Path $GuiExePath -Kind "GUI" -ExpectedExtension ".exe"
}

$shellExtDll = Resolve-Artifact -Path $ShellExtDllPath -Kind "shell extension" -ExpectedExtension ".dll"
$assets = Join-Path $stage "Assets"

New-Item -ItemType Directory -Path $stage -Force | Out-Null
New-Item -ItemType Directory -Path $assets -Force | Out-Null

Copy-Item -LiteralPath $guiExe -Destination (Join-Path $stage $StagedGuiExeName) -Force
Copy-Item -LiteralPath $shellExtDll -Destination (Join-Path $stage $StagedShellExtDllName) -Force

$logoBytes = [Convert]::FromBase64String("iVBORw0KGgoAAAANSUhEUgAAACwAAAAsCAIAAADt2u7VAAAATUlEQVR4nO3OwQkAIBDAsI7sv2Y36QZBIoLzCwNzZpKZ6wN8lh6XHpcelyKXHpcelyKXHpcelyKXHpcelyKXHpcelyKXHpcelyKXHpfyALHCAhU8Lzv8AAAAAElFTkSuQmCC")
[System.IO.File]::WriteAllBytes((Join-Path $assets "Logo44.png"), $logoBytes)
[System.IO.File]::WriteAllBytes((Join-Path $assets "Logo150.png"), $logoBytes)

$manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  xmlns:desktop4="http://schemas.microsoft.com/appx/manifest/desktop/windows10/4"
  xmlns:desktop5="http://schemas.microsoft.com/appx/manifest/desktop/windows10/5"
  xmlns:com="http://schemas.microsoft.com/appx/manifest/com/windows10"
  IgnorableNamespaces="uap uap10 rescap desktop4 desktop5 com">
  <Identity Name="LinkForge.ContextMenu" Publisher="CN=LinkForge" Version="$SparsePackageVersion" ProcessorArchitecture="x64" />
  <Properties>
    <DisplayName>LinkForge</DisplayName>
    <PublisherDisplayName>LinkForge</PublisherDisplayName>
    <Logo>Assets\Logo150.png</Logo>
    <uap10:AllowExternalContent>true</uap10:AllowExternalContent>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.22000.0" MaxVersionTested="10.0.26100.0" />
  </Dependencies>
  <Resources>
    <Resource Language="en-us" />
  </Resources>
  <Applications>
    <Application Id="LinkForge" Executable="linkforge-gui.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="LinkForge" Description="LinkForge" BackgroundColor="transparent" Square44x44Logo="Assets\Logo44.png" Square150x150Logo="Assets\Logo150.png" />
      <Extensions>
        <com:Extension Category="windows.comServer">
          <com:ComServer>
            <com:SurrogateServer DisplayName="LinkForge Context Menu">
              <com:Class Id="7D4D6E4B-2C72-4A54-9367-6D2F4A3D1C8E" Path="linkforge_context_menu_windows.dll" ThreadingModel="STA" />
            </com:SurrogateServer>
          </com:ComServer>
        </com:Extension>
        <desktop4:Extension Category="windows.fileExplorerContextMenus">
          <desktop4:FileExplorerContextMenus>
            <desktop5:ItemType Type="*">
              <desktop5:Verb Id="LinkForgeFiles" Clsid="7D4D6E4B-2C72-4A54-9367-6D2F4A3D1C8E" />
            </desktop5:ItemType>
            <desktop5:ItemType Type="Directory">
              <desktop5:Verb Id="LinkForgeDirectories" Clsid="7D4D6E4B-2C72-4A54-9367-6D2F4A3D1C8E" />
            </desktop5:ItemType>
            <desktop5:ItemType Type="Directory\Background">
              <desktop5:Verb Id="LinkForgeDirectoryBackground" Clsid="7D4D6E4B-2C72-4A54-9367-6D2F4A3D1C8E" />
            </desktop5:ItemType>
          </desktop4:FileExplorerContextMenus>
        </desktop4:Extension>
      </Extensions>
    </Application>
  </Applications>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
    <rescap:Capability Name="unvirtualizedResources" />
  </Capabilities>
</Package>
"@

$manifestPath = Join-Path $stage "AppxManifest.xml"
Set-Content -LiteralPath $manifestPath -Value $manifest -Encoding UTF8

Assert-StagedFile -Path (Join-Path $stage $StagedGuiExeName) -Kind "GUI" -ExpectedExtension ".exe" -SkipStrictChecks:$SkipGuiCheck
Assert-StagedFile -Path (Join-Path $stage $StagedShellExtDllName) -Kind "shell extension" -ExpectedExtension ".dll"
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Failed to write Appx manifest: $manifestPath"
}

[xml] $parsedManifest = Get-Content -LiteralPath $manifestPath
if ($parsedManifest.Package.Identity.Name -ne $PackageName) {
    throw "Generated Appx manifest identity is not $PackageName."
}

Write-Host "Prepared LinkForge Windows 11 context menu staging:"
Write-Host "  Configuration: $Configuration"
Write-Host "  Staging dir: $stage"
Write-Host "  GUI: $(Join-Path $stage $StagedGuiExeName)"
Write-Host "  Shell extension: $(Join-Path $stage $StagedShellExtDllName)"
Write-Host "  Manifest: $manifestPath"

if ($PrepareOnly) {
    Write-Host "Prepared only; skipped sparse package registration because -PrepareOnly was provided."
    exit 0
}

if (-not (Test-AppxSideLoadingEnabled)) {
    throw @"
Windows rejected sparse package registration because Developer Mode or app sideloading is not enabled.

Enable Developer Mode in Windows Settings:
  Settings > System > For developers > Developer Mode

Then rerun this script.
"@
}

try {
    Add-AppxPackage -Register $manifestPath -ExternalLocation $stage
} catch {
    Show-RegistrationFailureHint -ErrorRecord $_
    throw
}

Test-LinkForgeModernRegistration -Stage $stage

Write-Host "Registered LinkForge Windows 11 context menu package from $stage"
Write-Host "Open a new Explorer window and right-click a file or directory. Restart Explorer only if the menu does not refresh."
