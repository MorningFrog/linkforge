# Contributing

Thank you for contributing to LinkForge.

## Local Development

LinkForge is a Cargo workspace with separate crates for the core library, CLI, and GUI entry point.

Common local commands:

```text
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

Run the CLI locally with `cargo run -p linkforge-cli --` followed by a LinkForge command:

```text
cargo run -p linkforge-cli -- link-count path/to/file
cargo run -p linkforge-cli -- same-file path/to/a path/to/b
cargo run -p linkforge-cli -- scan-groups path/to/root
```

To manually test the installed CLI and PowerShell completions on Windows:

```powershell
cargo install --path crates/linkforge-cli --force
linkforge help
linkforge help symlink
$completion = linkforge completions powershell | Out-String
Invoke-Expression $completion
linkforge <Tab>
cargo uninstall linkforge-cli
```

Use `Out-String` when loading generated PowerShell completions into the current session. Piping directly to `Invoke-Expression` can pass empty lines as empty commands.

The completion command prints scripts to stdout and does not modify shell profiles. If you append completions to `$PROFILE` or another shell startup file during manual testing, remove those lines after testing to restore the local environment.

Before submitting changes, run `cargo fmt`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`.

On Windows, symbolic link creation without administrator privileges requires Windows Developer Mode. LinkForge asks Windows to allow unprivileged symlink creation, but Windows still rejects the request when Developer Mode is disabled and the process is not elevated. Tests account for missing symlink privileges, but manual symlink commands can still fail with the operating system permission error.

## Git Commit Message

Git commit messages in this project must follow the Conventional Commits specification.

Recommended format:

```text
<type>(optional scope): <description>
```

Common types include:

- `feat`: A new feature.
- `fix`: A bug fix.
- `docs`: Documentation changes.
- `refactor`: Code restructuring that does not change external behavior.
- `test`: Test-related changes.
- `chore`: Build, tooling, dependency, or other maintenance changes.

Examples:

```text
feat(cli): add symlink creation command
fix(core): handle existing target path on Windows
docs: update platform support notes
```
