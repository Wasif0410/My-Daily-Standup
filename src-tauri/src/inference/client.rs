//! The HTTP conversation with `llama-server`.
//!
//! `llama-server` speaks the OpenAI chat-completions shape, so the request and
//! the reply below will look familiar. Nothing here knows how the server got
//! there or how it will be stopped — that is [`super::process`].
//!
//! Loopback only, and plaintext by construction: the client is built from a
//! reqwest with TLS compiled out (see `Cargo.toml`), so it cannot reach the
//! internet even if a URL somehow pointed there.

use std::time::Duration;

use serde_json::{json, Value};

use crate::commands::{CommandError, ErrorKind};

/// What the model is told before the user's message.
///
/// Two rules, and both earn their place:
///
/// - **Concise**, because the answer lands in a small desktop sticky note, not
///   a chat window with a scrollbar.
/// - **Never invent tasks**, because a planning assistant that makes up
///   commitments is worse than no assistant at all: the user cannot tell an
///   invented task from one they forgot, so every answer becomes something to
///   double-check against the board it was supposed to summarise.
///
/// Fixed here rather than assembled from the database. Building real context
/// out of the user's tasks is PR 25; doing it now would mean shipping a
/// prompt-assembly design before there is anything to test it against.
pub const SYSTEM_PROMPT: &str = "You are a planning assistant inside a desktop standup app. \
     Be concise: answer in a few short sentences, no preamble and no sign-off. \
     Never invent tasks, deadlines, or commitments that are not listed. \
     If you were not told something, say you do not know it.";

/// How long one completion may take before the request is abandoned.
///
/// Generous against the 1.0s measured for a short answer, because a long
/// question at 78 tok/s can legitimately run for a while, and the server is on
/// loopback so there is no network to blame for a stall.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// How long a single health poll waits. Short: the server either answers this
/// immediately or is not listening yet, and the caller is polling anyway.
const HEALTH_POLL_TIMEOUT: Duration = Duration::from_secs(2);

/// One answer from the model, with the token counts that came back with it.
#[derive(Debug, Clone)]
pub struct Completion {
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Talks to one `llama-server` on one port.
#[derive(Debug, Clone)]
pub struct ChatClient {
    http: reqwest::Client,
    base: String,
}

impl ChatClient {
    pub fn new(port: u16) -> Self {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            // No proxy. A user with a system proxy configured would otherwise
            // have their loopback traffic routed through it on some platforms,
            // which is the one way this client could produce a packet that
            // leaves the machine.
            .no_proxy()
            .build()
            .unwrap_or_default();

        Self {
            http,
            base: format!("http://127.0.0.1:{port}"),
        }
    }

    /// Whether the server is loaded and willing to answer.
    ///
    /// Anything other than a 200 carrying `{"status":"ok"}` is "not yet",
    /// including a refused connection: during startup the port is not even
    /// listening, and that is a normal state to poll through rather than an
    /// error to report.
    pub async fn is_ready(&self) -> bool {
        let Ok(response) = self
            .http
            .get(format!("{}/health", self.base))
            .timeout(HEALTH_POLL_TIMEOUT)
            .send()
            .await
        else {
            return false;
        };

        let Ok(body) = response.json::<Value>().await else {
            return false;
        };

        is_healthy(&body)
    }

    /// Sends one message and waits for one answer.
    ///
    /// No history: see [`chat_request`].
    pub async fn complete(&self, system: &str, user: &str) -> Result<Completion, CommandError> {
        let response = self
            .http
            .post(format!("{}/v1/chat/completions", self.base))
            .json(&chat_request(system, user))
            .send()
            .await
            .map_err(|error| CommandError {
                kind: ErrorKind::Internal,
                message: format!("could not reach the local model: {error}"),
            })?;

        let status = response.status();
        if !status.is_success() {
            // The body carries llama.cpp's own explanation — a context
            // overflow, say — which is far more useful than the status code.
            let detail = response.text().await.unwrap_or_default();

            return Err(CommandError {
                kind: ErrorKind::Internal,
                message: format!("the local model refused the request ({status}): {detail}"),
            });
        }

        let body = response
            .json::<Value>()
            .await
            .map_err(|error| CommandError {
                kind: ErrorKind::Internal,
                message: format!("the local model returned something that was not JSON: {error}"),
            })?;

        parse_completion(&body)
    }
}

/// Builds the request body for one exchange.
///
/// # Reasoning is off, deliberately
///
/// Qwen3 is a **hybrid thinking model**. Left to its default it writes its
/// chain of thought into `choices[0].message.reasoning_content` and leaves
/// `content` **empty**, spending the entire completion budget on thinking and
/// finishing with `finish_reason: "length"`. Measured on this machine, same
/// prompt, same build:
///
/// | reasoning | time  | answer               |
/// |-----------|-------|----------------------|
/// | on        | 13.9s | empty, cut at length |
/// | off       | 1.0s  | 78 tok/s, correct    |
///
/// `chat_template_kwargs.enable_thinking = false` is what turns it off, and it
/// has to go in the request because it is a *template* argument rather than a
/// sampling parameter — there is no server flag that fixes it after the fact.
///
/// The failure it prevents is nasty precisely because it does not look like a
/// failure: HTTP 200, a well-formed body, and a blank answer that reads to the
/// user as a hang. If this ever needs to become configurable, the default must
/// still be off.
///
/// # One exchange, no history
///
/// The system prompt and one user message, every time. This PR keeps no
/// conversation state: one message in, one reply out. Multi-turn is PR 26's
/// state machine, and a history assembled here would be a context window with
/// nothing managing its length.
pub fn chat_request(system: &str, user: &str) -> Value {
    json!({
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
        // One JSON object back. Streaming needs an event channel to the
        // frontend, which is not what this PR is proving.
        "stream": false,
        "chat_template_kwargs": { "enable_thinking": false },
    })
}

/// Reads the answer out of a chat-completions response.
///
/// An empty `content` is refused rather than returned, because the frontend
/// cannot tell a blank bubble from a hang — and an empty answer has exactly
/// one likely cause, which the error names so the next reader does not have to
/// rediscover it.
pub fn parse_completion(response: &Value) -> Result<Completion, CommandError> {
    let message = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .ok_or_else(|| CommandError {
            kind: ErrorKind::Internal,
            message: "the local model returned no completion".to_string(),
        })?;

    let content = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if content.is_empty() {
        return Err(CommandError {
            kind: ErrorKind::Internal,
            message: "the local model returned an empty answer — it most likely spent the whole \
                      budget on reasoning, which means enable_thinking was not switched off"
                .to_string(),
        });
    }

    // Token counts are a diagnostic, not the answer. A server build that omits
    // the usage block must not cost the user their reply.
    let usage = |field: &str| -> u32 {
        response
            .get("usage")
            .and_then(|usage| usage.get(field))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32
    };

    Ok(Completion {
        content,
        prompt_tokens: usage("prompt_tokens"),
        completion_tokens: usage("completion_tokens"),
    })
}

/// Whether a `/health` body says the model is loaded.
///
/// The status field, not merely the fact of a reply: `llama-server` answers
/// this endpoint with `{"status":"loading model"}` for several seconds after
/// it starts listening, and treating that as ready is what would send the
/// user's first message into a server that refuses it.
pub fn is_healthy(body: &Value) -> bool {
    body.get("status").and_then(Value::as_str) == Some("ok")
}
