# Security Policy

## Supported Versions

LinkForge is still in a pre-1.0 release preparation phase. Security fixes are handled on the active `main` branch until the first public stable release policy is finalized.

## Reporting A Vulnerability

Please report suspected vulnerabilities privately by email:

```text
1098224028@qq.com
```

Include the affected version or commit, operating system, reproduction steps, expected impact, and whether the issue affects CLI, GUI, Windows Explorer integration, or GNOME Files integration.

Do not open a public issue for a vulnerability until it has been triaged.

## Distribution Security

Public Windows distribution is blocked until LinkForge has an Authenticode signing path for the CLI executable, GUI executable, Windows shell-extension DLL, installer, and timestamped signatures. Draft local packaging artifacts may be unsigned, but unsigned artifacts must not be submitted to public package repositories.
