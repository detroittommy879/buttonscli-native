# ButtonsCLI account

Open **Settings → Account** to sign in with an email code. ButtonsCLI does not
ask for or store an account password. The native account feature is free
(`accountSignIn`); all implemented native features work without signing in.

The session token and its expiry are saved in the operating system's
credential store under a separate ButtonsCLI Native Account service and a
profile-scoped reference. They are not written to Preferences, provider
settings, logs, or import files. On startup, the app checks a saved session
with the account service before loading its server-granted features. It
refreshes grants every five minutes. If the session expires, is revoked, or
the service cannot confirm it, those grants close. **Sign out** removes the
local credential and requests server revocation when the service is
reachable.

AI Help, Agent Mode, AI themes/shaders, Quick Secrets, profile management and
local CLI/MCP control are free and require no server grant or local feature flag.
AI uses your own provider or connection, with its own costs and usage limits.
Account sign-in does not enable native cloud sync or theme sharing; those
features are not implemented in this app.

The legacy metrics-hosted runtime-config fetch and feedback sender remain
disabled pending a separate native service. Account sign-in still uses the
separate auth host; its session/grant checks do not restrict the free features.

The native email-code form does not complete a browser Turnstile challenge.
If the hosted auth service requires Turnstile, email-code requests will be
rejected until a native challenge flow is added. Live sign-in and revocation
have not yet been certified against the hosted service.
