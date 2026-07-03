# Supported Install Surface

Native packages should install the complete LinkForge desktop tool by default. Optional split packages may exist for distribution conventions, but the default user-facing package should include the core workflow.

## Default Native Package Contents

| Surface | Default |
| --- | --- |
| CLI binary | Install `linkforge` |
| Tauri GUI | Install `linkforge-gui` |
| Windows Explorer context menu | Install and register the Windows 11 modern menu |
| GNOME Files context menu | Install the `nautilus-python` extension through the native package |
| Shell completions | Install generated Bash, Zsh, Fish, and PowerShell completions when supported by the channel |
| Linux desktop file | Install `io.github.morningfrog.LinkForge.desktop` |
| AppStream metadata | Install `io.github.morningfrog.LinkForge.metainfo.xml` |
| Icons | Install platform-appropriate LinkForge icons |

## Channel Decisions

- Windows winget uses a Tauri NSIS x64 installer as the public installer strategy.
- Debian/Ubuntu packaging uses an Ubuntu PPA as the first apt-compatible path. Official Debian packaging is documented for later sponsor/maintainer work.
- Flatpak uses `io.github.morningfrog.LinkForge` and provides the GUI/CLI inside the sandbox, but does not install host GNOME Files integration. Users who need file-manager integration should install native deb/rpm packages.

## Upgrade And Uninstall Behavior

- Upgrades must preserve user data under the platform state directory and replace installed binaries in place.
- Windows uninstall must unregister the sparse package / Explorer menu before removing files.
- GNOME native uninstall must remove the installed Nautilus extension and tell users to restart GNOME Files with `nautilus -q` if the menu remains visible.
- Flatpak uninstall relies on Flatpak cleanup and must not attempt host file-manager extension cleanup.
