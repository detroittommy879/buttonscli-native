//! `buttonsclictl mcp`: a stdio MCP server (newline-delimited JSON-RPC 2.0)
//! with the same 14 tools as `scripts/buttonscli-mcp.mjs`, plus tool
//! annotations so clients can tell reads from terminal input.

use std::io::{BufRead, Read, Write};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::cli::{object, read_path, wait_for_quiet, wait_for_text};
use super::client::{encode_component, Connection};

/// Newest protocol first; an older version a client asks for is echoed back.
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

const HELP_TEXT: &str = "ButtonsCLI Native MCP tools mirror the local terminal control surface.

Available tools:
- help
- status
- tabs
- create_tab
- rename_tab
- open_layout
- read
- wait_for_text
- wait_for_quiet
- send
- run
- key
- presets
- preset_run

Selectors:
- active: currently focused tab
- <ptyId>: numeric PTY id
- <tabId>: internal tab id
- <title>: exact tab title, case-insensitive
";

/// Opens a connection for each tool call (the app may have restarted).
pub type Connector = Arc<dyn Fn() -> Result<Connection, String> + Send + Sync>;

fn tool(name: &str, title: &str, description: &str, kind: Kind, schema: Value) -> Value {
    let (read_only, destructive) = match kind {
        Kind::Read => (true, false),
        Kind::Layout => (false, false),
        Kind::Input => (false, true),
    };
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": schema,
        "annotations": {
            "title": title,
            "readOnlyHint": read_only,
            "destructiveHint": destructive,
            "idempotentHint": read_only,
            "openWorldHint": false,
        },
    })
}

#[derive(Clone, Copy)]
enum Kind {
    /// Reads app or terminal state only.
    Read,
    /// Creates or renames tabs; sends nothing to a shell.
    Layout,
    /// Sends input to a terminal: commands may run.
    Input,
}

fn empty() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

fn payload_schema(extra: Value) -> Value {
    let mut properties = json!({
        "tab": { "type": "string" },
        "text": { "type": "string" },
        "payloadBase64": { "type": "string" },
        "delivery": { "enum": ["raw", "bracketed", "slow-typed"] },
        "delayMs": { "type": "number" },
        "enter": { "type": "boolean" },
    });
    if let (Some(base), Some(extra)) = (properties.as_object_mut(), extra.as_object()) {
        base.extend(extra.clone());
    }
    json!({
        "type": "object",
        "properties": properties,
        "oneOf": [{ "required": ["text"] }, { "required": ["payloadBase64"] }],
        "additionalProperties": false,
    })
}

/// The tool list, in the Node helper's order.
pub fn tools() -> Vec<Value> {
    vec![
        tool("help", "Help", "Show the ButtonsCLI MCP tool list and selector rules.", Kind::Read, empty()),
        tool("status", "Status", "Show local ButtonsCLI control server status.", Kind::Read, empty()),
        tool("tabs", "Tabs", "List current ButtonsCLI tabs and PTYs.", Kind::Read, empty()),
        tool(
            "create_tab",
            "Create Tab",
            "Create a new terminal tab.",
            Kind::Layout,
            json!({
                "type": "object",
                "properties": { "name": { "type": "string" }, "shell": { "type": "string" }, "cwd": { "type": "string" } },
                "required": ["name"],
                "additionalProperties": false,
            }),
        ),
        tool(
            "rename_tab",
            "Rename Tab",
            "Rename an existing tab selected by active/title/id.",
            Kind::Layout,
            json!({
                "type": "object",
                "properties": { "tab": { "type": "string" }, "name": { "type": "string" } },
                "required": ["name"],
                "additionalProperties": false,
            }),
        ),
        tool(
            "open_layout",
            "Open Layout",
            "Open named tabs into a horizontal, vertical, or grid layout.",
            Kind::Layout,
            json!({
                "type": "object",
                "properties": {
                    "layout": { "type": "string", "enum": ["horizontal", "vertical", "grid"] },
                    "names": { "type": "array", "items": { "type": "string" }, "minItems": 2 },
                    "columns": { "type": "number" },
                    "shell": { "type": "string" },
                    "cwd": { "type": "string" },
                },
                "required": ["layout", "names"],
                "additionalProperties": false,
            }),
        ),
        tool(
            "read",
            "Read Output",
            "Read recent terminal output from a tab.",
            Kind::Read,
            json!({
                "type": "object",
                "properties": {
                    "tab": { "type": "string" },
                    "chars": { "type": "number" },
                    "lines": { "type": "number" },
                    "from": { "type": "string", "enum": ["top", "bottom"] },
                },
                "additionalProperties": false,
            }),
        ),
        tool(
            "wait_for_text",
            "Wait For Text",
            "Poll a tab until matching text appears.",
            Kind::Read,
            json!({
                "type": "object",
                "properties": {
                    "tab": { "type": "string" },
                    "text": { "type": "string" },
                    "chars": { "type": "number" },
                    "timeoutMs": { "type": "number" },
                    "intervalMs": { "type": "number" },
                    "ignoreCase": { "type": "boolean" },
                },
                "required": ["text"],
                "additionalProperties": false,
            }),
        ),
        tool(
            "wait_for_quiet",
            "Wait For Quiet",
            "Poll a tab until output stays unchanged for a quiet window.",
            Kind::Read,
            json!({
                "type": "object",
                "properties": {
                    "tab": { "type": "string" },
                    "chars": { "type": "number" },
                    "quietMs": { "type": "number" },
                    "timeoutMs": { "type": "number" },
                    "intervalMs": { "type": "number" },
                },
                "additionalProperties": false,
            }),
        ),
        tool(
            "send",
            "Send Payload",
            "Send text to a tab, optionally pressing Enter. Prefer payloadBase64 for multiline or quote-heavy content.",
            Kind::Input,
            payload_schema(json!({})),
        ),
        tool(
            "run",
            "Run Command",
            "Send command text, optionally press Enter, and wait for quiet output or matching text. Prefer payloadBase64 for multiline or quote-heavy content.",
            Kind::Input,
            payload_schema(json!({
                "chars": { "type": "number" },
                "quietMs": { "type": "number" },
                "maxWaitMs": { "type": "number" },
                "intervalMs": { "type": "number" },
                "waitForText": { "type": "string" },
                "ignoreCase": { "type": "boolean" },
            })),
        ),
        tool(
            "key",
            "Send Key",
            "Send a control key like ctrl+c to a tab.",
            Kind::Input,
            json!({
                "type": "object",
                "properties": { "tab": { "type": "string" }, "key": { "type": "string" } },
                "required": ["key"],
                "additionalProperties": false,
            }),
        ),
        tool("presets", "List Presets", "List saved terminal presets.", Kind::Read, empty()),
        tool(
            "preset_run",
            "Run Preset",
            "Run a saved preset into a selected tab.",
            Kind::Input,
            json!({
                "type": "object",
                "properties": { "label": { "type": "string" }, "tab": { "type": "string" } },
                "required": ["label"],
                "additionalProperties": false,
            }),
        ),
    ]
}

fn text_result(text: String, structured: Value, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": structured,
        "isError": is_error,
    })
}

fn json_result(value: Value) -> Value {
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    // structuredContent must be an object.
    let structured = if value.is_object() {
        value
    } else {
        json!({ "result": value })
    };
    text_result(text, structured, false)
}

struct Args<'a>(&'a Value);

impl Args<'_> {
    fn string(&self, key: &str) -> Option<String> {
        self.0
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
    }
    fn number(&self, key: &str) -> Option<f64> {
        self.0
            .get(key)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    }
    fn number_value(&self, key: &str) -> Option<Value> {
        self.number(key).map(super::client::number)
    }
    fn boolean(&self, key: &str) -> Option<bool> {
        self.0.get(key).and_then(Value::as_bool)
    }
    fn tab(&self) -> String {
        self.string("tab").unwrap_or_else(|| "active".into())
    }
    /// `text` or `payloadBase64`, exactly one.
    fn payload(&self, tool: &str) -> Result<(&'static str, String), String> {
        match (self.string("text"), self.string("payloadBase64")) {
            (Some(text), None) => Ok(("text", text)),
            (None, Some(data)) => Ok(("payloadBase64", data)),
            _ => Err(format!(
                "{tool} requires exactly one payload source: text or payloadBase64."
            )),
        }
    }
}

fn is_tool(name: &str) -> bool {
    tools().iter().any(|tool| tool["name"] == name)
}

/// Runs one tool. Errors become `isError` results, like the Node helper.
fn call_tool(name: &str, arguments: &Value, connector: &Connector) -> Value {
    if name == "help" {
        return text_result(HELP_TEXT.to_owned(), json!({ "help": HELP_TEXT }), false);
    }
    let result = connector().and_then(|connection| run_tool(name, &Args(arguments), &connection));
    match result {
        Ok(value) => json_result(value),
        Err(error) => text_result(error.clone(), json!({ "error": error }), true),
    }
}

fn run_tool(name: &str, args: &Args, c: &Connection) -> Result<Value, String> {
    let tab_path = |action: &str| format!("/v1/tabs/{}/{action}", encode_component(&args.tab()));
    match name {
        "status" => c.call("GET", "/v1/status", None),
        "tabs" => c.call("GET", "/v1/tabs", None),
        "presets" => c.call("GET", "/v1/presets", None),
        "create_tab" => c.call(
            "POST",
            "/v1/tabs",
            Some(&object(vec![
                ("name", args.string("name").map(Value::from)),
                ("shell", args.string("shell").map(Value::from)),
                ("cwd", args.string("cwd").map(Value::from)),
            ])),
        ),
        "rename_tab" => c.call(
            "POST",
            &tab_path("rename"),
            Some(&object(vec![(
                "name",
                args.string("name").map(Value::from),
            )])),
        ),
        "open_layout" => {
            let names: Vec<&str> = args
                .0
                .get("names")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|name| !name.is_empty())
                .collect();
            c.call(
                "POST",
                "/v1/layout/open",
                Some(&object(vec![
                    ("layout", args.string("layout").map(Value::from)),
                    ("names", Some(json!(names))),
                    ("columns", args.number_value("columns")),
                    ("shell", args.string("shell").map(Value::from)),
                    ("cwd", args.string("cwd").map(Value::from)),
                ])),
            )
        }
        "read" => c.call(
            "GET",
            &read_path(
                &args.tab(),
                args.number("chars"),
                args.number("lines"),
                args.string("from").as_deref(),
            ),
            None,
        ),
        "wait_for_text" => {
            let text = args.string("text").ok_or("wait_for_text requires text.")?;
            wait_for_text(
                c,
                &args.tab(),
                &text,
                Some(args.number("chars").unwrap_or(12_000.0)),
                args.number("timeoutMs").unwrap_or(15_000.0),
                args.number("intervalMs").unwrap_or(250.0),
                args.boolean("ignoreCase").unwrap_or(false),
            )
        }
        "wait_for_quiet" => wait_for_quiet(
            c,
            &args.tab(),
            args.number("chars").unwrap_or(12_000.0),
            args.number("quietMs").unwrap_or(1_200.0),
            args.number("timeoutMs").unwrap_or(15_000.0),
            args.number("intervalMs").unwrap_or(250.0),
        ),
        "send" => {
            let (key, value) = args.payload("send")?;
            c.call(
                "POST",
                &tab_path("send"),
                Some(&object(vec![
                    (key, Some(Value::from(value))),
                    ("delivery", args.string("delivery").map(Value::from)),
                    ("delayMs", args.number_value("delayMs")),
                    (
                        "enter",
                        Some(Value::from(args.boolean("enter").unwrap_or(false))),
                    ),
                ])),
            )
        }
        "run" => {
            let (key, value) = args.payload("run")?;
            c.call(
                "POST",
                &tab_path("run"),
                Some(&object(vec![
                    (key, Some(Value::from(value))),
                    ("delivery", args.string("delivery").map(Value::from)),
                    ("delayMs", args.number_value("delayMs")),
                    (
                        "enter",
                        Some(Value::from(args.boolean("enter").unwrap_or(true))),
                    ),
                    ("chars", args.number_value("chars")),
                    ("quietMs", args.number_value("quietMs")),
                    ("maxWaitMs", args.number_value("maxWaitMs")),
                    ("intervalMs", args.number_value("intervalMs")),
                    ("waitForText", args.string("waitForText").map(Value::from)),
                    (
                        "ignoreCase",
                        Some(Value::from(args.boolean("ignoreCase").unwrap_or(false))),
                    ),
                ])),
            )
        }
        "key" => {
            let key = args.string("key").ok_or("key requires a key value.")?;
            c.call("POST", &tab_path("key"), Some(&json!({ "key": key })))
        }
        "preset_run" => c.call(
            "POST",
            "/v1/presets/run",
            Some(&object(vec![
                ("label", args.string("label").map(Value::from)),
                ("tab", args.string("tab").map(Value::from)),
            ])),
        ),
        other => Err(format!("Unknown tool: {other}")),
    }
}

type Output = Arc<Mutex<Box<dyn Write + Send>>>;

fn send(output: &Output, message: &Value) {
    if let Ok(mut out) = output.lock() {
        let _ = writeln!(out, "{message}");
        let _ = out.flush();
    }
}

fn reply(output: &Output, id: &Value, result: Value) {
    send(
        output,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    );
}

fn error(output: &Output, id: &Value, code: i64, message: &str) {
    send(
        output,
        &json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
    );
}

/// Handles one message. Tool calls run on their own thread, so a long `run`
/// or wait never blocks `ping` or other calls.
fn handle(
    message: Value,
    output: &Output,
    connector: &Connector,
    workers: &mut Vec<std::thread::JoinHandle<()>>,
) {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let is_request = !id.is_null();
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => {
            let requested = params["protocolVersion"].as_str().unwrap_or_default();
            let version = PROTOCOL_VERSIONS
                .iter()
                .find(|version| **version == requested)
                .unwrap_or(&PROTOCOL_VERSIONS[0]);
            reply(
                output,
                &id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": {
                        "name": "buttonscli-native",
                        "title": "ButtonsCLI",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "instructions": "Tools read and drive terminals in the running ButtonsCLI app. send, run, key and preset_run type into a shell: confirm with the user before running anything destructive.",
                }),
            );
        }
        "ping" => reply(output, &id, json!({})),
        "tools/list" => reply(output, &id, json!({ "tools": tools() })),
        "resources/list" => reply(output, &id, json!({ "resources": [] })),
        "resources/templates/list" => reply(output, &id, json!({ "resourceTemplates": [] })),
        "prompts/list" => reply(output, &id, json!({ "prompts": [] })),
        "tools/call" => {
            let Some(name) = params["name"].as_str().map(str::to_owned) else {
                return error(output, &id, -32602, "tools/call requires a tool name.");
            };
            if !is_tool(&name) {
                return error(output, &id, -32602, &format!("Unknown tool: {name}"));
            }
            let arguments = match params.get("arguments") {
                Some(Value::Object(_)) => params["arguments"].clone(),
                _ => json!({}),
            };
            let (output, connector) = (output.clone(), connector.clone());
            workers.retain(|worker| !worker.is_finished());
            workers.push(std::thread::spawn(move || {
                let result = call_tool(&name, &arguments, &connector);
                reply(&output, &id, result);
            }));
        }
        _ if method.starts_with("notifications/") => {}
        _ if is_request => error(output, &id, -32601, &format!("Method not found: {method}")),
        _ => {}
    }
}

/// Reads one line of at most `MAX_MESSAGE_BYTES`. Returns `None` at EOF and
/// `Some(None)` for an oversized line (its rest is skipped).
fn read_line(input: &mut impl BufRead) -> std::io::Result<Option<Option<Vec<u8>>>> {
    let mut line = Vec::new();
    let read = input
        .by_ref()
        .take(MAX_MESSAGE_BYTES as u64 + 1)
        .read_until(b'\n', &mut line)?;
    if read == 0 {
        return Ok(None);
    }
    if line.len() > MAX_MESSAGE_BYTES && line.last() != Some(&b'\n') {
        let mut rest = Vec::new();
        input.read_until(b'\n', &mut rest)?;
        return Ok(Some(None));
    }
    Ok(Some(Some(line)))
}

/// Serves MCP over `input`/`output` until `input` ends, then waits for
/// in-flight tool calls to finish.
pub fn serve(mut input: impl BufRead, output: Box<dyn Write + Send>, connector: Connector) {
    let output: Output = Arc::new(Mutex::new(output));
    let mut workers = Vec::new();
    while let Ok(Some(line)) = read_line(&mut input) {
        let Some(line) = line else {
            eprintln!("buttonscli-native-mcp: discarded oversized input line.");
            continue;
        };
        let text = String::from_utf8_lossy(&line);
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(text) {
            Ok(Value::Array(batch)) => {
                for message in batch {
                    handle(message, &output, &connector, &mut workers);
                }
            }
            Ok(message) => handle(message, &output, &connector, &mut workers),
            Err(parse_error) => {
                eprintln!("buttonscli-native-mcp: invalid JSON input: {parse_error}");
                error(&output, &Value::Null, -32700, "Parse error");
            }
        }
    }
    for worker in workers {
        let _ = worker.join();
    }
}

/// `buttonsclictl mcp`: serve on the process's stdin/stdout.
pub fn run() -> i32 {
    let stdin = std::io::stdin();
    serve(
        stdin.lock(),
        Box::new(std::io::stdout()),
        Arc::new(super::client::connect),
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn exchange(input: &str) -> Vec<Value> {
        let sink = Sink::default();
        let connector: Connector = Arc::new(|| Err("no app in this test".into()));
        serve(input.as_bytes(), Box::new(sink.clone()), connector);
        let bytes = sink.0.lock().unwrap().clone();
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[test]
    fn handshake_lists_annotated_tools_and_reports_errors_as_results() {
        let replies = exchange(concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "\n\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"status"}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nope"}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":5,"method":"bogus"}"#,
            "\n",
            "not json\n",
        ));
        let by_id = |id: i64| replies.iter().find(|r| r["id"] == id).unwrap();
        assert_eq!(by_id(1)["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(
            by_id(1)["result"]["serverInfo"]["version"],
            env!("CARGO_PKG_VERSION")
        );
        let tools = by_id(2)["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 14);
        let annotation = |name: &str, key: &str| {
            tools.iter().find(|t| t["name"] == name).unwrap()["annotations"][key].clone()
        };
        assert_eq!(annotation("read", "readOnlyHint"), true);
        assert_eq!(annotation("run", "destructiveHint"), true);
        assert_eq!(annotation("create_tab", "destructiveHint"), false);
        let status = &by_id(3)["result"];
        assert_eq!(status["isError"], true);
        assert_eq!(status["structuredContent"]["error"], "no app in this test");
        assert_eq!(by_id(4)["error"]["code"], -32602);
        assert_eq!(by_id(5)["error"]["code"], -32601);
        assert!(replies.iter().any(|r| r["error"]["code"] == -32700));
        assert_eq!(replies.len(), 6, "notifications get no reply");
    }

    #[test]
    fn oversized_lines_are_skipped_and_the_next_message_still_works() {
        let mut input = "x".repeat(MAX_MESSAGE_BYTES + 10);
        input.push('\n');
        input.push_str(r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#);
        input.push('\n');
        let replies = exchange(&input);
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0]["id"], 7);
    }

    #[test]
    fn help_needs_no_running_app() {
        let replies = exchange(concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"help"}}"#,
            "\n"
        ));
        assert_eq!(replies[0]["result"]["isError"], false);
        assert!(replies[0]["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("wait_for_quiet"));
    }
}
