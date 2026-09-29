# HSync native renderer decision

Date: 2026-09-29

## Source behavior

The original CPU path in `G:\ccc\z_terminals\w111erd\src\plugins\effects\terminalHsync.ts`
composites the xterm canvas layers, then shifts each physical pixel scanline
horizontally with a sine wave. Its phase is based on scanline position, elapsed
time, and `wavePeriod`; displacement is bounded by `shiftRange`. The sibling
GPU path implements the same image warp as a fragment shader. A whole-row or
whole-cell offset would not preserve this behavior.

## Native renderer constraint

The native app uses pinned egui/eframe 0.31 with the vendored
`vendor/egui_term/src/view.rs::TerminalView::show`. It paints the terminal
background, cell backgrounds, cursor, and glyphs as epaint shapes directly
into the app's egui painter. There is no terminal-owned texture or
post-processing hook to sample and warp. Moving cell rows would change line
layout and terminal geometry; adding one mesh offset per terminal row would
only create a coarse distortion and would not be HSync parity.

## Decision

No-go for implementing HSync in the current direct-paint path. Keep the feature
listed as unsupported until a bounded renderer spike can render terminal
content into an offscreen texture and composite it with a native GPU pass. That
spike must first demonstrate glyph crispness, selection/cursor behavior,
multi-pane cost, and a no-effect fast path on the supported native backends.
Do not describe cell-row displacement as HSync.

This is a source/architecture decision only. No GUI screenshot or GPU
performance claim was made. Row banding is implemented independently using the
native renderer's measured terminal cell height.
