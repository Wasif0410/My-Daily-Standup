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

pub mod client;
pub mod process;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod process_tests;

pub use client::{ChatClient, Completion};
pub use process::{LlamaServer, ServerPaths, PID_KEY};
