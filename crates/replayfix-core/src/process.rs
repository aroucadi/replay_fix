//! Bounded subprocess execution for external tools (FFmpeg, ffprobe, curl).
//!
//! Every call has a wall-clock deadline and an stdout size cap. On breach the whole
//! process tree is terminated through [`Platform::terminate_process_tree`], so a hung
//! or runaway tool can never stall the harness or leave zombies behind.

use crate::error::{CoreError, Result};
use crate::platform::Platform;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// How often the supervisor checks the child for exit.
const POLL_INTERVAL: Duration = Duration::from_millis(10);
/// How long to wait for reader threads once the child is gone.
const READER_GRACE: Duration = Duration::from_secs(2);
/// stderr is diagnostic only; keep the tail of at most this many bytes.
const MAX_STDERR_BYTES: usize = 64 * 1024;

/// Captured result of a finished subprocess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedOutput {
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl BoundedOutput {
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

fn read_capped(mut pipe: impl Read, cap: usize, overflow: &AtomicBool) -> Vec<u8> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if data.len() + n > cap {
                    overflow.store(true, Ordering::SeqCst);
                    break;
                }
                data.extend_from_slice(&chunk[..n]);
            }
        }
    }
    data
}

fn drain_stderr_tail(mut pipe: impl Read) -> Vec<u8> {
    let mut tail: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                tail.extend_from_slice(&chunk[..n]);
                if tail.len() > MAX_STDERR_BYTES {
                    let excess = tail.len() - MAX_STDERR_BYTES;
                    tail.drain(..excess);
                }
            }
        }
    }
    tail
}

/// Run `command` to completion within `timeout`, capturing at most `max_stdout` bytes.
///
/// `stdin` (if any) is written on a helper thread so a child that never reads its input
/// cannot block the supervisor. A non-zero exit code is reported in the output, not as
/// an error; timeouts and oversized output are errors.
pub fn run_bounded(
    mut command: Command,
    stdin: Option<Vec<u8>>,
    timeout: Duration,
    max_stdout: usize,
) -> Result<BoundedOutput> {
    let program = command.get_program().to_string_lossy().into_owned();
    command
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| CoreError::Execution(format!("Failed to start '{program}': {e}")))?;
    let pid = child.id();

    if let (Some(payload), Some(mut pipe)) = (stdin, child.stdin.take()) {
        thread::spawn(move || {
            // A broken pipe just means the child exited early; its exit code tells the story.
            let _ = pipe.write_all(&payload);
        });
    }

    let overflow = Arc::new(AtomicBool::new(false));
    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    if let Some(pipe) = child.stdout.take() {
        let flag = Arc::clone(&overflow);
        thread::spawn(move || {
            let _ = out_tx.send(read_capped(pipe, max_stdout, &flag));
        });
    }
    if let Some(pipe) = child.stderr.take() {
        thread::spawn(move || {
            let _ = err_tx.send(drain_stderr_tail(pipe));
        });
    }

    let deadline = Instant::now() + timeout;
    let exit_status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(e) => break Err(CoreError::Io(e)),
        }
        if overflow.load(Ordering::SeqCst) {
            break Err(CoreError::Execution(format!(
                "'{program}' produced more than {max_stdout} bytes of output"
            )));
        }
        if Instant::now() >= deadline {
            break Err(CoreError::CdpTimeout(format!(
                "'{program}' did not finish within {} ms",
                timeout.as_millis()
            )));
        }
        thread::sleep(POLL_INTERVAL);
    };

    let status = match exit_status {
        Ok(status) => status,
        Err(err) => {
            // Fail closed: kill the whole tree, reap, then report the original cause.
            let _ = Platform::terminate_process_tree(pid);
            let _ = child.kill();
            let _ = child.wait();
            return Err(err);
        }
    };

    let stdout = out_rx.recv_timeout(READER_GRACE).unwrap_or_default();
    if overflow.load(Ordering::SeqCst) {
        return Err(CoreError::Execution(format!(
            "'{program}' produced more than {max_stdout} bytes of output"
        )));
    }
    let stderr_bytes = err_rx.recv_timeout(READER_GRACE).unwrap_or_default();

    Ok(BoundedOutput {
        stdout,
        stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
        exit_code: status.code(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(script: &str) -> Command {
        if cfg!(windows) {
            let mut c = Command::new("powershell");
            c.args(["-NoProfile", "-Command", script]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", script]);
            c
        }
    }

    #[test]
    fn test_captures_stdout_and_exit_code() {
        let out = run_bounded(
            shell("Write-Output hello"),
            None,
            Duration::from_secs(20),
            1024,
        )
        .unwrap();
        assert!(out.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("hello"));
    }

    #[test]
    fn test_nonzero_exit_is_reported_not_raised() {
        let out = run_bounded(shell("exit 3"), None, Duration::from_secs(20), 1024).unwrap();
        assert_eq!(out.exit_code, Some(3));
        assert!(!out.success());
    }

    #[test]
    fn test_timeout_kills_hung_process() {
        let started = Instant::now();
        let err = run_bounded(
            shell("Start-Sleep -Seconds 60"),
            None,
            Duration::from_millis(500),
            1024,
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::CdpTimeout(_)), "got {err:?}");
        assert!(started.elapsed() < Duration::from_secs(15));
    }

    #[test]
    fn test_oversized_stdout_is_rejected() {
        let err = run_bounded(
            shell("1..5000 | ForEach-Object { 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' }"),
            None,
            Duration::from_secs(30),
            256,
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::Execution(_)), "got {err:?}");
    }

    #[test]
    fn test_missing_program_is_a_typed_error() {
        let err = run_bounded(
            Command::new("replayfix-no-such-binary"),
            None,
            Duration::from_secs(5),
            16,
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::Execution(_)));
    }

    #[test]
    fn test_stdin_is_delivered() {
        let out = run_bounded(
            shell("[Console]::In.ReadToEnd()"),
            Some(b"ping".to_vec()),
            Duration::from_secs(20),
            1024,
        )
        .unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).contains("ping"));
    }
}
