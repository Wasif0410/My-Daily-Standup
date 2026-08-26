//! The `llama-server` child process: where it lives, how it starts, how it dies.
//!
//! This module owns the only thing in the app that is not a window or a row in
//! SQLite — a second operating-system process holding several gigabytes of
//! VRAM. That is why so much of what follows is about *ending* it.
//!
//! Deliberately minimal. There is one model at one known path and one runtime
//! at another, both put there by hand. No registry, no download, no catalog,
//! no hardware detection: this PR exists to prove the chain works inside the
//! app, and every one of those is a later PR that would be easier to design
//! once the chain is real.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::commands::{CommandError, ErrorKind};

/// Where the child's process id is recorded while it runs.
///
/// In `ui_state` rather than `settings` because it is not a preference — it is
/// a note this run leaves for the next one, and a settings reset must not be
/// able to lose track of a live process. See [`crate::storage::ui_state`].
pub const PID_KEY: &str = "inference.pid";

/// The one model this PR knows about, inside `<app data>/models`.
pub const MODEL_FILENAME: &str = "Qwen3-4B-Q4_K_M.gguf";

/// The llama.cpp server binary, inside `<app data>/runtime/vulkan`.
#[cfg(windows)]
pub const SERVER_BINARY: &str = "llama-server.exe";
#[cfg(not(windows))]
pub const SERVER_BINARY: &str = "llama-server";

/// The context window, in tokens. Matches the measured working invocation.
const CONTEXT_SIZE: u32 = 8192;

/// Every layer offloaded to the GPU. 99 rather than a real layer count because
/// llama.cpp clamps it, and the real count changes with the model.
const GPU_LAYERS: u32 = 99;

/// How long the server is given to load weights before the attempt is refused.
///
/// Generous because a cold read of a 2.5 GB file off a spinning disk is slow,
/// and the failure mode of being too impatient is far worse than waiting: the
/// app kills a server that was about to work, and the user sees a timeout they
/// cannot do anything about.
const HEALTH_TIMEOUT: Duration = Duration::from_secs(60);

/// How often the health endpoint is polled while waiting.
const HEALTH_INTERVAL: Duration = Duration::from_millis(250);

/// The two files that have to exist before anything is spawned.
#[derive(Debug, Clone)]
pub struct ServerPaths {
    pub model: PathBuf,
    pub binary: PathBuf,
}

impl ServerPaths {
    /// Locates the model and the runtime beneath `app_data_dir`.
    ///
    /// Both files are checked here, before any process is created, so that a
    /// first run reports *which file is missing and where it should go* rather
    /// than an operating-system spawn failure or — worse — a server that
    /// starts, fails to open the model, and dies during the health wait, which
    /// would surface as a sixty-second timeout with no explanation.
    pub fn resolve(app_data_dir: &Path) -> Result<Self, CommandError> {
        let model = app_data_dir.join("models").join(MODEL_FILENAME);
        let binary = app_data_dir
            .join("runtime")
            .join("vulkan")
            .join(SERVER_BINARY);

        // The model first: it is the file a user is overwhelmingly more likely
        // to be missing, and reporting both at once buries the actionable one.
        if !model.is_file() {
            return Err(missing("model file", &model));
        }

        if !binary.is_file() {
            return Err(missing("inference runtime", &binary));
        }

        Ok(Self { model, binary })
    }
}

/// A running `llama-server`, and the port it was given.
#[derive(Debug)]
pub struct LlamaServer {
    child: Child,
    port: u16,
    /// Set once the child has been killed, so shutting down twice is a no-op
    /// rather than an error against a reaped handle.
    stopped: bool,
}

impl LlamaServer {
    /// Starts the server on a freshly allocated loopback port.
    ///
    /// Does **not** wait for it to be usable — see [`Self::wait_until_healthy`].
    /// Separated because spawning is synchronous and quick while waiting is
    /// neither, and the caller has to be able to record the PID in between.
    pub fn spawn(paths: &ServerPaths) -> Result<Self, CommandError> {
        let port = free_port()?;

        let mut command = Command::new(&paths.binary);
        command
            .args(server_arguments(&paths.model, port))
            // The child's output is dropped rather than inherited. Inheriting
            // it would print llama.cpp's very chatty loading log into whatever
            // console a packaged app happens to have, and piping it without
            // draining the pipe would deadlock the child once the buffer
            // filled — which, given how much it logs, would be during startup.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());

        no_console_window(&mut command);

        let child = command.spawn().map_err(|error| CommandError {
            kind: ErrorKind::Internal,
            message: format!(
                "could not start the inference runtime at {}: {error}",
                paths.binary.display()
            ),
        })?;

        Ok(Self {
            child,
            port,
            stopped: false,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The child's process id, for recording against [`PID_KEY`].
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Polls `/health` until the server reports ready, or gives up.
    ///
    /// The wait is real work, not a formality: `llama-server` binds its port
    /// and answers HTTP several seconds before the weights are loaded, so a
    /// caller that reported "running" the moment the process existed would send
    /// the first message into a server that refuses it.
    ///
    /// A child that exits during the wait is reported immediately rather than
    /// waited out. That is the shape of every real startup failure — a corrupt
    /// model, no Vulkan device — and making the user sit through the full
    /// timeout for it would hide the cause behind an unrelated symptom.
    pub async fn wait_until_healthy(&mut self) -> Result<(), CommandError> {
        let client = super::ChatClient::new(self.port);
        let deadline = Instant::now() + HEALTH_TIMEOUT;

        loop {
            if let Some(status) = self.exited() {
                return Err(CommandError {
                    kind: ErrorKind::Internal,
                    message: format!(
                        "the inference runtime stopped while loading the model ({status}); \
                         the model file may be corrupt or no GPU may be available"
                    ),
                });
            }

            if client.is_ready().await {
                return Ok(());
            }

            if Instant::now() >= deadline {
                // The half-started child is not left behind: the caller is
                // about to be told this failed, and a server nobody has a
                // handle to is exactly the VRAM leak this module exists to
                // prevent.
                self.shutdown();

                return Err(CommandError {
                    kind: ErrorKind::Internal,
                    message: format!(
                        "the inference runtime did not become ready within {} seconds on port {}",
                        HEALTH_TIMEOUT.as_secs(),
                        self.port
                    ),
                });
            }

            // Awaited rather than `std::thread::sleep`, which would hold a
            // runtime worker for the whole sixty-second wait.
            tokio::time::sleep(HEALTH_INTERVAL).await;
        }
    }

    /// The child's exit status, if it has already stopped.
    fn exited(&mut self) -> Option<std::process::ExitStatus> {
        // A failure to read the status is treated as "still running": the
        // alternative is reporting a startup failure because a wait call went
        // wrong, which would be a worse lie than one extra poll.
        self.child.try_wait().ok().flatten()
    }

    /// Stops the server. Calling this twice is not an error.
    ///
    /// Idempotent because there are three things that all want to stop it — the
    /// `chat_stop` command, a failed health wait, and app exit — and any two of
    /// them can happen in either order. Making the second one fail would turn
    /// "quit while stopping" into an error dialog on the way out.
    ///
    /// Killed rather than asked politely: `llama-server` has no shutdown
    /// endpoint, and the app has nothing in flight worth draining.
    pub fn shutdown(&mut self) {
        if self.stopped {
            return;
        }

        self.stopped = true;

        // An already-exited child reports an error here on some platforms.
        // That is the outcome being asked for, so it is not worth reporting.
        let _ = self.child.kill();

        // Reaped, so the child does not linger as a zombie holding its PID —
        // which would make the number recorded in `ui_state` keep matching a
        // process that is no longer using any VRAM.
        let _ = self.child.wait();
    }
}

impl Drop for LlamaServer {
    /// The last line of defence.
    ///
    /// Every deliberate path calls [`Self::shutdown`] first; this catches the
    /// ones that are not deliberate, such as a panic unwinding past the state
    /// that owns the handle.
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Asks the operating system for an unused loopback port.
///
/// Bind, read, drop. There is an unavoidable race — something else may take
/// the port between the drop and the child's own bind — but the alternative is
/// a fixed port, which fails *reliably* rather than rarely: a second copy of
/// the app, or a child the OS has not finished reaping, would collide every
/// single time, and the app would then be talking to a server it did not start.
///
/// Bound to 127.0.0.1 rather than 0.0.0.0 so the probe itself is never visible
/// on the network either.
pub fn free_port() -> Result<u16, CommandError> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| CommandError {
        kind: ErrorKind::Internal,
        message: format!("could not reserve a loopback port for the inference runtime: {error}"),
    })?;

    let port = listener
        .local_addr()
        .map_err(|error| CommandError {
            kind: ErrorKind::Internal,
            message: format!("could not read the reserved port: {error}"),
        })?
        .port();

    drop(listener);

    Ok(port)
}

/// The command line, as measured working on this machine.
///
/// A function rather than a literal so a test can assert on it — in particular
/// that `--host` is always `127.0.0.1`. An unauthenticated model server on
/// 0.0.0.0 would be reachable from every machine on the user's network, which
/// is the one line here with a security consequence.
///
/// `--cache-type-k`/`--cache-type-v` quantise the KV cache to 8 bits, which is
/// what keeps an 8192-token context inside VRAM alongside the weights.
pub fn server_arguments(model: &Path, port: u16) -> Vec<String> {
    vec![
        "--model".into(),
        model.display().to_string(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        port.to_string(),
        "--n-gpu-layers".into(),
        GPU_LAYERS.to_string(),
        "--ctx-size".into(),
        CONTEXT_SIZE.to_string(),
        "--cache-type-k".into(),
        "q8_0".into(),
        "--cache-type-v".into(),
        "q8_0".into(),
    ]
}

// --- orphan reaping ----------------------------------------------------------

/// Whether a PID recorded by a previous run should be killed on startup.
///
/// The decision is split out from the killing so it can be tested without a
/// real process, and because getting it wrong in the permissive direction is
/// dangerous: PIDs are recycled, so by the time the app restarts the number in
/// the database may well belong to the user's editor. Matching the image name
/// is what keeps this from being a random-process killer.
///
/// `None` — nothing running under that id — is the ordinary clean case and is
/// not an error.
pub fn should_reap(running_process_name: Option<&str>) -> bool {
    let Some(name) = running_process_name else {
        return false;
    };

    // Case-insensitive because Windows reports image names in whatever case
    // the file was created with, and the file did not come from this app.
    let name = name.to_ascii_lowercase();

    name == "llama-server" || name == "llama-server.exe"
}

/// Reads a PID back out of `ui_state`, where everything is text.
///
/// Anything unparseable reads as "nothing recorded" rather than as an error. A
/// corrupt value must not be able to stop the app from starting, and the worst
/// case of ignoring it is one leaked server the user can end themselves.
pub fn parse_pid(recorded: &str) -> Option<u32> {
    recorded.trim().parse().ok()
}

/// Kills an orphaned server left behind by a crashed run, if that is what the
/// recorded PID still is.
///
/// Returns whether anything was killed, which is what the caller logs.
pub fn reap_orphan(pid: u32) -> bool {
    if !should_reap(process_name(pid).as_deref()) {
        return false;
    }

    kill(pid)
}

/// The image name of the process with this id, if it is running.
///
/// Shelling out rather than taking a dependency on `sysinfo` or the Windows
/// crate: this runs exactly once per app launch, so a process spawn costs
/// nothing measurable, and it keeps a whole system-information crate out of a
/// tree that needs one string from it.
///
/// The flags are worth stating exactly, because getting them wrong fails
/// *silently*: an unrecognised switch makes `tasklist` print usage to stderr
/// and exit non-zero, this returns `None`, and reaping is then permanently
/// disabled with no symptom until a crash leaves a server behind. `/NH` and
/// `/FO CSV` are the whole set — there is no flag to suppress the "INFO: No
/// tasks are running" line, which is why the parse below looks for a quoted
/// field rather than trusting the first line.
pub(crate) fn process_name(pid: u32) -> Option<String> {
    #[cfg(windows)]
    {
        let mut command = Command::new("tasklist");
        command.args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"]);
        no_console_window(&mut command);

        let output = command.output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout);

        let first = text.lines().find(|line| line.starts_with('"'))?;
        Some(first.trim_start_matches('"').split('"').next()?.to_string())
    }

    #[cfg(not(windows))]
    {
        let output = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .ok()?;

        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if name.is_empty() {
            None
        } else {
            // `ps` may report a path; only the file name is compared.
            Some(
                Path::new(&name)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or(name),
            )
        }
    }
}

/// Ends a process this app does not hold a handle to.
fn kill(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/F", "/T"]);
        no_console_window(&mut command);

        command
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Keeps a spawned console program from flashing a window on Windows.
///
/// Without this every launch of the app pops a black console for
/// `llama-server`, and every startup reap pops two more for `tasklist` and
/// `taskkill`. A no-op everywhere else.
fn no_console_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        // CREATE_NO_WINDOW. Spelled out rather than pulled from the `windows`
        // crate to avoid a dependency for one constant.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

fn missing(what: &str, path: &Path) -> CommandError {
    CommandError {
        kind: ErrorKind::NotFound,
        message: format!(
            "no {what} at {} — put it there and try again",
            path.display()
        ),
    }
}
