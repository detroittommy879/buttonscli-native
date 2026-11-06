//! `buttonsclictl`: console helper that lets scripts and AI agents drive a
//! running ButtonsCLI app (`buttonsclictl --help`), or serves MCP over stdio
//! (`buttonsclictl mcp`). A separate console binary because the release app
//! is a Windows GUI program, which cannot print to the terminal that ran it.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(buttonscli::ctl::main(&args));
}
