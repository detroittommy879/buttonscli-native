# Metrics and feedback: v2 recommendation

**Use a second small container and separate database for v2. Reuse tested validation/storage helpers, not the old service's entire product contract. Use HTTPS; do not generate GPG keys on installation.** Assessment: October 12, 2026. No server changes or submissions were made.

## What exists

| Component | Finding |
| --- | --- |
| Old client | `analyticsService.ts`: random **per-profile** install ID, session ID, broad feature events; queue capped at 400, batches of 20, two-minute flush. A first-run ID is not proof of an actual installation. |
| Old feedback | `feedbackService.ts`: explicit message/contact plus install/session IDs and contact identity metadata. Message 2,000 characters; contact 160. This unnecessarily links feedback to usage. |
| Native v2 | `src/account.rs` disables legacy metrics. `src/feedback.rs` is also disabled at that gate; its implemented request uses fresh temporary IDs, bounded text and best-effort secret redaction. No v2 tracking was enabled here. |
| Live service | AMP `193.122.157.24`, `buttonscli-insights`, Node 22.23.1; localhost port 38173, Apache `/bcli-metrics/` proxy, SQLite bind mount. Public HTTPS GET health succeeds. Running source SHA-256 matches the inspected legacy source: `409dfa2e…795f8f`. |
| Useful safeguards | 256 KiB request limit, event/property allowlists, SQLite deduplication, HTML escaping, feedback limit 5/10 minutes/IP, restricted browser origins, non-default admin password. |
| Gaps | Runs as root with writable root filesystem. Ingest has no application rate limiter; state grows in memory. Feedback IP buckets also grow. Forwarded IP headers are trusted without a trusted-proxy rule. Audit files rotate, but SQLite rows have no retention/deletion schedule. Admin Basic auth needs stronger access restrictions. CORS does not authenticate native clients. |

Live inspection was read-only: container identity/configuration, source hashes, non-secret configuration booleans, proxy routes and health. Cloudflare-to-origin TLS mode, host firewall, backups and full dependency vulnerabilities were not certified.

## Minimal new design

- **Usage is optional, off by default:** explain fields before enabling; disabling drops queued events. Only `first_seen` and `app_opened`, version, OS family and distribution channel. No commands, screenshots, file paths, model choices, machine fingerprint or account identity. Prefer Store acquisition reports for Store installs; label app counts **observed first runs**, not verified installs.
- If deduplicating installations is required, create a random app-local ID **after consent**, independent of profiles; allow reset/deletion. It is pseudonymous, not guaranteed anonymous. Opens are events, not distinct people. Offline users, resets and abuse make totals approximate.
- **Feedback stays independent:** category, explicit message, optional reply address; show exactly what will be sent. No install/session join, automatic terminal attachment or silent retry. Use a request ID for idempotent, user-triggered retry.
- HTTPS with certificate validation protects transit; verify TLS at the public edge and onward to the origin (localhost proxy HTTP is acceptable). A client GPG key neither authenticates a genuine install nor improves ordinary TLS transport. If feedback must stay unreadable to the server/proxy, encrypt to a **server/operator public key**; that adds key rotation/support costs and still exposes metadata. Baseline: TLS, restricted storage/admin access and encrypted backups.
- Strict schemas, small body/batch caps, per-IP and global limits, trusted-proxy handling, bounded retry/backoff and deduplication. Never embed a shared secret in a public binary as proof of authenticity.
- Non-root container, read-only root, dropped capabilities, resource limits, localhost ingress, separate data/admin credentials. Suggested retention: raw usage 30 days then aggregate; feedback 180 days or delete after resolution; short IP log retention. These are proposed product choices, not statutory periods.

## Store requirements and release gate

Microsoft's current policy requires a privacy policy for Win32 products, disclosure of collection/use/storage/security/parties/user controls, and modern cryptography; relevant sharing requires opt-in and withdrawal. Publish matching policy text in-app and Partner Center; test denied consent, offline startup, deletion, size/rate abuse and feedback confirmation before enabling the service. This design is a recommendation, not a certification guarantee. [Microsoft Store policy §10.5](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies#105-personal-information).

Server controls above follow [OWASP REST security guidance](https://cheatsheetseries.owasp.org/cheatsheets/REST_Security_Cheat_Sheet.html). Store acquisition/installation reporting has its own coverage limitations; keep its totals separate from consenting app usage. [Partner Center acquisitions](https://learn.microsoft.com/en-us/windows/apps/publish/acquisitions-report).

For a future Mac App Store build, also check Apple's collection consent, withdrawal, retention/deletion disclosures and privacy policy requirements. Optional usage collection is compatible with this baseline; packaging and review remain separate. [Apple §5.1.1](https://developer.apple.com/app-store/review/guidelines/#data-collection-and-storage).

Source: legacy `src/services/{analyticsService,feedbackService}.ts`, `w111erd-insights/server.mjs`; native `src/{account,feedback}.rs`. New container implementation/deployment remains a later task.
