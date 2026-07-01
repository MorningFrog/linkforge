# Changelog

## 0.1.0 - Unreleased

- Added a Tauri GUI for symbolic link, hard link, same-file, link count, sibling discovery, hard-link group scanning, and hard-link-preserving clone workflows.
- Added Windows Explorer and GNOME Files context-menu launch helpers for GUI quick actions.
- Added a Windows 11 Explorer command extension registration path for top-level context-menu integration.
- Added a GNOME Files advanced menu extension through `nautilus-python`.
- Added a two-step Windows Explorer workflow for picking a link source and directly creating symlinks or hard links in a target folder without opening the full GUI.
- Added dedicated workspace crates for Windows and GNOME context-menu integration.
- Added CLI help descriptions and shell completion generation for PowerShell, Bash, Zsh, and Fish.
- Added Windows symbolic link creation support that requests unprivileged symlink creation when Developer Mode is enabled.
- Added core and CLI support for creating symbolic links and hard links.
- Added same-file checks, hard link count inspection, hard-link sibling discovery, and directory hard-link group scanning.
- Added directory tree cloning that copies files and symbolic links while preserving internal hard link relationships.
- Documented the LinkForge feature overview, including symbolic links, hard links, same-file checks, link counts, hard-link sibling discovery, hard-link group scanning, and hard-link-preserving directory cloning.
