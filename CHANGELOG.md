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
- GUI hard-link sibling lookup now hides scan-root controls on Windows and requires or prefills scan roots on platforms that need directory scanning.
- Native system dialogs for GUI path selection.
- Cross-platform CI gate for formatting, tests, and clippy on Windows and Linux.
- Release version sync now covers workspace manifests, Tauri GUI version metadata, and the Windows sparse-package manifest version.

### File Manager Integration

- Windows 11 Explorer top-level context-menu integration.
- GNOME Files advanced context-menu integration through `nautilus-python`.
- GNOME Files integration supports Nautilus GI 4.0 and 3.0, including Ubuntu 22.04 / Nautilus 42.
- File-manager quick actions for inspecting links, comparing two files, picking link sources, and creating links in a target folder.
- Fixed GNOME Files selected-folder and background drop actions by writing picked-source state directly from the extension and falling back to path-based directory checks when Nautilus metadata is incomplete.
- Fixed GNOME Files drop actions installed with relative `--gui-exe` paths by storing the resolved absolute GUI executable path in the Nautilus extension.
- Lightweight GUI dialogs for file-manager results, conflicts, warnings, and errors.

### Release Preparation

- Added stable release identity metadata for `io.github.morningfrog.LinkForge`.
- Added the Tauri GUI PNG icon asset required by local builds while retaining the Windows icon for bundling.
- Added preparation-only release checklist and Windows/GNOME context-menu smoke-test templates.
- Added draft winget, Debian/Ubuntu, and Flatpak packaging metadata without enabling public submissions.
- Added non-publishing release-draft validation scripts and a CI workflow for draft Windows context-menu staging, Debian packages, Flatpak bundles, checksums, and validation reports.
- Updated the Flatpak draft to use the current GNOME runtime branch and the Rust SDK extension during sandboxed builds.
- Hardened the Debian packaging draft for vendored Cargo sources and WSL validation by preserving `vendor/` during clean and normalizing installed data-file permissions.
- Recorded WSLg GNOME Files precheck results, Debian binary/source package validation status, and Flatpak local validation blockers for the release gate.
- Added explicit Tauri content security policy for the local frontend and required Tauri IPC.

### Cleanup

- Removed legacy picked-source single-file state compatibility; context-menu picks now only use `picked-sources.json`.
- Removed the undocumented GUI `--path` context-launch alias; use `--paths` for all file-manager launches.
- Removed GNOME wrapper scripts in favor of the `linkforge-context-menu-gnome` installer commands.
- Removed the non-target macOS reveal branch from the GUI backend.

### Platform Notes

- Core link management and the CLI support Windows and Linux.
- The GUI targets Windows desktop environments and Linux GNOME desktop environments.
- Windows symbolic link creation can use unprivileged symlink support when Developer Mode is enabled.

For feature details, usage examples, platform notes, and context-menu behavior, see `README.md`.
