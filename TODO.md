# TODO

## Core Feature Scope

- [x] Create symbolic links for files and directories.
- [x] Create hard links for files.
- [x] Check whether two paths point to the same underlying file.
- [x] Show a file's hard link count.
- [x] Show sibling paths that are hard links to the same file.
  - Linux note: this may require scanning selected filesystem trees because inode-to-path reverse lookup is not generally available as a direct filesystem operation.
- [x] Scan a directory tree to find hard link groups.
- [x] Clone a directory tree while preserving hard link relationships.

## GUI Feature Scope

- [x] Provide a Tauri GUI entry point for LinkForge core operations.
- [x] Expose symbolic link, hard link, same-file, link count, sibling discovery, group scanning, and hard-link-preserving clone workflows in the GUI.
- [x] Provide Windows Explorer and GNOME Files context-menu launch helpers.
- [x] Provide a Windows 11 top-level Explorer command extension through a sparse package.
- [x] Provide a GNOME Files advanced menu through `nautilus-python`.
- [x] Provide a two-step Windows Explorer workflow for picking a link source and creating symlinks or hard links in a target folder.
- [x] Keep Windows and GNOME context-menu integrations in dedicated crates.
- [ ] Consider richer multi-select Explorer workflows, such as direct two-file compare from the shell menu.
