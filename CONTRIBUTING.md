# Contributing

Thanks for your interest. This document covers how to set up the project, how work is sequenced, and what a change has to satisfy before it merges.

## Project status

The project is being built in a strict sequence of pull requests. The full plan lives in
[`docs/superpowers/plans/2026-08-20-pr-sequence.md`](docs/superpowers/plans/2026-08-20-pr-sequence.md) —
read it before proposing work, since most of what you might want to add is probably already
scheduled, and PRs land one at a time in dependency order.

## Prerequisites

You need all four of these before the app will build.

| Tool | Version | Notes |
|---|---|---|
| [Node.js](https://nodejs.org) | 20 LTS or newer | `node --version` |
| [Rust](https://rustup.rs) | stable | `rustc --version` |
| **Visual Studio Build Tools** | 2022 | **Windows only, and required.** Install the "Desktop development with C++" workload — Tauri cannot link without the MSVC toolchain. |
| [Git](https://git-scm.com) | 2.40+ | |

Verify your setup:

```bash
node --version && rustc --version && cargo --version
```

### First build

```bash
npm install
npm run tauri dev
```

The first Rust build compiles the entire dependency tree and takes several minutes. Later builds are incremental and fast.

## Branching

`main` is always releasable and is protected — no direct pushes. One branch per pull request, cut fresh from the latest `main`:

```
<type>/pr-<NN>-<short-slug>
```

Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`.

Examples: `feat/pr-11-priority-board`, `ci/pr-03-pipeline`, `fix/pr-23-orphan-reap`.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/). Commit often — ideally once per red → green → refactor cycle, not once per PR.

```
feat(boards): persist sticky window geometry on move
test(obsidian): add fixture for nested checkbox parsing
fix(inference): reap orphaned llama.cpp process on crash
docs(readme): correct the High Quality profile RAM floor
```

## Tests come first

This project is built test-first. For any behavior change:

1. Write the failing test.
2. Run it and watch it fail for the reason you expect.
3. Write the minimum code that makes it pass.
4. Run it again and watch it pass.
5. Commit.

A PR whose tests were clearly written after the implementation will be asked to justify itself. Tests that cannot fail are worse than no tests.

Run everything locally before opening a PR:

```bash
npm run lint && npm run typecheck && npm test && cargo test --manifest-path src-tauri/Cargo.toml
```

## Non-negotiable constraints

These come from the product spec and hold for every contribution, regardless of how good the code is:

- **No telemetry, no remote logging, no accounts.** Not behind a flag. Absent.
- **No network calls in core functionality.** The app must work fully offline once models are downloaded. Model and backend downloads are the sole exception, and they are explicit and user-initiated.
- **Board interactions must never start a model.** Completing, editing, moving, or adding a task while no session is running must leave inference processes untouched. This is the project's central architectural promise.
- **Never write to a user's Obsidian vault without explicit per-change approval.** They see the file, the diff, and the reason first.
- **Excluded vault folders are never indexed, searched, or included in a model prompt.** Enforce this in the data layer, not the UI.
- **Never act on unvalidated model output.** All structured output passes a Rust-side validator before it touches the database or the filesystem.
- **Rust owns state, files, dates, and process lifecycle.** The model contributes language, not decisions or arithmetic.

## Definition of done

Before requesting review:

- [ ] Tests written before the implementation, and they fail without it
- [ ] `npm test` and `cargo test` pass locally
- [ ] `npm run lint` and `cargo clippy -- -D warnings` are clean
- [ ] No `TODO`, commented-out code, or placeholder strings
- [ ] The PR touches only its stated scope — unrelated cleanup goes in its own PR
- [ ] Docs updated if user-visible behavior changed
- [ ] The deliverable is demonstrable end-to-end, not merely unit-tested

## Reporting bugs

Open an issue using the bug report template. Include your OS version, hardware profile (RAM and VRAM), and which milestone you are running.

**Never paste vault contents into an issue.** If a bug involves a specific note, reduce it to a minimal synthetic example first.

## Security

Do not open a public issue for a security problem. See [SECURITY.md](SECURITY.md).

## Code of conduct

Participation is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).
