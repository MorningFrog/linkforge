# winget Draft

Package ID: `MorningFrog.LinkForge`

Installer strategy: Tauri NSIS x64 installer.

The installer is expected to install:

- `linkforge.exe`
- `linkforge-gui.exe`
- `linkforge_context_menu_windows.dll`
- LinkForge icons/resources
- Windows 11 modern Explorer context-menu sparse-package registration

Uninstall must unregister the sparse package before removing files. Upgrade must install over an existing version without requiring a reboot.

## NSIS Switches For winget

Draft switches:

```text
Silent: /S
SilentWithProgress: /S
UninstallSilent: /S
```

These switches must be validated against the final Tauri NSIS artifact before winget submission.

## Local Validation

After producing a release artifact:

```powershell
winget validate packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0
winget install --manifest packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0 --silent
winget upgrade --manifest packaging/winget/manifests/m/MorningFrog/LinkForge/0.1.0 --silent
winget uninstall MorningFrog.LinkForge --silent
```

The draft manifest currently keeps an all-zero `InstallerSha256` and future
GitHub release URL as placeholders. `winget validate` may pass, but
`winget install --manifest` must not be used until those fields are replaced
with a real release artifact URL and SHA256.

Also validate in Windows Sandbox:

- No forced reboot.
- Deterministic exit code on install, upgrade, and uninstall.
- CLI and GUI launch from a clean machine.
- Windows Explorer menu appears after install and disappears after uninstall.
- `Get-FileHash -Algorithm SHA256 <installer>` matches `InstallerSha256`.

## Submission Path

Do not submit before explicit release approval. When approved, create or update the package in `microsoft/winget-pkgs`, open one pull request per package version, and attach validation notes for silent install, upgrade, uninstall, and Windows Sandbox smoke tests.
