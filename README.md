# OpenWhisper

**100% free, local-first, Wispr-Flow-class voice dictation for Mac, Windows, and Linux.**

Hold a hotkey anywhere on your computer, speak naturally, and clean formatted text is pasted at your cursor. Your voice never leaves your machine. Zero subscription. Zero telemetry by default.

> **Status:** Phase 0 — forked from [Handy](https://github.com/cjpais/Handy) (MIT) on 2026-05-24. Active development toward full Wispr Flow feature parity.

## Why OpenWhisper

[Wispr Flow](https://wisprflow.ai) is the polished commercial voice-dictation app loved by founders, devs, lawyers, and creators. It costs $12–15/month and **transmits every word you speak to its servers**. There is no offline mode.

OpenWhisper delivers the same UX with one fundamental difference: **everything runs locally on your machine**. Speech recognition (Whisper), AI cleanup (Gemma / Phi / Apple Foundation Models / Phi Silica), dictionary biasing — all on-device.

## Key features (target — see [roadmap](./research/06-architecture-and-roadmap.md))

- Push-to-talk + hands-free + mouse-button hotkeys
- Whisper Large-v3 Turbo + Parakeet V3 (CPU-optimized) with auto-selection per system
- AI auto-cleanup (4 levels) via Apple Foundation Models / Phi Silica / bundled llama.cpp
- Personal Dictionary with auto-learn + `initial_prompt` biasing
- Snippets / voice shortcuts (text expansion)
- Command Mode — highlight text, speak edit instruction, get replacement
- Per-app Style presets (Formal / Casual / Very Casual / Excited)
- Vibe Coding — variable recognition + file tagging in Cursor / VS Code / Windsurf
- Transforms — hotkey-bound post-dictation AI rewrites
- Insights / Voice Profile dashboard (100% local data)
- Scratchpad / Notes (markdown editor with image support)
- 100+ languages with auto-detect per session
- **Zero-touch auto-provisioning** — installer detects your hardware and downloads the optimal model stack ($0 cost)

## Built on the shoulders of giants

OpenWhisper is a fork of [Handy](https://github.com/cjpais/Handy) (MIT, 22k stars). Massive credit to CJ Pais and the Handy contributors — without their work, this project would not exist. OpenWhisper extends Handy with a Wispr-Flow-class feature set targeted at productivity workers.

Core libraries:
- [whisper-rs](https://github.com/tazz4843/whisper-rs) / [transcribe-rs](https://crates.io/crates/transcribe-rs) — Whisper + Parakeet inference
- [cpal](https://github.com/RustAudio/cpal) — cross-platform audio
- [vad-rs](https://github.com/cjpais/vad-rs) — Silero VAD
- [rdev](https://github.com/rustdesk-org/rdev) — global hotkeys
- [Tauri 2](https://tauri.app) — desktop shell
- [llama.cpp](https://github.com/ggml-org/llama.cpp) / [Ollama](https://ollama.com) — local LLM cleanup

## Build from source

See [BUILD.md](./BUILD.md). Short version on Windows:

```bash
# Prerequisites: Rust (rustup), Bun, Visual Studio 2022 C++ Build Tools
git clone https://github.com/openwhisper/openwhisper.git
cd openwhisper
bun install
bun run tauri dev
```

## License

MIT — see [LICENSE](./LICENSE). Includes Handy's original MIT copyright as required.

## Roadmap and research

The full research, decisions log, and 16-week build plan live in [`research/`](../research/):

- `00-INDEX.md` — start here
- `08-decisions.md` — locked architecture decisions
- `09-auto-provisioning.md` — zero-touch model installer spec
- `06-architecture-and-roadmap.md` — 9-phase build plan
