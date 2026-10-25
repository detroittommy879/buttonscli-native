# Benchmarks

Two tools, one workload set (vtebench-style: `dense_cells`, `scrolling`,
`scrolling_in_region`, `unicode`, `truecolor`, `cursor_motion`).

## 1. Headless pipeline benchmark (any OS, CI)

```text
cargo run --release --locked -p egui_term --example term_bench
cargo run --release --locked -p egui_term --example term_bench -- --quick --json target/bench.json
```

No window, GPU or shell. It measures the parts ButtonsCLI owns:

| Stage | What it times |
| --- | --- |
| Parser | bytes through the Alacritty parser into the grid (MiB/s) |
| Snapshot | `TerminalBackend::sync()` with 0 / 1k / 10k / 100k retained lines |
| Frame | `TerminalView` shape build + egui tessellation for one 200x62 screen |
| `cat` | parser plus one frame per 16.6 ms, like a real `cat bigfile` |

GPU upload/draw time is not included. CI runs `--quick` and uploads the JSON.

### Results (cloud Linux, Xeon 2.1 GHz, 4 vCPU, 2026-10-25)

| Metric | Before | After |
| --- | --- | --- |
| `sync()` at 10k scrollback (default) | 27.0 ms | 0.15 ms |
| `sync()` at 100k scrollback | 265 ms | 0.12 ms |
| Frame, scrolling text (sync + build + tessellate) | 3.6 ms | 2.0 ms |
| Frame, dense colored cells | 6.8 ms | 7.1 ms (unchanged: per-cell shapes) |
| `cat` scrolling log | 27–38 MiB/s | 76 MiB/s |
| Parser alone | 68–125 MiB/s | same |

Before: every output frame cloned the whole grid, including all scrollback,
while holding the parser lock. Now only the visible cells are copied.

What is still slow: dense colored screens cost ~3.7 ms to build and ~3.3 ms to
tessellate (113k vertices) per pane, because every cell is its own egui text
shape. Four busy panes approach a 60 fps frame budget on this CPU.

## 2. Comparison against other terminals (Windows)

```powershell
.\scripts\bench-terminals.ps1                       # all installed terminals
.\scripts\bench-terminals.ps1 -Terminals buttonscli,alacritty -MiB 32 -Runs 5
```

Each terminal runs the same writer (`term_bench --cat`) with the same bytes.
The write finishes only as fast as the terminal reads its PTY, so MiB/s is
end-to-end throughput (ConPTY included). It also records time to first window
and peak working set. ButtonsCLI runs in an isolated temporary profile.
Supported: ButtonsCLI, Alacritty, WezTerm, Windows Terminal, conhost, Ghostty
(if installed). Missing terminals are skipped.

Not measured by the script:

- **Input latency** (key press to pixel). Use
  [Typometer](https://pavelfatin.com/typometer/) on each terminal with the same
  font size and window size. This is the number people "feel".
- **Idle CPU/GPU** with a blinking cursor; check Task Manager after 1 minute.

## Tests that guard rendering parity

`cargo test -p egui_term` paints frames headlessly and checks the shapes:
underline variants, strikeout, hidden text, combining marks, cursor
visibility/shape/focus and the character under a block cursor.
`cargo test --lib fonts::tests` checks that every bundled terminal font can
draw box drawing, block elements, braille, powerline/nerd icons, emoji and CJK
through its fallback chain.
