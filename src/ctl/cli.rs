//! `buttonsclictl <command>`: the same commands, flags and output as the
//! original Node helper (`scripts/buttonsclictl.mjs`).

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use super::client::{encode_component, number, Connection};

pub const HELP: &str = "buttonsclictl - local control CLI for a running ButtonsCLI app

Usage:
  buttonsclictl status [--json]
  buttonsclictl tabs [--json]
  buttonsclictl create-tab --name <name> [--shell <shell>] [--cwd <cwd>] [--json]
  buttonsclictl rename-tab --name <name> [--tab <selector>] [--json]
  buttonsclictl open-layout --layout <horizontal|vertical|grid> --name <name> --name <name> [--name <name> ...] [--columns <n>] [--shell <shell>] [--cwd <cwd>] [--json]
  buttonsclictl read [--tab <selector>] [--chars <n> | --lines <n>] [--from top|bottom] [--json]
  buttonsclictl wait-for-text --text <text> [--tab <selector>] [--chars <n>] [--timeout-ms <n>] [--interval-ms <n>] [--ignore-case] [--json]
  buttonsclictl wait-for-quiet [--tab <selector>] [--chars <n>] [--quiet-ms <n>] [--timeout-ms <n>] [--interval-ms <n>] [--json]
  buttonsclictl send [--tab <selector>] [--delivery <raw|bracketed|slow-typed>] [--delay-ms <n>] [--enter] [--json] (--text <text> | --base64 <payload> | --file <path> | --stdin | <text...>)
  buttonsclictl run [--tab <selector>] [--delivery <raw|bracketed|slow-typed>] [--delay-ms <n>] [--chars <n>] [--quiet-ms <n>] [--timeout-ms <n>] [--interval-ms <n>] [--wait-for-text <text>] [--ignore-case] [--no-enter] [--json] (--text <text> | --base64 <payload> | --file <path> | --stdin | <text...>)
  buttonsclictl key <key> [--tab <selector>] [--json]
  buttonsclictl presets [--json]
  buttonsclictl preset-run --label <label> [--tab <selector>] [--json]
  buttonsclictl mcp                      (stdio MCP server for AI agents)

Selectors:
  active      currently focused tab
  <ptyId>     numeric PTY id
  tab-<id>    internal tab id
  <title>     exact tab title, case-insensitive

Connection:
  Uses BUTTONSCLI_CONTROL_INFO_PATH (a connection file or the control folder),
  else the only running instance in ~/.buttonscli-native/control.

Examples:
  buttonsclictl create-tab --name \"logs-prod\"
  buttonsclictl read --tab active --lines 120
  buttonsclictl wait-for-text --tab active --text \"server ready\"
  buttonsclictl run --tab active \"git status\"
  buttonsclictl run --tab active --wait-for-text \"Build complete\" \"pnpm build\"
  Get-Content ./script.sh -Raw | buttonsclictl send --tab active --stdin
  buttonsclictl send --tab active --delivery bracketed --file ./long-paste.txt
  buttonsclictl key ctrl+c --tab active
  buttonsclictl preset-run --label \"prod ssh\"
";

#[derive(Debug, Default, PartialEq)]
pub(crate) enum Payload {
    #[default]
    None,
    Text(String),
    Base64(String),
    File(String),
    Stdin,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Help,
    Status,
    Tabs,
    Presets,
    CreateTab {
        name: String,
        shell: Option<String>,
        cwd: Option<String>,
    },
    RenameTab {
        tab: String,
        name: String,
    },
    OpenLayout {
        layout: String,
        names: Vec<String>,
        columns: Option<f64>,
        shell: Option<String>,
        cwd: Option<String>,
    },
    Read {
        tab: String,
        chars: Option<f64>,
        lines: Option<f64>,
        from: String,
    },
    WaitForText {
        tab: String,
        text: String,
        chars: Option<f64>,
        timeout_ms: f64,
        interval_ms: f64,
        ignore_case: bool,
    },
    WaitForQuiet {
        tab: String,
        chars: f64,
        quiet_ms: f64,
        timeout_ms: f64,
        interval_ms: f64,
    },
    Send {
        tab: String,
        payload: Payload,
        delivery: Option<String>,
        delay_ms: Option<f64>,
        enter: bool,
    },
    Run {
        tab: String,
        payload: Payload,
        delivery: Option<String>,
        delay_ms: Option<f64>,
        chars: Option<f64>,
        quiet_ms: f64,
        timeout_ms: f64,
        interval_ms: f64,
        wait_for_text: Option<String>,
        ignore_case: bool,
        enter: bool,
    },
    Key {
        tab: String,
        key: String,
    },
    PresetRun {
        tab: String,
        label: String,
    },
}

struct Tokens(Vec<String>);

impl Tokens {
    fn flag(&mut self, flag: &str) -> bool {
        match self.0.iter().position(|token| token == flag) {
            Some(index) => {
                self.0.remove(index);
                true
            }
            None => false,
        }
    }

    fn option(&mut self, flag: &str) -> Result<Option<String>, String> {
        let Some(index) = self.0.iter().position(|token| token == flag) else {
            return Ok(None);
        };
        match self.0.get(index + 1) {
            Some(value) if !value.is_empty() && !value.starts_with("--") => {
                let value = value.clone();
                self.0.drain(index..=index + 1);
                Ok(Some(value))
            }
            _ => Err(format!("{flag} requires a value.")),
        }
    }

    fn repeated(&mut self, flag: &str) -> Result<Vec<String>, String> {
        let mut values = Vec::new();
        while let Some(value) = self.option(flag)? {
            values.push(value);
        }
        Ok(values)
    }

    fn number(&mut self, flag: &str) -> Result<Option<f64>, String> {
        self.option(flag)?
            .map(|raw| match raw.trim().parse::<f64>() {
                Ok(value) if value.is_finite() && value > 0.0 => Ok(value),
                _ => Err(format!("{flag} must be a positive number.")),
            })
            .transpose()
    }

    fn tab(&mut self) -> Result<String, String> {
        Ok(self.option("--tab")?.unwrap_or_else(|| "active".into()))
    }

    fn delivery(&mut self) -> Result<Option<String>, String> {
        match self.option("--delivery")? {
            None => Ok(None),
            Some(mode) if ["raw", "bracketed", "slow-typed"].contains(&mode.as_str()) => {
                Ok(Some(mode))
            }
            Some(_) => Err("--delivery must be one of: raw, bracketed, slow-typed.".into()),
        }
    }

    fn done(&self) -> Result<(), String> {
        match self.0.first() {
            Some(extra) => Err(format!("Unexpected argument '{extra}'.")),
            None => Ok(()),
        }
    }

    /// Exactly one of --text, --base64, --file, --stdin or positional words.
    fn payload(&mut self, command: &str) -> Result<Payload, String> {
        let text = self.option("--text")?;
        let base64 = self.option("--base64")?;
        let file = self.option("--file")?;
        let stdin = self.flag("--stdin");
        if let Some(extra) = self.0.iter().find(|token| token.starts_with("--")) {
            return Err(format!("Unexpected argument '{extra}'."));
        }
        let positional = (!self.0.is_empty()).then(|| self.0.join(" "));
        self.0.clear();
        let mut sources = Vec::new();
        sources.extend(text.map(Payload::Text));
        sources.extend(base64.map(Payload::Base64));
        sources.extend(positional.map(Payload::Text));
        sources.extend(file.map(Payload::File));
        if stdin {
            sources.push(Payload::Stdin);
        }
        if sources.len() != 1 {
            return Err(format!(
                "{command} requires exactly one payload source: positional text, --text, --base64, --file, or --stdin."
            ));
        }
        Ok(sources.remove(0))
    }
}

/// Parses the arguments after the program name. Returns the command and `--json`.
pub(crate) fn parse(args: &[String]) -> Result<(Command, bool), String> {
    let mut t = Tokens(args.to_vec());
    let json = t.flag("--json");
    if t.0.is_empty() {
        return Ok((Command::Help, json));
    }
    let name = t.0.remove(0);
    let command = match name.as_str() {
        "help" | "--help" | "-h" => Command::Help,
        "status" | "tabs" | "presets" => {
            t.done()?;
            match name.as_str() {
                "status" => Command::Status,
                "tabs" => Command::Tabs,
                _ => Command::Presets,
            }
        }
        "create-tab" => {
            let name = t.option("--name")?;
            let shell = t.option("--shell")?;
            let cwd = t.option("--cwd")?;
            let name = name.ok_or("create-tab requires --name.")?;
            t.done()?;
            Command::CreateTab { name, shell, cwd }
        }
        "rename-tab" => {
            let tab = t.tab()?;
            let name = t.option("--name")?.ok_or("rename-tab requires --name.")?;
            t.done()?;
            Command::RenameTab { tab, name }
        }
        "open-layout" => {
            let layout = t.option("--layout")?;
            let names = t.repeated("--name")?;
            let columns = t.number("--columns")?;
            let shell = t.option("--shell")?;
            let cwd = t.option("--cwd")?;
            let layout = layout.ok_or("open-layout requires --layout.")?;
            if !["horizontal", "vertical", "grid"].contains(&layout.as_str()) {
                return Err("--layout must be one of: horizontal, vertical, grid.".into());
            }
            if names.len() < 2 {
                return Err("open-layout requires at least two --name values.".into());
            }
            t.done()?;
            Command::OpenLayout {
                layout,
                names,
                columns,
                shell,
                cwd,
            }
        }
        "read" => {
            let tab = t.tab()?;
            let chars = t.number("--chars")?;
            let lines = t.number("--lines")?;
            let from = t.option("--from")?.unwrap_or_else(|| "bottom".into());
            t.done()?;
            Command::Read {
                tab,
                chars,
                lines,
                from,
            }
        }
        "wait-for-text" => {
            let tab = t.tab()?;
            let text = t.option("--text")?;
            let chars = t.number("--chars")?;
            let timeout_ms = t.number("--timeout-ms")?.unwrap_or(15_000.0);
            let interval_ms = t.number("--interval-ms")?.unwrap_or(250.0);
            let ignore_case = t.flag("--ignore-case");
            let text = text.ok_or("wait-for-text requires --text.")?;
            t.done()?;
            Command::WaitForText {
                tab,
                text,
                chars,
                timeout_ms,
                interval_ms,
                ignore_case,
            }
        }
        "wait-for-quiet" => {
            let tab = t.tab()?;
            let chars = t.number("--chars")?.unwrap_or(12_000.0);
            let quiet_ms = t.number("--quiet-ms")?.unwrap_or(1_200.0);
            let timeout_ms = t.number("--timeout-ms")?.unwrap_or(15_000.0);
            let interval_ms = t.number("--interval-ms")?.unwrap_or(250.0);
            t.done()?;
            Command::WaitForQuiet {
                tab,
                chars,
                quiet_ms,
                timeout_ms,
                interval_ms,
            }
        }
        "send" => {
            let tab = t.tab()?;
            let delivery = t.delivery()?;
            let delay_ms = t.number("--delay-ms")?;
            let enter = t.flag("--enter");
            let payload = t.payload("send")?;
            Command::Send {
                tab,
                payload,
                delivery,
                delay_ms,
                enter,
            }
        }
        "run" => {
            let tab = t.tab()?;
            let delivery = t.delivery()?;
            let delay_ms = t.number("--delay-ms")?;
            let chars = t.number("--chars")?;
            let quiet_ms = t.number("--quiet-ms")?.unwrap_or(1_200.0);
            let timeout_ms = t.number("--timeout-ms")?.unwrap_or(8_000.0);
            let interval_ms = t.number("--interval-ms")?.unwrap_or(250.0);
            let wait_for_text = t.option("--wait-for-text")?;
            let ignore_case = t.flag("--ignore-case");
            let enter = !t.flag("--no-enter");
            let payload = t.payload("run")?;
            Command::Run {
                tab,
                payload,
                delivery,
                delay_ms,
                chars,
                quiet_ms,
                timeout_ms,
                interval_ms,
                wait_for_text,
                ignore_case,
                enter,
            }
        }
        "key" => {
            let tab = t.tab()?;
            if t.0.is_empty() {
                return Err("key requires a key name like ctrl+c.".into());
            }
            let key = t.0.remove(0);
            t.done()?;
            Command::Key { tab, key }
        }
        "preset-run" => {
            let tab = t.tab()?;
            let label = t.option("--label")?.ok_or("preset-run requires --label.")?;
            t.done()?;
            Command::PresetRun { tab, label }
        }
        other => {
            return Err(format!(
                "Unknown command '{other}'. Use --help to see available commands."
            ))
        }
    };
    Ok((command, json))
}

/// Builds a JSON object, leaving out `None` fields like `JSON.stringify` does.
pub(crate) fn object(fields: Vec<(&str, Option<Value>)>) -> Value {
    let mut map = Map::new();
    for (key, value) in fields {
        if let Some(value) = value {
            map.insert(key.to_owned(), value);
        }
    }
    Value::Object(map)
}

fn resolve_payload(payload: &Payload) -> Result<(&'static str, String), String> {
    match payload {
        Payload::Text(text) => Ok(("text", text.clone())),
        Payload::Base64(data) => Ok(("payloadBase64", data.clone())),
        Payload::File(path) => std::fs::read_to_string(path)
            .map(|text| ("text", text))
            .map_err(|error| format!("Could not read {path}: {error}")),
        Payload::Stdin => {
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .map_err(|error| format!("Could not read stdin: {error}"))?;
            Ok(("text", text))
        }
        Payload::None => Err("No send payload source was provided.".into()),
    }
}

pub(crate) fn read_path(
    tab: &str,
    chars: Option<f64>,
    lines: Option<f64>,
    from: Option<&str>,
) -> String {
    let mut query = Vec::new();
    if let Some(chars) = chars {
        query.push(format!("chars={}", number(chars)));
    }
    if let Some(lines) = lines {
        query.push(format!("lines={}", number(lines)));
    }
    if let Some(from) = from {
        query.push(format!("from={}", encode_component(from)));
    }
    format!(
        "/v1/tabs/{}/read?{}",
        encode_component(tab),
        query.join("&")
    )
}

fn matches(text: &str, needle: &str, ignore_case: bool) -> bool {
    if ignore_case {
        text.to_lowercase().contains(&needle.to_lowercase())
    } else {
        text.contains(needle)
    }
}

/// Polls a tab's tail until `needle` appears; each read stays within the deadline.
pub(crate) fn wait_for_text(
    connection: &Connection,
    tab: &str,
    needle: &str,
    chars: Option<f64>,
    timeout_ms: f64,
    interval_ms: f64,
    ignore_case: bool,
) -> Result<Value, String> {
    let started = Instant::now();
    let deadline = started + Duration::from_millis(timeout_ms as u64);
    let path = read_path(tab, chars, None, Some("bottom"));
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let read_timeout = left.clamp(Duration::from_millis(1), Duration::from_secs(10));
        let payload = connection.request("GET", &path, None, read_timeout)?;
        let text = payload["text"].as_str().unwrap_or_default();
        if matches(text, needle, ignore_case) {
            return Ok(json!({
                "ok": true,
                "type": "wait-for-text",
                "needle": needle,
                "ignoreCase": ignore_case,
                "elapsedMs": started.elapsed().as_millis() as u64,
                "tab": payload["tab"],
                "text": text,
            }));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Timed out after {}ms waiting for text '{needle}' on tab '{tab}'.",
                number(timeout_ms)
            ));
        }
        let left = deadline.saturating_duration_since(Instant::now());
        std::thread::sleep(
            Duration::from_millis(interval_ms as u64)
                .min(left)
                .max(Duration::from_millis(1)),
        );
    }
}

/// Polls until the tail and its update time stop changing for `quiet_ms`.
pub(crate) fn wait_for_quiet(
    connection: &Connection,
    tab: &str,
    chars: f64,
    quiet_ms: f64,
    timeout_ms: f64,
    interval_ms: f64,
) -> Result<Value, String> {
    let started = Instant::now();
    let deadline = started + Duration::from_millis(timeout_ms as u64);
    let path = read_path(tab, Some(chars), None, Some("bottom"));
    let mut last_signature: Option<String> = None;
    let mut stable_since = Instant::now();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let read_timeout = left.clamp(Duration::from_millis(1), Duration::from_secs(10));
        let payload = connection.request("GET", &path, None, read_timeout)?;
        let now = Instant::now();
        let text = payload["text"].as_str().unwrap_or_default();
        let signature = format!("{}:{text}", payload["tab"]["lastUpdatedAtMs"]);
        if last_signature.as_deref() != Some(signature.as_str()) {
            last_signature = Some(signature);
            stable_since = now;
        }
        if now.duration_since(stable_since) >= Duration::from_millis(quiet_ms as u64) {
            return Ok(json!({
                "ok": true,
                "type": "wait-for-quiet",
                "elapsedMs": now.duration_since(started).as_millis() as u64,
                "quietMs": number(quiet_ms),
                "tab": payload["tab"],
                "text": text,
            }));
        }
        if now >= deadline {
            return Err(format!(
                "Timed out after {}ms waiting for quiet on tab '{tab}'.",
                number(timeout_ms)
            ));
        }
        let left = deadline.saturating_duration_since(Instant::now());
        std::thread::sleep(
            Duration::from_millis(interval_ms as u64)
                .min(left)
                .max(Duration::from_millis(1)),
        );
    }
}

fn pad(value: &Value, width: usize) -> String {
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    format!("{text:<width$}")
}

fn render_tabs(out: &mut dyn Write, tabs: &Value) -> std::io::Result<()> {
    writeln!(
        out,
        "{} {} {} {} {} TAB ID",
        pad(&json!("A"), 2),
        pad(&json!("PTY"), 6),
        pad(&json!("TITLE"), 24),
        pad(&json!("SHELL"), 16),
        pad(&json!("STORED"), 8)
    )?;
    for tab in tabs.as_array().into_iter().flatten() {
        let shell = tab["shell"]
            .as_str()
            .filter(|shell| !shell.is_empty())
            .unwrap_or("default");
        writeln!(
            out,
            "{} {} {} {} {} {}",
            pad(&json!(if tab["isActive"] == true { "*" } else { "" }), 2),
            pad(&tab["ptyId"], 6),
            pad(&tab["title"], 24),
            pad(&json!(shell), 16),
            pad(&tab["storedOutputChars"], 8),
            tab["tabId"].as_str().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn render_presets(out: &mut dyn Write, presets: &Value) -> std::io::Result<()> {
    writeln!(
        out,
        "{} {} {} COMMAND",
        pad(&json!("KIND"), 12),
        pad(&json!("ENTER"), 7),
        pad(&json!("LABEL"), 24)
    )?;
    for preset in presets.as_array().into_iter().flatten() {
        writeln!(
            out,
            "{} {} {} {}",
            pad(&preset["kind"], 12),
            pad(
                &json!(if preset["sendEnter"] == true {
                    "yes"
                } else {
                    "no"
                }),
                7
            ),
            pad(&preset["label"], 24),
            preset["command"].as_str().unwrap_or_default()
        )?;
    }
    Ok(())
}

fn tab_title(payload: &Value) -> &str {
    payload["tab"]["title"].as_str().unwrap_or_default()
}

fn write_snapshot(out: &mut dyn Write, text: &str) -> std::io::Result<()> {
    if !text.is_empty() {
        out.write_all(text.as_bytes())?;
        if !text.ends_with('\n') {
            out.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn render_delivery(out: &mut dyn Write, payload: &Value) -> std::io::Result<()> {
    if let Some(bytes) = payload["bytesSent"].as_u64() {
        writeln!(out, "bytes sent: {bytes}")?;
    }
    if let Some(delivery) = payload["delivery"].as_str() {
        writeln!(out, "delivery: {delivery}")?;
    }
    Ok(())
}

fn render_action(out: &mut dyn Write, payload: &Value) -> std::io::Result<()> {
    writeln!(out, "{}", payload["detail"].as_str().unwrap_or_default())?;
    render_delivery(out, payload)?;
    if payload["tab"].is_object() {
        let shell = payload["tab"]["shell"]
            .as_str()
            .filter(|shell| !shell.is_empty())
            .unwrap_or("default");
        writeln!(
            out,
            "{} ({shell}, PTY {})",
            tab_title(payload),
            payload["tab"]["ptyId"]
        )?;
    }
    Ok(())
}

fn render_run(out: &mut dyn Write, payload: &Value) -> std::io::Result<()> {
    let elapsed = &payload["elapsedMs"];
    let reason = payload["completionReason"].as_str();
    if payload["timedOut"] == true || reason == Some("timeout") {
        writeln!(
            out,
            "{} timed out after {elapsed}ms. Returning the latest captured output.",
            tab_title(payload)
        )?;
    } else if let (Some("matched-text"), Some(matched)) = (reason, payload["matchedText"].as_str())
    {
        writeln!(
            out,
            "Matched '{matched}' on {} after {elapsed}ms.",
            tab_title(payload)
        )?;
    } else {
        writeln!(
            out,
            "{} stayed quiet after {elapsed}ms total.",
            tab_title(payload)
        )?;
    }
    render_delivery(out, payload)?;
    write_snapshot(out, payload["text"].as_str().unwrap_or_default())
}

/// Runs a parsed command against `connection`, writing the result to `out`.
pub(crate) fn execute(
    connection: &Connection,
    command: &Command,
    json_output: bool,
    out: &mut dyn Write,
) -> Result<(), String> {
    let io = |error: std::io::Error| error.to_string();
    let payload = match command {
        Command::Help => {
            out.write_all(HELP.as_bytes()).map_err(io)?;
            return Ok(());
        }
        Command::Status => connection.call("GET", "/v1/status", None)?,
        Command::Tabs => connection.call("GET", "/v1/tabs", None)?,
        Command::Presets => connection.call("GET", "/v1/presets", None)?,
        Command::CreateTab { name, shell, cwd } => connection.call(
            "POST",
            "/v1/tabs",
            Some(&object(vec![
                ("name", Some(json!(name))),
                ("shell", shell.as_ref().map(|v| json!(v))),
                ("cwd", cwd.as_ref().map(|v| json!(v))),
            ])),
        )?,
        Command::RenameTab { tab, name } => connection.call(
            "POST",
            &format!("/v1/tabs/{}/rename", encode_component(tab)),
            Some(&json!({ "name": name })),
        )?,
        Command::OpenLayout {
            layout,
            names,
            columns,
            shell,
            cwd,
        } => connection.call(
            "POST",
            "/v1/layout/open",
            Some(&object(vec![
                ("names", Some(json!(names))),
                ("layout", Some(json!(layout))),
                ("columns", columns.map(number)),
                ("shell", shell.as_ref().map(|v| json!(v))),
                ("cwd", cwd.as_ref().map(|v| json!(v))),
            ])),
        )?,
        Command::Read {
            tab,
            chars,
            lines,
            from,
        } => connection.call("GET", &read_path(tab, *chars, *lines, Some(from)), None)?,
        Command::WaitForText {
            tab,
            text,
            chars,
            timeout_ms,
            interval_ms,
            ignore_case,
        } => wait_for_text(
            connection,
            tab,
            text,
            *chars,
            *timeout_ms,
            *interval_ms,
            *ignore_case,
        )?,
        Command::WaitForQuiet {
            tab,
            chars,
            quiet_ms,
            timeout_ms,
            interval_ms,
        } => wait_for_quiet(
            connection,
            tab,
            *chars,
            *quiet_ms,
            *timeout_ms,
            *interval_ms,
        )?,
        Command::Send {
            tab,
            payload,
            delivery,
            delay_ms,
            enter,
        } => {
            let (key, value) = resolve_payload(payload)?;
            connection.call(
                "POST",
                &format!("/v1/tabs/{}/send", encode_component(tab)),
                Some(&object(vec![
                    (key, Some(json!(value))),
                    ("delivery", delivery.as_ref().map(|v| json!(v))),
                    ("delayMs", delay_ms.map(number)),
                    ("enter", Some(json!(enter))),
                ])),
            )?
        }
        Command::Run {
            tab,
            payload,
            delivery,
            delay_ms,
            chars,
            quiet_ms,
            timeout_ms,
            interval_ms,
            wait_for_text,
            ignore_case,
            enter,
        } => {
            let (key, value) = resolve_payload(payload)?;
            connection.call(
                "POST",
                &format!("/v1/tabs/{}/run", encode_component(tab)),
                Some(&object(vec![
                    (key, Some(json!(value))),
                    ("delivery", delivery.as_ref().map(|v| json!(v))),
                    ("delayMs", delay_ms.map(number)),
                    ("enter", Some(json!(enter))),
                    ("chars", chars.map(number)),
                    ("quietMs", Some(number(*quiet_ms))),
                    ("maxWaitMs", Some(number(*timeout_ms))),
                    ("intervalMs", Some(number(*interval_ms))),
                    ("waitForText", wait_for_text.as_ref().map(|v| json!(v))),
                    ("ignoreCase", Some(json!(ignore_case))),
                ])),
            )?
        }
        Command::Key { tab, key } => connection.call(
            "POST",
            &format!("/v1/tabs/{}/key", encode_component(tab)),
            Some(&json!({ "key": key })),
        )?,
        Command::PresetRun { tab, label } => connection.call(
            "POST",
            "/v1/presets/run",
            Some(&json!({ "label": label, "tab": tab })),
        )?,
    };
    if json_output {
        let text = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        return writeln!(out, "{text}").map_err(io);
    }
    let result = match command {
        Command::Status => writeln!(
            out,
            "baseUrl: {}\ninfoPath: {}\nconnectedTabs: {}",
            payload["baseUrl"].as_str().unwrap_or_default(),
            payload["infoPath"].as_str().unwrap_or_default(),
            payload["connectedTabs"]
        ),
        Command::Tabs => render_tabs(out, &payload["tabs"]),
        Command::Presets => render_presets(out, &payload["presets"]),
        Command::CreateTab { .. } | Command::RenameTab { .. } => render_action(out, &payload),
        Command::OpenLayout { .. } => {
            writeln!(out, "{}", payload["detail"].as_str().unwrap_or_default())
                .and_then(|()| render_tabs(out, &payload["tabs"]))
        }
        Command::Read { .. } => write_snapshot(out, payload["text"].as_str().unwrap_or_default()),
        Command::WaitForText { text, .. } => writeln!(
            out,
            "Matched '{text}' on {} after {}ms.",
            tab_title(&payload),
            payload["elapsedMs"]
        )
        .and_then(|()| write_snapshot(out, payload["text"].as_str().unwrap_or_default())),
        Command::WaitForQuiet { .. } => writeln!(
            out,
            "{} stayed quiet for {}ms after {}ms total.",
            tab_title(&payload),
            payload["quietMs"],
            payload["elapsedMs"]
        )
        .and_then(|()| write_snapshot(out, payload["text"].as_str().unwrap_or_default())),
        Command::Run { .. } => render_run(out, &payload),
        Command::Send { .. } | Command::Key { .. } | Command::PresetRun { .. } => writeln!(
            out,
            "{} {}",
            payload["detail"].as_str().unwrap_or_default(),
            tab_title(&payload)
        ),
        Command::Help => Ok(()),
    };
    result.map_err(io)
}

/// `buttonsclictl <args>`: 0 on success, 1 on any error (printed to stderr).
pub fn run(args: &[String]) -> i32 {
    let mut stdout = std::io::stdout().lock();
    let result = parse(args).and_then(|(command, json_output)| {
        if command == Command::Help {
            return stdout
                .write_all(HELP.as_bytes())
                .map_err(|error| error.to_string());
        }
        let connection = super::client::connect()?;
        execute(&connection, &command, json_output, &mut stdout)
    });
    let _ = stdout.flush();
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn parses_node_compatible_commands_and_defaults() {
        assert_eq!(parse(&[]).unwrap().0, Command::Help);
        let (command, json) = parse(&args("tabs --json")).unwrap();
        assert_eq!((command, json), (Command::Tabs, true));
        assert_eq!(
            parse(&args("run --tab Deploy git status")).unwrap().0,
            Command::Run {
                tab: "Deploy".into(),
                payload: Payload::Text("git status".into()),
                delivery: None,
                delay_ms: None,
                chars: None,
                quiet_ms: 1_200.0,
                timeout_ms: 8_000.0,
                interval_ms: 250.0,
                wait_for_text: None,
                ignore_case: false,
                enter: true,
            }
        );
        assert_eq!(
            parse(&args("key ctrl+c")).unwrap().0,
            Command::Key {
                tab: "active".into(),
                key: "ctrl+c".into()
            }
        );
        let (layout, _) = parse(&args(
            "open-layout --layout grid --name a --name b --columns 2",
        ))
        .unwrap();
        assert!(
            matches!(layout, Command::OpenLayout { ref names, columns: Some(c), .. } if names.len() == 2 && c == 2.0)
        );
    }

    #[test]
    fn rejects_what_the_node_helper_rejects() {
        for (input, message) in [
            ("send", "exactly one payload source"),
            ("send --text a b", "exactly one payload source"),
            ("send --bogus x", "Unexpected argument '--bogus'"),
            ("read --chars -5", "--chars must be a positive number."),
            ("read --tab", "--tab requires a value."),
            ("open-layout --layout grid --name a", "at least two --name"),
            (
                "open-layout --layout diagonal --name a --name b",
                "--layout must be one of",
            ),
            ("send --delivery fast hi", "--delivery must be one of"),
            ("tabs extra", "Unexpected argument 'extra'"),
            ("frobnicate", "Unknown command 'frobnicate'"),
        ] {
            let error = parse(&args(input)).unwrap_err();
            assert!(error.contains(message), "{input}: {error}");
        }
    }

    #[test]
    fn read_paths_encode_selectors_and_send_integers() {
        assert_eq!(
            read_path("my tab", Some(120.0), None, Some("bottom")),
            "/v1/tabs/my%20tab/read?chars=120&from=bottom"
        );
    }
}
