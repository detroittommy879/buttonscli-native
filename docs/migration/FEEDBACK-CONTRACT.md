# Native feedback contract

The native Help → Send Feedback action is a one-shot, user-authored request to
`https://buttonscli.com/bcli-metrics/api/v1/feedback`. Its compatibility
fixture snapshots the original receiver contract at source revision
`032c9f2`; it is not a live-service check. The original schema source was
`w111erd-insights/openapi/insights-v1.yaml`.

The receiver requires `installId`, `sessionId`, `category`, `message`, and
`appVersion`; `contact` and `meta` are optional. Native creates fresh random
`installId` and `sessionId` values for each submission and does not persist
them. `meta` contains only OS and locale. Native uses these compatible
categories: `feature-request`, `bug-report`, `ui-idea`, `ai-help`,
`performance`, `pricing`, and `other`. It omits `waitlist`, which has no native
feedback flow.

The client bounds message/contact lengths to 2,000/160 characters, uses the
existing redirect-blocking and timeout-bounded HTTP transport, rejects any
endpoint other than the fixed HTTPS host and path, and accepts success only
when the service returns a 2xx response with `{ "ok": true, "id": "..." }`.
Non-2xx responses, invalid receipts, and transport errors leave the draft in
the dialog and do not show success. The request body is not logged.

The focused mock tests compare the outgoing fields and category values to
[`tests/fixtures/feedback-contract.json`](../../tests/fixtures/feedback-contract.json),
exercise secret masking and endpoint validation, and verify 400, 429, and 500
responses. They do not contact production. The native app emits no routine
analytics events; this is the chosen privacy policy for this migration rather
than a consent toggle with no corresponding event sender.
