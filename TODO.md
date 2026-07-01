# TODO

## P1: Installer And Registration Robustness

- [x] Make Windows context-menu registration scripts more robust.
  - Validate that required GUI and shell-extension build artifacts exist before registration.
  - Support debug and release artifact paths clearly.
  - Improve verification after sparse-package registration.
- [x] Make GNOME Files extension installation more robust.
  - Improve dependency checks for `nautilus-python`.
  - Validate that the configured `linkforge-gui` executable can be launched or resolved.
  - Add clearer install, uninstall, and verification output.
