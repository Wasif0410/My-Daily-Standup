//! Tests for the inference command layer.
//!
//! These exercise the recording and clearing of the child's PID, which is the
//! part of orphan reaping that does not need a real process. The decision
//! about *whether* to kill a given PID is tested next to the function that
//! makes it, in [`crate::inference::process`].

use super::inference::*;
use super::AppState;
use crate::inference::PID_KEY;

fn state() -> AppState {
    AppState::in_memory().expect("open in-memory state")
}

#[test]
fn nothing_is_reaped_when_no_pid_was_ever_recorded() {
    // The first launch on a machine. Absence is the normal case, not a
    // failure, and it must not be able to stop the app from starting.
    let state = state();

    assert_eq!(reap_orphaned_server(&state), None);
}

#[test]
fn a_recorded_pid_is_cleared_even_when_nothing_was_killed() {
    // The clean-shutdown-then-recycled-pid case: the number no longer names a
    // llama-server, so nothing is killed — but leaving it recorded would mean
    // inspecting a stale PID on every launch from here on, forever.
    let state = state();

    // PID 1 exists on every platform and is emphatically not a llama-server,
    // so this exercises the "found something, refused to kill it" path.
    state.set_ui_state(PID_KEY, "1").unwrap();

    assert_eq!(reap_orphaned_server(&state), None);
    assert_eq!(state.ui_state(PID_KEY).unwrap().as_deref(), Some(""));
}

#[test]
fn an_unparseable_pid_is_ignored_rather_than_failing_the_launch() {
    // A corrupt value must not be able to stop the app. The worst case of
    // ignoring it is one leaked server the user can end themselves; the worst
    // case of propagating it is an app that will not open.
    let state = state();
    state.set_ui_state(PID_KEY, "not-a-pid").unwrap();

    assert_eq!(reap_orphaned_server(&state), None);
}

#[test]
fn a_cleared_pid_reads_back_as_nothing_to_reap() {
    // What `chat_stop` leaves behind. An empty string parses to no PID, so the
    // next launch has nothing to inspect.
    let state = state();
    state.set_ui_state(PID_KEY, "4242").unwrap();
    state.set_ui_state(PID_KEY, "").unwrap();

    assert_eq!(reap_orphaned_server(&state), None);
}

#[test]
fn a_stopped_status_carries_neither_a_model_nor_a_port() {
    // The frontend branches on `running`; the other two must not linger with
    // values from a previous run, or a stopped model would still look
    // reachable. Read through `InferenceState` rather than the command, which
    // needs a `State` only a running app can hand out — the same reason the
    // rest of this boundary keeps its logic off the command functions.
    let inference = InferenceState::new(std::env::temp_dir());

    let status = inference.status();

    assert!(!status.running);
    assert_eq!(status.model, None);
    assert_eq!(status.port, None);
}

#[test]
fn shutting_down_a_state_that_never_started_anything_is_a_no_op() {
    // Idempotence, from the direction app exit takes: the window handler calls
    // this whether or not anyone ever pressed start.
    let inference = InferenceState::new(std::env::temp_dir());

    inference.shutdown();
    inference.shutdown();

    assert!(!inference.status().running);
}
