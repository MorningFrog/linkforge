# Contributing

Thank you for contributing to LinkForge.

## Local Development

LinkForge is a Cargo workspace with separate crates for the core library, CLI, and GUI entry point.

### Common Commands

```text
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

### CLI

Run the CLI locally with `cargo run -p linkforge-cli --` followed by a LinkForge command:

```text
cargo run -p linkforge-cli -- link-count path/to/file
cargo run -p linkforge-cli -- same-file path/to/a path/to/b
cargo run -p linkforge-cli -- scan-groups path/to/root
```

### GUI

Run the GUI locally with:

```text
cargo run -p linkforge-gui
```

To test GUI context-menu launches without installing file-manager entries:

```text
cargo run -p linkforge-gui -- --context-action link-count --paths path/to/file
cargo run -p linkforge-gui -- --context-action same-file --paths path/to/a path/to/b
```

The shared context-menu launch protocol is:

```text
linkforge-gui --context-action <action> --paths <path>...
```

Supported GUI-opening actions are `symlink`, `hardlink`, `same-file`, `link-count`, `siblings`, `scan-groups`, `clone-tree`, `drop-symlink`, and `drop-hardlink`. Context-menu entries also use `pick-source`; it succeeds silently after writing the picked-source state. Drop actions start the Tauri WebView hidden, exit silently on clean success, and only show a lightweight Tauri-rendered dialog for conflicts, errors, renames, skips, failures, or cancellations.

### Context Menu Integration

Windows Explorer context-menu entries are installed with `scripts/context-menu/windows/modern/Register-LinkForgeModernContextMenu.ps1` for the Windows 11 top-level menu. GNOME Files advanced entries are installed by the `linkforge-context-menu-gnome` crate.

#### Windows 11 Top-Level Menu

To locally test the Windows 11 top-level Explorer context menu:

```powershell
cargo build -p linkforge-gui
cargo build -p linkforge-context-menu-windows --target x86_64-pc-windows-msvc
powershell -ExecutionPolicy Bypass -File scripts/context-menu/windows/modern/Unregister-LinkForgeModernContextMenu.ps1
powershell -ExecutionPolicy Bypass -File scripts/context-menu/windows/modern/Register-LinkForgeModernContextMenu.ps1
Start-Process explorer
```

If a previous registration attempt appears stuck, stop it with `Ctrl+C` before rerunning the command.

Explorer usually notices new per-user context-menu entries in newly opened windows. After changing the GUI, command extension, or sparse-package scripts, rebuild both crates, unregister/register the sparse package again, and test from a newly opened Explorer window so stale COM/DLL state is not reused. If the menu does not appear or old behavior persists, close existing Explorer windows and open a new one. Restart Explorer only as a last resort because it can disrupt open file-manager windows and the desktop shell.

If registration fails with `0x80073D2E`, check that the generated manifest contains `<uap10:AllowExternalContent>true</uap10:AllowExternalContent>`. Sparse packages registered with `-ExternalLocation` must explicitly allow external content.

If registration fails with `0x80073CFF`, enable Developer Mode in Windows Settings under `Settings > System > Advanced > Developer Mode`, then rerun the script. Windows requires Developer Mode or app sideloading to register the sparse package used by the Windows 11 top-level menu.

If registration fails with `0x80070057` and says `x-generate` is not a valid language, update the modern registration script so the manifest uses a concrete resource language such as `en-us`.

Right-click a file to test `Pick Link Source`, `Create Symbolic Link`, `Create Hard Link`, `Show Link Count`, and `Find Hard Link Siblings`. Select exactly two files to test `Compare Same File`. Select multiple files or folders to test `Pick N Link Sources`. Right-click a directory to test `Pick Link Source`, `Create Symlink from ...`, `Create Hard Link from ...`, `Create Symbolic Link`, `Find Hard Link Siblings`, `Scan Hard Link Groups`, and `Clone Tree Preserving Hard Links`. Right-click a directory background to confirm `LinkForge` expands with `Create Symlink from ...` and `Create Hard Link from ...` instead of an empty submenu.

For the two-step workflow, first pick one or more files or folders as sources. Then right-click one target folder or folder background and create symlinks or hard links from the picked sources. Directory sources in a hard-link drop create a hard-link tree: regular files become hard links to the source files and symbolic links are copied as links. A clean drop should create the links and exit without showing any LinkForge window. If a target name already exists, test that only the lightweight Tauri-rendered conflict dialog appears, without the full LinkForge shell, and that it supports rename, overwrite, skip, cancel, applying one choice to remaining conflicts, and `Open LinkForge`. After a non-clean batch, confirm the lightweight summary dialog appears; after clicking `Open LinkForge`, confirm the same window expands into the full LinkForge interface instead of launching a second process.

Remove the Windows 11 top-level context-menu entries after testing:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/context-menu/windows/modern/Unregister-LinkForgeModernContextMenu.ps1
Start-Process explorer
```

#### GNOME Files

To locally test GNOME Files advanced context-menu extension installation:

```text
cargo run -p linkforge-context-menu-gnome -- install
cargo run -p linkforge-context-menu-gnome -- uninstall
```

Pass `--gui-exe /path/to/linkforge-gui` to `install` when `linkforge-gui` is not on `PATH`.

The GNOME extension requires `nautilus-python`. Restart GNOME Files with `nautilus -q` after installing or uninstalling if the menu does not refresh.
After installing, select exactly two files in GNOME Files to test `Compare Same File` from the LinkForge advanced menu.
Also test selecting multiple files or folders, choosing `Pick N Link Sources`, and dropping them into one target folder with `Create Symlink...` or `Create Hard Link...`. Clean drops should exit silently. If a target name already exists or a non-clean result occurs, confirm the lightweight Tauri-rendered dialog matches the Windows flow.

The compatibility wrappers under `scripts/context-menu/gnome` delegate to the GNOME context-menu crate:

```text
scripts/context-menu/gnome/install-gnome-extension.sh
scripts/context-menu/gnome/uninstall-gnome-extension.sh
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

### Before Submitting

Before submitting changes, run `cargo fmt`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`.

On Windows, symbolic link creation without administrator privileges requires Windows Developer Mode. LinkForge asks Windows to allow unprivileged symlink creation, but Windows still rejects the request when Developer Mode is disabled and the process is not elevated. Tests account for missing symlink privileges, but manual symlink commands can still fail with the operating system permission error.

## Git Commit Message

Git commit messages in this project must follow the Conventional Commits specification.

Recommended format:

```text
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

```text
feat(cli): add symlink creation command
fix(core): handle existing target path on Windows
docs: update platform support notes
```
