# AGENTS

This file provides working guidance for AI agents collaborating on the LinkForge project.

## Context

AI agents should choose which project documents to read based on the specific task:

- For tasks involving project positioning, feature scope, usage notes, or platform support, read `README.md` first.
- For tasks involving contribution workflow, commit conventions, or generating a git commit message, read `CONTRIBUTING.md` first.
- For tasks involving TODO items, release notes, or project collaboration conventions, read related documents such as `TODO.md`, `CHANGELOG.md`, and `AGENTS.md` as needed.

## Documentation Sync

After changing code, AI agents should check whether related documentation needs to be updated. Pay particular attention to:

- `README.md`: Whether features, platform support, usage, or project structure changed.
- `CHANGELOG.md`: Whether user-visible changes should be recorded.
- `TODO.md`: Whether completed, postponed, or newly added tasks should be synchronized.
- `AGENTS.md`: Whether AI collaboration workflow, conventions, or project guidance changed.
- `CONTRIBUTING.md`: Whether contribution workflow, commit conventions, or development conventions changed.

## Context Menu Sync

When changing context-menu behavior, menu labels, action routing, installer scripts, or related documentation, AI agents must consider both supported menu integrations:

- Windows 11 modern Explorer menu (`crates/linkforge-context-menu-windows` and `scripts/context-menu/windows/modern`).
- GNOME Files advanced menu (`crates/linkforge-context-menu-gnome`).

Keep `README.md` synchronized with any intentional behavior differences between these menu integrations.

## Compatibility and Fallback Policy

Do not preserve obsolete code, behavior, configuration, or fallback paths by default.

When making changes that do **not** affect public API compatibility, remove outdated implementations and avoid keeping legacy branches, compatibility shims, deprecated aliases, or unused fallback logic.

When a change may affect API compatibility, stop and ask the user how to proceed. If the user explicitly says that breaking compatibility is acceptable, do not retain any deprecated APIs, legacy behavior, or compatibility fallbacks. Implement the clean new design directly.

Fallbacks should only be kept when they are explicitly required for a known, current compatibility target or when the user asks for them. Avoid speculative compatibility code.

## Missing Tooling Policy

If a task requires software, system packages, command-line tools, SDKs, package managers, signing tools, GUI/file-manager components, or other external tooling that is not already installed and available in the current environment, stop the task immediately and tell the user exactly what is missing and how to install or enable it. Prefer official installers, package-manager commands, or project-documented setup steps when giving installation guidance. Do not install tools, auto-download dependencies, switch to a compatibility fallback, use a CI-only workaround, or keep trying alternate paths unless the user explicitly resumes the task after providing the required tooling.

## Markdown Formatting

When writing or editing Markdown files, keep normal prose and list items on a single line unless a hard line break is semantically required. In Markdown, a single newline usually renders like a space, so avoid inserting visual-wrap line breaks in paragraphs, bullets, or similar text. Preserve intentional structure such as blank lines between paragraphs, code blocks, tables, and explicit line breaks.

## Windows PowerShell: Prefer Scripts Over Complex Inline Commands

When operating in a native Windows environment, use PowerShell only for simple command invocation and orchestration. Do not spend time constructing or debugging complex inline PowerShell expressions.

Simple commands may be executed directly, for example:

```powershell
git status --short
cargo fmt --check
cargo test
Get-ChildItem src
```

Use an existing repository script, or create a temporary `.ps1` script, whenever a command involves any of the following:

- Nested or escaped quotes
- Multiline strings
- Embedded JSON, TOML, YAML, XML, source code, or regular expressions
- Multiple pipelines or redirections
- Complex environment-variable expansion
- Dynamic construction of native-process arguments
- Paths or arguments that require nontrivial escaping
- PowerShell invoking another shell such as `cmd.exe`, Bash, or WSL
- A command that has already failed because of quoting or parsing

Run PowerShell scripts with a simple invocation:

```powershell
pwsh -NoLogo -NoProfile -NonInteractive -File path\to\script.ps1
```

When Windows PowerShell rather than PowerShell 7 is required, use:

```powershell
powershell.exe -NoLogo -NoProfile -NonInteractive -File path\to\script.ps1
```

Inside PowerShell scripts:

- Pass native-process arguments as arrays rather than constructing a command string.
- Invoke native programs with the call operator `&`.
- Check `$LASTEXITCODE` after native-process execution.
- Use `try`/`finally` when temporary files or directories require cleanup.
- Prefer files or standard input for transferring complex structured data.
- Prefer repository-provided task scripts over newly generated scripts.
- Keep temporary agent scripts under `.agent-tmp/` and remove them after successful or failed execution unless they are useful as permanent project tooling.

Example:

```powershell
$arguments = @(
    "test"
    "--workspace"
    "--all-features"
)

& cargo @arguments

if ($LASTEXITCODE -ne 0) {
    throw "cargo failed with exit code $LASTEXITCODE"
}
```

Do not use `Invoke-Expression` to execute generated command strings.

Do not wrap a complex command in `cmd.exe /c`, `bash -lc`, `wsl.exe ...`, or another nested shell merely to avoid PowerShell syntax. This adds another parsing layer and usually makes quoting less reliable. Calling an existing `.cmd`, `.sh`, or WSL-side script is acceptable when the invocation itself remains simple.

When a complex inline command fails because of parsing or quoting, do not repeatedly attempt alternate escaping forms. Move the operation into a script immediately and continue working on the actual task.
