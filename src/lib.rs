pub mod app;
pub mod theme;

#[cfg(not(target_arch = "wasm32"))]
pub mod terminal;

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
