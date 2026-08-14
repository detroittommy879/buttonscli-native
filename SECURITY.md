# Security policy

Please report security issues privately to the project maintainers rather than
opening a public issue. Replace this paragraph with the project's security
contact before publishing the repository.

ButtonsCLI launches a local user shell. Treat every feature that writes bytes
to a PTY as command execution: never run remote suggestions, pasted content, or
website-provided commands without clear user action. The browser demo is
strictly in-memory and must never gain direct local-shell access.

Terminal output, typed commands, clipboard contents, working directories, and
environment variables are private by default and must not enter analytics or
crash reports.

