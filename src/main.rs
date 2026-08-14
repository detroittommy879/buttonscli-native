#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod theme;

#[cfg(not(target_arch = "wasm32"))]
mod terminal;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "buttonscli=info".into()),
        )
        .compact()
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ButtonsCLI")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ButtonsCLI",
        options,
        Box::new(|cc| Ok(Box::new(app::ButtonsApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
