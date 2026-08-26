//! Tests for the HTTP conversation with `llama-server`.
//!
//! The request body and the reply parser are plain functions over
//! `serde_json::Value`, so the shape of both can be pinned down without a
//! server, a model, or a GPU. The measurements that motivate these assertions
//! are recorded on the functions themselves.

use serde_json::json;

use super::client::*;
use crate::commands::ErrorKind;

// --- the request -------------------------------------------------------------

#[test]
fn reasoning_is_disabled_on_every_request() {
    // The most important assertion in this module. Qwen3 is a hybrid thinking
    // model: left to itself it spends the entire token budget in
    // `reasoning_content` and returns an empty `content` with finish_reason
    // "length". Measured on this machine: 13.9s and no answer with thinking
    // on, 1.0s at 78 tok/s with it off. If this assertion ever fails, the
    // feature is broken even though every request still returns 200.
    let body = chat_request("be concise", "what is on today?");

    assert_eq!(
        body["chat_template_kwargs"]["enable_thinking"],
        json!(false),
        "reasoning must default to OFF"
    );
}

#[test]
fn the_request_carries_the_system_prompt_first() {
    let body = chat_request("be concise", "what is on today?");

    let messages = body["messages"]
        .as_array()
        .expect("messages must be a list");

    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[0]["content"], "be concise");
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "what is on today?");
}

#[test]
fn the_request_carries_exactly_one_exchange() {
    // No conversation history in this PR — one message in, one reply out.
    // Multi-turn is PR 26's state machine, and pretending to support it here
    // would mean shipping a context window nothing manages.
    let body = chat_request("system", "user");

    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
}

#[test]
fn the_request_does_not_ask_for_streaming() {
    // A single blocking call returns one JSON object. Streaming would need an
    // event channel to the frontend, which is not what this PR is proving.
    assert_eq!(chat_request("s", "u")["stream"], json!(false));
}

// --- the reply ---------------------------------------------------------------

fn ok_response(content: &str) -> serde_json::Value {
    json!({
        "choices": [{ "message": { "content": content }, "finish_reason": "stop" }],
        "usage": { "prompt_tokens": 31, "completion_tokens": 12 }
    })
}

#[test]
fn a_normal_reply_yields_its_content_and_token_counts() {
    let completion = parse_completion(&ok_response("Two tasks today.")).expect("parse the reply");

    assert_eq!(completion.content, "Two tasks today.");
    assert_eq!(completion.prompt_tokens, 31);
    assert_eq!(completion.completion_tokens, 12);
}

#[test]
fn a_reply_whose_content_went_to_reasoning_is_reported_rather_than_returned_empty() {
    // This is what a regression on `enable_thinking` looks like from the
    // outside: HTTP 200, a well-formed body, and an empty answer. Returning
    // that to the frontend would render as a blank bubble and read as a hang.
    let response = json!({
        "choices": [{
            "message": { "content": "", "reasoning_content": "Let me think about this..." },
            "finish_reason": "length"
        }],
        "usage": { "prompt_tokens": 31, "completion_tokens": 2048 }
    });

    let error = parse_completion(&response).expect_err("an empty answer must not be returned");

    assert_eq!(error.kind, ErrorKind::Internal);
    assert!(
        error.message.contains("reasoning"),
        "the message must point at the cause: {}",
        error.message
    );
}

#[test]
fn a_reply_with_no_choices_is_an_error_rather_than_a_panic() {
    let error = parse_completion(&json!({ "choices": [] }))
        .expect_err("a reply with no choices must be refused");

    assert_eq!(error.kind, ErrorKind::Internal);
}

#[test]
fn a_reply_missing_its_usage_block_still_returns_the_answer() {
    // Token counts are a diagnostic. Losing them must not lose the answer.
    let response = json!({ "choices": [{ "message": { "content": "Done." } }] });

    let completion = parse_completion(&response).expect("parse the reply");

    assert_eq!(completion.content, "Done.");
    assert_eq!(completion.prompt_tokens, 0);
}

#[test]
fn content_is_trimmed_of_surrounding_whitespace() {
    // Chat templates habitually emit a leading newline; the frontend should
    // not have to know that.
    let completion = parse_completion(&ok_response("\n  Two tasks today.\n")).unwrap();

    assert_eq!(completion.content, "Two tasks today.");
}

// --- the health probe --------------------------------------------------------

#[test]
fn the_server_is_ready_only_once_it_reports_ok() {
    // `llama-server` answers /health while it is still loading weights, so the
    // status field is the readiness signal, not the fact that it replied.
    assert!(is_healthy(&json!({ "status": "ok" })));
    assert!(!is_healthy(&json!({ "status": "loading model" })));
    assert!(!is_healthy(&json!({})));
}

// --- the system prompt -------------------------------------------------------

#[test]
fn the_system_prompt_forbids_inventing_tasks() {
    // A planning assistant that makes up commitments is worse than none: the
    // user cannot tell an invented task from a forgotten one.
    let prompt = SYSTEM_PROMPT.to_ascii_lowercase();

    assert!(prompt.contains("concise"));
    assert!(
        prompt.contains("invent") || prompt.contains("make up"),
        "the prompt must forbid inventing tasks: {SYSTEM_PROMPT}"
    );
}
