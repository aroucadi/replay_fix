//! Universal Modern Web SPA environment management (Spec 130):
//! - **Attached Mode (`--url <endpoint> --repo <path>`)**: Validates and connects to an
//!   externally running application server without starting an internal static file server.
//! - **Managed Mode (`--repo <path>`)**: Auto-detects standard SPA toolchains (`vite`, `next`,
//!   `react-scripts`, or Node dev scripts) from `package.json` and filesystem config markers,
//!   pre-allocates an ephemeral port via `Platform::allocate_ephemeral_port()` (with `:0`
//!   stdout/stderr regex confirmation), polls HTTP readiness up to a 30-second deadline, and
//!   enforces RAII process-tree cleanup on `Drop`.

use crate::error::{CoreError, Result};
use crate::platform::Platform;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Default timeout for a managed SPA dev server to bind a dynamic port and pass HTTP readiness.
pub const DEFAULT_SPA_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum lines of child stdout/stderr retained for diagnostic reporting.
const MAX_DIAGNOSTIC_LINES: usize = 64;

/// Detected SPA toolchain from `package.json` and filesystem markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpaToolchain {
    Vite,
    NextJs,
    ReactScripts,
    NodeScript,
}

impl SpaToolchain {
    /// Human-readable slug for receipts and diagnostics.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Vite => "vite",
            Self::NextJs => "next",
            Self::ReactScripts => "react-scripts",
            Self::NodeScript => "node-script",
        }
    }

    /// Inspect `package.json` and filesystem markers inside `repo_dir` to determine the
    /// toolchain and launch command (`Vite` > `NextJs` > `ReactScripts` > `scripts.dev` > `scripts.start`).
    ///
    /// Returns `(toolchain, program, args)`.
    pub fn detect(repo_dir: &Path) -> Result<(Self, String, Vec<String>)> {
        let pkg_path = repo_dir.join("package.json");
        if !pkg_path.is_file() {
            return Err(CoreError::Environment(format!(
                "No package.json found in repository '{}'",
                repo_dir.display()
            )));
        }
        let raw = fs::read_to_string(&pkg_path)?;
        let has_vite_cfg = ["vite.config.ts", "vite.config.js", "vite.config.mjs"]
            .iter()
            .any(|f| repo_dir.join(f).is_file());
        let has_next_cfg = ["next.config.js", "next.config.mjs", "next.config.ts"]
            .iter()
            .any(|f| repo_dir.join(f).is_file());

        Self::detect_with_markers(&raw, has_vite_cfg, has_next_cfg, 0)
    }

    /// Parse `package.json` content and determine `(toolchain, program, args)`.
    pub fn detect_from_package_json_str(raw_json: &str) -> Result<(Self, String, Vec<String>)> {
        Self::detect_with_markers(raw_json, false, false, 0)
    }

    /// Parse `package.json` content with filesystem markers and target port (`0` or pre-allocated).
    pub fn detect_with_markers(
        raw_json: &str,
        has_vite_cfg: bool,
        has_next_cfg: bool,
        port: u16,
    ) -> Result<(Self, String, Vec<String>)> {
        let val: serde_json::Value = serde_json::from_str(raw_json).map_err(|e| {
            CoreError::Environment(format!("Invalid package.json syntax in target repo: {e}"))
        })?;

        let port_str = port.to_string();
        let scripts = val.get("scripts").and_then(|s| s.as_object());
        let deps = val.get("dependencies").and_then(|d| d.as_object());
        let dev_deps = val.get("devDependencies").and_then(|d| d.as_object());

        let has_dep = |name: &str| -> bool {
            deps.is_some_and(|d| d.contains_key(name))
                || dev_deps.is_some_and(|d| d.contains_key(name))
        };

        let dev_script = scripts
            .and_then(|s| s.get("dev"))
            .and_then(|v| v.as_str())
            .map(str::trim);
        let start_script = scripts
            .and_then(|s| s.get("start"))
            .and_then(|v| v.as_str())
            .map(str::trim);

        // Priority 1: Vite (dependency, dev script, or vite.config.* marker)
        if has_vite_cfg || has_dep("vite") || dev_script.is_some_and(|s| s.contains("vite")) {
            let prog = if cfg!(windows) { "npx.cmd" } else { "npx" }.to_string();
            return Ok((
                Self::Vite,
                prog,
                vec![
                    "vite".to_string(),
                    "--port".to_string(),
                    port_str,
                    "--host".to_string(),
                    "127.0.0.1".to_string(),
                ],
            ));
        }

        // Priority 2: Next.js (dependency, dev script, or next.config.* marker)
        if has_next_cfg || has_dep("next") || dev_script.is_some_and(|s| s.contains("next")) {
            let prog = if cfg!(windows) { "npm.cmd" } else { "npm" }.to_string();
            return Ok((
                Self::NextJs,
                prog,
                vec![
                    "run".to_string(),
                    "dev".to_string(),
                    "--".to_string(),
                    "-p".to_string(),
                    port_str,
                ],
            ));
        }

        // Priority 3: Create-React-App (react-scripts)
        if has_dep("react-scripts")
            || start_script.is_some_and(|s| s.contains("react-scripts"))
            || dev_script.is_some_and(|s| s.contains("react-scripts"))
        {
            let prog = if cfg!(windows) { "npm.cmd" } else { "npm" }.to_string();
            let sub = if dev_script.is_some() { "dev" } else { "start" };
            return Ok((
                Self::ReactScripts,
                prog,
                vec!["run".to_string(), sub.to_string()],
            ));
        }

        // Priority 4 & 5: Direct `node <file>` or `scripts.dev` > `scripts.start`
        if let Some(script) = dev_script.or(start_script) {
            if let Some(rest) = script.strip_prefix("node ") {
                let mut parts: Vec<String> =
                    rest.split_whitespace().map(ToString::to_string).collect();
                if !parts.is_empty() {
                    if !parts.iter().any(|p| p == "--port" || p == "-p") {
                        parts.push("--port".to_string());
                        parts.push(port_str);
                    }
                    return Ok((Self::NodeScript, "node".to_string(), parts));
                }
            }
        }

        if dev_script.is_some() {
            let prog = if cfg!(windows) { "npm.cmd" } else { "npm" }.to_string();
            return Ok((
                Self::NodeScript,
                prog,
                vec![
                    "run".to_string(),
                    "dev".to_string(),
                    "--".to_string(),
                    "--port".to_string(),
                    port_str,
                ],
            ));
        }

        if start_script.is_some() {
            let prog = if cfg!(windows) { "npm.cmd" } else { "npm" }.to_string();
            return Ok((
                Self::NodeScript,
                prog,
                vec![
                    "run".to_string(),
                    "start".to_string(),
                    "--".to_string(),
                    "--port".to_string(),
                    port_str,
                ],
            ));
        }

        Err(CoreError::Environment(
            "package.json contains neither 'scripts.dev' nor 'scripts.start'".to_string(),
        ))
    }
}

fn url_port_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https?://(?:localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\]):(\d{1,5})\b").ok()
    })
    .as_ref()
}

fn fallback_port_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bport\s*[:=]?\s*(\d{2,5})\b").ok())
        .as_ref()
}

/// Extract a dynamic port (`1..=65535`) from a single line of dev-server stdout/stderr.
#[must_use]
pub fn extract_port_from_line(line: &str) -> Option<u16> {
    let stripped = strip_ansi_codes(line);
    if let Some(re) = url_port_regex() {
        for caps in re.captures_iter(&stripped) {
            if let Some(m) = caps.get(1) {
                if let Ok(port) = m.as_str().parse::<u16>() {
                    if port > 0 {
                        return Some(port);
                    }
                }
            }
        }
    }
    if let Some(re) = fallback_port_regex() {
        for caps in re.captures_iter(&stripped) {
            if let Some(m) = caps.get(1) {
                if let Ok(port) = m.as_str().parse::<u16>() {
                    if port > 0 {
                        return Some(port);
                    }
                }
            }
        }
    }
    None
}

fn strip_ansi_codes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                let _ = chars.next();
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Ephemeral managed SPA child dev server with hybrid port pre-allocation + `:0` stdout/stderr
/// port capture, HTTP health polling, and deterministic RAII process-tree teardown.
#[derive(Debug)]
pub struct ManagedSpaServer {
    toolchain: SpaToolchain,
    port: u16,
    child: Child,
    pid: u32,
    diagnostics: Arc<Mutex<Vec<String>>>,
    shut_down: bool,
}

impl ManagedSpaServer {
    /// Returns `true` when `repo_dir` contains a `package.json` with a runnable dev/start script.
    #[must_use]
    pub fn has_package_json(repo_dir: &Path) -> bool {
        SpaToolchain::detect(repo_dir).is_ok()
    }

    /// Returns `true` when `working_dir` has a detectable `package.json` dev/start script AND
    /// the required runtime (`node <script>` file or `node_modules`) is present on disk.
    #[must_use]
    pub fn is_runnable(working_dir: &Path, base_repo: Option<&Path>) -> bool {
        let Ok((_, prog, args)) = SpaToolchain::detect(working_dir) else {
            return false;
        };
        if prog == "node" {
            return args
                .first()
                .is_some_and(|script| working_dir.join(script).is_file());
        }
        working_dir.join("node_modules").is_dir()
            || base_repo.is_some_and(|b| b.join("node_modules").is_dir())
    }

    /// Alias for [`Self::has_package_json`].
    #[must_use]
    pub fn is_managed_spa(repo_dir: &Path) -> bool {
        Self::has_package_json(repo_dir)
    }

    /// Spawn the managed SPA dev server in `working_dir`, inheriting `node_modules` from
    /// `base_repo` when present, using the default 30-second readiness timeout.
    pub fn spawn(working_dir: &Path, base_repo: Option<&Path>) -> Result<Self> {
        Self::start_with_timeout(working_dir, base_repo, DEFAULT_SPA_STARTUP_TIMEOUT)
    }

    /// Alias for [`Self::spawn`].
    pub fn start(working_dir: &Path, base_repo: Option<&Path>) -> Result<Self> {
        Self::spawn(working_dir, base_repo)
    }

    /// Boot the managed SPA dev server with an explicit readiness timeout.
    pub fn start_with_timeout(
        working_dir: &Path,
        base_repo: Option<&Path>,
        startup_timeout: Duration,
    ) -> Result<Self> {
        let pkg_path = working_dir.join("package.json");
        if !pkg_path.is_file() {
            return Err(CoreError::Environment(format!(
                "No package.json found in repository '{}'",
                working_dir.display()
            )));
        }
        let raw_pkg = fs::read_to_string(&pkg_path)?;
        let has_vite_cfg = ["vite.config.ts", "vite.config.js", "vite.config.mjs"]
            .iter()
            .any(|f| working_dir.join(f).is_file());
        let has_next_cfg = ["next.config.js", "next.config.mjs", "next.config.ts"]
            .iter()
            .any(|f| working_dir.join(f).is_file());

        // Hybrid pre-allocation: allocate a free OS port via Platform::allocate_ephemeral_port()
        // and also monitor stdout/stderr in case the dev server binds a dynamic :0 port.
        let preallocated_port = Platform::allocate_ephemeral_port().unwrap_or(0);
        let (toolchain, program, args) = SpaToolchain::detect_with_markers(
            &raw_pkg,
            has_vite_cfg,
            has_next_cfg,
            preallocated_port,
        )?;

        let mut cmd = Platform::create_command(&program);
        cmd.args(&args)
            .current_dir(working_dir)
            .env("PORT", preallocated_port.to_string())
            .env("BROWSER", "none")
            .env("CI", "true")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Wire NODE_PATH and PATH to base_repo/node_modules if the scratch sandbox omitted node_modules.
        if let Some(base) = base_repo {
            let nm = base.join("node_modules");
            if nm.is_dir() {
                if let Ok(canon_nm) = fs::canonicalize(&nm) {
                    let clean_nm_str = canon_nm
                        .to_string_lossy()
                        .trim_start_matches(r"\\?\")
                        .to_string();
                    let clean_nm = Path::new(&clean_nm_str);
                    cmd.env("NODE_PATH", &clean_nm_str);
                    let bin_dir = clean_nm.join(".bin");
                    if bin_dir.is_dir() {
                        let sep = if cfg!(windows) { ";" } else { ":" };
                        let current_path = std::env::var("PATH").unwrap_or_default();
                        cmd.env("PATH", format!("{}{sep}{current_path}", bin_dir.display()));
                    }
                }
            }
        }

        Self::spawn_and_await_ready(cmd, toolchain, preallocated_port, startup_timeout)
    }

    fn spawn_and_await_ready(
        mut cmd: Command,
        toolchain: SpaToolchain,
        preallocated_port: u16,
        startup_timeout: Duration,
    ) -> Result<Self> {
        let mut child = cmd.spawn().map_err(|e| {
            CoreError::Environment(format!("Failed to spawn managed SPA server: {e}"))
        })?;
        let pid = child.id();

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::Environment("Managed SPA stdout was not piped".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CoreError::Environment("Managed SPA stderr was not piped".into()))?;

        let diagnostics = Arc::new(Mutex::new(Vec::new()));
        let (port_tx, port_rx) = mpsc::channel::<u16>();

        spawn_output_monitor(stdout, Arc::clone(&diagnostics), port_tx.clone());
        spawn_output_monitor_stderr(stderr, Arc::clone(&diagnostics), port_tx);

        let deadline = Instant::now() + startup_timeout;
        let mut discovered_port: Option<u16> = (preallocated_port > 0).then_some(preallocated_port);

        loop {
            if let Ok(Some(status)) = child.try_wait() {
                let _ = Platform::terminate_process_tree(pid);
                let diag = format_diagnostics(&diagnostics);
                return Err(CoreError::Environment(format!(
                    "Managed SPA dev server ({}) exited prematurely with status {status}{diag}",
                    toolchain.as_str()
                )));
            }

            let now = Instant::now();
            if now >= deadline {
                let _ = Platform::terminate_process_tree(pid);
                let _ = child.wait();
                let diag = format_diagnostics(&diagnostics);
                return Err(CoreError::Environment(format!(
                    "Managed SPA dev server ({}) timed out after {}ms waiting for HTTP readiness{diag}",
                    toolchain.as_str(),
                    startup_timeout.as_millis()
                )));
            }

            while let Ok(p) = port_rx.try_recv() {
                discovered_port = Some(p);
            }

            if let Some(port) = discovered_port {
                if probe_http_ready("127.0.0.1", port, "/api/health")
                    || probe_http_ready("127.0.0.1", port, "/")
                {
                    return Ok(Self {
                        toolchain,
                        port,
                        child,
                        pid,
                        diagnostics,
                        shut_down: false,
                    });
                }
            }

            if let Ok(p) = port_rx.recv_timeout(Duration::from_millis(25)) {
                discovered_port = Some(p);
            }
        }
    }

    /// Verify that the managed dev server is still running and does not return HTTP 500.
    pub fn check_healthy(&mut self) -> Result<()> {
        if let Ok(Some(status)) = self.child.try_wait() {
            let diag = self.diagnostics_summary();
            return Err(CoreError::Environment(format!(
                "SYNTAX_COMPILATION_ERROR: Managed SPA dev server ({}) crashed with status {status}{diag}",
                self.toolchain.as_str()
            )));
        }
        if let Some(status_code) = probe_http_status("127.0.0.1", self.port, "/") {
            if status_code >= 500 {
                let diag = self.diagnostics_summary();
                return Err(CoreError::Environment(format!(
                    "SYNTAX_COMPILATION_ERROR: Managed SPA dev server ({}) returned HTTP {status_code}{diag}",
                    self.toolchain.as_str()
                )));
            }
        }
        Ok(())
    }

    /// Detected SPA toolchain.
    #[must_use]
    pub fn toolchain(&self) -> SpaToolchain {
        self.toolchain
    }

    /// Bound dynamic localhost port.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Base URL (`http://127.0.0.1:<port>`).
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Child process ID for sandbox supervision.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Captured stdout/stderr diagnostic tail lines from the dev server.
    #[must_use]
    pub fn diagnostics_summary(&self) -> String {
        format_diagnostics(&self.diagnostics)
    }

    /// Idempotently kills the dev-server process tree and reaps the child handle.
    pub fn shutdown(&mut self) {
        if self.shut_down {
            return;
        }
        self.shut_down = true;
        let _ = Platform::terminate_process_tree(self.pid);
        let _ = self.child.wait();
    }
}

impl Drop for ManagedSpaServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn spawn_output_monitor(
    stdout: ChildStdout,
    diagnostics: Arc<Mutex<Vec<String>>>,
    port_tx: mpsc::Sender<u16>,
) {
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(std::result::Result::ok) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(port) = extract_port_from_line(trimmed) {
                let _ = port_tx.send(port);
            }
            push_diagnostic(&diagnostics, trimmed);
        }
    });
}

fn spawn_output_monitor_stderr(
    stderr: ChildStderr,
    diagnostics: Arc<Mutex<Vec<String>>>,
    port_tx: mpsc::Sender<u16>,
) {
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(std::result::Result::ok) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(port) = extract_port_from_line(trimmed) {
                let _ = port_tx.send(port);
            }
            push_diagnostic(&diagnostics, trimmed);
        }
    });
}

fn push_diagnostic(diagnostics: &Arc<Mutex<Vec<String>>>, line: &str) {
    let capped: String = line.chars().take(300).collect();
    if let Ok(mut guard) = diagnostics.lock() {
        if guard.len() >= MAX_DIAGNOSTIC_LINES {
            guard.remove(0);
        }
        guard.push(capped);
    }
}

fn format_diagnostics(diagnostics: &Arc<Mutex<Vec<String>>>) -> String {
    thread::sleep(Duration::from_millis(15));
    let Ok(guard) = diagnostics.lock() else {
        return String::new();
    };
    if guard.is_empty() {
        String::new()
    } else {
        format!(" [output: {}]", guard.join(" | "))
    }
}

/// Probe `host:port` at `path` and return the HTTP status code (`200`, `404`, `500`, etc.).
#[must_use]
pub fn probe_http_status(host: &str, port: u16, path: &str) -> Option<u16> {
    let addr_str = format!("{host}:{port}");
    let mut addrs = addr_str.to_socket_addrs().ok()?;
    let addr: SocketAddr = addrs.next()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(200)).ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));

    let req = format!(
        "GET {path} HTTP/1.1\x0d\x0aHost: {host}:{port}\x0d\x0aConnection: close\x0d\x0a\x0d\x0a"
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).ok()?;
    if n == 0 {
        return None;
    }
    let head = String::from_utf8_lossy(&buf[..n]);
    let status_line = head.lines().next()?;
    let mut parts = status_line.split_whitespace();
    let proto = parts.next()?;
    if !proto.starts_with("HTTP/1.") {
        return None;
    }
    parts.next()?.parse::<u16>().ok()
}

/// Perform a lightweight bounded HTTP GET probe against `host:port` at `path`.
#[must_use]
pub fn probe_http_ready(host: &str, port: u16, path: &str) -> bool {
    matches!(probe_http_status(host, port, path), Some(200..=399))
}

/// Validated external SPA endpoint for Attached Mode (`--url <endpoint> --repo <path>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedSpaEndpoint {
    url: String,
    host: String,
    port: u16,
    path: String,
}

impl AttachedSpaEndpoint {
    /// Parse and validate an external HTTP/HTTPS application URL syntax.
    pub fn parse(raw_url: &str) -> Result<Self> {
        let trimmed = raw_url.trim();
        let (scheme, rest) = if let Some(r) = trimmed.strip_prefix("http://") {
            ("http", r)
        } else if let Some(r) = trimmed.strip_prefix("https://") {
            ("https", r)
        } else {
            return Err(CoreError::Manifest(format!(
                "Invalid --url '{trimmed}': must start with http:// or https://"
            )));
        };

        let (authority, path_part) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, "/"),
        };

        if authority.is_empty() {
            return Err(CoreError::Manifest(format!(
                "Invalid --url '{trimmed}': missing host"
            )));
        }

        let default_port = if scheme == "https" { 443 } else { 80 };
        let (host, port) = if let Some((h, p_str)) = authority.rsplit_once(':') {
            let p = p_str.parse::<u16>().map_err(|_| {
                CoreError::Manifest(format!("Invalid port '{p_str}' in --url '{trimmed}'"))
            })?;
            if p == 0 {
                return Err(CoreError::Manifest(format!(
                    "Invalid port 0 in --url '{trimmed}'"
                )));
            }
            (h.to_string(), p)
        } else {
            (authority.to_string(), default_port)
        };

        if host.is_empty() {
            return Err(CoreError::Manifest(format!(
                "Invalid --url '{trimmed}': empty host"
            )));
        }

        Ok(Self {
            url: trimmed.to_string(),
            host,
            port,
            path: path_part.to_string(),
        })
    }

    /// Validate URL syntax and verify TCP/HTTP reachability within a bounded timeout.
    pub fn validate(raw_url: &str) -> Result<Self> {
        let ep = Self::parse(raw_url)?;
        ep.await_reachable(Duration::from_secs(10))?;
        Ok(ep)
    }

    /// Full target URL string.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Target host.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Target TCP port.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Wait up to `timeout` for the attached endpoint to respond over HTTP (failing closed on 500 or timeout).
    pub fn await_reachable(&self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        let mut last_status: Option<u16> = None;
        loop {
            if let Some(code) = probe_http_status(&self.host, self.port, &self.path)
                .or_else(|| probe_http_status(&self.host, self.port, "/"))
            {
                last_status = Some(code);
                if (200..400).contains(&code) {
                    return Ok(());
                }
                if code >= 500 {
                    return Err(CoreError::Environment(format!(
                        "Attached SPA endpoint '{}' returned HTTP {code} server error",
                        self.url
                    )));
                }
            }
            if Instant::now() >= deadline {
                let detail = last_status
                    .map(|c| format!(" (last HTTP status: {c})"))
                    .unwrap_or_default();
                return Err(CoreError::Environment(format!(
                    "Attached SPA endpoint '{}' did not respond within {}ms{detail}",
                    self.url,
                    timeout.as_millis()
                )));
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_vite_next_cra_and_node_toolchains_with_priority_cascade() {
        let vite_pkg =
            r#"{"name":"app","devDependencies":{"vite":"^5.0.0"},"scripts":{"dev":"vite"}}"#;
        let (tc, _, args) = SpaToolchain::detect_from_package_json_str(vite_pkg).unwrap();
        assert_eq!(tc, SpaToolchain::Vite);
        assert!(args.contains(&"vite".to_string()));
        assert!(args.contains(&"0".to_string()));

        // Priority cascade: Vite beats Next.js and ReactScripts when both exist
        let multi_pkg = r#"{"name":"app","dependencies":{"next":"14.0.0","react-scripts":"5.0.1"},"devDependencies":{"vite":"^5.0.0"},"scripts":{"dev":"vite"}}"#;
        let (tc, _, _) = SpaToolchain::detect_from_package_json_str(multi_pkg).unwrap();
        assert_eq!(tc, SpaToolchain::Vite);

        let next_pkg =
            r#"{"name":"app","dependencies":{"next":"14.0.0"},"scripts":{"dev":"next dev"}}"#;
        let (tc, _, args) = SpaToolchain::detect_from_package_json_str(next_pkg).unwrap();
        assert_eq!(tc, SpaToolchain::NextJs);
        assert!(args.contains(&"-p".to_string()));

        let cra_pkg = r#"{"name":"app","dependencies":{"react-scripts":"5.0.1"},"scripts":{"start":"react-scripts start"}}"#;
        let (tc, _, _) = SpaToolchain::detect_from_package_json_str(cra_pkg).unwrap();
        assert_eq!(tc, SpaToolchain::ReactScripts);

        let node_pkg = r#"{"name":"metapulse","scripts":{"dev":"node server.js"}}"#;
        let (tc, prog, args) = SpaToolchain::detect_from_package_json_str(node_pkg).unwrap();
        assert_eq!(tc, SpaToolchain::NodeScript);
        assert_eq!(prog, "node");
        assert_eq!(args, vec!["server.js", "--port", "0"]);
    }

    #[test]
    fn test_extract_port_from_line_handles_ansi_and_ignores_zero() {
        assert_eq!(
            extract_port_from_line("Listening on http://127.0.0.1:0"),
            None
        );
        assert_eq!(
            extract_port_from_line("  ➜  Local:   \x1b[36mhttp://localhost:5173/\x1b[0m"),
            Some(5173)
        );
        assert_eq!(
            extract_port_from_line("MetaPulse server listening on http://127.0.0.1:43210"),
            Some(43210)
        );
    }

    #[test]
    fn test_attached_spa_endpoint_parsing() {
        let ep = AttachedSpaEndpoint::parse("http://localhost:3000/feed").unwrap();
        assert_eq!(ep.host(), "localhost");
        assert_eq!(ep.port(), 3000);
        assert_eq!(ep.url(), "http://localhost:3000/feed");

        assert!(AttachedSpaEndpoint::parse("ftp://localhost:3000").is_err());
        assert!(AttachedSpaEndpoint::parse("http://localhost:0").is_err());
    }

    #[test]
    fn test_managed_spa_server_lifecycle_and_raii_drop() {
        let dir =
            std::env::temp_dir().join(format!("replayfix-managed-spa-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join("package.json"),
            r#"{"name":"test-spa","scripts":{"dev":"node server.js"}}"#,
        )
        .unwrap();
        fs::write(
            dir.join("server.js"),
            r#"const http = require('http');
const server = http.createServer((req, res) => {
  res.writeHead(200, {'Content-Type': 'text/plain'});
  res.end('ok');
});
server.listen(0, '127.0.0.1', () => {
  console.log('Server listening on http://127.0.0.1:' + server.address().port);
});
"#,
        )
        .unwrap();

        let port;
        {
            let mut server = ManagedSpaServer::spawn(&dir, None).unwrap();
            port = server.port();
            assert!(port > 0);
            assert!(probe_http_ready("127.0.0.1", port, "/"));
            assert!(server.check_healthy().is_ok());
        }
        // After Drop, port should no longer be serving.
        thread::sleep(Duration::from_millis(50));
        assert!(!probe_http_ready("127.0.0.1", port, "/"));
        let _ = fs::remove_dir_all(&dir);
    }
}
