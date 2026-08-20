# My Daily Standup

> Turn your long-term Obsidian goals into realistic monthly commitments, weekly milestones, and daily actions through private voice standups that run entirely on your computer.

**Status: under construction.** Nothing is runnable yet — the first milestone (`v0.1.0`, a working non-AI desktop planner) is in progress.

## What it is

A local-first desktop planning companion. It reads your Obsidian vault for long-term goals and projects, runs a short standup with you by voice, helps you commit to a realistic set of actions for today, and keeps those commitments visible as lightweight sticky-note windows on your desktop.

Everything runs on your machine. No account, no cloud, no telemetry.

## The core idea

The boards are always available, but AI inference is always on demand.

```
Always running (under 1 GB, no model memory):
├── Sticky-note windows
├── System tray
├── Local task database
└── Vault file watcher

Only during a session, then fully terminated:
├── llama.cpp        (local language model)
├── whisper.cpp      (speech to text)
└── Sherpa-ONNX      (text to speech)
```

The sticky notes stay on your desktop all day. The models start when you begin a standup and shut down when you finish, so several gigabytes of RAM and VRAM are not held hostage the rest of the time.

## Planning hierarchy

```
Long-term Obsidian goals
        ↓
Monthly commitments
        ↓
Weekly milestones
        ↓
Daily actions
        ↓
Completed work and reflections
        ↓
Weekly/monthly progress written back to Obsidian
```

## Roadmap

| Milestone | What it unlocks |
|---|---|
| `v0.1.0` | A full non-AI desktop planner with persistent sticky boards |
| `v0.2.0` | Promote real tasks out of your Obsidian vault onto the boards |
| `v0.3.0` | Typed local-LLM standup that proposes and saves a daily plan |
| `v0.4.0` | The whole standup by voice — MVP feature-complete |
| `v1.0.0` | Evening, weekly, and monthly reviews with approved Obsidian writeback |

## Principles

- **Local first.** All core functionality works offline once models are downloaded.
- **Obsidian is the source of truth.** The app stores operational state; your notes stay yours.
- **Nothing is written to your vault without your approval.** You see the file, the diff, and the reason first.
- **Deterministic app, assisted by AI.** Rust owns state, files, dates, and process lifecycle. The model helps with conversation, summarization, and drafting — never with calculations or file operations.
- **Useful without the AI.** Every board and task interaction works with no model loaded.

## Stack

Tauri 2 · React + TypeScript · Rust · SQLite · llama.cpp · whisper.cpp · Sherpa-ONNX

## Hardware

One installer, one binary. The profile is detected at runtime and can be overridden in settings — there is no separate "laptop build."

| Profile | Minimum | Model | Session footprint |
|---|---|---|---|
| Lightweight | 8 GB RAM, no dGPU | Qwen3-1.7B | ~2 GB RAM |
| Balanced | 16 GB RAM, iGPU or entry dGPU | Qwen3-4B | ~3.7 GB RAM |
| High Quality | 16 GB RAM + ≥8 GB VRAM | Qwen3-8B | ~6 GB VRAM + ~1 GB RAM |

VRAM is what gates the High Quality profile, not system RAM — when the model is fully offloaded, the weights and KV cache live on the GPU.

With the boards open and no session running, the app uses **under 1 GB and no model memory at all.**

## Documentation

- [Product and technical specification](docs/spec.md) — the full design
- [PR sequence and contribution guide](docs/superpowers/plans/2026-08-20-pr-sequence.md) — how the project is being built, PR by PR
- [Contributing](CONTRIBUTING.md) — setup, workflow, and what a change must satisfy
- [Security policy](SECURITY.md) — threat model and privacy commitments

## Contributing

The project is being built in a strict sequence of pull requests. Read the
[PR sequence](docs/superpowers/plans/2026-08-20-pr-sequence.md) before proposing work —
most of the roadmap is already scheduled.

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup and workflow, and note that **Windows builds
require the Visual Studio Build Tools with the "Desktop development with C++" workload.**

## License

[MIT](LICENSE)
