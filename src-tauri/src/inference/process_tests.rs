//! Tests for the child-process side of inference.
//!
//! Nothing here spawns `llama-server`. Everything that decides *whether* to
//! spawn, *where* to look, and *what* to kill is a plain function precisely so
//! that CI — which has no GPU and no model — can still check the reasoning.
//! The one test that needs a real server is gated behind an environment
//! variable at the bottom of this file.

use std::fs;

use super::process::*;
use crate::commands::ErrorKind;

/// A temporary directory laid out the way the real app data directory is.
///
/// Returns the directory; the caller decides which of the two files to create,
/// because half the tests here are about a file being absent.
fn app_data_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("standup-inference-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the temporary app data directory");
    dir
}

fn write_model(dir: &std::path::Path) {
    let models = dir.join("models");
    fs::create_dir_all(&models).expect("create the models directory");
    fs::write(models.join(MODEL_FILENAME), b"not really a gguf").expect("write the model");
}

fn write_runtime(dir: &std::path::Path) {
    let runtime = dir.join("runtime").join("vulkan");
    fs::create_dir_all(&runtime).expect("create the runtime directory");
    fs::write(runtime.join(SERVER_BINARY), b"not really an exe").expect("write the runtime");
}

// --- port allocation ---------------------------------------------------------

#[test]
fn two_port_allocations_do_not_collide() {
    // A fixed port is the bug this exists to prevent: a second instance, or a
    // stale child the OS has not reaped yet, would silently bind-fail and the
    // app would then talk to somebody else's server.
    let first = free_port().expect("allocate a first port");
    let second = free_port().expect("allocate a second port");

    assert_ne!(
        first, second,
        "each allocation must ask the OS for a fresh ephemeral port"
    );
}

#[test]
fn an_allocated_port_is_outside_the_privileged_range() {
    let port = free_port().expect("allocate a port");

    assert!(
        port > 1024,
        "the OS hands out ephemeral ports, so {port} would mean we bound something reserved"
    );
}

// --- path resolution ---------------------------------------------------------

#[test]
fn resolving_paths_succeeds_when_both_files_are_present() {
    let dir = app_data_dir("both-present");
    write_model(&dir);
    write_runtime(&dir);

    let paths = ServerPaths::resolve(&dir).expect("resolve the paths");

    assert!(paths.model.ends_with(MODEL_FILENAME));
    assert!(paths.binary.ends_with(SERVER_BINARY));
}

#[test]
fn a_missing_model_names_the_exact_path_it_looked_for() {
    // "failed to spawn" tells the user nothing they can act on. The path does:
    // it is where they have to put the file.
    let dir = app_data_dir("no-model");
    write_runtime(&dir);

    let error = ServerPaths::resolve(&dir).expect_err("a missing model must be refused");

    assert_eq!(error.kind, ErrorKind::NotFound);
    assert!(
        error.message.contains(MODEL_FILENAME),
        "the message must name the file: {}",
        error.message
    );
    assert!(
        error.message.contains("models"),
        "the message must name the directory it searched: {}",
        error.message
    );
}

#[test]
fn a_missing_runtime_binary_names_the_exact_path_it_looked_for() {
    let dir = app_data_dir("no-runtime");
    write_model(&dir);

    let error = ServerPaths::resolve(&dir).expect_err("a missing runtime must be refused");

    assert_eq!(error.kind, ErrorKind::NotFound);
    assert!(
        error.message.contains(SERVER_BINARY),
        "the message must name the binary: {}",
        error.message
    );
}

#[test]
fn the_model_is_checked_before_the_runtime() {
    // Both missing is the first-run case. Reporting the model first is the
    // right order because it is the file the user is far more likely to be
    // missing, and reporting both at once buries the actionable one.
    let dir = app_data_dir("neither");

    let error = ServerPaths::resolve(&dir).expect_err("neither file present must be refused");

    assert!(
        error.message.contains(MODEL_FILENAME),
        "expected the model to be reported first: {}",
        error.message
    );
}

// --- the command line --------------------------------------------------------

#[test]
fn the_server_is_bound_to_loopback_and_never_to_all_interfaces() {
    // Binding 0.0.0.0 would put an unauthenticated model server on the user's
    // LAN. This is the single line of this PR with a security consequence.
    let args = server_arguments(std::path::Path::new("model.gguf"), 41234);

    let host = args
        .iter()
        .position(|a| a == "--host")
        .map(|i| args[i + 1].clone())
        .expect("--host must be passed explicitly rather than left to a default");

    assert_eq!(host, "127.0.0.1");
    assert!(
        !args.iter().any(|a| a == "0.0.0.0"),
        "nothing in the argument list may open the server beyond loopback"
    );
}

#[test]
fn the_server_is_told_the_allocated_port() {
    let args = server_arguments(std::path::Path::new("model.gguf"), 41234);

    let port = args
        .iter()
        .position(|a| a == "--port")
        .map(|i| args[i + 1].clone())
        .expect("--port must be passed");

    assert_eq!(port, "41234");
}

#[test]
fn the_measured_working_flags_are_all_passed() {
    // These are not decoration: the quantised KV cache is what keeps an 8k
    // context inside VRAM on this machine, and offloading every layer is what
    // makes the difference between seconds and minutes.
    let args = server_arguments(std::path::Path::new("model.gguf"), 1);

    for flag in [
        "--model",
        "--n-gpu-layers",
        "--ctx-size",
        "--cache-type-k",
        "--cache-type-v",
    ] {
        assert!(args.iter().any(|a| a == flag), "{flag} must be passed");
    }
}

// --- orphan reaping ----------------------------------------------------------

#[test]
fn a_recorded_pid_running_llama_server_is_reaped() {
    // The crash case this whole mechanism exists for: the app died, the child
    // did not, and several gigabytes of VRAM are still held by a process
    // nothing is talking to.
    assert!(should_reap(Some("llama-server.exe")));
    assert!(should_reap(Some("llama-server")));
}

#[test]
fn a_recorded_pid_belonging_to_something_else_is_left_alone() {
    // PIDs are recycled. By the time the app restarts, the number in the
    // database may well be the user's editor, and killing that would be a far
    // worse bug than leaking a model.
    assert!(!should_reap(Some("chrome.exe")));
    assert!(!should_reap(Some("explorer.exe")));
    assert!(!should_reap(Some("my-daily-standup.exe")));
}

#[test]
fn a_recorded_pid_that_no_longer_exists_is_left_alone() {
    // The ordinary clean-shutdown-then-crash-later case. Not an error.
    assert!(!should_reap(None));
}

#[test]
fn reaping_matches_the_process_name_case_insensitively() {
    // Windows reports image names in whatever case the file was created with.
    assert!(should_reap(Some("LLAMA-SERVER.EXE")));
}

#[test]
fn the_running_process_can_be_looked_up_by_its_own_pid() {
    // The lookup fails *silently* when it is wrong — a bad flag makes the
    // helper return None, reaping is then disabled forever, and there is no
    // symptom until a crash leaves a server holding VRAM. This caught exactly
    // that: an invalid `/NOTITLEINFO` switch that made tasklist exit non-zero.
    //
    // Asking about this very test binary is the one PID guaranteed to exist.
    let name = process_name(std::process::id()).expect("this process must be findable by its pid");

    assert!(
        name.to_ascii_lowercase().contains("my_daily_standup"),
        "expected the test binary's own image name, got {name:?}"
    );
}

#[test]
fn a_pid_that_names_nothing_running_is_reported_as_absent() {
    // The other half: absence must read as None rather than as some parse
    // artefact of the "no tasks are running" notice.
    //
    // Nothing on Windows or Linux runs as this PID; it is above every
    // configured maximum.
    assert_eq!(process_name(4_000_000_000), None);
}

#[test]
fn a_pid_records_and_parses_as_the_plain_number() {
    // The value lands in ui_state, which is text. Round-tripping it is what
    // startup reaping depends on, so a change of format here must break a test
    // rather than silently disable the reaper.
    assert_eq!(parse_pid("12345"), Some(12345));
    assert_eq!(parse_pid(""), None);
    assert_eq!(parse_pid("not-a-pid"), None);
}

// --- the gated end-to-end check ---------------------------------------------

/// Spawns the real server and asks it a real question.
///
/// Gated behind `STANDUP_INFERENCE_E2E=1` because it needs a GPU, a model file
/// and about ten seconds. CI has none of the three, and a test suite that is
/// slow is a test suite that stops being run.
#[test]
fn the_real_server_starts_answers_and_stops() {
    if std::env::var("STANDUP_INFERENCE_E2E").as_deref() != Ok("1") {
        return;
    }

    let dir = std::env::var("APPDATA")
        .map(|appdata| std::path::PathBuf::from(appdata).join("com.wasif.dailystandup"))
        .expect("APPDATA must name the app data directory for the end-to-end run");

    let paths = ServerPaths::resolve(&dir).expect("resolve the real paths");
    let mut server = LlamaServer::spawn(&paths).expect("spawn the real server");

    // The reaper's whole premise, checked against a real child rather than a
    // string literal: the recorded PID really does resolve to a process named
    // llama-server, so a crashed run really would be cleaned up.
    let pid = server.pid();
    assert!(
        should_reap(process_name(pid).as_deref()),
        "a live server must be recognised as reapable, got {:?}",
        process_name(pid)
    );

    tauri::async_runtime::block_on(server.wait_until_healthy())
        .expect("the server must become healthy");

    let client = crate::inference::ChatClient::new(server.port());
    let reply = tauri::async_runtime::block_on(
        client.complete("Be concise.", "Say the word ready and nothing else."),
    )
    .expect("the model must answer");

    assert!(
        !reply.content.trim().is_empty(),
        "reasoning is off, so content must carry the answer rather than being spent on thinking"
    );

    // Twice, because three separate paths want the server stopped and any two
    // of them can race.
    server.shutdown();
    server.shutdown();

    assert_eq!(
        process_name(pid),
        None,
        "shutdown must actually end the child rather than leaving it holding VRAM"
    );
}
