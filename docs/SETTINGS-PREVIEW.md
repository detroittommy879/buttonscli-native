# Settings preview

Open **Settings** from the status bar or menu. On desktop it opens in its own
window; if the graphics backend cannot create another viewport, Settings opens
as an in-window panel instead. Theme, terminal color, font, and divider edits
preview against the running workspace without restarting terminal sessions.

Choose **Keep changes and close** to keep the current values, or **Revert and
close** to restore the app preferences and per-terminal theme choices from when
Settings opened. Revert does not close, restart, or retarget terminals. A
successfully imported profile resets the rollback point to that imported
profile. External changes already made to the operating system credential store
are not undone.

Closing Settings with the window close button keeps the current values. Reopen
Settings to start a new preview and rollback point.

The viewport and embedded fallback are source-checked on Windows. Moving the
window between monitors, DPI scaling, focus behavior, and keyboard interaction
still need manual desktop review.
