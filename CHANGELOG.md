# Changelog

## 0.1.0 - Unreleased

- Added a Tauri GUI for symbolic link, hard link, same-file, link count, sibling discovery, hard-link group scanning, and hard-link-preserving clone workflows.
- Added Windows Explorer and GNOME Files context-menu launch helpers for GUI quick actions.
- Added a Windows 11 Explorer command extension registration path for top-level context-menu integration.
- Added a GNOME Files advanced menu extension through `nautilus-python`.
- Added a two-step Windows Explorer workflow for picking a link source and creating symlinks or hard links in a target folder through the GUI.
- Added Windows Explorer and GNOME Files two-file context-menu comparison through `Compare Same File`.
- Added multi-source file-manager picking and batch symlink/hard-link creation, including hard-link directory trees for picked folders.
- Changed file-manager drops to start LinkForge hidden, exit silently on clean success, and show lightweight Tauri-rendered dialogs only for conflicts, errors, and non-clean summaries instead of platform-native message boxes or the full main window.
- Changed direct file-manager same-file and link-count actions to use lightweight Tauri-rendered result dialogs instead of opening the full LinkForge window.
- Fixed file-manager drop batches to preflight picked sources, target directories, conflicts, and likely hard-link failures before creating links.
- Fixed file-manager source-picking failures so missing paths and state-file write errors appear in a lightweight Tauri-rendered dialog while successful picks remain silent.
- Fixed Windows 11 Explorer batch link drops so directory-background menus show drop actions and conflicts ask before renaming.
- Fixed full-GUI hard-link creation so directory sources create hard-link directory trees, and replaced the custom GUI path picker with native system dialogs.
- Added dedicated workspace crates for Windows and GNOME context-menu integration.
- Added CLI help descriptions and shell completion generation for PowerShell, Bash, Zsh, and Fish.
- Added Windows symbolic link creation support that requests unprivileged symlink creation when Developer Mode is enabled.
- Added core and CLI support for creating symbolic links and hard links.
- Added same-file checks, hard link count inspection, hard-link sibling discovery, and directory hard-link group scanning.
- Added directory tree cloning that copies files and symbolic links while preserving internal hard link relationships.
- Documented the LinkForge feature overview, including symbolic links, hard links, same-file checks, link counts, hard-link sibling discovery, hard-link group scanning, and hard-link-preserving directory cloning.
