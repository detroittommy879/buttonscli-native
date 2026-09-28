#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod client;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod credentials;
pub(crate) mod provider;
pub(crate) mod reply;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod transport;
