//! Offline MCP stdio regression tests.
//!
//! `hkjc mcp` used to panic on the first tool call because it built a Tokio
//! runtime inside `#[tokio::main]`. These tests cover both documented entry
//! points and only use requests that do not touch the network.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};

const WAIT: Duration = Duration::from_secs(15);

struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl Server {
    fn spawn(mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn MCP server");

        let stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            child,
            stdin: Some(stdin),
            lines,
        }
    }

    fn cli() -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hkjc"));
        command.arg("mcp");
        Self::spawn(command)
    }

    fn standalone() -> Self {
        Self::spawn(Command::new(env!("CARGO_BIN_EXE_hkjc-mcp")))
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin still open");
        writeln!(stdin, "{message}").expect("write request");
        stdin.flush().expect("flush request");
    }

    fn receive(&mut self) -> Value {
        let line = self
            .lines
            .recv_timeout(WAIT)
            .unwrap_or_else(|_| panic!("server did not answer within {WAIT:?}"));
        serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("non-JSON line {line:?}: {error}"))
    }

    fn finish(mut self) {
        drop(self.stdin.take());
        let (sender, status) = mpsc::channel();
        let mut child = self.child;
        std::thread::spawn(move || {
            let _ = sender.send(child.wait());
        });
        let status = status
            .recv_timeout(WAIT)
            .expect("server exits after EOF")
            .expect("wait for server");
        assert!(status.success(), "server exit status: {status}");
    }
}

fn initialize(server: &mut Server) {
    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "hkjc-test", "version": "0" }
        }
    }));
    let initialize = server.receive();
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(
        initialize["result"]["serverInfo"]["name"],
        "hongkong-jockeyclub-skill"
    );
}

#[test]
fn cli_mcp_answers_tool_calls_without_panicking() {
    let mut server = Server::cli();
    initialize(&mut server);

    server.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    server.send(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
    let list = server.receive();
    assert_eq!(list["id"], 2, "notifications must not produce a response");
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 18);
    assert!(tools.iter().any(|tool| tool["name"] == "hk_race_meetings"));
    assert!(tools.iter().any(|tool| tool["name"] == "hk_race_standings"));

    // Regression: this request used to panic inside the async runtime.
    server.send(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": { "name": "not_a_tool", "arguments": {} }
    }));
    let unknown = server.receive();
    assert_eq!(unknown["id"], 3);
    assert_eq!(unknown["result"]["isError"], true);

    server.send(&json!({ "jsonrpc": "2.0", "id": 4, "method": "ping" }));
    let ping = server.receive();
    assert_eq!(ping["id"], 4);
    assert!(ping["result"].is_object());

    server.finish();
}

#[test]
fn standalone_binary_still_serves() {
    let mut server = Server::standalone();
    initialize(&mut server);
    server.send(&json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }));
    let ping = server.receive();
    assert_eq!(ping["id"], 2);
    assert!(ping["result"].is_object());
    server.finish();
}
