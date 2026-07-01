# TODO

## P0: Context Menu UX

- [x] Reduce full-GUI launches from file-manager context-menu actions.
  - Only open the full LinkForge window when the action genuinely needs the full interface.
  - For actions that can complete directly, show only a lightweight success, conflict, error, or summary dialog when user feedback is needed.
- [x] Show a lightweight failure dialog when `pick-source` fails.
  - Keep successful source picking silent.
  - Surface state-file write failures, missing source paths, and other pick errors through the same lightweight Tauri dialog style used by drop actions.

## P0: Drop Batch Reliability

- [x] Add preflight validation before GUI drop batch operations.
  - Check source existence before starting the batch.
  - Check target directory validity and writability.
  - Detect target-name conflicts up front.
  - Warn about likely hard-link failures such as unsupported directory/file combinations or cross-device file links where detectable.
  - Present preflight problems before creating links so users can choose how to proceed.

## P1: GUI Behavior Consistency

- [x] Make GUI hard-link behavior match context-menu hard-link behavior for directories.
  - File sources should create hard links.
  - Directory sources should create hard-link directory trees, with regular files hard-linked and symbolic links copied as links.
  - Update GUI labels and result messages so users can distinguish file hard links from hard-link trees.
- [x] Replace the custom GUI path picker with system file and directory dialogs.
  - Use native file/directory selection through Tauri.
  - Do not keep a custom fallback picker.
  - Support both source selection and destination selection workflows through system dialogs.

## P1: Installer And Registration Robustness

- [ ] Make Windows context-menu registration scripts more robust.
  - Validate that required GUI and shell-extension build artifacts exist before registration.
  - Support debug and release artifact paths clearly.
  - Improve verification after sparse-package registration.
- [ ] Make GNOME Files extension installation more robust.
  - Improve dependency checks for `nautilus-python`.
  - Validate that the configured `linkforge-gui` executable can be launched or resolved.
  - Add clearer install, uninstall, and verification output.
