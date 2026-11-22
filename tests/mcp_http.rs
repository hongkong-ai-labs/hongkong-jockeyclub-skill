//! Offline regression tests for the MCP streamable HTTP transport.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

struct Server {
    child: Child,
    address: String,
}

impl Server {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hkjc"))
            .args(["mcp", "--http", "127.0.0.1:0", "--token", "test-secret"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn HTTP MCP server");
        let stderr = child.stderr.take().expect("server stderr");
        let mut line = String::new();
        BufReader::new(stderr)
            .read_line(&mut line)
            .expect("read server address");
        let address = line
            .trim()
            .strip_prefix("hkjc MCP server listening on http://")
            .and_then(|rest| rest.strip_suffix("/mcp"))
            .unwrap_or_else(|| panic!("unexpected server line: {line:?}"))
            .to_string();
        Self { child, address }
    }

    fn request(&self, method: &str, path: &str, body: &str, token: Option<&str>) -> Response {
        let mut stream = TcpStream::connect(&self.address).expect("connect to MCP HTTP server");
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .expect("read timeout");
        let authorization = token
            .map(|token| format!("Authorization: Bearer {token}\r\n"))
            .unwrap_or_default();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n{authorization}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.address,
            body.len()
        );
        stream
            .write_all(request.as_bytes())
            .expect("write HTTP request");
        stream.flush().expect("flush HTTP request");
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).expect("read HTTP response");
        Response::parse(&raw)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Response {
    status: u16,
    body: String,
}

impl Response {
    fn parse(raw: &[u8]) -> Self {
        let text = String::from_utf8(raw.to_vec()).expect("UTF-8 response");
        let (head, body) = text
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("malformed HTTP response: {text:?}"));
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("missing HTTP status: {head:?}"));
        Self {
            status,
            body: body.to_string(),
        }
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("response was not JSON ({error}): {:?}", self.body))
    }
}

#[test]
fn http_transport_serves_mcp_and_requires_its_token() {
    let server = Server::spawn();

    let unauthorized = server.request(
        "POST",
        "/mcp",
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }).to_string(),
        None,
    );
    assert_eq!(unauthorized.status, 401);

    let initialize = server.request(
        "POST",
        "/mcp",
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "hkjc-http-test", "version": "0" }
            }
        })
        .to_string(),
        Some("test-secret"),
    );
    assert_eq!(initialize.status, 200);
    let initialize = initialize.json();
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["protocolVersion"], "2025-06-18");

    let list = server.request(
        "POST",
        "/mcp",
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }).to_string(),
        Some("test-secret"),
    );
    assert_eq!(list.status, 200);
    let list = list.json();
    assert_eq!(list["id"], 2);
    assert_eq!(
        list["result"]["tools"]
            .as_array()
            .expect("tools array")
            .len(),
        18
    );

    let notification = server.request(
        "POST",
        "/mcp",
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string(),
        Some("test-secret"),
    );
    assert_eq!(notification.status, 202);
    assert!(notification.body.is_empty());

    let get = server.request("GET", "/mcp", "", Some("test-secret"));
    assert_eq!(get.status, 405);
}
