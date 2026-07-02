# LinkForge

LinkForge is a tool for quickly creating and managing file symbolic links and hard links. It is designed to reduce the cost of manually maintaining links in everyday development and file organization workflows.

## Feature Overview

LinkForge currently supports these core and CLI features:

- Creating symbolic links for files and directories.
- Creating hard links for files.
- Checking whether two paths point to the same underlying file.
- Showing a file's hard link count.
- Showing the sibling paths that are hard links to the same file. On Linux, this may require scanning one or more filesystem trees because filesystems generally do not maintain a direct reverse index from an inode to every pathname that references it.
- Scanning a directory tree to find hard link groups.
- Cloning a directory tree while preserving hard link relationships, so files that were hard-linked in the source remain hard-linked in the clone.
- Creating a hard-link directory tree for a source directory, with regular files hard-linked and symbolic links copied as links.
- Batch creation of symbolic links or hard links into a target directory, with dry-run preflight and configurable conflict handling.

The GUI exposes the same file-link management and inspection features through a Tauri desktop app and reuses the same core batch-link workflow as the CLI. Shell completion generation remains a CLI-only helper.

## CLI Usage

```text
linkforge symlink <source> <link> [--force]
linkforge hardlink <source> <link> [--force]
linkforge same-file <path-a> <path-b>
linkforge link-count <path>
linkforge siblings <path> [--root <dir>]
linkforge scan-groups <root>
linkforge clone-tree <source-dir> <dest-dir> [--force]
linkforge batch-symlink --target-dir <dir> [--dry-run] [--on-conflict fail|overwrite|rename|skip] <sources>...
linkforge batch-hardlink --target-dir <dir> [--dry-run] [--on-conflict fail|overwrite|rename|skip] <sources>...
linkforge completions <shell>
```

Commands that create links or clone directory trees fail when the destination already exists. Pass `--force` to replace an existing file or symbolic link; existing real directories are never replaced.
Batch commands create one link per source in the target directory. They run a preflight before creating links, support `--dry-run`, and default to `--on-conflict fail`; use `rename`, `overwrite`, or `skip` to choose a different conflict policy. `batch-hardlink` creates a hard-link directory tree when a source is a directory.

Run `linkforge help` to list commands, or `linkforge help <command>` to show help for a specific command.

The `clone-tree` command copies the source directory tree and preserves hard link relationships inside the clone. Symbolic links are copied as links rather than followed.

On Windows, `siblings` can enumerate sibling hard-link paths directly through the operating system. On Linux, pass `--root <dir>` to scan the selected directory tree for sibling hard links.

On Windows, creating symbolic links without administrator privileges requires Windows Developer Mode. If Developer Mode is disabled and the process is not elevated, the operating system will reject symlink creation.

## Shell Completions

LinkForge can generate shell completion scripts for PowerShell, Bash, Zsh, and Fish. The command prints the script to stdout so you can install it wherever your shell expects completions.

```powershell
$completion = linkforge completions powershell | Out-String
Invoke-Expression $completion
```

```bash
linkforge completions bash
linkforge completions zsh
linkforge completions fish
```

## GUI Usage

The GUI supports:

- Quick creation of symbolic links, file hard links, and directory hard-link trees.
- Link count and same-file inspection.
- Hard-link sibling discovery, including scan-root selection on platforms that require it.
- Hard-link group scanning for a directory tree.
- Directory tree cloning while preserving internal hard-link relationships.

GUI browse buttons use native system file, directory, and save dialogs for source, scan-root, link-path, and clone-destination selection.

The GUI can also be launched by file-manager context menu entries. Windows 11 uses a modern Explorer command extension for the top-level context menu, and GNOME Files uses a `nautilus-python` extension.

The file-manager menus also support a two-step link workflow: right-click one or more files or folders and choose `LinkForge > Pick Link Source`, then right-click a single target folder or folder background and choose `Create Symlink from ...` or `Create Hard Link from ...`. Drop actions start LinkForge hidden, preflight sources and the target directory before creating links, perform clean batches silently, and only show a lightweight Tauri-rendered dialog when a conflict, warning, error, skip, rename, cancellation, or failure needs attention. For picked directories, hard-link creation builds a directory tree whose regular files are hard links to the source files and whose symbolic links are copied as links. If preflight finds target-name conflicts or hard-link warnings, LinkForge shows a review dialog before creating any links; remaining conflicts can still be resolved by renaming, overwriting, skipping, or cancelling in the lightweight dialog, with an option to apply the choice to remaining conflicts and a button to expand the current window into the full LinkForge interface.
This two-step workflow uses the same core batch-link preflight and conflict handling as the CLI batch commands.

For local development, context-menu registration, and manual testing commands, see `CONTRIBUTING.md`.

### Context Menu Behavior

LinkForge maintains two context-menu integrations: Windows 11 modern and GNOME Files advanced. Both use a `LinkForge` menu and launch `linkforge-gui --context-action <action> --paths <path>...`.
Both integrations show `Compare Same File` when exactly two files are selected; this opens the Inspect view and runs the same-file comparison automatically.

| Target | Windows 11 modern | GNOME Files advanced |
| --- | --- | --- |
| File | `Pick Link Source`, `Open Symlink in LinkForge...`, `Open Hard Link in LinkForge...`, `Show Link Count`, `Find Hard Link Siblings...` | Same items under `LinkForge` |
| Multiple files/folders | `Pick N Link Sources` | Same item under `LinkForge` |
| Two files | `Compare Same File` | Same item under `LinkForge` |
| Directory | `Pick Link Source`, `Create Symlink(s) from ...`, `Create Hard Link(s) from ...`, `Open Symlink in LinkForge...`, `Find Hard Link Siblings...`, `Scan Hard Link Groups`, `Clone Tree Preserving Hard Links...` | Same dynamic items under `LinkForge` |
| Directory background | `Create Symlink from ...`, `Create Hard Link from ...` | Same dynamic items under `LinkForge` |

Windows 11 modern can dynamically hide unsupported items and include the picked source name or source count in menu labels. GNOME Files advanced dynamically builds its menu through `nautilus-python`; it requires `nautilus-python` and may need `nautilus -q` after installation. Both integrations preflight drop batches and route conflicts, warnings, errors, and non-clean completion summaries through LinkForge's lightweight Tauri-rendered dialogs instead of platform-native message boxes; clean drop batches exit silently.

### Windows Explorer Context Menu

Windows 11 top-level menu integration is implemented by `crates/linkforge-context-menu-windows`. It supports selected files, selected directories, and directory-background targets.

### GNOME Files Context Menu

GNOME Files integration is implemented by `crates/linkforge-context-menu-gnome`, which installs and removes the LinkForge `nautilus-python` extension. Nautilus scripts are not installed as a fallback.

## Platform Support

- Core link management and the CLI support Windows and Linux.
- The GUI is designed for Windows desktop environments and the Linux GNOME desktop environment.

## Project Structure

- `crates/linkforge-core`: Core link management logic.
- `crates/linkforge-shared`: Shared desktop integration protocol helpers, including context action names and picked-source state.
- `crates/linkforge-cli`: Command-line interface entry point.
- `crates/linkforge-gui`: Graphical interface entry point.
- `crates/linkforge-context-menu-windows`: Windows Explorer command extension for the Windows 11 top-level context menu.
- `crates/linkforge-context-menu-gnome`: GNOME Files/Nautilus `nautilus-python` context-menu extension installer.
- `scripts/context-menu/windows/modern`: Windows sparse-package registration scripts.
