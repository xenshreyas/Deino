//! A minimal MCP client for the protocol tests: spawns `deino mcp` and
//! speaks newline-delimited JSON-RPC over its stdin and stdout.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::{Value, json};

/// How long one response may take.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);
/// The protocol version Claude Code offers.
pub const PROTOCOL: &str = "2025-06-18";

/// A running `deino mcp`.
pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
}

impl McpClient {
    /// Starts `deino --db <database> mcp` in UTC.
    pub fn start(database: &Path) -> Self {
        let mut child = Command::new(assert_cmd::cargo::cargo_bin("deino-cli"))
            .env("TZ", "UTC")
            .env_remove("DEINO_DB")
            .arg("--db")
            .arg(database)
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            lines,
            next_id: 1,
        }
    }

    /// Starts the server and completes the handshake; returns the
    /// `initialize` result.
    pub fn connect(database: &Path) -> (Self, Value) {
        let mut client = Self::start(database);
        let info = client.request(
            "initialize",
            &json!({
                "protocolVersion": PROTOCOL,
                "capabilities": {},
                "clientInfo": { "name": "deino-tests", "version": "1.0.0" },
            }),
        );
        client.notify("notifications/initialized", &json!({}));
        (client, info)
    }

    /// Sends a request and returns its whole response (`result` or `error`).
    pub fn request(&mut self, method: &str, params: &Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let line = self.lines.recv_timeout(RESPONSE_TIMEOUT).unwrap();
            let message: Value = serde_json::from_str(&line).unwrap();
            if message["id"] == id {
                return message;
            }
        }
    }

    /// Calls `tool` with `arguments`; returns the call's result.
    pub fn call(&mut self, tool: &str, arguments: &Value) -> Value {
        let response = self.request(
            "tools/call",
            &json!({ "name": tool, "arguments": arguments }),
        );
        response["result"].clone()
    }

    /// Sends a notification.
    pub fn notify(&mut self, method: &str, params: &Value) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Only the child this client started.
        let _killed = self.child.kill();
        let _exited = self.child.wait();
    }
}

/// The text of a tool result's first content block.
pub fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap()
}
