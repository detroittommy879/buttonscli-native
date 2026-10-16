#[cfg(not(target_arch = "wasm32"))]
mod account;
#[cfg(not(target_arch = "wasm32"))]
mod account_api;
pub mod app;
mod assistant;
#[cfg(not(target_arch = "wasm32"))]
mod autotile;
#[cfg(not(target_arch = "wasm32"))]
mod control;
#[cfg(not(target_arch = "wasm32"))]
mod cool_stuff;
#[cfg(not(target_arch = "wasm32"))]
mod display;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
mod distribution;
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
mod shader_ai;
mod shortcuts;
#[cfg(not(target_arch = "wasm32"))]
pub mod startup;
#[cfg(not(target_arch = "wasm32"))]
mod terminal_history;
pub mod theme;
mod theme_browser;
#[cfg(not(target_arch = "wasm32"))]
mod theme_files;
#[cfg(not(target_arch = "wasm32"))]
mod theme_generation;
mod ui_text;
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

        pub fn skip_onboarding(&self) -> Result<(), JsValue> {
            self.runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("The preview is still starting."))?
                .skip_browser_onboarding();
            Ok(())
        }

        pub fn open_settings(&self) -> Result<(), JsValue> {
            let mut app = self
                .runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("The preview is still starting."))?;
            app.open_browser_settings();
            Ok(())
        }

        pub fn import_theme(&self, json: &str) -> Result<String, JsValue> {
            let mut app = self
                .runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("The preview is still starting."))?;
            app.import_browser_theme(json)
                .map_err(|error| JsValue::from_str(&error))
        }

        pub fn open_simulator_settings(&self) -> Result<(), JsValue> {
            self.runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("Preview is starting."))?
                .open_browser_simulation_settings();
            Ok(())
        }
        pub fn configure_simulator(&self, available: bool, model: &str) -> Result<(), JsValue> {
            self.runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("Preview is starting."))?
                .configure_browser_backend(available, model);
            Ok(())
        }
        pub fn use_ai_simulator(&self, enabled: bool) -> Result<(), JsValue> {
            self.runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("Preview is starting."))?
                .browser_ai_mode(enabled)
                .map_err(|error| JsValue::from_str(&error))
        }
        pub fn simulator_status(&self) -> Result<String, JsValue> {
            Ok(self
                .runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("Preview is starting."))?
                .browser_status())
        }
        pub fn simulator_snapshot(&self) -> Result<String, JsValue> {
            Ok(self
                .runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("Preview is starting."))?
                .browser_snapshot())
        }
        pub fn take_simulator_request(&self) -> Option<String> {
            self.runner
                .app_mut::<crate::app::ButtonsApp>()?
                .browser_request()
        }
        pub fn finish_simulator_request(
            &self,
            id: u32,
            reply: &str,
            model: &str,
        ) -> Result<bool, JsValue> {
            self.runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("Preview is starting."))?
                .finish_browser_request(id, reply, model)
                .map_err(|error| JsValue::from_str(&error))
        }
        pub fn fail_simulator_request(&self, id: u32, error: &str) {
            if let Some(mut app) = self.runner.app_mut::<crate::app::ButtonsApp>() {
                app.fail_browser_request(id, error);
            }
        }
        pub fn submit_simulator_command(&self, command: &str) -> Result<(), JsValue> {
            let mut app = self
                .runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("Preview is starting."))?;
            if serde_json::from_str::<serde_json::Value>(&app.browser_status()).unwrap()["busy"]
                == true
            {
                return Err(JsValue::from_str("Simulator is busy."));
            }
            app.run_demo_command(command);
            Ok(())
        }
        pub fn cancel_simulator(&self) {
            if let Some(mut app) = self.runner.app_mut::<crate::app::ButtonsApp>() {
                app.browser_cancel();
            }
        }
        pub fn reset_simulator(&self) {
            if let Some(mut app) = self.runner.app_mut::<crate::app::ButtonsApp>() {
                app.browser_reset();
            }
        }
        pub fn undo_simulator(&self) -> bool {
            self.runner
                .app_mut::<crate::app::ButtonsApp>()
                .is_some_and(|mut app| app.browser_undo())
        }
        pub fn import_simulator_setup(&self, json: &str) -> Result<(), JsValue> {
            self.runner
                .app_mut::<crate::app::ButtonsApp>()
                .ok_or_else(|| JsValue::from_str("Preview is starting."))?
                .import_browser_simulation(json)
                .map_err(|error| JsValue::from_str(&error))
        }

        pub fn destroy(&self) {
            self.runner.destroy();
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::WebHandle;

#[cfg(not(target_arch = "wasm32"))]
pub mod web_preview;
