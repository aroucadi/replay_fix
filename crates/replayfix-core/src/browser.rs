//! Embedded sandbox HTTP server and Node.js Playwright stdio JSON-lines bridge.
//!
//! `SandboxServer` serves files from an isolated `ScratchSandbox` over
//! `127.0.0.1:<ephemeral_port>` with strict path validation.
//! `PlaywrightBridge` spawns `node` running the embedded bridge script and
//! communicates via flushed, line-delimited JSON (`ProbePage` -> Rust
//! `LocatorResolver` -> `ClickAndObserve` -> `Shutdown`).

use crate::error::{CoreError, Result};
use crate::locator::DomCandidate;
use crate::manifest::TaskManifest;
use crate::platform::Platform;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Filename used when materializing the bridge script inside a scratch sandbox.
pub const BRIDGE_SCRIPT_FILENAME: &str = ".replayfix-playwright-bridge.cjs";

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

function sendLine(obj) {
  process.stdout.write(JSON.stringify(obj) + '\n');
}

(async () => {
  let pw;
  let browser = null;
  let page = null;
  let pageErrors = [];

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
      sendLine({ kind: 'ERROR', message: 'Invalid JSON command: ' + String(err.message || err) });
      continue;
    }

    try {
      if (cmd.cmd === 'START_BROWSER') {
        if (!pw) pw = loadPlaywright();
        if (browser) {
          await browser.close().catch(() => {});
        }
        browser = await pw.chromium.launch({ headless: !cmd.headed });
        const context = await browser.newContext({
          viewport: {
            width: cmd.viewport_width || 1280,
            height: cmd.viewport_height || 720,
          },
        });
        page = await context.newPage();
        pageErrors = [];
        page.on('pageerror', (err) => {
          pageErrors.push(String(err && err.stack ? err.stack : err));
        });
        sendLine({ kind: 'READY' });
      } else if (cmd.cmd === 'PROBE_PAGE') {
        if (!page) throw new Error('Browser page not initialized');
        pageErrors = [];
        await page.goto(cmd.url, { waitUntil: 'load', timeout: 10000 });
        const ariaSnapshot = await page.locator('body').ariaSnapshot();
        const candidates = await page.evaluate(({ x, y }) => {
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
        }, { x: cmd.click_x, y: cmd.click_y });

        sendLine({
          kind: 'PROBE_RESULT',
          navigated: true,
          aria_snapshot: ariaSnapshot,
          candidates: candidates,
          page_errors: pageErrors.slice(),
        });
      } else if (cmd.cmd === 'CLICK_AND_OBSERVE') {
        if (!page) throw new Error('Browser page not initialized');
        pageErrors = [];
        let locator;
        if (cmd.role && cmd.accessible_name) {
          locator = page.getByRole(cmd.role, { name: cmd.accessible_name, exact: true });
        } else if (cmd.test_id) {
          locator = page.getByTestId(cmd.test_id);
        } else {
          throw new Error('Grounded click requires (role + accessible_name) or test_id');
        }
        await locator.click({ timeout: 5000 });
        await page.waitForTimeout(50);
        let resultText = '';
        if (cmd.output_selector) {
          const outEl = page.locator(cmd.output_selector);
          if ((await outEl.count()) > 0) {
            resultText = ((await outEl.textContent()) || '').trim();
          }
        }
        sendLine({
          kind: 'CLICK_RESULT',
          clicked: true,
          page_errors: pageErrors.slice(),
          result_text: resultText,
        });
      } else if (cmd.cmd === 'SHUTDOWN') {
        if (browser) {
          await browser.close().catch(() => {});
          browser = null;
        }
        sendLine({ kind: 'SHUTDOWN_OK' });
        break;
      } else {
        sendLine({ kind: 'ERROR', message: 'Unknown command: ' + String(cmd.cmd) });
      }
    } catch (err) {
      sendLine({
        kind: 'ERROR',
        message: String(err && err.message ? err.message : err),
      });
    }
  }

  if (browser) {
    await browser.close().catch(() => {});
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
            while !flag_clone.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = handle_http_connection(stream, &root);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
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
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

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
    if meta.file_type().is_symlink() || !meta.is_file() {
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

/// Line-delimited JSON-lines stdio client managing the Node.js Playwright bridge child process.
#[derive(Debug)]
pub struct PlaywrightBridge {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    script_path: PathBuf,
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
        let script_path = sandbox_dir.join(BRIDGE_SCRIPT_FILENAME);
        fs::write(&script_path, PLAYWRIGHT_BRIDGE_JS)?;

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

        let mut bridge = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            script_path,
        };

        match bridge.send_command(&BridgeCommand::StartBrowser {
            headed,
            viewport_width,
            viewport_height,
        })? {
            BridgeResponse::Ready => Ok(bridge),
            BridgeResponse::Error { message } => Err(CoreError::Browser(message)),
            other => Err(CoreError::Browser(format!(
                "Unexpected response to START_BROWSER: {other:?}"
            ))),
        }
    }

    /// Child process ID for sandbox process tree supervision.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Sends a single JSON-line command to the bridge and reads one JSON-line response.
    pub fn send_command(&mut self, command: &BridgeCommand) -> Result<BridgeResponse> {
        let mut line = serde_json::to_string(command)?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .map_err(|e| CoreError::Browser(format!("Failed writing to bridge stdin: {e}")))?;
        self.stdin
            .flush()
            .map_err(|e| CoreError::Browser(format!("Failed flushing bridge stdin: {e}")))?;

        let mut response_line = String::new();
        let bytes = self
            .stdout
            .read_line(&mut response_line)
            .map_err(|e| CoreError::Browser(format!("Failed reading from bridge stdout: {e}")))?;

        if bytes == 0 {
            return Err(CoreError::Browser(
                "Playwright bridge closed stdout unexpectedly".into(),
            ));
        }

        let parsed: BridgeResponse = serde_json::from_str(response_line.trim()).map_err(|e| {
            CoreError::Browser(format!(
                "Invalid JSON from Playwright bridge ('{}'): {e}",
                response_line.trim()
            ))
        })?;
        Ok(parsed)
    }

    /// Gracefully shuts down the browser and removes the temporary bridge script.
    pub fn shutdown(&mut self) {
        let _ = self.send_command(&BridgeCommand::Shutdown);
        let _ = Platform::terminate_process_tree(self.child.id());
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.script_path);
    }
}

impl Drop for PlaywrightBridge {
    fn drop(&mut self) {
        self.shutdown();
    }
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

        bridge.shutdown();
        server.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }
}
