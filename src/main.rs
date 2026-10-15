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
        wgpu_options: low_memory_wgpu(),
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

/// A terminal draws little: prefer the power-saving GPU and small GPU memory
/// blocks over egui's high-performance defaults. `WGPU_POWER_PREF` still wins.
#[cfg(not(target_arch = "wasm32"))]
fn low_memory_wgpu() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuSetup, WgpuSetupCreateNew};
    use eframe::wgpu;
    let mut setup = WgpuSetupCreateNew::default();
    setup.power_preference =
        wgpu::PowerPreference::from_env().unwrap_or(wgpu::PowerPreference::LowPower);
    let base = setup.device_descriptor.clone();
    setup.device_descriptor = std::sync::Arc::new(move |adapter| wgpu::DeviceDescriptor {
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        ..base(adapter)
    });
    eframe::egui_wgpu::WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}
