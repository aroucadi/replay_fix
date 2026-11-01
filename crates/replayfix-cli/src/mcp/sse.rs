//! MCP Streamable HTTP & Server-Sent Events (`SSE`) Transport (`crates/replayfix-cli/src/mcp/sse.rs`).
//!
//! Implements a dependency-free, bounded `127.0.0.1:<port>` HTTP/SSE server for headless
//! and cloud coding agents:
//! - `GET /sse`: Returns a `text/event-stream` response announcing `/message`.
//! - `POST /message` or `POST /mcp`: Accepts a JSON-RPC 2.0 request body and returns both
//!   `application/json` (or `text/event-stream` when `Accept: text/event-stream` is requested).

use crate::mcp::McpServer;
use replayfix_core::error::{CoreError, Result};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Maximum allowed HTTP request payload size (1 MiB).
pub const MAX_HTTP_REQUEST_BYTES: usize = 1024 * 1024;

/// Running handle for an MCP HTTP/SSE server bound to `127.0.0.1:<port>`.
#[derive(Debug)]
pub struct McpSseServerHandle {
    port: u16,
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl McpSseServerHandle {
    /// Start a background MCP HTTP/SSE listener on `127.0.0.1:<port>` (`0` for ephemeral port).
    pub fn start(server: McpServer, port: u16) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| {
            CoreError::McpProtocol(format!("Failed to bind MCP SSE port {port}: {e}"))
        })?;
        let bound_port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);

        let worker = thread::spawn(move || {
            while !shutdown_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        let _ = handle_http_connection(stream, &server);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(25));
                    }
                }
            }
        });

        Ok(Self {
            port: bound_port,
            shutdown,
            worker: Some(worker),
        })
    }

    /// Bound TCP port on `127.0.0.1`.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Base HTTP endpoint URL (`http://127.0.0.1:<port>`).
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Gracefully stop the background HTTP/SSE server.
    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for McpSseServerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Run the MCP HTTP/SSE server in the foreground until `max_requests` (if provided) are served.
pub fn run_sse_foreground(
    server: McpServer,
    port: u16,
    max_requests: Option<usize>,
) -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| CoreError::McpProtocol(format!("Failed to bind MCP SSE port {port}: {e}")))?;
    let bound_port = listener.local_addr()?.port();
    let mut served = 0usize;

    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let _ = handle_http_connection(stream, &server);
                served += 1;
                if let Some(limit) = max_requests {
                    if served >= limit {
                        break;
                    }
                }
            }
            Err(e) => {
                return Err(CoreError::McpProtocol(format!(
                    "MCP HTTP accept error: {e}"
                )));
            }
        }
    }
    Ok(bound_port)
}

const CRLF: &str = "\x0d\x0a";
const DOUBLE_CRLF: &str = "\x0d\x0a\x0d\x0a";
const DOUBLE_LF: &str = "\x0a\x0a";

fn handle_http_connection(mut stream: TcpStream, server: &McpServer) -> Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let mut buf = vec![0u8; 65536];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let raw = String::from_utf8_lossy(&buf[..n]);
    let (headers_part, body_part) = raw
        .split_once(DOUBLE_CRLF)
        .or_else(|| raw.split_once(DOUBLE_LF))
        .unwrap_or((&raw, ""));
    let first_line = headers_part.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");

    if method == "GET" && (path == "/sse" || path.starts_with("/sse?")) {
        let sse_payload = "event: endpoint\x0adata: /message\x0a\x0a";
        let resp = [
            "HTTP/1.1 200 OK",
            "Content-Type: text/event-stream",
            "Cache-Control: no-cache",
            "Connection: close",
            &format!("Content-Length: {}", sse_payload.len()),
            "",
            sse_payload,
        ]
        .join(CRLF);
        stream.write_all(resp.as_bytes())?;
        stream.flush()?;
        return Ok(());
    }

    if method == "POST" && (path == "/message" || path == "/mcp" || path == "/") {
        let wants_sse = headers_part
            .lines()
            .any(|l| l.to_ascii_lowercase().contains("text/event-stream"));

        if let Some(rpc_resp) = server.handle_raw_message(body_part) {
            let json_body = serde_json::to_string(&rpc_resp)?;
            if wants_sse {
                let sse_event = format!("event: message\x0adata: {json_body}\x0a\x0a");
                let http_resp = [
                    "HTTP/1.1 200 OK",
                    "Content-Type: text/event-stream",
                    "Cache-Control: no-cache",
                    "Connection: close",
                    &format!("Content-Length: {}", sse_event.len()),
                    "",
                    &sse_event,
                ]
                .join(CRLF);
                stream.write_all(http_resp.as_bytes())?;
            } else {
                let http_resp = [
                    "HTTP/1.1 200 OK",
                    "Content-Type: application/json",
                    "Connection: close",
                    &format!("Content-Length: {}", json_body.len()),
                    "",
                    &json_body,
                ]
                .join(CRLF);
                stream.write_all(http_resp.as_bytes())?;
            }
        } else {
            let http_resp = [
                "HTTP/1.1 202 Accepted",
                "Content-Length: 0",
                "Connection: close",
                "",
                "",
            ]
            .join(CRLF);
            stream.write_all(http_resp.as_bytes())?;
        }
        stream.flush()?;
        return Ok(());
    }

    let not_found = [
        "HTTP/1.1 404 Not Found",
        "Content-Length: 0",
        "Connection: close",
        "",
        "",
    ]
    .join(CRLF);
    stream.write_all(not_found.as_bytes())?;
    stream.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::McpServerConfig;

    fn raw_http_request(port: u16, req: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(req.as_bytes()).unwrap();
        stream.flush().unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        resp
    }

    #[test]
    fn test_mcp_sse_and_http_post_roundtrip() {
        let srv = McpServer::new(McpServerConfig::default());
        let mut handle = McpSseServerHandle::start(srv, 0).unwrap();
        let port = handle.port();

        let get_req = [
            "GET /sse HTTP/1.1",
            "Host: 127.0.0.1",
            "Connection: close",
            "",
            "",
        ]
        .join(CRLF);
        let sse_raw = raw_http_request(port, &get_req);
        assert!(sse_raw.starts_with("HTTP/1.1 200 OK"));
        assert!(sse_raw.contains("event: endpoint"));
        assert!(sse_raw.contains("data: /message"));

        let rpc_body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let post_req = [
            "POST /message HTTP/1.1",
            "Host: 127.0.0.1",
            "Content-Type: application/json",
            &format!("Content-Length: {}", rpc_body.len()),
            "Connection: close",
            "",
            rpc_body,
        ]
        .join(CRLF);
        let post_raw = raw_http_request(port, &post_req);
        assert!(post_raw.starts_with("HTTP/1.1 200 OK"));
        let (_, json_part) = post_raw.split_once(DOUBLE_CRLF).unwrap();
        let body: serde_json::Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(body["result"]["tools"].as_array().unwrap().len(), 5);

        handle.shutdown();
    }
}
