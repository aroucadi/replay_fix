//! MCP JSON-RPC 2.0 `stdio` Transport (`crates/replayfix-cli/src/mcp/stdio.rs`).
//!
//! Reads line-delimited JSON-RPC 2.0 messages from `stdin` with a strict
//! maximum frame size guard (`MAX_STDIO_FRAME_BYTES`) and writes flushed JSON-RPC 2.0
//! responses to `stdout`.

use crate::mcp::{JsonRpcResponse, McpServer, ERR_INVALID_REQUEST};
use replayfix_core::error::{CoreError, Result};
use serde_json::Value;
use std::io::{self, BufRead, Write};

/// Maximum allowed JSON-RPC 2.0 frame size over stdio (2 MiB).
pub const MAX_STDIO_FRAME_BYTES: usize = 2 * 1024 * 1024;

/// Run the MCP JSON-RPC 2.0 server loop over standard input and standard output.
pub fn run_stdio_server(server: &McpServer) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    process_stdio_stream(server, &mut reader, &mut writer)
}

/// Process a `BufRead` stream of line-delimited JSON-RPC 2.0 requests and write responses to `W`.
pub fn process_stdio_stream<R: BufRead, W: Write>(
    server: &McpServer,
    reader: &mut R,
    writer: &mut W,
) -> Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .map_err(|e| CoreError::McpProtocol(format!("MCP stdio read error: {e}")))?;
        if bytes_read == 0 {
            break;
        }
        if line.len() > MAX_STDIO_FRAME_BYTES {
            let err_resp = JsonRpcResponse::error(
                Value::Null,
                ERR_INVALID_REQUEST,
                format!(
                    "Frame size ({} bytes) exceeds MAX_STDIO_FRAME_BYTES ({MAX_STDIO_FRAME_BYTES})",
                    line.len()
                ),
                None,
            );
            let serialized = serde_json::to_string(&err_resp)?;
            writeln!(writer, "{serialized}")?;
            writer.flush()?;
            continue;
        }

        if let Some(resp) = server.handle_raw_message(&line) {
            let serialized = serde_json::to_string(&resp)?;
            writeln!(writer, "{serialized}")?;
            writer.flush()?;
        }
    }
    Ok(())
}
