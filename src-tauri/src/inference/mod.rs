//! The local language model.
//!
//! The app never calls a hosted model. It spawns `llama-server` — the HTTP
//! server that ships with llama.cpp — as a child process bound to loopback,
//! talks to it over plain HTTP, and kills it again. Everything the user types
//! and everything the model answers stays inside this machine, which is what
//! makes the promise in `SECURITY.md` enforceable rather than aspirational.
//!
//! Split three ways on purpose:
//!
//! - [`process`] owns the child: where its files live, which port it got,
//!   whether it is ready, and how it dies.
//! - [`client`] owns the conversation: the request shape and the reply shape,
//!   with no knowledge of how the server got there.
//! - The command layer in [`crate::commands::inference`] owns the state
//!   machine that joins them.
//!
//! Both halves are written so that the parts worth testing — port allocation,
//! path resolution, the orphan-reaping decision, the request body, the reply
//! parser — are plain functions that need neither a GPU nor a running server.

//! A third concern joined the two above in PR 25: what the model is *told*.
//!
//! - [`map`] is tier 1, the commitment map — what exists.
//! - [`context`] is tiers 2 and 3, the ranked detail and the on-demand
//!   subtree — what is urgent, and what can be asked for.
//! - [`prompt`] is the template layer that wraps them.
//!
//! None of the three knows anything about a running server, and none of them
//! needs one to be tested: they turn rows into text, and text is checkable.

pub mod client;
pub mod context;
pub mod map;
pub mod process;
pub mod prompt;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod map_tests;
#[cfg(test)]
mod process_tests;
#[cfg(test)]
mod prompt_tests;

pub use client::{ChatClient, Completion};
pub use context::{
    build_context, estimate_tokens, fetch_task_subtree, ContextItem, SessionContext, SessionKind,
    TaskSubtree, TokenBudget,
};
pub use map::{build_map, CommitmentMap};
pub use process::{LlamaServer, ServerPaths, PID_KEY};
pub use prompt::{load_template, render_prompt, render_session, PromptError};
