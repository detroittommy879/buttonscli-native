//! Agent control client: the `buttonsclictl` console binary (and
//! `buttonscli ctl …` / `buttonscli mcp`). Talks to a running app's loopback
//! control API (`docs/CONTROL-API.md`). Replaces the Node helpers, which stay
//! available as an optional legacy route for now.

pub mod cli;
pub mod client;
pub mod mcp;

/// File name of the companion console binary next to the app.
pub const CTL_BINARY: &str = if cfg!(windows) {
    "buttonsclictl.exe"
} else {
    "buttonsclictl"
};

/// Entry point for the arguments after the program name: `mcp` serves MCP
/// on stdio, anything else is a CLI command. Returns the exit code.
pub fn main(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("mcp") if args.len() == 1 => mcp::run(),
        Some("mcp") => {
            eprintln!(
                "mcp takes no arguments; set BUTTONSCLI_CONTROL_INFO_PATH to pick an instance."
            );
            2
        }
        _ => cli::run(args),
    }
}

#[cfg(test)]
mod tests;
