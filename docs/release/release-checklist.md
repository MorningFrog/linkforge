# Local Release Checklist

This checklist intentionally stops before public package-manager submission. Do not upload to winget-pkgs, Flathub, Debian, Ubuntu PPA, Fedora, openSUSE, AUR, or another public repository without explicit 1.0 release approval.

## 1. Clean Source

- [ ] Start from a clean worktree.
- [ ] Check out the intended release tag, for example `v0.1.0`.
- [ ] Confirm `README.md`, `CHANGELOG.md`, `TODO.md`, `SECURITY.md`, `PRIVACY.md`, and `docs/release/identity.md` match the release.
- [ ] Run `scripts/set-version.ps1 <version> -NoLock` on Windows or `scripts/set-version.sh <version> --no-lock` on Linux, then refresh `Cargo.lock` once.

## 2. Required Checks

- [ ] `cargo fmt -- --check`
- [ ] `cargo test`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] Windows Explorer smoke test recorded in `docs/release/context-menu-smoke-tests.md`.
- [ ] GNOME Files smoke test recorded in `docs/release/context-menu-smoke-tests.md`.

## 3. Windows Artifacts

- [ ] Build release CLI: `cargo build -p linkforge-cli --release`.
- [ ] Build release GUI / Tauri bundle.
- [ ] Build release shell extension: `cargo build -p linkforge-context-menu-windows --target x86_64-pc-windows-msvc --release`.
- [ ] Produce a Tauri NSIS x64 installer that installs CLI, GUI, shell-extension DLL, icons/resources, and registers the Windows 11 context menu.
- [ ] Sign `linkforge.exe`, `linkforge-gui.exe`, `linkforge_context_menu_windows.dll`, and the installer with Authenticode and timestamping.
- [ ] Verify signatures in CI or a clean Windows validation machine.
- [ ] Generate SHA256 checksums.
- [ ] Fill in `packaging/winget/` installer URL and SHA256 from the release artifact.
- [ ] Validate locally with `winget validate packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0`, `winget install --manifest packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0 --silent`, upgrade, uninstall, and Windows Sandbox smoke tests.

## 4. Linux Native Artifacts

- [ ] Create a network-independent release source tarball with vendored Cargo dependencies and Cargo offline config, keeping generated `vendor/` and `.cargo/` files out of normal commits.
- [ ] Build Debian source package from the repository root using the root `debian/` metadata.
- [ ] Build in a clean chroot with `sbuild` or equivalent.
- [ ] Run `lintian`.
- [ ] Validate install, upgrade, remove, shell completions, desktop file, AppStream metadata, and GNOME context-menu smoke tests.
- [ ] If approved, dry-run a private/test Ubuntu PPA upload only; do not upload to a public PPA.

## 5. Flatpak Draft

- [ ] Generate a vendored Cargo source directory and `.cargo/config.toml` for the Flatpak build, or replace the local source with generated Flatpak Cargo sources.
- [ ] Replace the local Flatpak `type: dir` source with a release tag archive and real SHA256 before public submission.
- [ ] Build/install/run with `flatpak-builder`.
- [ ] Run `flatpak-builder-lint manifest`, `flatpak-builder-lint repo`, and `appstreamcli validate`.
- [ ] Confirm sandbox file-link behavior and document limitations compared with native packages.

## 6. Dependency Inventory

- [ ] Generate `cargo metadata --locked --format-version 1` output for the release.
- [ ] Optionally generate an SBOM if the release adopts a specific SBOM tool.
- [ ] Attach checksums, signature verification notes, dependency inventory, and package validation notes to the draft release.
