param(
    [string] $GuiExePath = "target/debug/linkforge-gui.exe",
    [string] $ShellExtDllPath = "target/x86_64-pc-windows-msvc/debug/linkforge_context_menu_windows.dll",
    [string] $StagingDir = "target/linkforge-modern-context-menu"
)

$ErrorActionPreference = "Stop"

function Test-AppxSideLoadingEnabled {
    $key = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock"
    if (-not (Test-Path -LiteralPath $key)) {
        return $false
    }

    $settings = Get-ItemProperty -LiteralPath $key
    return ($settings.AllowDevelopmentWithoutDevLicense -eq 1) -or ($settings.AllowAllTrustedApps -eq 1)
}

if (-not (Test-AppxSideLoadingEnabled)) {
    throw @"
Windows rejected sparse package registration because Developer Mode or app sideloading is not enabled.

Enable Developer Mode in Windows Settings:
  Settings > System > For developers > Developer Mode

Then rerun this script. If you do not want to enable Developer Mode, use the classic context-menu fallback instead:
  scripts/context-menu/windows/Register-LinkForgeContextMenu.ps1

Classic entries appear under "Show more options" on Windows 11.
"@
}

$guiExe = (Resolve-Path -LiteralPath $GuiExePath).Path
$shellExtDll = (Resolve-Path -LiteralPath $ShellExtDllPath).Path
$stage = Join-Path (Get-Location) $StagingDir
$assets = Join-Path $stage "Assets"

New-Item -ItemType Directory -Path $stage -Force | Out-Null
New-Item -ItemType Directory -Path $assets -Force | Out-Null

Copy-Item -LiteralPath $guiExe -Destination (Join-Path $stage "linkforge-gui.exe") -Force
Copy-Item -LiteralPath $shellExtDll -Destination (Join-Path $stage "linkforge_context_menu_windows.dll") -Force

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
  <Identity Name="LinkForge.ContextMenu" Publisher="CN=LinkForge" Version="0.1.0.0" ProcessorArchitecture="x64" />
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

Add-AppxPackage -Register $manifestPath -ExternalLocation $stage

Write-Host "Registered LinkForge Windows 11 context menu package from $stage"
Write-Host "Open a new Explorer window and right-click a file or directory. Restart Explorer only if the menu does not refresh."
