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

The GUI exposes the same file-link management and inspection features through a Tauri desktop app. Shell completion generation remains a CLI-only helper.

## CLI Usage

```text
linkforge symlink <source> <link> [--force]
linkforge hardlink <source> <link> [--force]
linkforge same-file <path-a> <path-b>
linkforge link-count <path>
linkforge siblings <path> [--root <dir>]
linkforge scan-groups <root>
linkforge clone-tree <source-dir> <dest-dir> [--force]
linkforge completions <shell>
```

Commands that create links or clone directory trees fail when the destination already exists. Pass `--force` to replace an existing file or symbolic link; existing real directories are never replaced.

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

- Quick creation of symbolic links and hard links.
- Link count and same-file inspection.
- Hard-link sibling discovery, including scan-root selection on platforms that require it.
- Hard-link group scanning for a directory tree.
- Directory tree cloning while preserving internal hard-link relationships.

The GUI can also be launched by file-manager context menu entries. Windows 11 uses a modern Explorer command extension for the top-level context menu, Windows 10 and the Windows 11 classic menu use registry-based entries, and GNOME Files uses Nautilus scripts.

On Windows, the Explorer menu also supports a two-step link workflow: right-click a file or folder and choose `LinkForge > Pick Link Source`, then right-click a target folder or folder background and choose `Create Symlink from ...` or `Create Hard Link from ...`. The direct symlink and hard-link commands create the link without opening the full GUI. If a target name already exists, LinkForge asks whether to overwrite it, create an automatically renamed link, or cancel.

For local development, context-menu registration, and manual testing commands, see `CONTRIBUTING.md`.

### Windows Explorer Context Menu

Windows 11 top-level menu integration is implemented by `crates/linkforge-context-menu-windows`. It supports selected files, selected directories, and directory-background targets. The classic registry fallback appears under "Show more options" on Windows 11. LinkForge does not recommend globally restoring the legacy Windows context menu because that changes system-wide Explorer behavior.

### GNOME Files Context Menu

GNOME Files integration is implemented by `crates/linkforge-context-menu-gnome`, which installs and removes LinkForge Nautilus scripts.

## Platform Support

- Core link management and the CLI support Windows and Linux.
- The GUI is designed for Windows desktop environments and the Linux GNOME desktop environment.

## Project Structure

- `crates/linkforge-core`: Core link management logic.
- `crates/linkforge-cli`: Command-line interface entry point.
- `crates/linkforge-gui`: Graphical interface entry point.
- `crates/linkforge-context-menu-windows`: Windows Explorer command extension for the Windows 11 top-level context menu.
- `crates/linkforge-context-menu-gnome`: GNOME Files/Nautilus context-menu script installer.
- `scripts/context-menu`: Compatibility wrappers and Windows registry/sparse-package entry points.
