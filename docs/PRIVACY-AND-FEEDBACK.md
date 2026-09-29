# Privacy and feedback

Open **Help → Send Feedback** to submit a report or idea. Nothing is sent when
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
has no analytics event queue. Its separate startup runtime-config request
retrieves rollout flags and does not include a feedback draft, terminal data,
or account identifiers. Account sign-in and AI provider requests are separate
actions and are made only when you use those features.

Feedback is a free feature. See the [receiver contract snapshot](migration/FEEDBACK-CONTRACT.md)
for the request fields and source revision used by the compatibility tests.
