# 12 — Final Status (2026-05-26)

> The honest accounting of what's done, what's not, and what's blocked.
> Use this as the running scorecard alongside [`11-phase-1-tasks.md`](./11-phase-1-tasks.md).

## Repo state

- **15 commits live on `origin/main`** at https://github.com/Razepriv/openwhisper.git
- **213 / 213 unit tests pass** (`cargo test --lib`)
- **`cargo check` is green** on Windows MSVC with CPU-only Whisper (Vulkan SDK disabled
  pending admin install).
- Working tree clean.

## Commits, newest → oldest

```
b3e3a66  feat(commands): expose all OpenWhisper backend modules via Tauri commands  (Phase 9.bridge)
14870a2  feat(insights+notes): Voice Profile analytics + local Scratchpad           (Phase 8.1/8.2)
148f217  feat(commands): Command Mode + Transforms backend modules                  (Phase 7.1/7.2)
0e2e2b1  feat(vibe-coding): active-app detection + Vibe Coding for IDEs and Claude Code/Codex (Phase 5/6)
bf950f4  feat(local-only): remove cloud LLM providers + enforce localhost-only URL whitelist  (Phase 1.10)
cfa1150  feat(snippets): voice-triggered text expansion backend                     (Phase 3.1/3.2)
6c931cb  feat(dictionary): structured Personal Dictionary with starred + replacement support (Phase 2.1)
409165e  feat(cleanup): add CleanupLevel + Ollama detect + llama.cpp sidecar manager (Phase 1.6/1.8/1.9)
8cd1e6a  feat(onboarding): add AutoSetupStep that consumes the auto_provisioner     (Phase 1.5b)
c8f4be3  feat(auto_provisioner): wire Probe + Resolver + ModelManager for zero-touch setup (Phase 1.5a)
121e0ec  feat(backend_resolver): map SystemProfile to optimal stack across 10 tiers  (Phase 1.3)
24e2652  feat(system_probe): add cross-platform hardware detection for BackendResolver (Phase 1.2)
0ba14d9  refactor(model): externalize hardcoded HashMap to models_manifest.json     (Phase 1.1)
77b64b4  chore: initial import from cjpais/Handy v0.8.3 (MIT) + OpenWhisper rebrand (Phase 0)
3e77e05  Initial commit                                                              (GitHub default — overwritten)
```

## Done (≈ 50–55% of full Wispr-Flow parity)

### Foundation

- ✅ Forked Handy (MIT) → OpenWhisper (MIT). Rebranded in 8 files; `LICENSE`
  preserves the Handy attribution.
- ✅ Toolchain: Rust 1.95, Bun 1.3, CMake 3.31 installed and verified.
- ✅ Repo on GitHub with full phase-by-phase history.

### Backend modules (all with 100% local-only enforcement)

- ✅ **`models_manifest.json`** + `model_manifest.rs` — 17 model entries, manifest-driven instead of 500 lines of hardcoded HashMap inserts.
- ✅ **`system_probe.rs`** — OS / CPU / RAM / disk / NPU / locale detection on Mac, Windows, Linux.
- ✅ **`backend_resolver.rs`** — 10-tier matching (S/A/B/C/D/E/F/G/H/Z) from SystemProfile to optimal STT + cleanup stack.
- ✅ **`auto_provisioner.rs`** + Tauri command — wires probe → resolve → download → activate.
- ✅ **`llama_sidecar.rs`** — manager for the bundled `llama-server` child process (binary still to be fetched at release-build time).
- ✅ **`ollama_detect.rs`** — auto-discovers a pre-installed Ollama on PATH + standard install locations.
- ✅ **`CleanupLevel`** enum (None / Light / Medium / High) + system prompts per level.
- ✅ **Local-only enforcement** — cloud providers removed, `llm_client::validate_local_url` rejects any non-loopback URL at the HTTP boundary.
- ✅ **`managers/dictionary.rs`** — structured Personal Dictionary with starred entries, replacement field, migration from legacy `custom_words`.
- ✅ **`managers/snippets.rs`** — voice-triggered text expansion with 60-char trigger + 4000-char expansion limits.
- ✅ **`active_app.rs`** — focused-app probe on Windows (Win32), Linux (X11 via xprop/xdotool), macOS (AppleScript). Wayland gracefully returns empty.
- ✅ **`vibe_coding.rs`** — full Vibe Coding pipeline:
  - GraphicalIde detection (Cursor, VS Code, Windsurf, JetBrains family)
  - **AgentTerminal detection — Claude Code + Codex + Aider** (per user directive)
  - File-tagging (`"tag main.py"` → `@main.py`) for both contexts
  - Backtick wrapping of identifiers in IDE contexts (suppressed in terminals)
- ✅ **`command_mode.rs`** — Command Mode validator + prompt builder with 1000-word selection cap, 200-word instruction cap, identical-output detection.
- ✅ **`transforms.rs`** — hotkey-bound rewrite library with shipping defaults (Polish, Prompt Engineer) and `{TEXT}` template substitution.
- ✅ **`managers/insights.rs`** — Voice Profile analytics: total words, WPM, top phrases, daily/hourly buckets, current/longest streak.
- ✅ **`managers/notes.rs`** — Scratchpad CRUD with atomic JSON persistence, 10k notes cap, 1 MB body cap.

### Frontend

- ✅ **`AutoSetupStep.tsx`** — zero-touch onboarding screen (Phase 1.5b).
- ✅ App state machine extended with `"auto-setup"` step.
- ✅ Manual-picker escape hatch preserved.
- ✅ English i18n strings for the new step.

### Tauri command surface (Phase 9.bridge)

- ✅ 24 new commands exposing every new backend module to the frontend via `tauri-specta`:
  - Insights: `get_voice_profile`
  - Notes: `list_notes`, `get_note`, `create_note`, `update_note`, `delete_note`
  - Dictionary: list / add / update / delete / bulk_set
  - Snippets: list / add / update / delete / bulk_set
  - Transforms: list / add / update / delete / preview_prompt
  - Vibe Coding: `apply_vibe_coding`, `get_active_app`
  - Command Mode: `prepare_command_mode`
- ✅ Plus Phase 1–3 commands: `get_system_profile`, `get_recommended_stack`, `start_auto_provisioning`, `detect_ollama`.

## Not done — UI work

These are deliberate Phase-X.b items that wait for the React surface to land. The
backend commands are ready; the visible UI sections are not.

- ✅ Snippets sidebar section (UI for the existing CRUD commands). — commit `dfcb70a`.
- ✅ Dictionary sidebar section (UI for the existing CRUD commands). — commit `dfcb70a`.
- ✅ Transforms sidebar section + per-transform hotkey binding UI. — commit `dfcb70a`.
- ✅ Voice Profile dashboard page. — commit `dfcb70a`.
- ✅ Scratchpad / Notes section. — commit `dfcb70a`.
- ✅ 4-tier Auto Cleanup picker in Advanced settings. — Phase UI.batch (`CleanupLevelPicker` mounted in `AdvancedSettings`).
- ✅ Sidebar rework to Wispr layout (Home / Insights / Dictionary / Snippets / Style / Transforms / Notes). — commit `dfcb70a`.
- ✅ **Persistent floating widget** (Wispr Flow's Flow Bar) — Phase UI.batch.
  Stays on screen in an idle clickable state between dictations. Click triggers
  the same flow as the push-to-talk hotkey via `trigger_dictation_from_widget`.
  User-controllable enable/disable + opacity slider in Advanced → App.
- ✅ **Onboarding wizard polish** — Phase Finalize.E. Three new steps inserted
  between auto-setup and "done": `MicTestStep` (live VU meter + Start button),
  `HotkeyConfigStep` (inline ShortcutInput for transcribe binding),
  `FirstDictationStep` (3-card tutorial with the user's actual hotkeys).
- ✅ **Style presets per app category** — Phase Finalize.D. New
  `style_presets.rs` module (Formal / Casual / VeryCasual / Concise / Neutral)
  + `AppCategory` classifier (Email / Chat / Code / Document / Other) +
  per-category overrides persisted in `AppSettings.style_overrides`. Cleanup
  LLM system prompt now prepends the active preset's fragment.

## Not done — wire-up between backend modules

- ✅ **Symbol extraction** — Phase Finalize.C. New `symbols.rs` does a Windows
  UI Automation tree walk on the focused HWND, extracts identifier-shaped
  tokens (`camelCase` / `snake_case` / digits), and feeds them into
  `vibe_coding::apply` as `known_symbols`. macOS / Linux still stubbed.
- ✅ **Per-app routing in `actions.rs`**: `process_transcription_output` now calls
  `active_app::detect()` once per dictation and runs the result through
  `vibe_coding::apply` before paste. File-tagging fires for IDEs and agent terminals;
  backtick-wrapping fires only in IDEs.
- ✅ **Auto-trigger of `auto_provisioner::provision`** — Phase Finalize.B.
  `lib::maybe_auto_provision` fires 1.5s after init when no model is selected
  and none are downloaded. Uses Ollama detection + system probe.
- ✅ **Command Mode hotkey wiring** — Phase Finalize.A. `CommandModeAction`
  captures selection via Ctrl+C → reads clipboard → stores in NEXT_PROCESSING
  → dispatches through TranscribeAction recording path → in
  `process_transcription_output`, branches to `command_mode::build_prompts`
  + LLM + paste. Default hotkey: Ctrl+Alt+Space (Win/Linux) / Cmd+Ctrl+Space (Mac).
- ✅ **Transforms hotkey registration** — Phase Finalize.A.
  `shortcut::register_transform_hotkeys` walks `settings.transforms` on app
  launch and registers `transform:<id>` global shortcuts for every transform
  with a `hotkey` set. `actions::TransformAction` then routes the dictation
  through the transform's prompt template + LLM before paste.

## Not done — production-readiness blockers

These can't be solved from a Windows dev machine without external artifacts:

- ✅ **llama.cpp binaries** — Phase Finalize.F. `build.yml` now downloads
  prebuilt `llama-server` from the upstream `llama.cpp` GitHub releases
  per-OS (macOS arm64/x86_64, Linux x86_64 Vulkan, Windows x86_64 Vulkan)
  into `src-tauri/resources/llama-cpp/` before `tauri build`. Bundled
  resource glob picks them up automatically.
- ✅ **Bundled Whisper Tiny** in the installer (~75 MB) — Phase Finalize.F.
  CI downloads `ggml-tiny.bin` into `src-tauri/resources/models/`. Whisper
  Small bundling is optional via `WHISPER_SMALL_BUNDLE=true` env (adds
  ~470 MB to installer); without it the model still arrives via the
  first-run auto-provisioner.
- ❌ **Vulkan SDK install** on Windows for GPU acceleration. The Cargo.toml feature
  is commented out; needs admin elevation to install the SDK then uncomment.
- ❌ **Code-signing certs**: Apple Developer ID for `.dmg` notarisation, Windows EV cert
  for `.msi`. Without these the installers ship as "unsigned" and SmartScreen / Gatekeeper
  warn the user.
- ❌ **GitHub Actions release workflow** that builds the matrix, signs, notarises,
  uploads to releases, generates the `latest.json` for the auto-updater.
- ❌ **Distribution channel submissions**: Homebrew tap, Winget manifest, AUR PKGBUILD.
- ❌ **Apple Foundation Models Swift sidecar** + Phi Silica WinRT sidecar — backend
  modules are stubbed; the actual FFI for these OS-provided LLMs hasn't been built.
- ❌ **Linux Wayland keyboard injection** — `rdev` doesn't support Wayland; need
  `ydotool` + uinput permissions plumbing.
- ❌ **`bindings.ts` regeneration** — the typed Tauri bindings file is auto-generated
  by `tauri-specta` only when `bun run tauri dev` actually launches. The 24 new commands
  exist in Rust but the frontend can't see them yet via `commands.x` — `AutoSetupStep`
  uses raw `invoke()` with local type aliases as a stop-gap.
- ❌ **i18n for 19 non-English locales** — the new `onboarding.autoSetup.*` strings
  are English-only; the other 19 ship locales fall back to English.

## What this means in plain English

If you ran `bun run tauri dev` on a Mac or Windows machine RIGHT NOW with the Vulkan SDK
installed and the llama-server binary placed in `src-tauri/resources/llama-cpp/`, you'd get:

- ✅ A launching OpenWhisper app with the auto-setup onboarding screen.
- ✅ Hotkey-driven dictation working with Whisper Small (downloaded on demand).
- ✅ Local-only LLM cleanup via the bundled llama.cpp sidecar (provided the binary is there).
- ✅ Personal Dictionary biasing Whisper's `initial_prompt`.
- ✅ Snippet expansion on transcribed text.
- ⏳ Vibe Coding for Cursor / VS Code / Windsurf / Claude Code / Codex — backend
  ready; frontend doesn't surface it yet.
- ⏳ Command Mode + Transforms — backend ready; hotkey + UI not wired.
- ⏳ Voice Profile dashboard + Scratchpad — backend ready; UI pages not built.

If you ran `bun run tauri build` for a distributable installer right now, it would fail
because llama.cpp isn't bundled and the Vulkan SDK isn't installed. Both gaps are
documented in `BUILD.md` (existing) and tracked here.

## Suggested next sprint (1 focused week)

1. **Symbol extraction** — accessibility-tree reader on Mac + Windows. Feeds Vibe Coding.
2. **Per-app routing in `actions.rs`** — branch on `active_app::detect()` at dictation
   start. Unlocks the visible Vibe Coding behaviour.
3. **GitHub Actions release workflow** — matrix build that fetches llama.cpp,
   produces `.dmg` / `.msi` / `.AppImage`, signs them, uploads to releases.
4. **Bundled-models CI step** — fetch Whisper Tiny + Small from Hugging Face during
   the release workflow and embed them.
5. **Sidebar UI for Snippets + Dictionary** — the two most-used features per Wispr
   user research; backend is already complete.
6. **Voice Profile dashboard** — most visible new feature; React charts over the
   existing `get_voice_profile` command.

After that sprint OpenWhisper crosses the line from "backend complete" to "shippable
v1 RC".
