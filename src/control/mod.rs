//! Authenticated loopback control for an explicitly selected native instance.

#[cfg(not(target_arch = "wasm32"))]
mod server;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use server::ControlServer;
