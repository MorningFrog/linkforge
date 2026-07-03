# Privacy Policy

LinkForge operates on local files selected by the user. The CLI, GUI, Windows Explorer context menu, and GNOME Files context menu do not collect telemetry, transmit file paths, or upload file contents.

The app stores local picked-source state for the two-step context-menu workflow:

- Windows: `%LOCALAPPDATA%\LinkForge\picked-sources.json`
- Linux: `$XDG_STATE_HOME/LinkForge/picked-sources.json` or `~/.local/state/LinkForge/picked-sources.json`

This state contains local filesystem paths only. It is used to complete pick/drop link workflows and can be removed safely when LinkForge is not running.

Support requests, issue reports, crash logs, or screenshots are only received when a user sends them intentionally.
