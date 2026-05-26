# OpenWhisper

**100% local, 100% free, Wispr-Flow-class voice dictation for Mac, Windows, and Linux.**

Hold a hotkey anywhere on your computer, speak naturally, and clean formatted text is pasted at your cursor. Your voice never leaves your machine. Zero subscription. Zero telemetry. **Zero cloud, zero API.**

## Privacy guarantee

OpenWhisper performs **all** transcription and AI cleanup on your own machine:

- **Speech recognition (Whisper / Parakeet / Moonshine)** — runs locally via `whisper.cpp` / `transcribe-rs`. The audio never hits the network.
- **AI cleanup (the LLM pass)** — runs locally via one of:
  - Apple Foundation Models (macOS 26+ on Apple Silicon) — OS-provided, on-device
  - Phi Silica via Windows AI APIs (Windows 11 24H2+ on Copilot+ PCs) — OS-provided, on-device
  - Bundled `llama.cpp` sidecar with Gemma 3 / Phi-4 mini / Llama 3.2 weights — runs on `127.0.0.1`
  - Optional Ollama (if you've already installed it) — runs on `127.0.0.1`
- **No cloud LLM providers ship with OpenWhisper.** The Handy upstream's OpenAI / Anthropic / Groq / Cerebras / Z.AI / OpenRouter / Bedrock-Mantle integrations were **removed** in Phase 1.10. Existing settings files that reference them are automatically migrated to local defaults on first launch.
- **Hard-coded URL whitelist.** The LLM client (`src-tauri/src/llm_client.rs::validate_local_url`) rejects any base URL that is not `127.0.0.1`, `::1`, `localhost`, `apple-intelligence://`, or `phi-silica://`. A bug in settings cannot leak transcripts off the machine — the network call itself is blocked.

The only network access OpenWhisper makes is for:
1. Downloading model weights from Hugging Face on first run (one-time, no user data sent).
2. Checking for app updates (opt-out in Settings).

If you want a fully air-gapped install, pre-place the Whisper + GGUF model files into `~/.local/share/openwhisper/models/` (Linux), `~/Library/Application Support/openwhisper/models/` (macOS), or `%APPDATA%\openwhisper\models\` (Windows) before first launch, and disable update checks.

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
