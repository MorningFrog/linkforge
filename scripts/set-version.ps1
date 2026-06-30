param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version,

    [switch]$DryRun,
    [switch]$NoLock
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$semverPattern = '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$'
if ($Version -notmatch $semverPattern) {
    throw "Invalid version '$Version'. Expected SemVer, for example: 1.2.3, 1.2.3-beta.1, or 1.2.3+build.4."
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$rootManifest = Join-Path $repoRoot "Cargo.toml"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo was not found in PATH."
}

Push-Location $repoRoot
try {
    $metadataJson = & cargo metadata --no-deps --format-version 1 --manifest-path $rootManifest
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed."
    }

    $metadata = $metadataJson | ConvertFrom-Json
    $workspaceMemberIds = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($id in $metadata.workspace_members) {
        [void]$workspaceMemberIds.Add([string]$id)
    }

    $workspacePackages = @(
        $metadata.packages |
            Where-Object { $workspaceMemberIds.Contains([string]$_.id) } |
            Sort-Object manifest_path
    )

    if ($workspacePackages.Count -eq 0) {
        throw "No workspace packages found."
    }

    $packageNames = @($workspacePackages | ForEach-Object { [regex]::Escape([string]$_.name) })
    $localDependencyPattern = "(?m)^(\s*(?:$($packageNames -join '|'))\s*=\s*\{[^\r\n]*\bpath\s*=[^\r\n]*\})"
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    $changedManifests = New-Object System.Collections.Generic.List[string]

    foreach ($package in $workspacePackages) {
        $manifestPath = [string]$package.manifest_path
        $content = [System.IO.File]::ReadAllText($manifestPath)

        $updated = [regex]::Replace(
            $content,
            '(?m)^(version\s*=\s*)"[^"]+"',
            "`${1}`"$Version`"",
            1
        )

        $updated = [regex]::Replace(
            $updated,
            $localDependencyPattern,
            {
                param($match)
                [regex]::Replace(
                    $match.Value,
                    '\bversion\s*=\s*"[^"]+"',
                    "version = `"$Version`""
                )
            }
        )

        if ($updated -ne $content) {
            $relativePath = Resolve-Path -Relative $manifestPath
            [void]$changedManifests.Add($relativePath)

            if (-not $DryRun) {
                [System.IO.File]::WriteAllText($manifestPath, $updated, $utf8NoBom)
            }
        }
    }

    if ($changedManifests.Count -eq 0) {
        Write-Host "All workspace manifests already use version $Version."
    } else {
        $mode = if ($DryRun) { "Would update" } else { "Updated" }
        foreach ($manifest in $changedManifests) {
            Write-Host "$mode $manifest"
        }
    }

    if (-not $DryRun -and -not $NoLock) {
        & cargo generate-lockfile --manifest-path $rootManifest
        if ($LASTEXITCODE -ne 0) {
            throw "cargo generate-lockfile failed."
        }

        Write-Host "Refreshed Cargo.lock"
    } elseif ($DryRun) {
        Write-Host "Dry run only; no files were changed."
    }
} finally {
    Pop-Location
}
