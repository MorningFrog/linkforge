# LinkForge Packaging Drafts

This directory contains release-preparation assets only. Do not submit these manifests or package files to public package repositories until an explicit 1.0 release approval is given.

Current decisions:

- Windows package-manager preparation targets winget through a Tauri NSIS x64 installer.
- Ubuntu PPA is the first practical apt-compatible path. Debian official packaging is documented for later sponsor/maintainer work; the actual Debian metadata lives in the repository-root `debian/` directory, with notes in `packaging/debian/`.
- Flatpak uses `io.github.morningfrog.LinkForge` and does not provide host GNOME Files integration.

Before any public release, follow `docs/release/release-checklist.md`.
