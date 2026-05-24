# Third-Party Notices

OpenWhisper is built on top of excellent open-source projects. This document credits the projects whose code is included or whose substantial influence shaped this codebase.

## Primary upstream

**OpenWhisper is a fork of [Handy](https://github.com/cjpais/Handy)** by CJ Pais and contributors, MIT licensed. The initial code import was made on **2026-05-24** from Handy `v0.8.3`. Without Handy this project would not exist. Major thanks to CJ and the Handy community.

OpenWhisper extends Handy with the additional features documented in `../research/02-features-and-mechanics.md` (Wispr Flow parity targets) and `../research/09-auto-provisioning.md` (zero-touch model installer).

## Core libraries (Rust)

| Library | License | Project |
|---|---|---|
| [tauri](https://github.com/tauri-apps/tauri) | MIT / Apache-2.0 | Desktop application framework |
| [transcribe-rs](https://crates.io/crates/transcribe-rs) | MIT | Whisper + Parakeet + ONNX speech recognition |
| [whisper-rs / whisper.cpp](https://github.com/ggml-org/whisper.cpp) | MIT | Whisper inference |
| [vad-rs](https://github.com/cjpais/vad-rs) | MIT | Silero VAD wrapper |
| [cpal](https://github.com/RustAudio/cpal) | Apache-2.0 | Cross-platform audio I/O |
| [rdev](https://github.com/rustdesk-org/rdev) | MIT | Global keyboard hooks |
| [enigo](https://github.com/enigo-rs/enigo) | MIT | Synthetic keyboard/mouse input |
| [rodio](https://github.com/cjpais/rodio) | Apache-2.0 / MIT | Audio playback |
| [rubato](https://github.com/HEnquist/rubato) | MIT | Audio resampling |
| [rusqlite](https://github.com/rusqlite/rusqlite) | MIT | SQLite bindings |
| [handy-keys](https://crates.io/crates/handy-keys) | MIT | Mouse-button hotkeys |
| [ferrous-opencc](https://crates.io/crates/ferrous-opencc) | Apache-2.0 / MIT | Chinese variant conversion |
| [specta / tauri-specta](https://github.com/oscartbeaumont/specta) | MIT | Type-safe Rust↔TS bindings |

## ML models

| Model | License | Provider |
|---|---|---|
| Whisper (Tiny/Small/Medium/Large/Large-v3 Turbo) | MIT | OpenAI |
| Silero VAD v4 | MIT | Silero Team |
| Parakeet TDT v3 | CC-BY-4.0 | NVIDIA |
| Moonshine | MIT | Useful Sensors |
| Gemma 3 | Gemma license | Google |
| Phi-4 mini | MIT | Microsoft |
| Llama 3.2 | Llama license | Meta |
| Qwen 3 | Apache-2.0 | Alibaba |

Models are downloaded on demand from Hugging Face, never bundled in installer (except small defaults).

## On-device LLM frameworks

| Framework | License | Vendor |
|---|---|---|
| llama.cpp | MIT | Georgi Gerganov + contributors |
| Apple Foundation Models | Proprietary OS API | Apple (macOS 26+) |
| Phi Silica (Windows AI) | Proprietary OS API | Microsoft (Win 11 24H2+ Copilot+) |
| Ollama | MIT | Ollama team |

## Inspiration / reference implementations

OpenWhisper's product surface is heavily inspired by [Wispr Flow](https://wisprflow.ai) but shares no code with it. The product design decisions (hotkey layout, dictionary biasing, command mode, transforms, style presets) are documented in `../research/02-features-and-mechanics.md`.

Other open-source dictation projects studied during design:

- [VoiceInk](https://github.com/Beingpax/VoiceInk) — GPL-3.0, Mac-only Swift app with "Power Mode"
- [Whispering / Epicenter](https://github.com/EpicenterHQ/epicenter) — MIT, cross-platform Tauri+Svelte
- [Buzz](https://github.com/chidiwilliams/buzz) — MIT, file-transcription focused PyQt
- [nerd-dictation](https://github.com/ideasman42/nerd-dictation) — GPL-3.0, Linux Vosk-based

## Full attribution

The `LICENSE` file contains the MIT copyright notice required by Handy upstream. Cargo + Bun lockfiles enumerate all transitive dependencies with their licenses; run `cargo about generate` and `bun pm ls` for current inventories.

If you believe an upstream project deserves credit here and isn't listed, please open an issue or PR.
