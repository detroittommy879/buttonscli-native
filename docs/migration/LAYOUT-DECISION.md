# Responsive layout decision (L01 source pass)

This is the source-based part of L01. Fedora X11/Wayland interaction and screenshots remain to be recorded before calling L01 accepted. Checked native source at `a5192cb` on Windows, 2026-09-28. No PTY or GUI observation is claimed here.

Since this source pass, L02 assigned `termN` titles, L04 wrapped the tab strip, and L03 added `src/layout.rs`. The geometry table below is historical baseline evidence. The new reducer uses stable session IDs, a 320 × 160 logical-point review target, a font-size estimate for 40 columns and 8 rows, and the available viewport size; COL fills across then down, ROW fills down then across, and GRID chooses a balanced feasible shape. Overflow sessions remain open and reachable through the tab strip. Resizing back restores the full requested set and any saved ratios for its prior shape. Fedora X11/Wayland GUI and PTY resize acceptance remain open.

## Current behavior to reproduce

`set_visible_pane_count` creates up to ten sessions, retains the focused pane first, and fills remaining slots from visible then existing tabs. Choosing a hidden tab replaces the focused visible slot. `pane_tree` puts every COL pane in one horizontal split chain, every ROW pane in one vertical chain, and GRID in `ceil(sqrt(n))` columns. The tab strip has a fixed 40-point height and horizontal scrolling. The 10-point split gap and 80-point minimum on each side do not guarantee useful leaf size in a long split chain.

| Count | Current COL | Current ROW | Current GRID (rows × columns) |
|---:|---|---|---:|
| 1 | single leaf | single leaf | 1 × 1 |
| 2 | 2 across | 2 stacked | 1 × 2 |
| 3 | 3 across | 3 stacked | 2 × 2 |
| 4 | 4 across | 4 stacked | 2 × 2 |
| 5 | 5 across | 5 stacked | 2 × 3 |
| 6 | 6 across | 6 stacked | 2 × 3 |
| 7 | 7 across | 7 stacked | 3 × 3 |
| 8 | 8 across | 8 stacked | 3 × 3 |
| 9 | 9 across | 9 stacked | 3 × 3 |
| 10 | 10 across | 10 stacked | 3 × 4 |

## Target behavior for L03/L04

The pane grid and tab strip both need wrapping. COL fills left to right, then starts a lower row. ROW fills top to bottom, then starts another column. GRID chooses rows and columns that maximize the smallest terminal viewport while preserving order. Use terminal cell measurements plus chrome and divider gaps, with an initial review target of at least 320 × 160 logical points per leaf. This is a design target, not a measured Fedora minimum. When even one leaf cannot meet it, keep the focused pane useful and give an explicit way to reach hidden panes rather than crushing all PTYs.

At 1280 × 820, 900 × 600, and the current minimum 760 × 480 window, test counts 1–10. At each size: add, focus, hide, close focused/hidden, rename, reorder, drag a divider, shrink then grow, and run `stty size` in each displayed PTY. Record pane order, actual leaf sizes and screenshots on Fedora X11 and Wayland. Preserve stable session IDs and focus while changing geometry. Tab wrapping is an independent check with long names and many tabs. A static source pass cannot certify those interactions.
