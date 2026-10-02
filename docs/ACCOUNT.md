# ButtonsCLI account

Open **Settings → Account** to sign in with an email code. ButtonsCLI does not
ask for or store an account password. The native account feature is free
(`accountSignIn`); signing in does not unlock every paid feature.

The session token and its expiry are saved in the operating system's
credential store under a separate ButtonsCLI Native Account service and a
profile-scoped reference. They are not written to Preferences, provider
settings, logs, or import files. On startup, the app checks a saved session
with the account service before loading its server-granted features. It
refreshes grants every five minutes. If the session expires, is revoked, or
the service cannot confirm it, those grants close. **Sign out** removes the
local credential and requests server revocation when the service is
reachable.

Paid access still requires the central feature catalog rollout, the broad
runtime flags, and a current server grant for that feature. AI Help and other
Pro features remain locked until those gates are explicitly released and
verified. Account sign-in does not enable native cloud sync or theme sharing;
those features are not implemented in this app.

The legacy metrics-hosted runtime-config fetch is disabled pending a separate
native service, so broad production rollout flags currently remain closed.
Account sign-in still uses the separate auth host.

The native email-code form does not complete a browser Turnstile challenge.
If the hosted auth service requires Turnstile, email-code requests will be
rejected until a native challenge flow is added. Live sign-in and revocation
have not yet been certified against the hosted service.
