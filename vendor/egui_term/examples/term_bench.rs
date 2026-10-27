//! Headless terminal pipeline benchmark: parser, grid snapshot, egui shape
//! build and tessellation. Needs no GPU, window or shell, so it runs in CI.
//!
//! ```text
//! cargo build --release --locked -p egui_term --example term_bench
//! ./target/release/examples/term_bench
//! ./target/release/examples/term_bench --quick --json out.json
//! ```
//!
//! Workloads mirror vtebench's categories so numbers can be read next to
//! Alacritty/WezTerm/Windows Terminal results from `scripts/bench-terminals.ps1`.
//! That script also uses two helper modes of this binary:
//!
//! ```text
//! term_bench --emit DIR [--mib N]   write each workload to DIR/<name>.vt
//! term_bench --cat FILE --out JSON  write FILE to this terminal, record timing
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Pos2, RawInput, Rect, Vec2};
use egui_term::{TerminalBackend, TerminalView};

const SCREEN: Vec2 = Vec2::new(1600.0, 1000.0);

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

/// Deterministic vtebench-style workloads, each roughly `target` bytes.
fn workloads(target: usize) -> Vec<(&'static str, Vec<u8>)> {
    let mut rng = Lcg(0x5eed);
    let mut out = Vec::new();

    // dense_cells: every cell rewritten with its own colors and attributes.
    let mut dense = Vec::with_capacity(target + 4096);
    while dense.len() < target {
        dense.extend_from_slice(b"\x1b[H");
        for _ in 0..50 {
            for _ in 0..190 {
                let fg = rng.below(256);
                let bg = rng.below(256);
                let attr = [0, 1, 3, 4, 7][rng.below(5) as usize];
                let ch = b'!' + rng.below(94) as u8;
                dense.extend_from_slice(
                    format!("\x1b[{attr};38;5;{fg};48;5;{bg}m").as_bytes(),
                );
                dense.push(ch);
            }
            dense.extend_from_slice(b"\r\n");
        }
    }
    out.push(("dense_cells", dense));

    // scrolling: plain `cat` of a log file.
    let mut scrolling = Vec::with_capacity(target + 256);
    let mut line = 0u64;
    while scrolling.len() < target {
        line += 1;
        scrolling.extend_from_slice(
            format!(
                "{line:>8} 2026-10-25T12:00:00Z INFO request handled path=/api/v1/items/{} status=200 bytes={}\r\n",
                rng.below(100_000),
                rng.below(65_536)
            )
            .as_bytes(),
        );
    }
    out.push(("scrolling", scrolling));

    // scrolling_in_region: DECSTBM margins, like a pager or tmux status line.
    let mut region = Vec::with_capacity(target + 256);
    region.extend_from_slice(b"\x1b[2;45r\x1b[45;1H");
    while region.len() < target {
        region.extend_from_slice(
            b"region scrolled line with some words in it\r\n",
        );
    }
    region.extend_from_slice(b"\x1b[r");
    out.push(("scrolling_in_region", region));

    // unicode: CJK, box drawing, emoji, combining marks and powerline glyphs.
    let samples = [
        "│ ├── src/ 한국어 日本語 中文 ─ ┼ ╭─╮ ╰─╯ ▁▂▃▄▅▆▇█ ⣿⡇ ",
        "\u{e0b0}\u{e0b2} \u{f115} git:(main) ✔ ✘ → ⇒ ≠ ≤ ≥ λ π ",
        "e\u{301}a\u{308} 😀👍🚀 ❤\u{fe0f} 🇺🇸 ⚠ ✓ ☐ ☑ ",
    ];
    let mut unicode = Vec::with_capacity(target + 256);
    while unicode.len() < target {
        unicode.extend_from_slice(samples[rng.below(3) as usize].as_bytes());
        if rng.below(3) == 0 {
            unicode.extend_from_slice(b"\r\n");
        }
    }
    out.push(("unicode", unicode));

    // truecolor: 24-bit SGR per character (bat/delta/lolcat style).
    let mut truecolor = Vec::with_capacity(target + 256);
    while truecolor.len() < target {
        for _ in 0..120 {
            truecolor.extend_from_slice(
                format!(
                    "\x1b[38;2;{};{};{}m#",
                    rng.below(256),
                    rng.below(256),
                    rng.below(256)
                )
                .as_bytes(),
            );
        }
        truecolor.extend_from_slice(b"\x1b[0m\r\n");
    }
    out.push(("truecolor", truecolor));

    // cursor_motion: random absolute moves, like a TUI redraw.
    let mut motion = Vec::with_capacity(target + 64);
    while motion.len() < target {
        motion.extend_from_slice(
            format!("\x1b[{};{}Hx", rng.below(50) + 1, rng.below(190) + 1)
                .as_bytes(),
        );
    }
    out.push(("cursor_motion", motion));

    out
}

fn backend(scrollback: usize) -> TerminalBackend {
    let mut backend = TerminalBackend::new_virtual(1, Arc::new(|_: &[u8]| {}));
    backend.set_scrollback_lines(scrollback);
    backend
}

fn input() -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
        ..Default::default()
    }
}

struct Frame {
    run: Duration,
    tessellate: Duration,
    vertices: usize,
}

fn frame(ctx: &egui::Context, backend: &mut TerminalBackend) -> Frame {
    let started = Instant::now();
    let output = ctx.run(input(), |ctx| {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(
            ctx,
            |ui| {
                let size = ui.available_size();
                let view = TerminalView::new(ui, backend).set_size(size);
                ui.add(view);
            },
        );
    });
    let run = started.elapsed();
    let started = Instant::now();
    let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
    let tessellate = started.elapsed();
    let vertices = meshes
        .iter()
        .map(|mesh| match &mesh.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.len(),
            egui::epaint::Primitive::Callback(_) => 0,
        })
        .sum();
    Frame {
        run,
        tessellate,
        vertices,
    }
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn mean_ms(samples: &[Duration]) -> f64 {
    samples.iter().map(|d| ms(*d)).sum::<f64>() / samples.len().max(1) as f64
}

fn p95_ms(samples: &[Duration]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort();
    sorted
        .get((sorted.len() * 95 / 100).min(sorted.len().saturating_sub(1)))
        .map(|d| ms(*d))
        .unwrap_or(0.0)
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|i| args.get(i + 1).cloned())
}

/// Write workloads to files so other terminals can replay the same bytes.
fn emit(dir: &str, mib: usize) {
    std::fs::create_dir_all(dir).expect("create workload directory");
    for (name, data) in workloads(mib << 20) {
        let path = std::path::Path::new(dir).join(format!("{name}.vt"));
        std::fs::write(&path, data).expect("write workload");
        println!("{}", path.display());
    }
}

/// Run inside the terminal under test: stdout is that terminal's PTY, so the
/// write only completes as fast as the terminal (and ConPTY) consumes it.
fn cat(file: &str, out: &str) {
    use std::io::Write;
    let started_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    let data = std::fs::read(file).expect("read workload");
    let started = Instant::now();
    let mut stdout = std::io::stdout().lock();
    for chunk in data.chunks(64 * 1024) {
        stdout.write_all(chunk).expect("write to terminal");
    }
    stdout.write_all(b"\x1b[0m\r\n").ok();
    stdout.flush().ok();
    let seconds = started.elapsed().as_secs_f64();
    let body = format!(
        "{{\"bytes\": {}, \"seconds\": {seconds:.6}, \"started_unix_ms\": {started_unix_ms}}}\n",
        data.len()
    );
    std::fs::write(out, body).expect("write result");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(dir) = arg_value(&args, "--emit") {
        let mib = arg_value(&args, "--mib")
            .and_then(|v| v.parse().ok())
            .unwrap_or(16);
        return emit(&dir, mib);
    }
    if let (Some(file), Some(out)) =
        (arg_value(&args, "--cat"), arg_value(&args, "--out"))
    {
        return cat(&file, &out);
    }
    let quick = args.iter().any(|arg| arg == "--quick");
    let json_path = arg_value(&args, "--json");
    let bytes = if quick { 1 << 20 } else { 8 << 20 };
    let frames = if quick { 30 } else { 120 };
    let mut report = Vec::<serde_like::Entry>::new();

    // Size the grid exactly as the app would, through one real frame.
    let ctx = egui::Context::default();
    let mut probe = backend(10_000);
    frame(&ctx, &mut probe);
    let cell = probe.last_content().terminal_size;
    let (cols, lines) = (probe.columns(), probe.last_content().screen_lines);
    println!(
        "grid {cols}x{lines} cells, cell {}x{} pt, screen {}x{}",
        cell.cell_width, cell.cell_height, SCREEN.x, SCREEN.y
    );

    println!("\n== 1. Parser throughput (no rendering, 10k scrollback) ==");
    for (name, data) in workloads(bytes) {
        let mut backend = backend(10_000);
        frame(&ctx, &mut backend);
        let started = Instant::now();
        for chunk in data.chunks(64 * 1024) {
            backend.feed_virtual_output(chunk);
        }
        let elapsed = started.elapsed();
        let mbps = data.len() as f64 / 1_048_576.0 / elapsed.as_secs_f64();
        println!("{name:<22} {mbps:>8.1} MiB/s");
        report.push(serde_like::Entry::new("parser_mib_s", name, mbps));
    }

    println!("\n== 2. Grid snapshot (sync) cost vs filled scrollback ==");
    for history in [0usize, 1_000, 10_000, 100_000] {
        if quick && history > 10_000 {
            continue;
        }
        let mut backend = backend(history);
        frame(&ctx, &mut backend);
        let line = format!("{}\r\n", "x".repeat(cols - 1));
        for _ in 0..history + lines {
            backend.feed_virtual_output(line.as_bytes());
        }
        backend.sync();
        let mut samples = Vec::new();
        for _ in 0..frames.min(40) {
            backend.feed_virtual_output(b"\x1b[H");
            let started = Instant::now();
            backend.sync();
            samples.push(started.elapsed());
        }
        let mean = mean_ms(&samples);
        println!(
            "{history:>7} lines history  {mean:>8.3} ms/sync  (p95 {:.3})",
            p95_ms(&samples)
        );
        report.push(serde_like::Entry::new(
            "sync_ms",
            &format!("history_{history}"),
            mean,
        ));
    }

    println!("\n== 3. Frame cost (egui shapes + tessellation, no GPU) ==");
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    let all = workloads(256 * 1024);
    cases.push(("empty_screen", Vec::new()));
    for (name, data) in all {
        if matches!(name, "dense_cells" | "scrolling" | "unicode" | "truecolor")
        {
            cases.push((name, data));
        }
    }
    for (name, data) in &cases {
        let ctx = egui::Context::default();
        let mut backend = backend(10_000);
        frame(&ctx, &mut backend);
        backend.feed_virtual_output(data);
        // Warm the galley cache the way a second identical frame would.
        frame(&ctx, &mut backend);
        let mut snapshot = Vec::new();
        let mut run = Vec::new();
        let mut tess = Vec::new();
        let mut vertices = 0;
        for i in 0..frames {
            // Mark dirty without changing content: worst case for idle repaint.
            backend.feed_virtual_output(if i % 2 == 0 {
                b"\x1b[s"
            } else {
                b"\x1b[u"
            });
            let started = Instant::now();
            backend.sync();
            snapshot.push(started.elapsed());
            let f = frame(&ctx, &mut backend);
            run.push(f.run);
            tess.push(f.tessellate);
            vertices = f.vertices;
        }
        let total = mean_ms(&snapshot) + mean_ms(&run) + mean_ms(&tess);
        println!(
            "{name:<14} sync {:>6.3} ms  build {:>7.3} ms  tessellate {:>7.3} ms  total {:>7.3} ms (~{:>5.0} fps CPU cap)  {:>7} vertices",
            mean_ms(&snapshot),
            mean_ms(&run),
            mean_ms(&tess),
            total,
            1000.0 / total,
            vertices
        );
        report.push(serde_like::Entry::new(
            "frame_sync_ms",
            name,
            mean_ms(&snapshot),
        ));
        report.push(serde_like::Entry::new(
            "frame_build_ms",
            name,
            mean_ms(&run),
        ));
        report.push(serde_like::Entry::new(
            "frame_tessellate_ms",
            name,
            mean_ms(&tess),
        ));
        report.push(serde_like::Entry::new(
            "frame_vertices",
            name,
            vertices as f64,
        ));
    }

    println!(
        "\n== 4. `cat` simulation: parse + one frame per 16.6 ms of work =="
    );
    for (name, data) in workloads(bytes) {
        if !matches!(name, "scrolling" | "dense_cells" | "unicode") {
            continue;
        }
        let ctx = egui::Context::default();
        let mut backend = backend(10_000);
        frame(&ctx, &mut backend);
        let started = Instant::now();
        let mut last_frame = Instant::now();
        let mut frames_drawn = 0;
        for chunk in data.chunks(64 * 1024) {
            backend.feed_virtual_output(chunk);
            if last_frame.elapsed() >= Duration::from_micros(16_600) {
                frame(&ctx, &mut backend);
                frames_drawn += 1;
                last_frame = Instant::now();
            }
        }
        frame(&ctx, &mut backend);
        let elapsed = started.elapsed();
        let mbps = data.len() as f64 / 1_048_576.0 / elapsed.as_secs_f64();
        println!(
            "{name:<22} {mbps:>8.1} MiB/s  ({frames_drawn} frames, {:.0} ms)",
            ms(elapsed)
        );
        report.push(serde_like::Entry::new("cat_mib_s", name, mbps));
    }

    if let Some(path) = json_path {
        let body = serde_like::to_json(&report);
        std::fs::write(&path, body).expect("write benchmark JSON");
        println!("\nwrote {path}");
    }
}

/// Tiny JSON writer so the example adds no dependencies to egui_term.
mod serde_like {
    pub struct Entry {
        metric: String,
        case: String,
        value: f64,
    }

    impl Entry {
        pub fn new(metric: &str, case: &str, value: f64) -> Self {
            Self {
                metric: metric.into(),
                case: case.into(),
                value,
            }
        }
    }

    pub fn to_json(entries: &[Entry]) -> String {
        let rows: Vec<String> = entries
            .iter()
            .map(|e| {
                format!(
                    "  {{\"metric\": \"{}\", \"case\": \"{}\", \"value\": {:.4}}}",
                    e.metric, e.case, e.value
                )
            })
            .collect();
        format!("[\n{}\n]\n", rows.join(",\n"))
    }
}
