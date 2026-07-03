# Contributing

Thank you for contributing to LinkForge.

## Local Development

LinkForge is a Cargo workspace with separate crates for the core library, CLI, and GUI entry point.

### Common Commands

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

CI runs `cargo fmt -- --check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings` on Windows and Linux.

### CLI

Run the CLI locally with `cargo run -p linkforge-cli --` followed by a LinkForge command:

```bash
cargo run -p linkforge-cli -- link-count path/to/file
cargo run -p linkforge-cli -- same-file path/to/a path/to/b
cargo run -p linkforge-cli -- scan-groups path/to/root
cargo run -p linkforge-cli -- batch-hardlink --target-dir path/to/target path/to/source-a path/to/source-b
cargo run -p linkforge-cli -- batch-symlink --target-dir path/to/target --dry-run path/to/source
```

### GUI

Run the GUI locally with:

```bash
cargo run -p linkforge-gui
```

To test GUI context-menu launches without installing file-manager entries:

```bash
cargo run -p linkforge-gui -- --context-action link-count --paths path/to/file
cargo run -p linkforge-gui -- --context-action same-file --paths path/to/a path/to/b
```

The shared context-menu launch protocol for GUI-opening actions is:

```bash
linkforge-gui --context-action <action> --paths <path>...
```

Supported GUI-opening actions are `symlink`, `hardlink`, `same-file`, `link-count`, `siblings`, `scan-groups`, `clone-tree`, `drop-symlink`, and `drop-hardlink`. Context-menu entries also use `pick-source`; it succeeds silently after writing the picked-source state. GNOME Files writes picked-source state directly from the `nautilus-python` extension so follow-up menus can update without waiting on a hidden GUI launch. Drop actions start the Tauri WebView hidden, exit silently on clean success, and only show a lightweight Tauri-rendered dialog for conflicts, errors, renames, skips, failures, or cancellations. Apart from GNOME's direct `pick-source` state write, context-menu entries only launch the GUI and pass context; picked-source state, action names, and menu labels are shared through `linkforge-shared`, while actual batch link preflight and creation are handled by `linkforge-core`.

### Context Menu Integration

Context-menu entries do not implement LinkForge workflows themselves. They launch `linkforge-gui --context-action <action> --paths <path>...`, so install a packaged GUI first, or build a local GUI artifact and point the context-menu installer at it.

For local testing, build the GUI before installing either context-menu integration:

```bash
cargo build -p linkforge-gui
```

Windows Explorer context-menu entries are installed with `scripts/context-menu/windows/modern/Register-LinkForgeModernContextMenu.ps1` for the Windows 11 top-level menu. GNOME Files advanced entries are installed by the `linkforge-context-menu-gnome` crate.

#### Windows 11 Top-Level Menu

##### Install

Optionally remove an older local registration before installing:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/context-menu/windows/modern/Unregister-LinkForgeModernContextMenu.ps1
```

Then build the shell extension and register the menu:

```powershell
cargo build -p linkforge-context-menu-windows --target x86_64-pc-windows-msvc
powershell -ExecutionPolicy Bypass -File scripts/context-menu/windows/modern/Register-LinkForgeModernContextMenu.ps1
Start-Process explorer
```

The registration script defaults to debug artifacts. For release artifacts, build with `--release` and pass `-Configuration Release`. Use `-GuiExePath` or `-ShellExtDllPath` when testing custom artifact locations.

##### Test

Open a new Explorer window after registration. Right-click one file, exactly two files, one directory, and a directory background to confirm the `LinkForge` menu appears and launches the GUI. For the two-step workflow, pick one or more sources, then create symlinks or hard links in a target directory.

##### Uninstall

Remove the Windows 11 top-level context-menu entries when testing is done:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/context-menu/windows/modern/Unregister-LinkForgeModernContextMenu.ps1
Start-Process explorer
```

#### GNOME Files

##### Install

The GNOME extension requires `nautilus-python`, PyGObject, and Nautilus GI bindings for either `4.0` or `3.0`. Ubuntu 22.04 / Nautilus 42 provides the compatible Nautilus 3.0 namespace through `gir1.2-nautilus-3.0`; newer distributions may provide `gir1.2-nautilus-4.0`.

Optionally remove an older local extension before installing:

```bash
cargo run -p linkforge-context-menu-gnome -- uninstall
```

Then install the extension, pointing it at the local GUI artifact when `linkforge-gui` is not already installed on `PATH`:

```bash
cargo run -p linkforge-context-menu-gnome -- install --gui-exe target/debug/linkforge-gui
nautilus -q
```

When GUI checking is enabled, the installer writes the resolved absolute GUI executable path into the Nautilus extension so GNOME Files can launch LinkForge from any working directory. Use `--skip-gui-check` only for packaging or special environments where the GUI path is expected to become valid later.

##### Test

After restarting GNOME Files, select one file, exactly two files, one directory, and a directory background to confirm the `LinkForge` menu appears and launches the GUI. Also test picking one or more sources and dropping them into a target folder.

##### Uninstall

Remove the GNOME Files extension when testing is done:

```bash
cargo run -p linkforge-context-menu-gnome -- uninstall
nautilus -q
```

### Shell Completions

To manually test the installed CLI and PowerShell completions on Windows:

```powershell
cargo install --path crates/linkforge-cli --force
linkforge help
linkforge help symlink
$completion = linkforge completions powershell | Out-String
Invoke-Expression $completion
linkforge <Tab>
cargo uninstall linkforge-cli
```

Use `Out-String` when loading generated PowerShell completions into the current session. Piping directly to `Invoke-Expression` can pass empty lines as empty commands.

The completion command prints scripts to stdout and does not modify shell profiles. If you append completions to `$PROFILE` or another shell startup file during manual testing, remove those lines after testing to restore the local environment.

### Batch Link Commands

To manually test CLI batch creation:

```bash
cargo run -p linkforge-cli -- batch-hardlink --target-dir path/to/target path/to/file-a path/to/file-b
cargo run -p linkforge-cli -- batch-hardlink --target-dir path/to/target --on-conflict rename path/to/file
cargo run -p linkforge-cli -- batch-symlink --target-dir path/to/target --dry-run path/to/file
```

Batch commands run the same core preflight used by GUI file-manager drops. The default conflict policy is `fail`; use `overwrite`, `rename`, or `skip` only when that behavior is intentional.

### Before Submitting

Before submitting changes, run `cargo fmt`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`.

On Windows, symbolic link creation without administrator privileges requires Windows Developer Mode. LinkForge asks Windows to allow unprivileged symlink creation, but Windows still rejects the request when Developer Mode is disabled and the process is not elevated. Tests account for missing symlink privileges, but manual symlink commands can still fail with the operating system permission error.

### Release Preparation

Use `scripts/set-version.ps1 <version>` or `scripts/set-version.sh <version>` to update workspace Cargo package versions, local path dependency versions, the Tauri GUI version, and the Windows sparse-package manifest version. The Windows sparse-package version is derived as `major.minor.patch.0`; SemVer pre-release and build metadata are intentionally not included in that four-part Windows version.

Release and packaging work must follow the preparation-only policy in `docs/release/release-checklist.md` and `packaging/README.md`. Draft manifests and local validation commands are allowed; public package-manager submissions are blocked until explicit 1.0 release approval.

## Git Commit Message

Git commit messages in this project must follow the Conventional Commits specification.

Recommended format:

```gitcommit
<type>(optional scope): <description>
```

Common types include:

- `feat`: A new feature.
- `fix`: A bug fix.
- `docs`: Documentation changes.
- `refactor`: Code restructuring that does not change external behavior.
- `test`: Test-related changes.
- `chore`: Build, tooling, dependency, or other maintenance changes.

Examples:

```gitcommit
feat(cli): add symlink creation command
fix(core): handle existing target path on Windows
docs: update platform support notes
```
