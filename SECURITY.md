# Security Policy

## Reporting a vulnerability

**Do not open a public issue for a security problem.**

Report it privately through GitHub Security Advisories:
[Report a vulnerability](https://github.com/Wasif0410/My-Daily-Standup/security/advisories/new)

Please include the affected version or commit, what an attacker could achieve, reproduction steps, and your assessment of severity. You will get an acknowledgement within a few days. If you would like credit in the fix release, say so and name how you would like to be credited.

## Threat model

This is a local desktop application that reads a user's personal notes and runs local language models. The assets worth protecting are:

1. **Vault contents** — personal notes, including folders the user explicitly excluded
2. **Voice recordings** — audio captured during a session
3. **Vault integrity** — the app must not corrupt or silently modify notes
4. **The user's machine** — via model output, downloaded binaries, or sidecar processes

## Security properties

These are commitments, not aspirations. A violation of any of them is a security bug and should be reported.

### No outbound connections

The application makes **no network requests** during normal operation. There is no telemetry, no crash reporting, no analytics, no update check, and no remote logging.

The only network activity is **downloading models and inference backends**, which is explicitly user-initiated, shows the source and size beforehand, and verifies a SHA-256 checksum before accepting the file.

If you observe any other outbound connection, that is a security bug. Please report it.

### Excluded folders never leave the machine

Folders the user excludes are never indexed, never searched, and never included in a model prompt. Exclusion is enforced in the indexing layer rather than the UI, so no code path can bypass it by querying differently.

Because the vault map is built only from indexed notes, excluded notes are absent by construction — the model is never informed they exist.

### Audio is not retained

Audio is processed in memory and the buffer is zeroed after transcription. No temporary audio file is written unless the user explicitly enables retention in settings. The microphone is opened only during an active session, never in the background.

### The vault is not modified without approval

No write to a user's vault occurs without them seeing the target file, the exact diff, and the reason, and then approving it. Review notes are written to dedicated folders. Original project notes are modified only for a single narrow case — ticking a source checkbox — which is off by default and verifies the line's text still matches before editing.

Files are backed up before modification, and a file changed externally since the proposal was created raises a conflict instead of being overwritten.

### Model output is never trusted

Structured output from the language model is validated by Rust against a strict schema before it can affect anything. File paths in model output are checked to be inside the vault and not excluded. No file operation, database write, or process invocation is ever driven by unvalidated model text.

The model runs as a local inference process with no tool access and no filesystem permissions of its own.

### Sidecar processes are contained

Bundled inference binaries (llama.cpp, whisper.cpp, Sherpa-ONNX):

- bind only to `127.0.0.1`, on a randomly selected port, and reject remote connections
- terminate when the session ends, on idle timeout, or when the app quits
- are reaped on next launch if orphaned by a crash
- are distributed with published checksums
- receive no unrestricted filesystem access

### Model weights are not executable

Model files are data consumed by the inference engine, not code. They are checksum-verified on download. Users may supply their own GGUF files, in which case the usual caution about running untrusted data through a parser applies.

## Supported versions

The project is pre-release. Only the latest tagged release receives security fixes.

| Version | Supported |
|---|---|
| Latest release | Yes |
| Everything else | No |
