# Release Identity

This document is the stable public identity source for LinkForge package metadata.

## Project Identity

| Field | Value |
| --- | --- |
| Project name | LinkForge |
| Publisher / maintainer | MorningFrog |
| Contact email | 1098224028@qq.com |
| Copyright holder | MorningFrog |
| License | Apache-2.0 |
| Homepage | https://github.com/MorningFrog/linkforge |
| Repository | https://github.com/MorningFrog/linkforge |
| Support URL | https://github.com/MorningFrog/linkforge/issues |
| Release notes URL | https://github.com/MorningFrog/linkforge/releases |
| Security policy URL | https://github.com/MorningFrog/linkforge/security/policy |
| Privacy policy URL | https://github.com/MorningFrog/linkforge/blob/main/PRIVACY.md |

## Package And Application IDs

| Surface | Identifier |
| --- | --- |
| Tauri app identifier | `io.github.morningfrog.LinkForge` |
| Flatpak app ID | `io.github.morningfrog.LinkForge` |
| AppStream MetaInfo ID | `io.github.morningfrog.LinkForge` |
| Linux desktop file ID | `io.github.morningfrog.LinkForge.desktop` |
| winget package ID | `MorningFrog.LinkForge` |
| Windows sparse-package name | `LinkForge.ContextMenu` |

## Executable And Integration Names

| Component | Name |
| --- | --- |
| CLI executable | `linkforge` |
| GUI executable | `linkforge-gui` |
| GNOME installer executable | `linkforge-context-menu-gnome` |
| Windows shell-extension DLL | `linkforge_context_menu_windows.dll` |
| Windows Explorer menu | `LinkForge` |
| GNOME Files menu | `LinkForge` |

## Descriptions

Short description:

```text
Create and inspect symbolic links and hard links.
```

Long description:

```text
LinkForge is a desktop and command-line tool for creating, inspecting, and managing symbolic links and hard links. It supports direct link creation, same-file checks, hard-link counts, sibling discovery, hard-link group scanning, hard-link-preserving directory clones, batch link creation, and Windows Explorer or GNOME Files context-menu workflows.
```

## Signing And Assets

- Windows public distribution requires Authenticode signatures on `linkforge.exe`, `linkforge-gui.exe`, `linkforge_context_menu_windows.dll`, and the NSIS installer, with timestamping.
- The current source icon is `crates/linkforge-gui/icons/icon.ico`.
- Draft Linux scalable packaging icon is `packaging/flatpak/io.github.morningfrog.LinkForge.svg`.
- Public package submissions need real screenshots under `docs/screenshots/` or equivalent release-hosted URLs before submission.
