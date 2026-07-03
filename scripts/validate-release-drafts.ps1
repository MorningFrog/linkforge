param(
    [string] $WindowsAppxManifest = "target/linkforge-modern-context-menu/AppxManifest.xml",
    [switch] $RequireWindowsAppxManifest,
    [string] $ReportPath = "target/release-validation/release-draft-checks.md"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$Checks = [System.Collections.Generic.List[object]]::new()

function Add-Check {
    param(
        [string] $Name,
        [ValidateSet("PASS", "WARN", "FAIL")]
        [string] $Status,
        [string] $Details
    )

    $Checks.Add([pscustomobject]@{
        Name = $Name
        Status = $Status
        Details = $Details
    }) | Out-Null
}

function Get-RepoPath {
    param([string] $RelativePath)
    Join-Path $RepoRoot $RelativePath
}

function Test-RequiredFile {
    param(
        [string] $RelativePath,
        [string] $Name
    )

    $path = Get-RepoPath $RelativePath
    if (Test-Path -LiteralPath $path -PathType Leaf) {
        Add-Check $Name "PASS" $RelativePath
        return $path
    }

    Add-Check $Name "FAIL" "Missing $RelativePath"
    return $null
}

function Assert-Text {
    param(
        [string] $Text,
        [string] $Needle,
        [string] $Name,
        [string] $Details
    )

    if ($Text.Contains($Needle)) {
        Add-Check $Name "PASS" $Details
    } else {
        Add-Check $Name "FAIL" "Missing expected text: $Needle"
    }
}

function Test-PowerShellSyntax {
    param([string] $RelativePath)

    $path = Test-RequiredFile $RelativePath "PowerShell script exists: $RelativePath"
    if ($null -eq $path) {
        return
    }

    $tokens = $null
    $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile($path, [ref] $tokens, [ref] $errors) | Out-Null
    if ($errors.Count -eq 0) {
        Add-Check "PowerShell syntax: $RelativePath" "PASS" "Parsed without syntax errors"
    } else {
        $messages = ($errors | ForEach-Object { $_.Message } | Select-Object -First 3) -join "; "
        Add-Check "PowerShell syntax: $RelativePath" "FAIL" $messages
    }
}

function Test-FlatpakDraft {
    $manifestPath = Test-RequiredFile "packaging/flatpak/io.github.morningfrog.LinkForge.yml" "Flatpak manifest exists"
    $desktopPath = Test-RequiredFile "packaging/flatpak/io.github.morningfrog.LinkForge.desktop" "Flatpak desktop file exists"
    $metainfoPath = Test-RequiredFile "packaging/flatpak/io.github.morningfrog.LinkForge.metainfo.xml" "Flatpak MetaInfo exists"
    Test-RequiredFile "packaging/flatpak/io.github.morningfrog.LinkForge.svg" "Flatpak icon exists" | Out-Null

    if ($manifestPath) {
        $manifest = Get-Content -Raw -LiteralPath $manifestPath
        Assert-Text $manifest "app-id: io.github.morningfrog.LinkForge" "Flatpak app id" "App ID matches release identity"
        Assert-Text $manifest "runtime: org.gnome.Platform" "Flatpak runtime" "Uses GNOME runtime"
        Assert-Text $manifest 'runtime-version: "50"' "Flatpak runtime version" "Uses the current GNOME runtime branch"
        Assert-Text $manifest "org.freedesktop.Sdk.Extension.rust-stable" "Flatpak Rust SDK extension" "Rust toolchain is available inside the SDK"
        Assert-Text $manifest "cargo build --release --locked --offline" "Flatpak offline build command" "Build remains locked and offline"
        Assert-Text $manifest "command: linkforge-gui" "Flatpak exported command" "GUI command is exported"
        Assert-Text $manifest "type: dir" "Flatpak local source marker" "Draft uses local source and must be replaced before public submission"
    }

    if ($desktopPath) {
        $desktop = Get-Content -Raw -LiteralPath $desktopPath
        Assert-Text $desktop "Exec=linkforge-gui" "Desktop Exec" "Desktop file launches the GUI"
        Assert-Text $desktop "Icon=io.github.morningfrog.LinkForge" "Desktop icon id" "Icon ID matches App ID"
    }

    if ($metainfoPath) {
        try {
            [xml] $metainfo = Get-Content -LiteralPath $metainfoPath
            $id = $metainfo.SelectSingleNode("//*[local-name()='id']")
            $launchable = $metainfo.SelectSingleNode("//*[local-name()='launchable']")
            $release = $metainfo.SelectSingleNode("//*[local-name()='release']")
            if ($id -and $id.InnerText -eq "io.github.morningfrog.LinkForge") {
                Add-Check "AppStream id" "PASS" "MetaInfo ID matches App ID"
            } else {
                Add-Check "AppStream id" "FAIL" "MetaInfo ID does not match io.github.morningfrog.LinkForge"
            }
            if ($launchable -and $launchable.InnerText -eq "io.github.morningfrog.LinkForge.desktop") {
                Add-Check "AppStream launchable" "PASS" "Launchable desktop ID is present"
            } else {
                Add-Check "AppStream launchable" "FAIL" "Launchable desktop ID is missing or mismatched"
            }
            if ($release) {
                Add-Check "AppStream release entry" "PASS" "Release metadata is present"
            } else {
                Add-Check "AppStream release entry" "FAIL" "Release metadata is missing"
            }
        } catch {
            Add-Check "AppStream XML" "FAIL" $_.Exception.Message
        }
    }
}

function Test-WingetDraft {
    Test-RequiredFile "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.yaml" "winget version manifest exists" | Out-Null
    Test-RequiredFile "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.locale.en-US.yaml" "winget locale manifest exists" | Out-Null
    $installerPath = Test-RequiredFile "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.installer.yaml" "winget installer manifest exists"
    $readmePath = Test-RequiredFile "packaging/winget/README.md" "winget README exists"

    if ($installerPath) {
        $installer = Get-Content -Raw -LiteralPath $installerPath
        Assert-Text $installer "InstallerType: nullsoft" "winget installer type" "NSIS/nullsoft installer strategy"
        Assert-Text $installer 'Silent: "/S"' "winget silent switch" "Silent switch is recorded"
        Assert-Text $installer 'SilentWithProgress: "/S"' "winget silent progress switch" "Silent-with-progress switch is recorded"
        Assert-Text $installer "InstallerUrl: `"https://github.com/MorningFrog/linkforge/releases/download/v0.1.0/LinkForge_0.1.0_x64-setup.exe`"" "winget release URL placeholder" "Release URL placeholder is present"
        if ($installer -match 'InstallerSha256:\s*"(?<hash>[0-9A-Fa-f]{64})"') {
            if ($Matches.hash -eq ("0" * 64)) {
                Add-Check "winget SHA256" "WARN" "Placeholder hash remains; install validation must wait for a real artifact"
            } else {
                Add-Check "winget SHA256" "PASS" "Installer SHA256 is populated"
            }
        } else {
            Add-Check "winget SHA256" "FAIL" "InstallerSha256 is missing or not a 64-character hex value"
        }
    }

    if ($readmePath) {
        $readme = Get-Content -Raw -LiteralPath $readmePath
        Assert-Text $readme "UninstallSilent: /S" "winget uninstall switch note" "Uninstall switch is documented for local validation"
        Assert-Text $readme "Windows Sandbox" "winget sandbox validation note" "Clean-machine validation is documented"
    }
}

function Test-DebianDraft {
    $controlPath = Test-RequiredFile "debian/control" "Debian control exists"
    $rulesPath = Test-RequiredFile "debian/rules" "Debian rules exists"
    Test-RequiredFile "debian/linkforge-cli.install" "Debian CLI install file exists" | Out-Null
    Test-RequiredFile "debian/linkforge-gui.install" "Debian GUI install file exists" | Out-Null
    Test-RequiredFile "debian/linkforge-context-menu-gnome.install" "Debian GNOME install file exists" | Out-Null

    if ($controlPath) {
        $control = Get-Content -Raw -LiteralPath $controlPath
        Assert-Text $control "Package: linkforge-cli" "Debian CLI package" "CLI package is declared"
        Assert-Text $control "Package: linkforge-gui" "Debian GUI package" "GUI package is declared"
        Assert-Text $control "Package: linkforge-context-menu-gnome" "Debian GNOME package" "GNOME integration package is declared"
    }

    if ($rulesPath) {
        $rules = Get-Content -Raw -LiteralPath $rulesPath
        Assert-Text $rules "cargo build --release --locked --offline" "Debian offline build" "Debian build remains locked and offline"
        Assert-Text $rules "cargo test --locked --offline" "Debian offline tests" "Debian tests remain locked and offline"
    }
}

function Test-WindowsAppxManifest {
    $path = Join-Path $RepoRoot $WindowsAppxManifest
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        if ($RequireWindowsAppxManifest) {
            Add-Check "Windows Appx manifest" "FAIL" "Missing $WindowsAppxManifest"
        } else {
            Add-Check "Windows Appx manifest" "WARN" "Not found; run Register-LinkForgeModernContextMenu.ps1 -PrepareOnly to generate it"
        }
        return
    }

    try {
        [xml] $appx = Get-Content -LiteralPath $path
        $identity = $appx.SelectSingleNode("//*[local-name()='Identity']")
        $application = $appx.SelectSingleNode("//*[local-name()='Application']")
        $comClass = $appx.SelectSingleNode("//*[local-name()='Class']")
        if ($identity -and $identity.Name -eq "LinkForge.ContextMenu" -and $identity.Version -match '^\d+\.\d+\.\d+\.0$') {
            Add-Check "Windows Appx identity" "PASS" "Identity and four-part version are valid"
        } else {
            Add-Check "Windows Appx identity" "FAIL" "Identity or version is invalid"
        }
        if ($application -and $application.Executable -eq "linkforge-gui.exe") {
            Add-Check "Windows Appx executable" "PASS" "Application launches linkforge-gui.exe"
        } else {
            Add-Check "Windows Appx executable" "FAIL" "Application executable is missing or mismatched"
        }
        if ($comClass -and $comClass.Path -eq "linkforge_context_menu_windows.dll") {
            Add-Check "Windows Appx COM server" "PASS" "COM class points to the shell extension DLL"
        } else {
            Add-Check "Windows Appx COM server" "FAIL" "COM class path is missing or mismatched"
        }
    } catch {
        Add-Check "Windows Appx XML" "FAIL" $_.Exception.Message
    }
}

function Write-Report {
    $reportFullPath = Join-Path $RepoRoot $ReportPath
    $reportDir = Split-Path -Parent $reportFullPath
    New-Item -ItemType Directory -Path $reportDir -Force | Out-Null

    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add("# Release Draft Validation") | Out-Null
    $lines.Add("") | Out-Null
    $lines.Add("| Check | Status | Details |") | Out-Null
    $lines.Add("| --- | --- | --- |") | Out-Null
    foreach ($check in $Checks) {
        $name = $check.Name.Replace("|", "\|")
        $details = $check.Details.Replace("|", "\|").Replace("`r", " ").Replace("`n", " ")
        $lines.Add("| $name | $($check.Status) | $details |") | Out-Null
    }

    Set-Content -LiteralPath $reportFullPath -Value $lines -Encoding UTF8
    Write-Host "Wrote release draft validation report: $reportFullPath"
}

Test-FlatpakDraft
Test-WingetDraft
Test-DebianDraft
Test-PowerShellSyntax "scripts/context-menu/windows/modern/Register-LinkForgeModernContextMenu.ps1"
Test-PowerShellSyntax "scripts/context-menu/windows/modern/Unregister-LinkForgeModernContextMenu.ps1"
Test-WindowsAppxManifest
Write-Report

$failures = @($Checks | Where-Object { $_.Status -eq "FAIL" })
foreach ($check in $Checks) {
    Write-Host "[$($check.Status)] $($check.Name): $($check.Details)"
}

if ($failures.Count -gt 0) {
    throw "$($failures.Count) release draft validation check(s) failed."
}
