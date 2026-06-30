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

## Platform Support

- Core link management and the CLI support Windows and Linux.
- The GUI is designed for Windows desktop environments and the Linux GNOME desktop environment.

## Project Structure

- `crates/linkforge-core`: Core link management logic.
- `crates/linkforge-cli`: Command-line interface entry point.
- `crates/linkforge-gui`: Graphical interface entry point.
