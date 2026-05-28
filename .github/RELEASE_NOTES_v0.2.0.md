# handy v0.2.0 — first public release

100% local, 100% free, Wispr-Flow-class voice dictation for Mac, Windows, and Linux.

## Download

Pick the installer for your OS from the **Assets** section below. **All builds are unsigned by design** — handy is open-source software distributed outside any app store, so you'll see a one-time "Run anyway" warning on first launch.

| OS                       | Asset                                                                       | First-launch step                                                  |
| ------------------------ | --------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Windows 10 / 11 (x64)    | `handy_0.2.0_x64_en-US.msi` or `handy_0.2.0_x64-setup.exe`                  | SmartScreen → "More info" → "Run anyway"                           |
| Windows 11 (ARM64)       | `handy_0.2.0_arm64_en-US.msi` or `handy_0.2.0_arm64-setup.exe`              | Same SmartScreen flow                                              |
| macOS (Apple Silicon)    | `handy_0.2.0_aarch64.dmg`                                                   | Gatekeeper → System Settings → Privacy & Security → "Open Anyway"  |
| macOS (Intel)            | `handy_0.2.0_x64.dmg`                                                       | Same Gatekeeper flow                                               |
| Linux (x86_64) — Ubuntu/Debian | `handy_0.2.0_amd64.deb`                                               | `sudo dpkg -i handy_0.2.0_amd64.deb`                               |
| Linux (x86_64) — Fedora/RHEL   | `handy-0.2.0-1.x86_64.rpm`                                            | `sudo dnf install ./handy-0.2.0-1.x86_64.rpm`                      |
| Linux (x86_64) — Universal     | `handy_0.2.0_amd64.AppImage`                                          | `chmod +x ./handy_0.2.0_amd64.AppImage && ./handy_0.2.0_amd64.AppImage` |
| Linux (arm64)            | `handy_0.2.0_arm64.{deb,rpm,AppImage}`                                      | As above                                                           |

## Highlights

- 🎙️ **Local-first dictation** — Whisper Tiny ships in the installer for instant first-use. Larger models (Small / Medium / Turbo) auto-download on demand based on your hardware.
- 🧠 **Local LLM cleanup** — bundled `llama.cpp` sidecar for grammar / tone fixups, with auto-detection of any pre-existing Ollama install. **No cloud providers ship with handy.**
- 💫 **Persistent floating widget** — Wispr Flow's "Flow Bar" pattern. Click to dictate; opacity slider in Advanced settings.
- ⌨️ **Command Mode** — highlight any text, hold `Ctrl+Alt+Space` (Windows / Linux) or `Cmd+Ctrl+Space` (macOS), speak an instruction like "make this more polite", and watch the selection get rewritten in place.
- 🪄 **Transforms** — bind hotkeys to user-defined LLM rewrites (defaults ship Polish + Prompt Engineer).
- 📚 **Personal Dictionary + Snippets** — bias Whisper toward your jargon; expand `"sig"` to your full email signature.
- 🧑‍💻 **Vibe Coding** — file-tagging (`tag main.py` → `@main.py`) and identifier backtick-wrapping for Cursor / VS Code / Windsurf / Claude Code / Codex.
- 🎨 **Editorial UI** — violet accent, Inter typography, hairline cards, light / dark / system theme toggle.
- 📊 **Voice Profile dashboard + Scratchpad** — 100% local analytics over your transcription history, plus a markdown notes editor.

## What's new in v0.2.0 vs prior development builds

### Slow-transcription fix
CPU-only systems (Windows / Linux without GPU) now default to Whisper Tiny instead of Small — short clips transcribe in <1 s on a typical x86_64 CPU.

### Transcribe progress indicator
The recording overlay now shows elapsed seconds while transcribing ("Transcribing… 4s") so long inference no longer feels like a hang.

### GPU selector in Performance settings
The Whisper acceleration picker (which lists detected GPUs with VRAM) moved out of the "experimental" toggle into a top-level Performance group in Advanced settings.

### handy rebrand
Window title, splash, and settings all show "handy". Switched from pink to violet accent. Loaded Inter font. Light / Dark / System theme toggle in Advanced → App → Appearance.

### Boot-resilience fixes
- Disabled WebView2 GPU compositing automatically — fixes white-screen-of-death on AMD / Intel iGPU systems.
- Wrapped `initialize_core_logic` in `catch_unwind` so a panic during manager setup no longer leaves the main window invisible.
- Global JS error sink in `index.html` surfaces runtime errors instead of dying silently.

## First-run setup

1. Grant accessibility / microphone permission when prompted (macOS / Windows).
2. **Auto-provisioner** picks the right model for your hardware and downloads it in the background. Whisper Tiny is already bundled so you can dictate immediately.
3. Walk through mic test → hotkey config → 3-card tutorial.
4. Land in the main settings window with the floating widget visible.

**Default hotkeys:**
- **Dictate**: `Ctrl+Space` (Windows / Linux) or `Option+Space` (macOS)
- **Command Mode**: `Ctrl+Alt+Space` (Windows / Linux) or `Cmd+Ctrl+Space` (macOS)

All hotkeys are remappable in Settings → Shortcuts.

## Privacy

handy performs **all** transcription and AI cleanup on your machine. The only outbound network calls are:

1. Downloading model weights from Hugging Face on first run (one-time, no user data).
2. Checking for app updates against this repo's Releases page (opt-out in Settings → About).

A hard-coded URL whitelist (`llm_client::validate_local_url`) rejects any non-loopback HTTP endpoint at the network boundary — a bug in user settings cannot leak a transcript to a cloud provider.

## Known limitations

- **Windows GPU acceleration**: this build was compiled without Vulkan, so the GPU dropdown in Performance settings shows CPU only. To enable GPU, install the [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home) and rebuild with the `whisper-vulkan` Cargo feature. See [BUILD.md](https://github.com/Razepriv/openwhisper/blob/main/BUILD.md).
- **macOS / Linux**: Vibe Coding's identifier backtick-wrapping is a no-op outside Windows because we haven't wired the AX-API / AT-SPI accessibility probes yet. File-tagging works everywhere.
- **Linux Wayland**: keyboard injection via `rdev` doesn't support Wayland. X11 works fully.
- **Apple Foundation Models / Phi Silica**: backend stubs exist, real FFI is not wired. Use bundled `llama.cpp` or install Ollama for local LLM cleanup.

## SHA256 checksums

<!-- Run `sha256sum *.{msi,exe,dmg,deb,rpm,AppImage}` against the downloaded artifacts before publishing. -->

```
TODO: fill in after build matrix completes.
```

---

Forked from [cjpais/Handy](https://github.com/cjpais/Handy) (MIT). Massive credit to CJ Pais + the Handy contributors — without their work, handy wouldn't exist.

🤖 Generated by Claude.
