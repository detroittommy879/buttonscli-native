#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod agent;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod client;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod credentials;
pub(crate) mod provider;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod reply;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod simulation;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod transport;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod virtual_terminal;

pub(crate) mod simulation_protocol;

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod browser_simulator;
