# LinkForge Packaging Drafts

This directory contains release-preparation assets only. Do not submit these manifests or package files to public package repositories until an explicit 1.0 release approval is given.

Current decisions:

- Windows package-manager preparation targets winget through an NSIS x64 installer that includes the CLI, Tauri GUI, and Windows Explorer context-menu integration.
- Ubuntu PPA is the first practical apt-compatible path. Debian official packaging is documented for later sponsor/maintainer work; the actual Debian metadata lives in the repository-root `debian/` directory, with notes in `packaging/debian/`.
- Flatpak uses `io.github.morningfrog.LinkForge` and does not provide host GNOME Files integration.

Before any public release, follow `docs/release/release-checklist.md`.

Draft packaging checks are automated by `.github/workflows/release-drafts.yml` and can also be run locally with `scripts/validate-release-drafts.ps1` on Windows or `scripts/validate-release-drafts.sh` on Linux. These checks build or inspect draft artifacts only; they do not publish to any package manager.
