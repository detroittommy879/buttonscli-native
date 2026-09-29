#[cfg(not(target_arch = "wasm32"))]
mod account;
#[cfg(not(target_arch = "wasm32"))]
mod account_api;
pub mod app;
mod assistant;
#[cfg(not(target_arch = "wasm32"))]
mod control;
#[cfg(not(target_arch = "wasm32"))]
mod cool_stuff;
#[cfg(not(target_arch = "wasm32"))]
mod display;
mod dock;
pub mod features;
#[cfg(not(target_arch = "wasm32"))]
mod feedback;
pub mod fonts;
pub mod i18n;
#[cfg(not(target_arch = "wasm32"))]
mod layout;
#[cfg(not(target_arch = "wasm32"))]
pub mod plugins;
#[cfg(not(target_arch = "wasm32"))]
mod scrollbar;
#[cfg(not(target_arch = "wasm32"))]
mod secret_vault;
#[cfg(not(target_arch = "wasm32"))]
mod session;
mod settings;
mod shortcuts;
pub mod theme;
#[cfg(not(target_arch = "wasm32"))]
mod theme_files;
#[cfg(not(target_arch = "wasm32"))]
mod theme_generation;
mod window_opacity;

#[cfg(not(target_arch = "wasm32"))]
pub mod terminal;

#[cfg(not(target_arch = "wasm32"))]
pub mod storage;

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub struct WebHandle {
        runner: eframe::WebRunner,
    }

    impl Default for WebHandle {
        fn default() -> Self {
            Self::new()
        }
    }

    #[wasm_bindgen]
    impl WebHandle {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Self {
            eframe::WebLogger::init(log::LevelFilter::Info).ok();
            Self {
                runner: eframe::WebRunner::new(),
            }
        }

        pub async fn start(
            &self,
            canvas: web_sys::HtmlCanvasElement,
        ) -> Result<(), wasm_bindgen::JsValue> {
            self.runner
                .start(
                    canvas,
                    eframe::WebOptions::default(),
                    Box::new(|cc| Ok(Box::new(crate::app::ButtonsApp::new(cc)))),
                )
                .await
        }

        pub fn destroy(&self) {
            self.runner.destroy();
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::WebHandle;
