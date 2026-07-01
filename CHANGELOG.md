# Changelog

## 0.1.0 - Unreleased

Initial pre-release for LinkForge, focused on creating, inspecting, and managing symbolic links and hard links from the CLI, GUI, and supported file managers.

### Core Features

- Symbolic link and hard link creation for common file-management workflows.
- Same-file checks, hard link count inspection, hard-link sibling discovery, and hard-link group scanning.
- Directory tree cloning that preserves internal hard link relationships.
- Batch symbolic link and hard link creation with preflight validation and configurable conflict handling.

### Interfaces

- Command-line interface for link creation, inspection, batch operations, hard-link-preserving clones, help output, and shell completion generation.
- Tauri desktop GUI for the main LinkForge workflows.
- Native system dialogs for GUI path selection.

### File Manager Integration

- Windows 11 Explorer top-level context-menu integration.
- GNOME Files advanced context-menu integration through `nautilus-python`.
- File-manager quick actions for inspecting links, comparing two files, picking link sources, and creating links in a target folder.
- Lightweight GUI dialogs for file-manager results, conflicts, warnings, and errors.

### Platform Notes

- Core link management and the CLI support Windows and Linux.
- The GUI targets Windows desktop environments and Linux GNOME desktop environments.
- Windows symbolic link creation can use unprivileged symlink support when Developer Mode is enabled.

For feature details, usage examples, platform notes, and context-menu behavior, see `README.md`.
