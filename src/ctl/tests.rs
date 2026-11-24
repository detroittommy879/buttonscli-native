//! Both front ends against a fake control API on a real loopback socket.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::cli::{execute, parse};
use super::client::{read_descriptor, Connection};

const TOKEN: &str = "abababababababababababababababababababababababababababababababab";
const ID: &str = "0123456789abcdef0123456789abcdef";

#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    host: String,
    auth: String,
    body: Value,
}

/// Answers every request like the app would and records what it got.
fn fake_api() -> (u16, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = Vec::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                head.push(line.trim_end().to_owned());
            }
            let header = |name: &str| {
                head.iter()
                    .find_map(|line| {
                        let (key, value) = line.split_once(':')?;
                        key.eq_ignore_ascii_case(name)
                            .then(|| value.trim().to_owned())
                    })
                    .unwrap_or_default()
            };
            let length: usize = header("content-length").parse().unwrap_or(0);
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let mut parts = head[0].split_whitespace();
            let method = parts.next().unwrap().to_owned();
            let path = parts.next().unwrap().to_owned();
            log.lock().unwrap().push(Seen {
                method: method.clone(),
                path: path.clone(),
                host: header("host"),
                auth: header("authorization"),
                body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            });
            let tab = json!({"title": "term1", "ptyId": 7, "shell": "", "lastUpdatedAtMs": 1});
            let route = path.split('?').next().unwrap_or_default();
            let (status, reply) = if path.contains("missing") {
                ("404 Not Found", json!({"error": "no such tab"}))
            } else {
                let reply = match route {
                    "/v1/status" => json!({"instanceId": ID, "connectedTabs": 1}),
                    "/v1/tabs" if method == "GET" => json!({"tabs": [tab]}),
                    "/v1/presets" => json!({"presets": []}),
                    r if r.ends_with("/read") => json!({"tab": tab, "text": "hello ready\n"}),
                    r if r.ends_with("/run") => {
                        // `false` fails like a shell with integration reports it.
                        let failing = serde_json::from_slice::<Value>(&body)
                            .is_ok_and(|request| request["text"] == "false");
                        json!({"tab": tab, "text": "out", "elapsedMs": 5,
                            "completionReason": "command-finished",
                            "exitCode": if failing { 1 } else { 0 }})
                    }
                    _ => json!({"ok": true, "detail": "Done.", "tab": tab}),
                };
                ("200 OK", reply)
            };
            let body = reply.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (port, seen)
}

fn connection(port: u16) -> Connection {
    let dir = std::env::temp_dir().join(format!(
        "buttonsclictl-e2e-{}-{port}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{ID}.json"));
    let descriptor = json!({
        "schemaVersion": 1,
        "instanceId": ID,
        "authToken": TOKEN,
        "baseUrl": format!("http://127.0.0.1:{port}"),
        "capabilities": [],
        "updatedAtMs": 1,
    });
    std::fs::write(&path, descriptor.to_string()).unwrap();
    let connection = read_descriptor(&path).unwrap();
    // Discovery checks the instance is live through /v1/status.
    assert_eq!(super::client::discover(&dir).unwrap().instance_id, ID);
    std::fs::remove_dir_all(&dir).unwrap();
    connection
}

fn cli(connection: &Connection, args: &[&str]) -> Result<String, String> {
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    let (command, json_output) = parse(&args)?;
    let mut out = Vec::new();
    execute(connection, &command, json_output, &mut out)?;
    Ok(String::from_utf8(out).unwrap())
}

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

#[test]
fn cli_and_mcp_send_authenticated_requests_with_node_compatible_bodies() {
    let (port, seen) = fake_api();
    let c = connection(port);

    assert!(cli(&c, &["tabs"]).unwrap().contains("term1"));
    let read = cli(&c, &["read", "--tab", "my tab", "--lines", "120"]).unwrap();
    assert!(read.contains("hello ready"));
    cli(&c, &["run", "--timeout-ms", "3000", "git", "status"]).unwrap();
    cli(
        &c,
        &[
            "send",
            "--enter",
            "--delivery",
            "bracketed",
            "--base64",
            "aGk=",
        ],
    )
    .unwrap();
    let layout = [
        "open-layout",
        "--layout",
        "grid",
        "--name",
        "a",
        "--name",
        "b",
        "--columns",
        "2",
    ];
    cli(&c, &layout).unwrap();
    let waited = cli(
        &c,
        &[
            "wait-for-text",
            "--text",
            "READY",
            "--ignore-case",
            "--json",
        ],
    )
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&waited).unwrap()["type"],
        "wait-for-text"
    );
    let error = cli(&c, &["key", "ctrl+c", "--tab", "missing"]).unwrap_err();
    assert_eq!(
        error, "no such tab",
        "the API's error message reaches the user"
    );

    let connector: super::mcp::Connector = Arc::new(move || Ok(connection(port)));
    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"run","arguments":{"tab":"2","text":"ls","maxWaitMs":2000}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"send","arguments":{"text":"a","payloadBase64":"Yg=="}}}"#,
        "\n",
    );
    let sink = Sink::default();
    super::mcp::serve(input.as_bytes(), Box::new(sink.clone()), connector);
    let output = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
    let replies: Vec<Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let reply = |id: i64| replies.iter().find(|r| r["id"] == id).unwrap()["result"].clone();
    assert_eq!(reply(1)["isError"], false);
    assert_eq!(reply(1)["structuredContent"]["text"], "out");
    assert_eq!(
        reply(2)["isError"],
        true,
        "two payload sources are rejected"
    );

    let seen = seen.lock().unwrap().clone();
    for request in &seen {
        assert_eq!(request.host, format!("127.0.0.1:{port}"), "{request:?}");
        assert_eq!(request.auth, format!("Bearer {TOKEN}"));
    }
    let post = |path: &str| {
        seen.iter()
            .find(|request| request.method == "POST" && request.path == path)
            .unwrap_or_else(|| panic!("no POST {path}: {seen:?}"))
            .body
            .clone()
    };
    assert!(seen
        .iter()
        .any(|r| r.path == "/v1/tabs/my%20tab/read?lines=120&from=bottom"));
    let run = post("/v1/tabs/active/run");
    assert_eq!(run["text"], "git status");
    assert_eq!(run["maxWaitMs"], json!(3000), "integers, not 3000.0");
    assert_eq!(run["enter"], true);
    assert_eq!(
        post("/v1/tabs/active/send"),
        json!({"payloadBase64": "aGk=", "delivery": "bracketed", "enter": true})
    );
    assert_eq!(
        post("/v1/layout/open"),
        json!({"names": ["a", "b"], "layout": "grid", "columns": 2})
    );
    assert_eq!(
        post("/v1/tabs/2/run"),
        json!({"text": "ls", "enter": true, "maxWaitMs": 2000, "ignoreCase": false})
    );
}

#[test]
fn mcp_batch_runs_steps_in_order_and_stops_at_a_failed_command() {
    let (port, seen) = fake_api();
    let connector: super::mcp::Connector = Arc::new(move || Ok(connection(port)));
    let call = |id: i64, arguments: Value| {
        json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "batch", "arguments": arguments}})
        .to_string()
    };
    let steps = json!({"steps": [
        {"tool": "run", "arguments": {"tab": "b", "text": "true"}},
        {"tool": "commands", "arguments": {"tab": "b", "chars": 100}},
        {"tool": "run", "arguments": {"tab": "b", "text": "false"}},
        {"tool": "run", "arguments": {"tab": "b", "text": "never"}},
    ]});
    let mut keep_going = steps.clone();
    keep_going["stopOnError"] = json!(false);
    let input = [
        call(1, steps),
        call(2, keep_going),
        call(3, json!({"steps": [{"tool": "batch"}]})),
        call(4, json!({"steps": []})),
    ]
    .join("\n")
        + "\n";
    let sink = Sink::default();
    super::mcp::serve(input.as_bytes(), Box::new(sink.clone()), connector);
    let output = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
    let replies: Vec<Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let reply = |id: i64| replies.iter().find(|r| r["id"] == id).unwrap()["result"].clone();

    let first = reply(1)["structuredContent"].clone();
    assert_eq!(first["ok"], false);
    assert_eq!(first["completed"], 3, "stops after the failed command");
    assert_eq!(first["steps"][2]["result"]["exitCode"], 1);
    assert_eq!(first["steps"][1]["ok"], true);
    assert_eq!(reply(2)["structuredContent"]["completed"], 4);
    assert_eq!(reply(3)["structuredContent"]["steps"][0]["ok"], false);
    assert_eq!(reply(4)["isError"], true, "an empty batch is an error");
    let seen = seen.lock().unwrap().clone();
    assert!(
        seen.iter()
            .any(|r| r.path == "/v1/tabs/b/commands?chars=100"),
        "{seen:?}"
    );
    assert_eq!(
        seen.iter().filter(|r| r.body["text"] == "never").count(),
        1,
        "only the batch with stopOnError false sends the last step"
    );
}

#[test]
fn look_commands_send_the_look_routes() {
    let (port, seen) = fake_api();
    let connection = connection(port);
    cli(&connection, &["look"]).unwrap();
    cli(
        &connection,
        &[
            "look-apply",
            "--theme",
            "aurora",
            "--target",
            "pane",
            "--tab",
            "2",
            "--skip",
            "fonts,app",
            "--client",
            "tester",
        ],
    )
    .unwrap();
    cli(&connection, &["look-undo"]).unwrap();
    assert!(
        cli(&connection, &["look-apply"]).is_err(),
        "theme or bundle"
    );
    assert!(cli(&connection, &["look-apply", "--theme", "a", "--skip", "x"]).is_err());
    let seen = seen.lock().unwrap();
    let find = |path: &str| seen.iter().rev().find(|s| s.path == path).unwrap().clone();
    assert_eq!(find("/v1/look").method, "GET");
    let apply = find("/v1/look/apply");
    assert_eq!(
        apply.body,
        json!({"theme": "aurora", "target": "pane", "tab": "2",
            "parts": {"fonts": false, "app": false}, "client": "tester"})
    );
    assert_eq!(
        find("/v1/look/undo").body,
        json!({"client": "buttonsclictl"})
    );
}
