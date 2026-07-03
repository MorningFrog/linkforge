#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
report_path="${REPORT_PATH:-target/release-validation/release-draft-checks.md}"
windows_appx_manifest="${WINDOWS_APPX_MANIFEST:-target/linkforge-modern-context-menu/AppxManifest.xml}"
require_windows_appx="${REQUIRE_WINDOWS_APPX:-0}"
failure_count=0

mkdir -p "$repo_root/$(dirname "$report_path")"
{
  echo "# Release Draft Validation"
  echo
  echo "| Check | Status | Details |"
  echo "| --- | --- | --- |"
} >"$repo_root/$report_path"

escape_md() {
  printf '%s' "$1" | tr '\n\r' '  ' | sed 's/|/\\|/g'
}

add_check() {
  local name="$1"
  local status="$2"
  local details="$3"
  printf '[%s] %s: %s\n' "$status" "$name" "$details"
  printf '| %s | %s | %s |\n' "$(escape_md "$name")" "$status" "$(escape_md "$details")" >>"$repo_root/$report_path"
  if [ "$status" = "FAIL" ]; then
    failure_count=$((failure_count + 1))
  fi
}

run_with_timeout() {
  local timeout_seconds="$1"
  shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "$timeout_seconds" "$@"
  else
    "$@"
  fi
}

require_file() {
  local relative_path="$1"
  local name="$2"
  if [ -f "$repo_root/$relative_path" ]; then
    add_check "$name" "PASS" "$relative_path"
    return 0
  fi
  add_check "$name" "FAIL" "Missing $relative_path"
  return 1
}

assert_contains() {
  local file="$1"
  local needle="$2"
  local name="$3"
  local details="$4"
  if grep -Fq "$needle" "$repo_root/$file"; then
    add_check "$name" "PASS" "$details"
  else
    add_check "$name" "FAIL" "Missing expected text: $needle"
  fi
}

require_file "packaging/flatpak/io.github.morningfrog.LinkForge.yml" "Flatpak manifest exists" || true
require_file "packaging/flatpak/io.github.morningfrog.LinkForge.desktop" "Flatpak desktop file exists" || true
require_file "packaging/flatpak/io.github.morningfrog.LinkForge.metainfo.xml" "Flatpak MetaInfo exists" || true
require_file "packaging/flatpak/io.github.morningfrog.LinkForge.svg" "Flatpak icon exists" || true
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.yml" "app-id: io.github.morningfrog.LinkForge" "Flatpak app id" "App ID matches release identity"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.yml" "runtime: org.gnome.Platform" "Flatpak runtime" "Uses GNOME runtime"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.yml" "runtime-version: \"50\"" "Flatpak runtime version" "Uses the current GNOME runtime branch"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.yml" "org.freedesktop.Sdk.Extension.rust-stable" "Flatpak Rust SDK extension" "Rust toolchain is available inside the SDK"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.yml" "cargo build --release --locked --offline" "Flatpak offline build command" "Build remains locked and offline"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.yml" "type: dir" "Flatpak local source marker" "Draft uses local source and must be replaced before public submission"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.desktop" "Exec=linkforge-gui" "Desktop Exec" "Desktop file launches the GUI"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.desktop" "Icon=io.github.morningfrog.LinkForge" "Desktop icon id" "Icon ID matches App ID"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.metainfo.xml" "<id>io.github.morningfrog.LinkForge</id>" "AppStream id" "MetaInfo ID matches App ID"
assert_contains "packaging/flatpak/io.github.morningfrog.LinkForge.metainfo.xml" "<launchable type=\"desktop-id\">io.github.morningfrog.LinkForge.desktop</launchable>" "AppStream launchable" "Launchable desktop ID is present"

if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$repo_root/packaging/flatpak/io.github.morningfrog.LinkForge.desktop"
  add_check "desktop-file-validate" "PASS" "Desktop metadata validates"
else
  add_check "desktop-file-validate" "WARN" "desktop-file-validate is not installed"
fi

if command -v appstreamcli >/dev/null 2>&1; then
  appstream_timeout="${APPSTREAMCLI_TIMEOUT_SECONDS:-60}"
  appstream_output=""
  appstream_status=0
  appstream_output="$(run_with_timeout "$appstream_timeout" appstreamcli validate "$repo_root/packaging/flatpak/io.github.morningfrog.LinkForge.metainfo.xml" 2>&1)" || appstream_status=$?
  if [ "$appstream_status" -eq 0 ]; then
    add_check "appstreamcli validate" "PASS" "AppStream metadata validates"
  elif [ "$appstream_status" -eq 124 ]; then
    add_check "appstreamcli validate" "FAIL" "Timed out after ${appstream_timeout}s"
  else
    add_check "appstreamcli validate" "FAIL" "$appstream_output"
  fi
else
  add_check "appstreamcli validate" "WARN" "appstreamcli is not installed"
fi

require_file "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.yaml" "winget version manifest exists" || true
require_file "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.locale.en-US.yaml" "winget locale manifest exists" || true
require_file "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.installer.yaml" "winget installer manifest exists" || true
assert_contains "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.installer.yaml" "InstallerType: nullsoft" "winget installer type" "NSIS/nullsoft installer strategy"
assert_contains "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.installer.yaml" "Silent: \"/S\"" "winget silent switch" "Silent switch is recorded"
assert_contains "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.installer.yaml" "SilentWithProgress: \"/S\"" "winget silent progress switch" "Silent-with-progress switch is recorded"
if grep -Eq 'InstallerSha256: "0{64}"' "$repo_root/packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.installer.yaml"; then
  add_check "winget SHA256" "WARN" "Placeholder hash remains; install validation must wait for a real artifact"
else
  assert_contains "packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0/MorningFrog.LinkForge.installer.yaml" "InstallerSha256:" "winget SHA256" "Installer SHA256 is present"
fi
assert_contains "packaging/winget/README.md" "UninstallSilent: /S" "winget uninstall switch note" "Uninstall switch is documented for local validation"
assert_contains "packaging/winget/README.md" "Windows Sandbox" "winget sandbox validation note" "Clean-machine validation is documented"

require_file "debian/control" "Debian control exists" || true
require_file "debian/rules" "Debian rules exists" || true
require_file "debian/linkforge-cli.install" "Debian CLI install file exists" || true
require_file "debian/linkforge-gui.install" "Debian GUI install file exists" || true
require_file "debian/linkforge-context-menu-gnome.install" "Debian GNOME install file exists" || true
assert_contains "debian/control" "Package: linkforge-cli" "Debian CLI package" "CLI package is declared"
assert_contains "debian/control" "Package: linkforge-gui" "Debian GUI package" "GUI package is declared"
assert_contains "debian/control" "Package: linkforge-context-menu-gnome" "Debian GNOME package" "GNOME integration package is declared"
assert_contains "debian/rules" "cargo build --release --locked --offline" "Debian offline build" "Debian build remains locked and offline"
assert_contains "debian/rules" "cargo test --locked --offline" "Debian offline tests" "Debian tests remain locked and offline"

if [ -f "$repo_root/$windows_appx_manifest" ]; then
  assert_contains "$windows_appx_manifest" 'Identity Name="LinkForge.ContextMenu"' "Windows Appx identity" "Identity is present"
  assert_contains "$windows_appx_manifest" 'Executable="linkforge-gui.exe"' "Windows Appx executable" "Application launches linkforge-gui.exe"
  assert_contains "$windows_appx_manifest" 'Path="linkforge_context_menu_windows.dll"' "Windows Appx COM server" "COM class points to the shell extension DLL"
elif [ "$require_windows_appx" = "1" ]; then
  add_check "Windows Appx manifest" "FAIL" "Missing $windows_appx_manifest"
else
  add_check "Windows Appx manifest" "WARN" "Not found; run Register-LinkForgeModernContextMenu.ps1 -PrepareOnly to generate it"
fi

printf 'Wrote release draft validation report: %s\n' "$repo_root/$report_path"

if [ "$failure_count" -gt 0 ]; then
  printf '%s release draft validation check(s) failed.\n' "$failure_count" >&2
  exit 1
fi
