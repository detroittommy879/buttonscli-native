# Privacy and feedback

The old metrics-hosted feedback sender and background rollout-config fetch
are disabled pending a separate native service. **Send Feedback** is hidden,
and the production adapter refuses submission before any request. The native
app has no routine analytics collector or event queue. Auth and user-selected
AI providers use separate services.

The retained feedback implementation and mock compatibility tests describe
the following behavior for a future reviewed service rollout.

When enabled, **Help → Send Feedback** submits a report or idea. Nothing is sent when
the window opens or while you type. The app sends one request only after you
choose **Send**; it does not queue a draft for later. If the request fails, the
draft stays in the window and the app reports the failure.

Before sending, the window previews best-effort masking of common secret-like
text. Review the preview and remove anything you do not want to share. Masking
cannot recognize every secret. The request contains the message, category,
optional reply email, app version, operating system, language, and random
identifiers created for that submission only. The identifiers are not saved as
a profile. The app does not attach terminal output, commands, clipboard
contents, files, or diagnostics. The feedback service can still observe network
connection data such as the sender's IP address; this app does not state or
control the service's retention policy.

Feedback is sent over HTTPS to the fixed ButtonsCLI feedback endpoint. The app
checks for a successful HTTP response and a valid receipt before showing
success. The native app currently emits no routine product analytics events and
has no analytics event queue. The retained runtime-config adapter retrieves
rollout flags without a feedback draft, terminal data or account identifiers;
its production path is currently disabled. Account sign-in and AI provider
requests are separate flows. Saved account sessions are revalidated at startup
and periodically; provider requests begin only through their explicit controls.

At startup the app asks GitHub's public API
(`api.github.com`) which ButtonsCLI release is the latest, to show a link in
the status bar when a newer one exists. The request carries no account,
terminal or profile data; GitHub sees the usual connection data such as the
IP address. Untick **Check GitHub for a new release at startup** in
Settings → Workspace, or set `BUTTONSCLI_NO_UPDATE_CHECK=1`, to skip it.

Feedback is a free feature.
