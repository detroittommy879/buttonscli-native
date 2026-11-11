#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use buttonscli::startup::StartupRequest;
    // `buttonscli ctl <command>` / `buttonscli mcp`: the agent control helper,
    // for installs that ship only this file. On Windows prefer
    // `buttonsclictl.exe`: this GUI build prints only into pipes, not consoles.
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("ctl") => std::process::exit(buttonscli::ctl::main(&args[1..])),
        Some("mcp") => std::process::exit(buttonscli::ctl::main(&args)),
        _ => {}
    }
    let startup = match StartupRequest::parse(std::env::args().skip(1)) {
        Ok(StartupRequest::Launch(options)) => options,
        Ok(StartupRequest::Help) => {
            println!("{}", buttonscli::startup::HELP);
            return Ok(());
        }
        Ok(StartupRequest::Version) => {
            println!("ButtonsCLI {}", buttonscli::BUILD_VERSION);
            return Ok(());
        }
        Err(error) => {
            eprintln!("{error}\n\n{}", buttonscli::startup::HELP);
            std::process::exit(2);
        }
    };
    // Log file and crash reports in ~/.buttonscli-native/logs (local only).
    buttonscli::diagnostics::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ButtonsCLI")
            .with_icon(buttonscli::app_icon())
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
    let defaults = WgpuSetupCreateNew::default();
    let base = defaults.device_descriptor.clone();
    let setup = WgpuSetupCreateNew {
        power_preference: wgpu::PowerPreference::from_env()
            .unwrap_or(wgpu::PowerPreference::LowPower),
        device_descriptor: std::sync::Arc::new(move |adapter| wgpu::DeviceDescriptor {
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            ..base(adapter)
        }),
        ..defaults
    };
    eframe::egui_wgpu::WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}
