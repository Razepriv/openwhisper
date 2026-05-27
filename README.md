# OpenWhisper

**100% local, 100% free, Wispr-Flow-class voice dictation for Mac, Windows, and Linux.**

Hold a hotkey anywhere on your computer, speak naturally, and clean formatted text is pasted at your cursor. Your voice never leaves your machine. Zero subscription. Zero telemetry. **Zero cloud, zero API.**

## Download

Grab the latest installer for your OS from the [**Releases page**](https://github.com/Razepriv/openwhisper/releases).

| OS                  | Asset                                   | What you'll see on first launch                                |
| ------------------- | --------------------------------------- | -------------------------------------------------------------- |
| **Windows 10/11**   | `OpenWhisper_x.y.z_x64_en-US.msi` or `OpenWhisper_x.y.z_x64-setup.exe` | SmartScreen warning → "More info" → "Run anyway". (We don't ship a code-signing cert.) |
| **macOS (Apple Silicon)** | `OpenWhisper_x.y.z_aarch64.dmg`   | Gatekeeper warning → System Settings → Privacy & Security → "Open Anyway". |
| **macOS (Intel)**   | `OpenWhisper_x.y.z_x64.dmg`             | Same Gatekeeper flow.                                          |
| **Linux (x86_64)**  | `OpenWhisper_x.y.z_amd64.deb` / `.AppImage` / `.rpm` | Install via `dpkg -i` (Debian/Ubuntu) or just `chmod +x` and run the AppImage. |
| **Linux (arm64)**   | `OpenWhisper_x.y.z_arm64.deb` / `.AppImage` / `.rpm` | Same.                                                    |

OpenWhisper ships **unsigned by design** — we don't pay $99/yr to Apple or $300+/yr to a Windows EV cert authority just so you can run a local-only tool. The one-time "this app is unsigned" warning is a normal part of installing open-source software outside the App Store / Microsoft Store. After the first launch the OS remembers your choice.

**Where to look for updates:** OpenWhisper has a built-in auto-updater that pulls from the same Releases page. Toggle it in Settings → About → "Check for updates".

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
git clone https://github.com/Razepriv/openwhisper.git
cd openwhisper
bun install
bun run tauri dev
```

### Cutting a release (for maintainers)

The release pipeline is fully unsigned by default, so producing a downloadable installer set requires nothing more than triggering the workflow:

1. Bump `version` in `src-tauri/tauri.conf.json`.
2. Commit + push.
3. GitHub → Actions → **Release** → **Run workflow**.
4. Leave the "Code-sign binaries" checkbox unchecked (it requires the Apple + Azure cert secrets to be populated).
5. Wait ~25 minutes for the matrix (Mac arm64/x64, Linux x64/arm64 deb/rpm/AppImage, Windows x64/arm64 msi/exe) to build. Each platform job downloads:
   - **Whisper Tiny** (~75 MB) from Hugging Face → bundled into the installer for instant first-use.
   - **`llama-server`** from the upstream `llama.cpp` release (Vulkan build on Win/Linux, Metal on Mac) → bundled for local LLM cleanup.
6. The workflow publishes everything as a draft GitHub Release. Edit the release notes and click "Publish release" when ready.

To also bundle **Whisper Small** (adds ~470 MB to the installer for users on slower networks who want a quality upgrade out of the box), set the repository variable `WHISPER_SMALL_BUNDLE=true` before running the workflow.

## License

MIT — see [LICENSE](./LICENSE). Includes Handy's original MIT copyright as required.

## Roadmap and research

The full research, decisions log, and 16-week build plan live in [`research/`](../research/):

- `00-INDEX.md` — start here
- `08-decisions.md` — locked architecture decisions
- `09-auto-provisioning.md` — zero-touch model installer spec
- `06-architecture-and-roadmap.md` — 9-phase build plan
