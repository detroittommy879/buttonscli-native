#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use buttonscli::startup::StartupRequest;
    let startup = match StartupRequest::parse(std::env::args().skip(1)) {
        Ok(StartupRequest::Launch(options)) => options,
        Ok(StartupRequest::Help) => {
            println!("{}", buttonscli::startup::HELP);
            return Ok(());
        }
        Ok(StartupRequest::Version) => {
            println!("ButtonsCLI {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Err(error) => {
            eprintln!("{error}\n\n{}", buttonscli::startup::HELP);
            std::process::exit(2);
        }
    };
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
        Box::new(move |cc| {
            Ok(Box::new(buttonscli::app::ButtonsApp::new_with_startup(
                cc, startup,
            )))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
