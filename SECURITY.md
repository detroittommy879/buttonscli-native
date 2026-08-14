# Security policy

Please report security issues through the repository's private Security
Advisories page rather than opening a public issue. Maintainers should enable
private vulnerability reporting when the repository is published.

ButtonsCLI launches a local user shell. Treat every feature that writes bytes
to a PTY as command execution: never run remote suggestions, pasted content, or
website-provided commands without clear user action. The browser demo is
strictly in-memory and must never gain direct local-shell access.

Terminal output, typed commands, clipboard contents, working directories, and
environment variables are private by default and must not enter analytics or
crash reports.
