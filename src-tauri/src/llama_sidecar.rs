//! Bundled llama.cpp sidecar manager.
//!
//! Phase 1.6 of the OpenWhisper roadmap. Spawns a pre-built `llama-server`
//! binary as a child process and exposes its HTTP base URL so the existing
//! `llm_client` module can drive it as an OpenAI-compatible provider.
//!
//! ## Why a sidecar?
//!
//! We don't want to link llama.cpp directly into the Rust binary:
//!
//! - Build complexity: llama.cpp ships its own CMake + GPU backends (CUDA,
//!   Metal, Vulkan, ROCm) each with their own toolchain quirks.
//! - Binary size: a statically linked llama-server with all backends is
//!   80+ MB; we'd pay that even for users who use Ollama or Apple FM.
//! - Update cadence: llama.cpp ships breaking ABI changes monthly. Process
//!   isolation lets us version the sidecar independently from OpenWhisper.
//!
//! `llama-server` already speaks OpenAI's chat-completions API, so once it's
//! up our existing `llm_client::generate()` works against it unchanged.
//!
//! ## Binary distribution
//!
//! The actual `llama-server[.exe]` binary is NOT committed to git (50+ MB
//! per platform). It's fetched at release-build time by the GitHub Actions
//! workflow and dropped into `src-tauri/resources/llama-cpp/` before
//! `tauri build` runs. Local dev users can fetch it manually via
//! `scripts/fetch-llama-cpp.sh` (TBD in a follow-up commit).
//!
//! When the binary is missing the sidecar reports `available()=false` and
//! the BackendResolver falls through to the next cleanup option (Ollama if
//! detected, or `CleanupBackend::None`).
//!
//! ## Lifecycle
//!
//! 1. `LlamaSidecar::new()` resolves the binary path. No process spawned.
//! 2. `start(model_path, port)` spawns `llama-server --model X --port N
//!    --host 127.0.0.1`, waits for the HTTP endpoint to respond.
//! 3. `base_url()` returns `http://127.0.0.1:N/v1` for the LLM client.
//! 4. `stop()` sends SIGTERM (Unix) / TerminateProcess (Windows) and waits
//!    for exit.
//! 5. `Drop` impl calls stop() to avoid orphan processes if the manager
//!    is dropped without explicit shutdown.

use std::io::ErrorKind;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use log::{info, warn};

/// Default port range to scan when picking a free port for the sidecar.
/// 11500–11599 chosen to avoid clashing with common dev servers (3000,
/// 5173, 8000, 8080) and Ollama (11434).
const PORT_SCAN_START: u16 = 11500;
const PORT_SCAN_END: u16 = 11599;

/// How long to wait for the sidecar to start listening before giving up.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

/// Poll interval while waiting for startup.
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Manager for a single llama.cpp sidecar process.
///
/// Use through Tauri state (`app.manage(Arc::new(LlamaSidecar::new()?))`).
/// `start` and `stop` mutate internal state behind a `Mutex` so the
/// manager is `Send + Sync`.
pub struct LlamaSidecar {
    /// Absolute path to the `llama-server[.exe]` binary. `None` if the
    /// binary wasn't found in any of the search locations.
    binary_path: Option<PathBuf>,
    state: Mutex<SidecarState>,
}

#[derive(Default)]
struct SidecarState {
    child: Option<Child>,
    port: Option<u16>,
    loaded_model: Option<PathBuf>,
}

impl LlamaSidecar {
    /// Construct a new manager. Resolves the binary path but does NOT spawn
    /// the child process. Safe to call at app startup even when the user
    /// has no LLM model downloaded.
    pub fn new() -> Self {
        let binary_path = find_binary();
        if binary_path.is_none() {
            warn!(
                "llama-server binary not found in any search location; \
                 sidecar will be unavailable. See llama_sidecar module docs \
                 for installation instructions."
            );
        }
        Self {
            binary_path,
            state: Mutex::new(SidecarState::default()),
        }
    }

    /// Whether the sidecar can run on this machine (binary present on disk).
    pub fn available(&self) -> bool {
        self.binary_path.is_some()
    }

    /// Start the sidecar against the given model file. Idempotent: if a
    /// sidecar is already running with the same model, this is a no-op.
    /// If a different model is requested, the existing sidecar is stopped
    /// first.
    ///
    /// Returns the base URL the LLM client should hit.
    pub fn start(&self, model_path: &PathBuf) -> Result<String> {
        let binary = self
            .binary_path
            .as_ref()
            .ok_or_else(|| anyhow!("llama-server binary not available"))?;

        let mut state = self.state.lock().unwrap();

        if let Some(loaded) = &state.loaded_model {
            if loaded == model_path && state.child.is_some() {
                let port = state.port.expect("port present whenever child is");
                return Ok(format!("http://127.0.0.1:{}/v1", port));
            }
        }

        // Tear down any previous sidecar before booting a new one.
        stop_in_state(&mut state);

        let port = find_free_port()
            .ok_or_else(|| anyhow!("no free port in range {}..{}", PORT_SCAN_START, PORT_SCAN_END))?;

        info!(
            "Spawning llama-server: binary={:?} model={:?} port={}",
            binary, model_path, port
        );

        let child = Command::new(binary)
            .args(["--host", "127.0.0.1"])
            .args(["--port", &port.to_string()])
            .args(["--model", &model_path.to_string_lossy()])
            // Reduce output verbosity — we don't tail stdout, only health-poll.
            .args(["--log-disable"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to spawn {:?}", binary))?;

        state.child = Some(child);
        state.port = Some(port);
        state.loaded_model = Some(model_path.clone());

        // Drop the lock before the blocking poll so health-check calls
        // can read state if they need to.
        drop(state);

        wait_for_health(port).with_context(|| {
            format!(
                "llama-server failed to become healthy on port {} within {:?}",
                port, STARTUP_TIMEOUT
            )
        })?;

        Ok(format!("http://127.0.0.1:{}/v1", port))
    }

    /// Stop the sidecar if running. Idempotent.
    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        stop_in_state(&mut state);
    }

    /// Current base URL, or `None` if the sidecar isn't running.
    pub fn base_url(&self) -> Option<String> {
        let state = self.state.lock().unwrap();
        state.port.map(|p| format!("http://127.0.0.1:{}/v1", p))
    }
}

impl Drop for LlamaSidecar {
    fn drop(&mut self) {
        // Best effort — don't panic in a destructor.
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        stop_in_state(&mut state);
    }
}

/// Internal helper to clean up a running child. Centralised so `start`,
/// `stop`, and `Drop` all share the same shutdown semantics.
fn stop_in_state(state: &mut SidecarState) {
    if let Some(mut child) = state.child.take() {
        // Try graceful termination first. On Unix `kill` sends SIGKILL;
        // we'd prefer SIGTERM but std::process::Child doesn't expose it.
        // For llama-server SIGKILL is fine — it has no flush-to-disk
        // obligations once the model is loaded into RAM.
        if let Err(e) = child.kill() {
            // ESRCH on Unix / "no process" on Windows is fine — the
            // process already exited.
            if e.kind() != ErrorKind::InvalidInput {
                warn!("Failed to kill llama-server child: {}", e);
            }
        }
        // Reap the zombie to avoid leaks.
        let _ = child.wait();
    }
    state.port = None;
    state.loaded_model = None;
}

/// Search known locations for the llama-server binary. Returns the first
/// match. Order matters: bundled-in-installer first, then dev paths.
fn find_binary() -> Option<PathBuf> {
    let exe_name = if cfg!(target_os = "windows") {
        "llama-server.exe"
    } else {
        "llama-server"
    };

    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1. Sibling of the OpenWhisper binary (bundled in installer).
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            candidates.push(dir.join("llama-cpp").join(exe_name));
            candidates.push(dir.join(exe_name));
        }
    }

    // 2. Dev path under src-tauri/resources/llama-cpp/ (when running via
    //    `bun run tauri dev`). Search up from cwd to find the project root.
    if let Ok(cwd) = std::env::current_dir() {
        for parent in cwd.ancestors() {
            let dev_path = parent
                .join("src-tauri")
                .join("resources")
                .join("llama-cpp")
                .join(exe_name);
            if dev_path.is_file() {
                candidates.push(dev_path);
                break;
            }
        }
    }

    candidates.into_iter().find(|p| p.is_file())
}

/// Scan the configured port range for an available TCP port. Tries to bind
/// to each in turn; returns the first that succeeds. Returns `None` if the
/// whole range is in use.
fn find_free_port() -> Option<u16> {
    (PORT_SCAN_START..=PORT_SCAN_END).find(|&port| TcpListener::bind(("127.0.0.1", port)).is_ok())
}

/// Block until the llama-server health endpoint responds, or until the
/// startup timeout elapses. Returns Err if the server didn't come up.
fn wait_for_health(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    let agent = ureq_min::agent();
    while Instant::now() < deadline {
        match agent.get(&url).call() {
            Ok(_) => return Ok(()),
            Err(_) => std::thread::sleep(STARTUP_POLL_INTERVAL),
        }
    }
    bail!("health check timed out");
}

/// Minimal HTTP client wrapper so the sidecar doesn't pull in a full
/// `reqwest` runtime just for the health-check ping. `reqwest` IS already
/// a dep, but using it here would require dragging tokio into a sync
/// context unnecessarily.
mod ureq_min {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    pub struct Agent;
    pub struct Request<'a> {
        url: &'a str,
    }
    pub struct Response;

    pub fn agent() -> Agent {
        Agent
    }

    impl Agent {
        pub fn get<'a>(&self, url: &'a str) -> Request<'a> {
            Request { url }
        }
    }

    impl<'a> Request<'a> {
        pub fn call(&self) -> Result<Response, ()> {
            // Very small HTTP/1.1 GET — sufficient for a /health ping.
            // Expects URL of the form http://host:port/path.
            let stripped = self.url.strip_prefix("http://").ok_or(())?;
            let (host_port, path) = stripped
                .find('/')
                .map(|i| (&stripped[..i], &stripped[i..]))
                .unwrap_or((stripped, "/"));
            let mut stream =
                TcpStream::connect_timeout(&host_port.parse().map_err(|_| ())?,
                                           Duration::from_millis(500))
                    .map_err(|_| ())?;
            stream.set_read_timeout(Some(Duration::from_millis(500))).map_err(|_| ())?;
            let request = format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                path, host_port
            );
            stream.write_all(request.as_bytes()).map_err(|_| ())?;
            let mut buf = [0u8; 12];
            let n = stream.read(&mut buf).map_err(|_| ())?;
            if n >= 12 && &buf[..9] == b"HTTP/1.1 " && (&buf[9..12] == b"200" || &buf[9..12] == b"404") {
                // 404 is acceptable — llama-server returns 404 on /health
                // before model is loaded but it means the HTTP server is up.
                Ok(Response)
            } else {
                Err(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `find_free_port` returns a real free port we can bind. Sanity check.
    #[test]
    fn find_free_port_returns_a_bindable_port() {
        let port = find_free_port().expect("should find a free port in range");
        assert!(port >= PORT_SCAN_START && port <= PORT_SCAN_END);
        // The port should be bindable right now (find_free_port released
        // the test bind).
        let _bind = TcpListener::bind(("127.0.0.1", port))
            .expect("returned port should be bindable");
    }

    /// `LlamaSidecar::new()` never panics, never errors. `available()`
    /// reflects whether the binary was found.
    #[test]
    fn new_does_not_panic_when_binary_missing() {
        let sidecar = LlamaSidecar::new();
        // On dev machines the binary almost certainly isn't on disk yet.
        // Don't assert false — a contributor who's pre-installed
        // llama-server should still pass this test. Just exercise the call.
        let _ = sidecar.available();
    }

    /// `stop()` on a not-yet-started sidecar is a no-op.
    #[test]
    fn stop_is_idempotent_when_not_running() {
        let sidecar = LlamaSidecar::new();
        sidecar.stop();
        sidecar.stop(); // double-stop must not panic
    }

    /// `base_url()` returns None before start.
    #[test]
    fn base_url_is_none_before_start() {
        let sidecar = LlamaSidecar::new();
        assert!(sidecar.base_url().is_none());
    }
}
