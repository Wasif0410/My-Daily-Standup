//! The local model, across the IPC boundary.
//!
//! Four commands and one piece of state. The state is the running child, held
//! here rather than on [`super::AppState`] because it is not storage: it is an
//! operating-system resource with a lifetime, and the two are worth keeping
//! separable — the database opens on every launch, the model on almost none.
//!
//! No conversation history lives anywhere in this PR. `chat_send` is one
//! message in, one reply out; PR 26 adds the state machine that remembers.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use tauri::State;

use super::{AppState, CommandError, ErrorKind};
use crate::inference::{ChatClient, LlamaServer, ServerPaths, PID_KEY};

/// Whether the model is up, and where.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStatus {
    pub running: bool,
    /// The model file's name, for the frontend to show. `None` when stopped.
    pub model: Option<String>,
    pub port: Option<u16>,
}

impl ChatStatus {
    fn stopped() -> Self {
        Self {
            running: false,
            model: None,
            port: None,
        }
    }
}

/// One answer, with what it cost.
///
/// The counts and the timing are here from the start because they are how a
/// regression on reasoning gets noticed: an answer that suddenly costs two
/// thousand completion tokens and fourteen seconds is the failure described on
/// [`crate::inference::client::chat_request`], and without these numbers it
/// just looks slow.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatReply {
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub elapsed_ms: u64,
}

/// The running server, if there is one.
///
/// A mutex around an `Option` rather than something cleverer because every
/// operation on it is short — take a port, take a handle, put one back — while
/// the long waits (loading weights, generating an answer) happen outside the
/// lock on purpose. Holding it across a sixty-second health wait would make
/// `chat_status` hang for a minute.
pub struct InferenceState {
    server: Mutex<Option<LlamaServer>>,
    app_data_dir: PathBuf,
}

impl InferenceState {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            server: Mutex::new(None),
            app_data_dir,
        }
    }

    /// The port of the running server, if any.
    fn port(&self) -> Option<u16> {
        self.server.lock().ok()?.as_ref().map(LlamaServer::port)
    }

    /// What [`chat_status`] reports.
    ///
    /// The logic lives here rather than on the command so a test can reach it
    /// without a Tauri runtime, exactly like the rest of this boundary.
    pub fn status(&self) -> ChatStatus {
        match self.port() {
            Some(port) => ChatStatus {
                running: true,
                model: Some(crate::inference::process::MODEL_FILENAME.to_string()),
                port: Some(port),
            },
            None => ChatStatus::stopped(),
        }
    }

    /// Stops the server if one is running. Safe to call twice, and safe to
    /// call on the way out of the app.
    pub fn shutdown(&self) {
        let Ok(mut guard) = self.server.lock() else {
            // A poisoned lock means a panic happened while the handle was
            // held. The `Drop` on `LlamaServer` still kills the child when the
            // state is dropped, so there is nothing better to do here than
            // leave it alone rather than panic again on the way out.
            return;
        };

        if let Some(mut server) = guard.take() {
            server.shutdown();
        }
    }
}

/// Whether the model is currently up.
///
/// Synchronous: it takes a lock and reads two fields. Nothing here blocks.
#[tauri::command]
pub fn chat_status(inference: State<'_, InferenceState>) -> Result<ChatStatus, CommandError> {
    Ok(inference.status())
}

/// Starts the model and waits until it can answer.
///
/// **Async on purpose.** This blocks for seconds — spawning a process and then
/// polling it while it loads two and a half gigabytes of weights. A
/// synchronous Tauri command runs *on the event loop*, so this would freeze
/// every window in the app for the whole load. This codebase has already paid
/// for that lesson once: a synchronous command building a window deadlocked
/// the loop and produced a blank rectangle. See [`crate::windows::dispatch`]
/// for the full account.
///
/// Starting an already-running server is not an error — it reports the running
/// one. The frontend can call this on a button press without first checking.
#[tauri::command]
pub async fn chat_start(
    state: State<'_, AppState>,
    inference: State<'_, InferenceState>,
) -> Result<ChatStatus, CommandError> {
    if let Some(port) = inference.port() {
        return Ok(ChatStatus {
            running: true,
            model: Some(crate::inference::process::MODEL_FILENAME.to_string()),
            port: Some(port),
        });
    }

    // Both files are checked before anything is spawned, so a first run names
    // the file it wants rather than failing inside the operating system.
    let paths = ServerPaths::resolve(&inference.app_data_dir)?;
    let mut server = LlamaServer::spawn(&paths)?;

    // Recorded *before* the health wait, not after. The crash this guards
    // against is far more likely during the load than after it — that is when
    // the machine is under the most memory pressure — and a PID written only
    // on success would miss exactly those cases.
    state.set_ui_state(PID_KEY, &server.pid().to_string())?;

    if let Err(error) = server.wait_until_healthy().await {
        // `wait_until_healthy` has already killed the child, so the recorded
        // PID now points at nothing. Clearing it keeps the next launch from
        // inspecting a number that has been recycled.
        let _ = state.set_ui_state(PID_KEY, "");
        return Err(error);
    }

    let port = server.port();

    let mut guard = inference.server.lock().map_err(poisoned)?;
    *guard = Some(server);

    Ok(ChatStatus {
        running: true,
        model: Some(
            paths
                .model
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| crate::inference::process::MODEL_FILENAME.to_string()),
        ),
        port: Some(port),
    })
}

/// Asks the model one question and returns its answer.
///
/// **No conversation history.** One message in, one reply out; the model is
/// told the system prompt and this message and nothing else. Multi-turn is PR
/// 26's state machine. Context built from the user's actual tasks is PR 25 —
/// this command deliberately does not touch the database.
///
/// **Async on purpose**, for the same reason as [`chat_start`]: generation
/// takes about a second at best and must not run on the event loop.
///
/// Does not start the server implicitly. A message that silently spent a
/// minute loading a model would look like a hang; the frontend calls
/// [`chat_start`] and can show that it is loading.
#[tauri::command]
pub async fn chat_send(
    inference: State<'_, InferenceState>,
    message: String,
) -> Result<ChatReply, CommandError> {
    if message.trim().is_empty() {
        return Err(CommandError {
            kind: ErrorKind::InvalidInput,
            message: "there is nothing to send".to_string(),
        });
    }

    let Some(port) = inference.port() else {
        return Err(CommandError {
            kind: ErrorKind::Internal,
            message: "the local model is not running — start it first".to_string(),
        });
    };

    // The client is built from the port rather than borrowed from the state,
    // so the mutex is not held across the request. A generation can run for a
    // minute, and `chat_status` must stay answerable throughout.
    let client = ChatClient::new(port);

    let started = Instant::now();
    let completion = client
        .complete(crate::inference::client::SYSTEM_PROMPT, &message)
        .await?;

    Ok(ChatReply {
        content: completion.content,
        prompt_tokens: completion.prompt_tokens,
        completion_tokens: completion.completion_tokens,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/// Stops the model and frees its VRAM.
///
/// Stopping something that is not running is not an error. Three things can
/// want the server stopped — this command, a failed start, and app exit — and
/// making the second call fail would turn an ordinary race into an error the
/// user sees.
#[tauri::command]
pub fn chat_stop(
    state: State<'_, AppState>,
    inference: State<'_, InferenceState>,
) -> Result<ChatStatus, CommandError> {
    inference.shutdown();

    // Cleared only on a clean stop. If the app dies instead, the value stays
    // and the next launch reaps what it names.
    state.set_ui_state(PID_KEY, "")?;

    Ok(ChatStatus::stopped())
}

/// Kills a server left behind by a run that crashed.
///
/// Called once at startup. An app that died without stopping its child leaves
/// several gigabytes of VRAM held by a process nothing is talking to, and the
/// user's only clue is that their next model load fails for want of memory.
///
/// The recorded PID is *not* trusted on its own: PIDs are recycled, so it is
/// checked against the running process's image name first. See
/// [`crate::inference::process::should_reap`].
pub fn reap_orphaned_server(state: &AppState) -> Option<u32> {
    let recorded = state.ui_state(PID_KEY).ok().flatten()?;
    let pid = crate::inference::process::parse_pid(&recorded)?;

    // Cleared whether or not anything was killed: either way the number has
    // been dealt with, and leaving it would mean re-inspecting a stale PID on
    // every launch from here on.
    let _ = state.set_ui_state(PID_KEY, "");

    crate::inference::process::reap_orphan(pid).then_some(pid)
}

fn poisoned<T>(_: T) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: "the inference lock was poisoned by an earlier panic".to_string(),
    }
}
