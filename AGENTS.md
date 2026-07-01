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
- GNOME Files advanced menu (`crates/linkforge-context-menu-gnome` and `scripts/context-menu/gnome`).

Keep `README.md` synchronized with any intentional behavior differences between these menu integrations.
