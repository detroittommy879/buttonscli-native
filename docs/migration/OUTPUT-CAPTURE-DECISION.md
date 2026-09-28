# Output capture decision

Date: 2026-09-28  
Scope: R02 decision for AI Help context. This does not complete the CLI-compatible R03 output service.

## Selected seam

AI Help takes a bounded plain-text tail from the `TerminalBackend::last_content.grid` snapshot already owned by the renderer. The snapshot is copied after `TerminalView` synchronizes with the Alacritty terminal. `plain_text_tail(200_000)` reads that retained grid without acquiring the terminal grid mutex or introducing another PTY reader. The context is therefore a rendering of current screen and scrollback state: escape sequences and overwritten carriage-return text are already represented as terminal cells, while cell attributes and raw byte history are not included.

The preview is generated only for the focused terminal when the user explicitly requests it. It is redacted and shown before sending. This is appropriate for plain AI Help context, which needs readable terminal state rather than an exact byte transcript.

## Alternatives and scope

- A second PTY reader is rejected: it would compete with the existing owner and can consume bytes the terminal needs.
- Capturing raw bytes in the PTY event loop would preserve a transcript, but requires instrumentation at the single existing reader, bounded per-session storage, UTF-8/ANSI handling, activity/freshness semantics and hidden-session lifecycle coverage. That is materially broader than AI context and is not implemented here.
- A `Wakeup` or repaint signal alone is not output data and cannot provide transcript semantics.

The selected implementation changes only the vendored backend snapshot accessor and the native AI Help preview. It allocates a bounded text copy on explicit preview; it does not measure sustained-output allocation cost in this batch.

## Fixture and remaining work

Use a future R03 transcript fixture containing ANSI color, cursor movement, carriage-return redraw, split UTF-8 bytes, blank lines and alternate-screen transitions. Assert separately on exact raw capture and normalized AI snapshot; do not compare the two as if they had identical contracts.

The current AI snapshot accessor is present and compiles on Windows. R03 remains open for a raw/CLI-compatible output service, per-session activity/freshness metadata, hidden-tab observation guarantees, and throughput/lock measurements. The AI snapshot path has not had an interactive GUI or sustained-output run.
