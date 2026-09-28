# `egui_dock` fit for ButtonsCLI Native

Research date: 2026-09-27. This is a source review, not a compiled prototype or an adoption decision. L06 requires a small Fedora build and interaction check before adding a production dependency.

## Findings

- The [upstream README](https://github.com/anhosh/egui_dock#features) describes tab open/close, moving tabs between nodes, resizing, window drag-out and extensive styling. That could help if ButtonsCLI later wants user-driven docking.
- The [upstream changelog](https://github.com/anhosh/egui_dock/blob/main/CHANGELOG.md) says **0.16.0 upgraded to egui 0.31**. The current README example uses egui 0.36 with `egui_dock` 0.21; adding that latest release to this app's pinned egui/eframe 0.31 would mix incompatible egui generations. Prototype version 0.16.x with the current lockfile, or separately scope a framework upgrade.
- Upstream [notes about alternatives](https://github.com/anhosh/egui_dock#alternatives) say `egui_dock` does not directly divide a node into more than two children, unlike `egui_tiles`. ButtonsCLI's desired COL/ROW/GRID rules and automatic row wrapping need product-specific layout behavior. A binary tree can compose a grid, but the library does not by itself determine when a pane should wrap.
- N `src/app.rs` already owns the session list, visible-pane list, focused session, a recursive splitter and pane ratios. Replacing it wholesale creates risk for focus, PTY ownership, resize and agent-control IDs. The visible problem can be addressed with a pure geometry reducer and independent tab-strip wrapping first.
- N `src/app.rs::tab_bar` currently uses `egui::ScrollArea::horizontal` inside an exact-height 40 px panel. A docking library does not automatically satisfy multi-row navigation; test actual wrap behavior before claiming it solves that requirement.
- N `vendor/egui_term` owns terminal rendering/scroll wheel. `egui_dock` would control tab containers, not provide a terminal scrollback scrollbar.
- Pinned egui 0.31.1 already includes `Context::show_viewport_deferred` and `show_viewport_immediate` in the locally installed crate source. Detachable Settings and Help can be probed directly in L10 without adopting docking for the entire terminal layout.

## Recommended experiment

Build a temporary `egui_dock` 0.16.x example with 3 fake tab IDs and fake terminal panels on Fedora. Check whether tab move between leaves, close/rename, resize, multi-row tab navigation, styling radius and drag-out all work on the chosen X11/Wayland session. Then add one real PTY only to measure whether its backend survives focus/move/undock and keeps correct size. Compare with the existing custom layout at 1/4/10 panes and record compilation time, runtime memory and UI responsiveness. Keep the experiment out of the production dependency graph until this is reviewed.

**Current recommendation:** retain and repair the native pane tree for automatic COL/ROW/GRID, and evaluate `egui_dock` for optional manual tab docking later. If the prototype shows a simpler reliable way to meet the same product rules without slowing the app, revise L03/L04 with evidence. Do not use it solely to round tabs; egui style already supports that.
