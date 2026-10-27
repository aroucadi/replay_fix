//! Embedded sandbox HTTP server and Node.js Playwright stdio JSON-lines bridge.
//!
//! `SandboxServer` serves files from an isolated `ScratchSandbox` over
//! `127.0.0.1:<ephemeral_port>` with strict path validation and non-blocking
//! connection handling.
//!
//! `PlaywrightBridge` spawns `node` running the embedded bridge script and
//! communicates via flushed, line-delimited JSON with monotonic `request_id`
//! correlation, background `stdout`/`stderr` reader threads, strict per-command
//! deadlines, and idempotent process-tree teardown.

use crate::error::{CoreError, Result};
use crate::locator::DomCandidate;
use crate::manifest::TaskManifest;
use crate::platform::Platform;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Filename used when materializing the bridge script inside a scratch sandbox.
pub const BRIDGE_SCRIPT_FILENAME: &str = ".replayfix-playwright-bridge.cjs";

/// Default timeout for launching Node.js + Chromium (`START_BROWSER`).
pub const DEFAULT_BRIDGE_STARTUP_TIMEOUT: Duration = Duration::from_secs(20);

/// Default timeout for individual bridge commands (`PROBE_PAGE`, `CLICK_AND_OBSERVE`).
pub const DEFAULT_BRIDGE_COMMAND_TIMEOUT: Duration = Duration::from_secs(15);

/// Bounded timeout for graceful `SHUTDOWN` IPC before forced process-tree termination.
const SHUTDOWN_COMMAND_TIMEOUT: Duration = Duration::from_millis(1500);

/// Bounded poll deadline when reaping the child process handle during shutdown.
const CHILD_REAP_TIMEOUT: Duration = Duration::from_millis(500);

/// Maximum bytes accepted for a single JSON-line on bridge stdout (4 MiB) to prevent OOM DoS.
const MAX_STDOUT_LINE_BYTES: usize = 4 * 1024 * 1024;

/// Maximum number of recent stderr lines retained for diagnostic reporting.
const MAX_STDERR_LINES: usize = 64;

/// Maximum single static file size served by `SandboxServer` (32 MiB).
const MAX_HTTP_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// Embedded Node.js Playwright bridge speaking line-delimited JSON over stdio.
pub const PLAYWRIGHT_BRIDGE_JS: &str = r#"const fs = require('fs');
const path = require('path');
const readline = require('readline');
const { execSync } = require('child_process');

function findPlaywrightInNpxCache(cacheDir) {
  if (!cacheDir || !fs.existsSync(cacheDir)) return null;
  const npxDir = path.join(cacheDir, '_npx');
  if (!fs.existsSync(npxDir)) return null;
  const entries = fs.readdirSync(npxDir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isDirectory()) {
      const candidate = path.join(npxDir, entry.name, 'node_modules', 'playwright');
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }
  }
  return null;
}

function loadPlaywright() {
  const candidates = [];
  if (process.env.REPLAYFIX_PLAYWRIGHT_PATH) {
    candidates.push(process.env.REPLAYFIX_PLAYWRIGHT_PATH);
  }
  candidates.push('playwright');

  try {
    const cacheOut = execSync(
      process.platform === 'win32' ? 'npm.cmd config get cache' : 'npm config get cache',
      { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 4000 }
    ).trim();
    const fromCache = findPlaywrightInNpxCache(cacheOut);
    if (fromCache) candidates.push(fromCache);
  } catch (_) {}

  if (process.env.LOCALAPPDATA) {
    const localCache = findPlaywrightInNpxCache(path.join(process.env.LOCALAPPDATA, 'npm-cache'));
    if (localCache) candidates.push(localCache);
  }

  for (const target of candidates) {
    try {
      return require(target);
    } catch (_) {}
  }
  throw new Error("Playwright module not found. Install via 'npm install -D playwright' and 'npx playwright install chromium'.");
}

function withTimeout(promise, ms, label) {
  let timer = null;
  const timeoutPromise = new Promise((_, reject) => {
    timer = setTimeout(() => {
      reject(new Error(label + ' timed out after ' + ms + 'ms'));
    }, ms);
  });
  return Promise.race([promise, timeoutPromise]).finally(() => {
    if (timer) clearTimeout(timer);
  });
}

function sendLine(obj, requestId) {
  const payload = Object.assign({}, obj);
  if (typeof requestId === 'number') {
    payload.request_id = requestId;
  }
  process.stdout.write(JSON.stringify(payload) + '\n');
}

(async () => {
  let pw;
  let browser = null;
  let browserConnected = false;
  let page = null;
  let pageCrashed = false;
  let pageErrors = [];

  function ensureActivePage(cmdName) {
    if (!browser || !browserConnected) {
      throw new Error('Browser WebSocket disconnected before ' + cmdName);
    }
    if (pageCrashed) {
      throw new Error('Browser target page crashed before ' + cmdName);
    }
    if (!page) {
      throw new Error('Browser page not initialized or already closed before ' + cmdName);
    }
  }

  const rl = readline.createInterface({
    input: process.stdin,
    crlfDelay: Infinity,
  });

  for await (const rawLine of rl) {
    const line = rawLine.trim();
    if (!line) continue;

    let cmd;
    try {
      cmd = JSON.parse(line);
    } catch (err) {
      sendLine({ kind: 'ERROR', message: 'Invalid JSON command: ' + String(err.message || err) }, undefined);
      continue;
    }

    const reqId = typeof cmd.request_id === 'number' ? cmd.request_id : undefined;

    try {
      if (cmd.cmd === 'START_BROWSER') {
        if (!pw) pw = loadPlaywright();
        if (browser) {
          await withTimeout(browser.close(), 2000, 'browser.close').catch(() => {});
          browser = null;
          browserConnected = false;
          page = null;
        }
        browser = await pw.chromium.launch({ headless: !cmd.headed, timeout: 15000 });
        browserConnected = true;
        pageCrashed = false;
        browser.on('disconnected', () => {
          browserConnected = false;
          page = null;
          browser = null;
        });
        const context = await browser.newContext({
          viewport: {
            width: cmd.viewport_width || 1280,
            height: cmd.viewport_height || 720,
          },
        });
        context.setDefaultTimeout(5000);
        context.setDefaultNavigationTimeout(10000);
        page = await context.newPage();
        pageErrors = [];
        page.on('pageerror', (err) => {
          pageErrors.push(String(err && err.stack ? err.stack : err));
        });
        page.on('crash', () => {
          pageCrashed = true;
        });
        page.on('close', () => {
          page = null;
        });
        sendLine({ kind: 'READY' }, reqId);
      } else if (cmd.cmd === 'PROBE_PAGE') {
        ensureActivePage('PROBE_PAGE');
        pageErrors = [];
        await page.goto(cmd.url, { waitUntil: 'load', timeout: 10000 });
        ensureActivePage('PROBE_PAGE(post-goto)');
        const ariaSnapshot = await withTimeout(
          page.locator('body').ariaSnapshot({ timeout: 5000 }),
          5500,
          'ariaSnapshot'
        );
        const candidates = await withTimeout(
          page.evaluate(({ x, y }) => {
            const selector = 'button, a, input, select, textarea, [role="button"], [role="link"], [role="checkbox"], [role="menuitem"], [data-testid]';
            const hitEl = document.elementFromPoint(x, y);
            const closestHit = hitEl ? hitEl.closest(selector) : null;
            const allInteractive = Array.from(document.querySelectorAll(selector));
            if (closestHit && !allInteractive.includes(closestHit)) {
              allInteractive.unshift(closestHit);
            }

            function inferRole(el) {
              const explicit = (el.getAttribute('role') || '').trim();
              if (explicit) return explicit.toLowerCase();
              const tag = el.tagName.toLowerCase();
              if (tag === 'button') return 'button';
              if (tag === 'a' && el.hasAttribute('href')) return 'link';
              if (tag === 'input') {
                const t = (el.getAttribute('type') || 'text').toLowerCase();
                if (t === 'button' || t === 'submit' || t === 'reset') return 'button';
                if (t === 'checkbox') return 'checkbox';
                return 'textbox';
              }
              return tag;
            }

            function inferAccessibleName(el) {
              const ariaLabel = (el.getAttribute('aria-label') || '').trim();
              if (ariaLabel) return ariaLabel;
              const text = (el.innerText || el.textContent || '').trim();
              if (text) return text;
              const val = (el.getAttribute('value') || '').trim();
              return val;
            }

            return allInteractive.map((el) => {
              const rect = el.getBoundingClientRect();
              const testId = el.getAttribute('data-testid');
              return {
                tag: el.tagName.toLowerCase(),
                role: inferRole(el),
                accessible_name: inferAccessibleName(el),
                test_id: testId ? testId.trim() : null,
                bbox: {
                  x: Math.round(rect.x),
                  y: Math.round(rect.y),
                  width: Math.max(0, Math.round(rect.width)),
                  height: Math.max(0, Math.round(rect.height)),
                },
                is_element_from_point: el === closestHit,
              };
            });
          }, { x: cmd.click_x, y: cmd.click_y }),
          5000,
          'DOM candidate evaluation'
        );

        sendLine({
          kind: 'PROBE_RESULT',
          navigated: true,
          aria_snapshot: ariaSnapshot,
          candidates: candidates,
          page_errors: pageErrors.slice(),
        }, reqId);
      } else if (cmd.cmd === 'CLICK_AND_OBSERVE') {
        ensureActivePage('CLICK_AND_OBSERVE');
        pageErrors = [];
        let locator;
        if (cmd.role && cmd.accessible_name) {
          locator = page.getByRole(cmd.role, { name: cmd.accessible_name, exact: true });
        } else if (cmd.test_id) {
          locator = page.getByTestId(cmd.test_id);
        } else {
          throw new Error('Grounded click requires (role + accessible_name) or test_id');
        }

        // Explicit TOCTOU verification: ensure the resolved node still exists, is unique, and has non-zero geometry.
        const matchCount = await withTimeout(locator.count(), 2000, 'Locator count check');
        if (matchCount === 0) {
          throw new Error('DOM_NODE_DETACHED: Target DOM node detached or disappeared before click (count=0)');
        }
        if (matchCount > 1) {
          throw new Error('AMBIGUOUS_GROUNDING: Target locator matched ' + matchCount + ' DOM nodes at click time (strict uniqueness violated)');
        }
        const box = await withTimeout(locator.boundingBox({ timeout: 2000 }), 2500, 'Locator boundingBox check');
        if (!box || box.width <= 0 || box.height <= 0) {
          throw new Error('DOM_NODE_DETACHED: Target DOM node became hidden or zero-sized before click');
        }

        await locator.click({ timeout: 5000 });
        await page.waitForTimeout(50);
        ensureActivePage('CLICK_AND_OBSERVE(post-click)');
        let resultText = '';
        if (cmd.output_selector) {
          const outEl = page.locator(cmd.output_selector);
          const outCount = await withTimeout(outEl.count(), 2000, 'Output selector count');
          if (outCount > 0) {
            resultText = ((await withTimeout(outEl.textContent({ timeout: 2000 }), 2500, 'Output textContent')) || '').trim();
          }
        }
        sendLine({
          kind: 'CLICK_RESULT',
          clicked: true,
          page_errors: pageErrors.slice(),
          result_text: resultText,
        }, reqId);
      } else if (cmd.cmd === 'SHUTDOWN') {
        if (browser) {
          await withTimeout(browser.close(), 2000, 'browser.close').catch(() => {});
          browser = null;
          browserConnected = false;
          page = null;
        }
        sendLine({ kind: 'SHUTDOWN_OK' }, reqId);
        break;
      } else {
        sendLine({ kind: 'ERROR', message: 'Unknown command: ' + String(cmd.cmd) }, reqId);
      }
    } catch (err) {
      sendLine({
        kind: 'ERROR',
        message: String(err && err.message ? err.message : err),
      }, reqId);
    }
  }

  if (browser) {
    await withTimeout(browser.close(), 1500, 'browser.close').catch(() => {});
  }
  process.exit(0);
})();
"#;

/// Commands sent from Rust to the Node.js Playwright bridge over stdin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "cmd", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BridgeCommand {
    StartBrowser {
        headed: bool,
        viewport_width: u32,
        viewport_height: u32,
    },
    ProbePage {
        url: String,
        click_x: i32,
        click_y: i32,
    },
    ClickAndObserve {
        role: String,
        accessible_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        test_id: Option<String>,
        output_selector: String,
    },
    Shutdown,
}

/// Responses sent from the Node.js Playwright bridge to Rust over stdout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BridgeResponse {
    Ready,
    ProbeResult {
        navigated: bool,
        aria_snapshot: String,
        candidates: Vec<DomCandidate>,
        page_errors: Vec<String>,
    },
    ClickResult {
        clicked: bool,
        page_errors: Vec<String>,
        result_text: String,
    },
    ShutdownOk,
    Error {
        message: String,
    },
}

/// Embedded localhost HTTP server serving files from a `ScratchSandbox`.
#[derive(Debug)]
pub struct SandboxServer {
    port: u16,
    shutdown_flag: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl SandboxServer {
    /// Starts an HTTP server on `127.0.0.1:0` (or `preferred_port` if available, falling back to `:0`)
    /// serving static files from `root_dir`.
    pub fn start(root_dir: &Path, preferred_port: Option<u16>) -> Result<Self> {
        let bind_addr = preferred_port
            .map(|p| format!("127.0.0.1:{p}"))
            .unwrap_or_else(|| "127.0.0.1:0".to_string());

        let listener = match TcpListener::bind(&bind_addr) {
            Ok(l) => l,
            Err(_) if preferred_port.is_some() => TcpListener::bind("127.0.0.1:0")?,
            Err(e) => return Err(CoreError::Io(e)),
        };

        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&shutdown_flag);
        let root = root_dir.to_path_buf();

        let worker = thread::spawn(move || {
            let mut conn_handles: Vec<JoinHandle<()>> = Vec::new();
            while !flag_clone.load(Ordering::Relaxed) {
                conn_handles.retain(|h| !h.is_finished());
                match listener.accept() {
                    Ok((stream, _)) => {
                        let root_clone = root.clone();
                        conn_handles.push(thread::spawn(move || {
                            let _ = handle_http_connection(stream, &root_clone);
                        }));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
            for handle in conn_handles {
                let _ = handle.join();
            }
        });

        Ok(Self {
            port,
            shutdown_flag,
            worker: Some(worker),
        })
    }

    /// The bound localhost TCP port (`127.0.0.1:<port>`).
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Base URL (`http://127.0.0.1:<port>`).
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Signals the server thread to stop and waits for it to join.
    pub fn shutdown(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for SandboxServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn handle_http_connection(mut stream: TcpStream, root: &Path) -> Result<()> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let req = String::from_utf8_lossy(&buf[..n]);
    let Some(first_line) = req.lines().next() else {
        return Ok(());
    };
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_target = parts.next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        return write_http_response(&mut stream, "405 Method Not Allowed", "text/plain", b"");
    }

    let path_part = raw_target.split('?').next().unwrap_or("/");
    let rel_path = match path_part.trim_start_matches('/') {
        "" => "index.html",
        other => other,
    };

    if rel_path == "favicon.ico" {
        return write_http_response(&mut stream, "204 No Content", "image/x-icon", b"");
    }

    let Ok(normalized) = TaskManifest::assert_safe_path(rel_path, "http_request_path") else {
        return write_http_response(&mut stream, "403 Forbidden", "text/plain", b"Forbidden");
    };

    if normalized
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return write_http_response(&mut stream, "403 Forbidden", "text/plain", b"Forbidden");
    }

    let full_path = root.join(&normalized);
    let Ok(meta) = fs::symlink_metadata(&full_path) else {
        return write_http_response(&mut stream, "404 Not Found", "text/plain", b"Not Found");
    };
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > MAX_HTTP_FILE_BYTES {
        return write_http_response(&mut stream, "403 Forbidden", "text/plain", b"Forbidden");
    }

    let body = fs::read(&full_path)?;
    let content_type = content_type_for(&normalized.to_string_lossy());
    write_http_response(&mut stream, "200 OK", content_type, &body)
}

fn content_type_for(path: &str) -> &'static str {
    if path.ends_with(".html") || path.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

/// Reads a single `\n`-delimited line from `reader` with a hard byte limit (`max_bytes`)
/// so a broken child process streaming gigabytes without a newline cannot cause OOM.
fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> std::io::Result<Option<String>> {
    let mut buf = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if buf.is_empty() {
                return Ok(None);
            }
            return Ok(Some(String::from_utf8_lossy(&buf).into_owned()));
        }
        if let Some(newline_pos) = available.iter().position(|&b| b == b'\n') {
            let take = newline_pos + 1;
            if buf.len().saturating_add(take) > max_bytes {
                reader.consume(take);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Bridge stdout line exceeded maximum size of {max_bytes} bytes"),
                ));
            }
            buf.extend_from_slice(&available[..take]);
            reader.consume(take);
            return Ok(Some(String::from_utf8_lossy(&buf).into_owned()));
        }
        let chunk_len = available.len();
        if buf.len().saturating_add(chunk_len) > max_bytes {
            reader.consume(chunk_len);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Bridge stdout line exceeded maximum size of {max_bytes} bytes"),
            ));
        }
        buf.extend_from_slice(available);
        reader.consume(chunk_len);
    }
}

fn spawn_stdout_reader(stdout: ChildStdout) -> Receiver<std::io::Result<String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_bounded_line(&mut reader, MAX_STDOUT_LINE_BYTES) {
                Ok(Some(line)) => {
                    if tx.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            }
        }
    });
    rx
}

fn spawn_stderr_drainer(stderr: ChildStderr) -> Arc<Mutex<Vec<String>>> {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let buf_clone = Arc::clone(&buffer);
    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        while let Ok(Some(line)) = read_bounded_line(&mut reader, 8192) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let capped: String = trimmed.chars().take(512).collect();
            if let Ok(mut guard) = buf_clone.lock() {
                if guard.len() >= MAX_STDERR_LINES {
                    guard.remove(0);
                }
                guard.push(capped);
            }
        }
    });
    buffer
}

/// Line-delimited JSON-lines stdio client managing the Node.js Playwright bridge child process.
#[derive(Debug)]
pub struct PlaywrightBridge {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout_rx: Receiver<std::io::Result<String>>,
    stderr_buf: Arc<Mutex<Vec<String>>>,
    script_path: PathBuf,
    next_request_id: u64,
    command_timeout: Duration,
    poisoned: bool,
    terminated: bool,
    shut_down: bool,
}

impl PlaywrightBridge {
    /// Materializes the bridge script at `sandbox_dir/.replayfix-playwright-bridge.cjs`,
    /// spawns `node`, and launches Chromium (headless or headed).
    pub fn start(
        sandbox_dir: &Path,
        headed: bool,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<Self> {
        Self::start_with_script_and_timeouts(
            sandbox_dir,
            PLAYWRIGHT_BRIDGE_JS,
            headed,
            viewport_width,
            viewport_height,
            DEFAULT_BRIDGE_STARTUP_TIMEOUT,
            DEFAULT_BRIDGE_COMMAND_TIMEOUT,
        )
    }

    /// Starts the bridge using a custom script body and explicit startup/command timeouts.
    pub fn start_with_script_and_timeouts(
        sandbox_dir: &Path,
        script_source: &str,
        headed: bool,
        viewport_width: u32,
        viewport_height: u32,
        startup_timeout: Duration,
        command_timeout: Duration,
    ) -> Result<Self> {
        let script_path = sandbox_dir.join(BRIDGE_SCRIPT_FILENAME);
        fs::write(&script_path, script_source)?;

        let mut cmd = Platform::create_command("node");
        cmd.arg(&script_path)
            .current_dir(sandbox_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| CoreError::Browser(format!("Failed to spawn Node.js bridge: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CoreError::Browser("Bridge child stdin was not piped".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::Browser("Bridge child stdout was not piped".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CoreError::Browser("Bridge child stderr was not piped".into()))?;

        let stdout_rx = spawn_stdout_reader(stdout);
        let stderr_buf = spawn_stderr_drainer(stderr);

        let mut bridge = Self {
            child,
            stdin: Some(stdin),
            stdout_rx,
            stderr_buf,
            script_path,
            next_request_id: 1,
            command_timeout,
            poisoned: false,
            terminated: false,
            shut_down: false,
        };

        match bridge.send_command_with_timeout(
            &BridgeCommand::StartBrowser {
                headed,
                viewport_width,
                viewport_height,
            },
            startup_timeout,
        ) {
            Ok(BridgeResponse::Ready) => Ok(bridge),
            Ok(BridgeResponse::Error { message }) => {
                bridge.shutdown();
                Err(CoreError::Browser(message))
            }
            Ok(other) => {
                bridge.shutdown();
                Err(CoreError::Browser(format!(
                    "Unexpected response to START_BROWSER: {other:?}"
                )))
            }
            Err(e) => {
                bridge.shutdown();
                Err(e)
            }
        }
    }

    /// Child process ID for sandbox process tree supervision.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Returns captured stderr tail lines joined for diagnostic reporting.
    pub fn stderr_summary(&self) -> String {
        let Ok(guard) = self.stderr_buf.lock() else {
            return String::new();
        };
        guard.join(" | ")
    }

    /// Sends a single JSON-line command using the bridge's default command timeout.
    pub fn send_command(&mut self, command: &BridgeCommand) -> Result<BridgeResponse> {
        let timeout = self.command_timeout;
        self.send_command_with_timeout(command, timeout)
    }

    /// Sends a single JSON-line command with a monotonic `request_id` and awaits the matching
    /// response up to `timeout`. On timeout or protocol desynchronization, poisons the bridge
    /// and terminates the child process tree immediately.
    pub fn send_command_with_timeout(
        &mut self,
        command: &BridgeCommand,
        timeout: Duration,
    ) -> Result<BridgeResponse> {
        if self.shut_down || self.poisoned {
            return Err(CoreError::Browser(
                "Playwright bridge is already shut down or poisoned by a prior error".into(),
            ));
        }

        let req_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);

        let mut payload = serde_json::to_value(command)?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("request_id".to_string(), serde_json::Value::from(req_id));
        }
        let mut line = serde_json::to_string(&payload)?;
        line.push('\n');

        let Some(stdin) = self.stdin.as_mut() else {
            self.poisoned = true;
            return Err(CoreError::Browser("Bridge stdin is closed".into()));
        };

        if let Err(e) = stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.flush())
        {
            self.poison_and_terminate();
            return Err(CoreError::Browser(format!(
                "Failed writing to bridge stdin (request_id={req_id}): {e}{}",
                self.format_stderr_suffix()
            )));
        }

        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            let Some(remaining) = deadline.checked_duration_since(now) else {
                self.poison_and_terminate();
                return Err(CoreError::Browser(format!(
                    "Playwright bridge timed out after {}ms waiting for request_id={req_id}{}",
                    timeout.as_millis(),
                    self.format_stderr_suffix()
                )));
            };

            match self.stdout_rx.recv_timeout(remaining) {
                Ok(Ok(raw_line)) => {
                    let trimmed = raw_line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let value: serde_json::Value = match serde_json::from_str(trimmed) {
                        Ok(v) => v,
                        Err(e) => {
                            self.poison_and_terminate();
                            return Err(CoreError::Browser(format!(
                                "Invalid JSON from Playwright bridge ('{}'): {e}{}",
                                truncate_str(trimmed, 240),
                                self.format_stderr_suffix()
                            )));
                        }
                    };

                    if let Some(resp_id) = value.get("request_id").and_then(|v| v.as_u64()) {
                        if resp_id < req_id {
                            // Late response from an earlier request; ignore and keep waiting for req_id.
                            continue;
                        }
                        if resp_id > req_id {
                            self.poison_and_terminate();
                            return Err(CoreError::Browser(format!(
                                "Protocol correlation mismatch: expected request_id={req_id}, received request_id={resp_id}"
                            )));
                        }
                    }

                    let parsed: BridgeResponse = serde_json::from_value(value).map_err(|e| {
                        self.poison_and_terminate();
                        CoreError::Browser(format!(
                            "Malformed BridgeResponse payload ('{}'): {e}",
                            truncate_str(trimmed, 240)
                        ))
                    })?;
                    return Ok(parsed);
                }
                Ok(Err(io_err)) => {
                    self.poison_and_terminate();
                    return Err(CoreError::Browser(format!(
                        "Failed reading from bridge stdout (request_id={req_id}): {io_err}{}",
                        self.format_stderr_suffix()
                    )));
                }
                Err(RecvTimeoutError::Timeout) => {
                    self.poison_and_terminate();
                    return Err(CoreError::Browser(format!(
                        "Playwright bridge timed out after {}ms waiting for request_id={req_id}{}",
                        timeout.as_millis(),
                        self.format_stderr_suffix()
                    )));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.poison_and_terminate();
                    return Err(CoreError::Browser(format!(
                        "Playwright bridge closed stdout unexpectedly (request_id={req_id}){}",
                        self.format_stderr_suffix()
                    )));
                }
            }
        }
    }

    fn format_stderr_suffix(&self) -> String {
        // Give the stderr drain thread a brief moment to flush any final crash lines.
        thread::sleep(Duration::from_millis(15));
        let summary = self.stderr_summary();
        if summary.is_empty() {
            String::new()
        } else {
            format!(" [stderr: {summary}]")
        }
    }

    fn poison_and_terminate(&mut self) {
        self.poisoned = true;
        let _ = self.stdin.take();
        if !self.terminated {
            self.terminated = true;
            let _ = Platform::terminate_process_tree(self.child.id());
            self.reap_child_bounded();
        }
    }

    fn reap_child_bounded(&mut self) {
        let deadline = Instant::now() + CHILD_REAP_TIMEOUT;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    /// Gracefully shuts down the browser, terminates the child process tree, reaps the
    /// child handle within a bounded deadline, and removes the temporary bridge script.
    ///
    /// Strictly idempotent: safe to call explicitly and again via `Drop`.
    pub fn shutdown(&mut self) {
        if self.shut_down {
            return;
        }
        if !self.poisoned {
            let _ =
                self.send_command_with_timeout(&BridgeCommand::Shutdown, SHUTDOWN_COMMAND_TIMEOUT);
        }
        self.shut_down = true;
        self.poison_and_terminate();
        let _ = fs::remove_file(&self.script_path);
    }
}

impl Drop for PlaywrightBridge {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::Fixture;
    use crate::locator::LocatorResolver;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    #[test]
    fn test_bridge_command_and_response_json_lines_roundtrip() {
        let cmd = BridgeCommand::ProbePage {
            url: "http://127.0.0.1:41234/index.html".into(),
            click_x: 160,
            click_y: 120,
        };
        let encoded = serde_json::to_string(&cmd).unwrap();
        assert!(!encoded.contains('\n'));
        let decoded: BridgeCommand = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn test_sandbox_server_serves_files_and_blocks_traversal() {
        let tmp = std::env::temp_dir().join(format!("replayfix-http-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(tmp.join("index.html"), "<h1>Hello Sandbox</h1>").unwrap();
        fs::write(tmp.join("src/app.js"), "export const x = 1;").unwrap();

        let mut server = SandboxServer::start(&tmp, None).unwrap();
        assert!(server.port() > 0);

        // Open an idle connection that sends nothing to verify it cannot block subsequent requests or shutdown.
        let _idle_stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();

        let mut stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.contains("<h1>Hello Sandbox</h1>"));

        let mut bad_stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        bad_stream
            .write_all(b"GET /../secret.txt HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut bad_resp = String::new();
        bad_stream.read_to_string(&mut bad_resp).unwrap();
        assert!(bad_resp.starts_with("HTTP/1.1 403 Forbidden"));

        let mut drive_stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        drive_stream
            .write_all(b"GET /C:/Windows/win.ini HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut drive_resp = String::new();
        drive_stream.read_to_string(&mut drive_resp).unwrap();
        assert!(drive_resp.starts_with("HTTP/1.1 403 Forbidden"));

        server.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_bridge_command_timeout_terminates_hung_process_and_collects_stderr() {
        let tmp =
            std::env::temp_dir().join(format!("replayfix-timeout-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Mock bridge script that replies READY to START_BROWSER, then logs to stderr and hangs forever on PROBE_PAGE.
        let hung_script = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on('line', (line) => {
  const cmd = JSON.parse(line);
  if (cmd.cmd === 'START_BROWSER') {
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: cmd.request_id }) + '\n');
  } else if (cmd.cmd === 'PROBE_PAGE') {
    process.stderr.write('SIMULATED_CDP_STALL: execution context wedged\n');
    // Intentionally never write to stdout
    setInterval(() => {}, 1000);
  }
});
"#;

        let mut bridge = PlaywrightBridge::start_with_script_and_timeouts(
            &tmp,
            hung_script,
            false,
            1280,
            720,
            Duration::from_secs(5),
            Duration::from_millis(300),
        )
        .unwrap();

        let start = Instant::now();
        let err = bridge
            .send_command(&BridgeCommand::ProbePage {
                url: "http://127.0.0.1:9999/index.html".into(),
                click_x: 100,
                click_y: 100,
            })
            .unwrap_err();
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(5),
            "Timeout enforcement took too long: {elapsed:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("timed out"),
            "Expected timeout error, got: {msg}"
        );
        assert!(
            msg.contains("SIMULATED_CDP_STALL"),
            "Expected captured stderr in error, got: {msg}"
        );

        // Subsequent commands on a poisoned bridge must fail immediately without blocking.
        assert!(bridge.send_command(&BridgeCommand::Shutdown).is_err());

        // Shutdown must complete rapidly without hanging on the already-terminated child.
        bridge.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_bridge_correlation_skips_stale_id_and_rejects_future_id_and_handles_stderr_flood() {
        let tmp = std::env::temp_dir().join(format!("replayfix-corr-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Script floods >128 KiB to stderr (testing pipe deadlock immunity), emits a stale request_id=0
        // before sending the matching request_id for PROBE_PAGE, and then sends a mismatched future request_id=999 on CLICK_AND_OBSERVE.
        let corr_script = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on('line', (line) => {
  const cmd = JSON.parse(line);
  if (cmd.cmd === 'START_BROWSER') {
    for (let i = 0; i < 1500; i++) {
      process.stderr.write('STDERR_FLOOD_LINE_' + i + '_' + 'X'.repeat(80) + '\n');
    }
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: cmd.request_id }) + '\n');
  } else if (cmd.cmd === 'PROBE_PAGE') {
    // Emit a late response from an earlier request_id (0 < cmd.request_id)
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: 0 }) + '\n');
    // Then emit the correlated response
    process.stdout.write(JSON.stringify({
      kind: 'PROBE_RESULT',
      request_id: cmd.request_id,
      navigated: true,
      aria_snapshot: '- button "OK"',
      candidates: [],
      page_errors: []
    }) + '\n');
  } else if (cmd.cmd === 'CLICK_AND_OBSERVE') {
    // Emit a corrupted future request_id=999
    process.stdout.write(JSON.stringify({
      kind: 'CLICK_RESULT',
      request_id: 999,
      clicked: true,
      page_errors: [],
      result_text: 'wrong'
    }) + '\n');
  }
});
"#;

        let mut bridge = PlaywrightBridge::start_with_script_and_timeouts(
            &tmp,
            corr_script,
            false,
            1280,
            720,
            Duration::from_secs(5),
            Duration::from_secs(5),
        )
        .unwrap();

        // PROBE_PAGE should skip stale request_id=0 and return the matching PROBE_RESULT.
        let probe = bridge
            .send_command(&BridgeCommand::ProbePage {
                url: "http://127.0.0.1:9999/".into(),
                click_x: 10,
                click_y: 10,
            })
            .unwrap();
        assert!(matches!(
            probe,
            BridgeResponse::ProbeResult {
                navigated: true,
                ..
            }
        ));

        // CLICK_AND_OBSERVE receives mismatched request_id=999 and must fail closed.
        let err = bridge
            .send_command(&BridgeCommand::ClickAndObserve {
                role: "button".into(),
                accessible_name: "OK".into(),
                test_id: None,
                output_selector: "#out".into(),
            })
            .unwrap_err();
        assert!(err.to_string().contains("Protocol correlation mismatch"));

        bridge.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_bridge_unexpected_eof_and_malformed_json_and_unresponsive_shutdown() {
        let tmp = std::env::temp_dir().join(format!("replayfix-eof-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // 1. Child exits abruptly (EOF) during PROBE_PAGE after writing to stderr.
        let eof_script = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on('line', (line) => {
  const cmd = JSON.parse(line);
  if (cmd.cmd === 'START_BROWSER') {
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: cmd.request_id }) + '\n');
  } else if (cmd.cmd === 'PROBE_PAGE') {
    process.stderr.write('FATAL_BRIDGE_CRASH: simulated segfault\n');
    process.exit(1);
  }
});
"#;
        let mut eof_bridge = PlaywrightBridge::start_with_script_and_timeouts(
            &tmp,
            eof_script,
            false,
            1280,
            720,
            Duration::from_secs(5),
            Duration::from_secs(5),
        )
        .unwrap();
        let eof_err = eof_bridge
            .send_command(&BridgeCommand::ProbePage {
                url: "http://127.0.0.1:9999/".into(),
                click_x: 10,
                click_y: 10,
            })
            .unwrap_err();
        assert!(eof_err.to_string().contains("closed stdout unexpectedly"));
        assert!(eof_err.to_string().contains("FATAL_BRIDGE_CRASH"));
        eof_bridge.shutdown();

        // 2. Child emits malformed JSON and ignores SHUTDOWN command.
        let malformed_script = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on('line', (line) => {
  const cmd = JSON.parse(line);
  if (cmd.cmd === 'START_BROWSER') {
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: cmd.request_id }) + '\n');
  } else if (cmd.cmd === 'PROBE_PAGE') {
    process.stdout.write('{not-valid-json\n');
  } else if (cmd.cmd === 'SHUTDOWN') {
    // Intentionally ignore SHUTDOWN to verify forced termination
    setInterval(() => {}, 1000);
  }
});
"#;
        let mut bad_bridge = PlaywrightBridge::start_with_script_and_timeouts(
            &tmp,
            malformed_script,
            false,
            1280,
            720,
            Duration::from_secs(5),
            Duration::from_secs(5),
        )
        .unwrap();
        let json_err = bad_bridge
            .send_command(&BridgeCommand::ProbePage {
                url: "http://127.0.0.1:9999/".into(),
                click_x: 10,
                click_y: 10,
            })
            .unwrap_err();
        assert!(json_err.to_string().contains("Invalid JSON"));
        bad_bridge.shutdown();

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_live_playwright_bridge_reverse_probes_and_clicks_rf01() {
        let tmp = std::env::temp_dir().join(format!("replayfix-pw-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();

        let fixture = Fixture::for_task("RF-01").unwrap();
        fs::write(tmp.join("index.html"), fixture.index_html).unwrap();
        fs::write(tmp.join(fixture.target_path), fixture.buggy_source).unwrap();

        let mut server = SandboxServer::start(&tmp, None).unwrap();
        let mut bridge =
            PlaywrightBridge::start(&tmp, false, fixture.viewport_width, fixture.viewport_height)
                .unwrap();

        let url = format!("{}/index.html", server.base_url());
        let probe = bridge
            .send_command(&BridgeCommand::ProbePage {
                url,
                click_x: fixture.click_x,
                click_y: fixture.click_y,
            })
            .unwrap();

        let BridgeResponse::ProbeResult {
            navigated,
            aria_snapshot,
            candidates,
            page_errors,
        } = probe
        else {
            panic!("Expected ProbeResult, got {probe:?}");
        };
        assert!(navigated);
        assert!(page_errors.is_empty());

        let grounded = LocatorResolver::resolve(
            fixture.click_x,
            fixture.click_y,
            fixture.viewport_width,
            fixture.viewport_height,
            &aria_snapshot,
            &candidates,
        )
        .unwrap();
        assert_eq!(
            grounded.selector,
            "page.getByRole('button', { name: 'Refresh Cart', exact: true })"
        );

        let click_res = bridge
            .send_command(&BridgeCommand::ClickAndObserve {
                role: grounded.candidate.role,
                accessible_name: grounded.candidate.accessible_name,
                test_id: grounded.candidate.test_id,
                output_selector: "#cart-summary-output".into(),
            })
            .unwrap();

        let BridgeResponse::ClickResult {
            clicked,
            page_errors,
            result_text,
        } = click_res
        else {
            panic!("Expected ClickResult, got {click_res:?}");
        };
        assert!(clicked);
        assert!(
            !page_errors.is_empty(),
            "Expected Gate 1 TypeError in page_errors"
        );
        assert_eq!(result_text, "Pending");

        // Verify TOCTOU detachment detection when clicking a non-existent/detached element.
        let detached_res = bridge
            .send_command(&BridgeCommand::ClickAndObserve {
                role: "button".into(),
                accessible_name: "Detached Button".into(),
                test_id: None,
                output_selector: "#cart-summary-output".into(),
            })
            .unwrap();
        match detached_res {
            BridgeResponse::Error { message } => {
                assert!(
                    message.contains("DOM_NODE_DETACHED"),
                    "Expected DOM_NODE_DETACHED error, got: {message}"
                );
            }
            other => panic!("Expected Error for detached node, got: {other:?}"),
        }

        bridge.shutdown();
        server.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }
}
