# Context Menu Smoke Tests

Record real end-to-end results here for each release candidate. Do not mark a test as passed unless it was run on the target file manager.

## Windows Explorer Modern Menu

Environment:

- Date:
- Windows version:
- LinkForge version:
- Installer or build artifact:
- Tester:

| Scenario | Result | Notes |
| --- | --- | --- |
| Register modern sparse package from release artifact | Not run | |
| Right-click one file and pick source | Not run | |
| Right-click multiple files/folders and pick N sources | Not run | |
| Drop picked source into a target folder as symlink | Not run | |
| Drop picked source into a target folder as hard link | Not run | |
| Drop picked directory as hard-link tree | Not run | |
| Existing target conflict opens lightweight dialog | Not run | |
| Rename / review each conflict flow works | Not run | |
| Overwrite, skip, cancel, and apply-to-remaining choices work | Not run | |
| Clean drop exits silently with no full GUI | Not run | |
| Non-clean result shows lightweight summary | Not run | |
| Open LinkForge expands lightweight window into full UI | Not run | |
| Compare Same File appears for exactly two files | Not run | |
| Show Link Count works for a file | Not run | |
| Find Hard Link Siblings works | Not run | |
| Scan Hard Link Groups works for a directory | Not run | |
| Clone Tree Preserving Hard Links opens clone view | Not run | |
| Directory background drop actions appear only with picked sources | Not run | |
| Unregister removes Explorer menu | Not run | |

## GNOME Files Advanced Menu

Environment:

- Date:
- Distribution and version:
- GNOME Files version:
- `nautilus-python` package:
- LinkForge version:
- Installer or build artifact:
- Tester:

| Scenario | Result | Notes |
| --- | --- | --- |
| Install GNOME Files extension from native package or local build | Not run | |
| Verify extension with configured GUI executable | Not run | |
| Restart GNOME Files with `nautilus -q` | Not run | |
| Right-click one file and pick source | Not run | |
| Right-click multiple files/folders and pick N sources | Not run | |
| Drop picked source into a target folder as symlink | Not run | |
| Drop picked source into a target folder as hard link | Not run | |
| Drop picked directory as hard-link tree | Not run | |
| Existing target conflict opens lightweight dialog | Not run | |
| Rename / review each conflict flow works | Not run | |
| Overwrite, skip, cancel, and apply-to-remaining choices work | Not run | |
| Clean drop exits silently with no full GUI | Not run | |
| Non-clean result shows lightweight summary | Not run | |
| Open LinkForge expands lightweight window into full UI | Not run | |
| Compare Same File appears for exactly two files | Not run | |
| Show Link Count works for a file | Not run | |
| Find Hard Link Siblings works, with scan root where required | Not run | |
| Scan Hard Link Groups works for a directory | Not run | |
| Clone Tree Preserving Hard Links opens clone view | Not run | |
| Directory background drop actions appear only with picked sources | Not run | |
| Uninstall removes GNOME Files menu | Not run | |

## GNOME Files WSLg Precheck

Environment:

- Date: 2026-07-03T15:13:54+08:00
- Scope: WSLg precheck only; this does not replace the native Ubuntu GNOME Files release gate.
- Distribution and version: Ubuntu 24.04.4 LTS on WSL2, kernel `6.18.33.2-microsoft-standard-WSL2`.
- GNOME Files version: Nautilus 46.4.
- `nautilus-python` package: `python3-nautilus` 4.0-1build4, `gir1.2-nautilus-4.0` 1:46.4-0ubuntu0.2, `python3-gi` 3.48.2-1.
- Toolchain: rustup `cargo 1.96.1` and `rustc 1.96.1`; apt `cargo`/`rustc` 1.75.0 were installed only to satisfy Debian build dependency checks.
- LinkForge artifact: source worktree build with `target/debug/linkforge-gui`, `target/debug/linkforge-context-menu-gnome`, and generated WSL test data at `/home/ma/linkforge-gnome-smoke-20260703-135533`.
- Source worktree status: dirty release-preparation worktree with packaging/docs changes plus ignored generated `.cargo/`, `vendor/`, `.cargo-home/`, `target/`, and Debian/Flatpak staging artifacts; package validation used ext4 copies under `/home/ma` to avoid DrvFS permission noise.
- Tester: Codex-assisted local precheck.

Automated checks:

| Check | Result | Notes |
| --- | --- | --- |
| WSL environment probe | Pass | Confirmed Ubuntu 24.04.4 LTS, WSL2, Nautilus 46.4, Python 3.12.3, WSLg display variables, `python3-nautilus`, Python GI, and Nautilus GI 4.0. |
| Rust quality gate | Pass | `cargo fmt -- --check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings` passed in WSL. |
| Release draft validator | Pass | `bash scripts/validate-release-drafts.sh` passed with the expected winget SHA256 placeholder warning. |
| GUI and GNOME integration builds | Pass | `cargo build -p linkforge-gui` and `cargo build -p linkforge-context-menu-gnome` passed. |
| GNOME extension install and verify | Pass | `cargo run -p linkforge-context-menu-gnome -- install --gui-exe target/debug/linkforge-gui` and `verify` passed; extension installed at `/home/ma/.local/share/nautilus-python/extensions/linkforge.py` with the resolved GUI path `/mnt/c/MyData/Projects/rust_projects/linkforge/target/debug/linkforge-gui`. |
| Nautilus restart and launch | Pass | `nautilus -q` and `nautilus /home/ma/linkforge-gnome-smoke-20260703-135533` launched under WSLg with WSLg/Mesa and missing-bookmarks warnings only. |
| Test data creation | Pass | Sources, targets, hard-link groups, clone-tree fixtures, and symlink fixture were created under ext4 home; hard-link groups and symlink targets were verified. |
| Desktop and AppStream metadata | Pass | `desktop-file-validate packaging/flatpak/io.github.morningfrog.LinkForge.desktop` passed and `appstreamcli validate packaging/flatpak/io.github.morningfrog.LinkForge.metainfo.xml` reported successful validation with one pedantic note. |

Menu workflow precheck:

| Scenario | Result | Notes |
| --- | --- | --- |
| Install GNOME Files extension from local build | Pass | User-level extension install and verify passed before launching Nautilus. |
| Verify extension with configured GUI executable | Pass | Marker checks found `class LinkForgeMenuProvider`, `drop-symlink`, `drop-hardlink`, `Compare Same File`, `--context-background`, and the configured GUI path. |
| Restart GNOME Files with `nautilus -q` | Pass | Nautilus restarted and opened the WSLg smoke directory. |
| Right-click one file and pick source | Blocked | Requires human confirmation in the open Nautilus WSLg window; not marked passed from automation. |
| Right-click multiple files/folders and pick N sources | Blocked | Requires human confirmation in the open Nautilus WSLg window. |
| Drop picked source into a target folder as symlink | Blocked | Requires human menu interaction; sandbox/CLI link creation was separately covered by automated tests and Flatpak smoke. |
| Drop picked source into a target folder as hard link | Blocked | Requires human menu interaction; hard-link creation was separately covered by automated tests and Flatpak smoke. |
| Drop picked directory as hard-link tree | Blocked | Requires human menu interaction. |
| Existing target conflict opens lightweight dialog | Blocked | Requires human menu interaction and visual dialog confirmation. |
| Rename / review each conflict flow works | Blocked | Requires human menu interaction and visual dialog confirmation. |
| Overwrite, skip, cancel, and apply-to-remaining choices work | Blocked | Requires human menu interaction and visual dialog confirmation. |
| Clean drop exits silently with no full GUI | Blocked | Requires human visual confirmation that no full GUI remains after a clean drop. |
| Non-clean result shows lightweight summary | Blocked | Requires human visual confirmation. |
| Open LinkForge expands lightweight window into full UI | Blocked | Requires human visual confirmation. |
| Compare Same File appears for exactly two files | Blocked | Requires human menu interaction. |
| Show Link Count works for a file | Blocked | Requires human menu interaction. |
| Find Hard Link Siblings works, with scan root where required | Blocked | Requires human menu interaction and scan-root selection in WSL/Linux. |
| Scan Hard Link Groups works for a directory | Blocked | Requires human menu interaction. |
| Clone Tree Preserving Hard Links opens clone view | Blocked | Requires human menu interaction and GUI confirmation. |
| Directory background drop actions appear only with picked sources | Blocked | Requires human menu interaction before and after picking sources. |
| Uninstall removes GNOME Files menu | Blocked | Not run because the extension was left installed for manual WSLg inspection; run `cargo run -p linkforge-context-menu-gnome -- uninstall && nautilus -q` after manual checks. |

Package and metadata follow-up from the same WSL host:

| Check | Result | Notes |
| --- | --- | --- |
| Debian vendored source config | Pass | `cargo vendor --versioned-dirs vendor > .cargo/config.toml` produced a 733 MB local vendor tree; `debian/rules` now preserves `vendor/` during `dh_clean` because vendored crates contain files such as `Cargo.toml.orig` and `src/target/*`. |
| Debian binary package build | Pass | `dpkg-buildpackage -us -uc -b` passed from an ext4 copy at `/home/ma/linkforge-package-build`; building directly under `/mnt/c` was blocked by DrvFS executable-bit semantics for debhelper config files. |
| Debian binary artifacts | Pass | Produced `/home/ma/linkforge-cli_0.1.0-0ubuntu1_amd64.deb`, `/home/ma/linkforge-gui_0.1.0-0ubuntu1_amd64.deb`, `/home/ma/linkforge-context-menu-gnome_0.1.0-0ubuntu1_amd64.deb`, `/home/ma/linkforge_0.1.0-0ubuntu1_all.deb`, `.buildinfo`, `.changes`, and dbgsym `.ddeb` files. |
| Debian lintian | Pass | `lintian --fail-on error /home/ma/linkforge_0.1.0-0ubuntu1_amd64.changes` exited 0; warnings remain for missing copyright notice text, missing man pages, and dbgsym packages with no debug symbols. |
| Debian install/remove lifecycle | Pass | `apt-get install` of the four local debs passed; CLI help, generated completions, desktop file, AppStream metadata, icon, and Nautilus extension file were verified; `apt-get purge` removed the packages and system Nautilus extension. |
| Debian source package build | Pass | A cleaned vendored source tarball was generated as `/home/ma/linkforge_0.1.0.orig.tar.xz` with SHA256 `7a86e919b5b0c41e7ab0360880f8a9dc22c8f54a8027e4f8ef842eea8bbf458d`; `debuild --no-lintian -S -us -uc` produced `/home/ma/linkforge_0.1.0-0ubuntu1.dsc` with SHA256 `286917799ea5b405eee0cf7716670dc8daccabdf8134eba523a9e057b986d724` and source `.changes`/`.buildinfo`. |
| Debian source lintian | Fail | `lintian --fail-on error /home/ma/linkforge_0.1.0-0ubuntu1_source.changes` emitted 30 `E:` tags and 8 `W:` tags in `/tmp/linkforge-source-lintian.log`; blockers include `source-is-missing` in vendored `html5ever`/Tauri JavaScript files and `unpack-message-for-orig` for vendored `winapi-*` static libraries. |
| Debian clean chroot build | Blocked | `sbuild` was not run because source lintian still has E-level blockers and no clean chroot is configured in this WSL environment. |
| Debian upgrade test | Blocked | No previous version artifact was available for a real upgrade path. |
| Flatpak metadata validators | Pass | Desktop file and AppStream metadata validators passed in the ext4 copy. |
| Flatpak build and repo export | Pass | `flatpak-builder --user --force-clean --install-deps-from=flathub --repo=repo build-dir packaging/flatpak/io.github.morningfrog.LinkForge.yml` passed after GNOME 50 runtime/sdk and Rust SDK extension installation; repo refs include `app/io.github.morningfrog.LinkForge/x86_64/master` and `runtime/io.github.morningfrog.LinkForge.Debug/x86_64/master`. |
| Flatpak bundle and install/run/uninstall | Pass | `flatpak build-bundle repo LinkForge.flatpak io.github.morningfrog.LinkForge` produced `/home/ma/LinkForge.flatpak` with SHA256 `d08d7d19119595884658d5dbeab27e6b1fe1e134c1d15386400aa0b714d3df2c`; local install, `flatpak run --command=linkforge ... --help`, WSLg GUI launch smoke, sandbox symlink creation, sandbox hard-link creation, `same-file`, `link-count`, `siblings --root`, `scan-groups`, `clone-tree`, and uninstall passed under `/home/ma/linkforge-flatpak-smoke-fixed`. |
| Flatpak linter | Fail | The Flathub linter was run through `org.flatpak.Builder`; manifest lint failed on `finish-args-home-filesystem-access`, and repo lint failed on `finish-args-home-filesystem-access`, `appstream-screenshots-not-mirrored-in-ostree`, and `metainfo-missing-screenshots`. |
