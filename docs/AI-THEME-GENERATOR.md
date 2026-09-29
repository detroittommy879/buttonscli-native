# AI theme generator

The native Themes settings include an AI theme generator. Describe a color
direction, and it asks the currently selected provider for a complete app and
terminal palette. It sends the brief and a small palette from the selected
theme. It does not send terminal contents or API keys in the request body.

The generator validates the response shape and checks contrast for app and
terminal text. If the first response is invalid, it asks the provider once to
correct it. Invalid output after that is rejected. The result keeps existing
font, effect, and unknown theme fields unchanged.

Generated output first appears as an in-memory candidate. **Preview candidate**
opens it in the custom theme editor and previews it in the workspace. **Edit in
theme library** opens it as a draft without previewing. Use **Save Current
Theme** in that editor to keep it. New generated themes use a collision-safe
filename and never replace a saved theme. You can cancel the preview or discard
the candidate without writing it.

This is a Pro feature. The release build keeps it locked until entitlement
integration is available. Debug builds can exercise it with
`BUTTONSCLI_NATIVE_DEV_THEME_GENERATOR=1`; that override does not work in
release builds. A configured endpoint and model are required. Provider requests
may include your brief and the selected palette, so review the provider you
have selected before generating.

The candidate flow and validation have Windows source tests. Live provider
requests and interactive Windows GUI behavior have not yet been certified.
