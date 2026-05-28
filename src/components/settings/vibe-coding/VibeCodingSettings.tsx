/**
 * Vibe Coding settings — toggle the IDE/agent-terminal rewriter on/off
 * and show the live-detected app context so the user knows what
 * handy's seeing when they're about to dictate.
 *
 * Backend: `vibe_coding.rs` + `commands::openwhisper::get_active_app`
 * + `commands::openwhisper::apply_vibe_coding` (preview-only).
 *
 * What the toggle controls (when OFF):
 *  - File-tagging substitution ("tag main.py" → "@main.py") — disabled
 *  - Identifier backtick-wrapping (the user-settings → `userSettings`) — disabled
 *  - Per-dictation per-app routing — disabled
 *
 * What stays unchanged regardless:
 *  - Snippets expansion (governed by Snippets settings)
 *  - Personal Dictionary biasing of Whisper (governed by Dictionary settings)
 *  - Auto Cleanup LLM pass (governed by Cleanup Level)
 *  - Transforms (always hotkey-triggered, never automatic)
 */

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Code, FileText, Hash } from "lucide-react";

import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { useSettings } from "../../../hooks/useSettings";

interface ActiveAppSnapshot {
  process_name: string;
  window_title: string;
  window_class: string;
}

type VibeContext = "GraphicalIde" | "AgentTerminal" | "None";

interface VibeCodingResult {
  text: string;
  context: VibeContext;
}

const CONTEXT_LABEL: Record<VibeContext, string> = {
  GraphicalIde: "Graphical IDE",
  AgentTerminal: "Agent terminal",
  None: "No vibe-coding context detected",
};

const CONTEXT_DESCRIPTION: Record<VibeContext, string> = {
  GraphicalIde:
    "File-tagging and identifier backtick-wrapping will apply to this dictation.",
  AgentTerminal:
    "File-tagging applies; identifier backtick-wrapping is suppressed (shells evaluate backticks).",
  None: "Dictation will be pasted verbatim. Switch to your IDE or coding-agent terminal to activate Vibe Coding.",
};

export const VibeCodingSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  // `vibe_coding_enabled` is new on the Rust side; bindings.ts isn't
  // regenerated until the next `tauri dev`, so we cast through `never`
  // (same pattern as ThemeToggle / FloatingWidget). Default true to
  // match the Rust-side serde default.
  const enabled =
    ((getSetting("vibe_coding_enabled" as never) as boolean | undefined) ??
      true) === true;

  const [activeApp, setActiveApp] = useState<ActiveAppSnapshot | null>(null);
  const [context, setContext] = useState<VibeContext>("None");

  // Poll the focused app + classification every 2 s so the "currently
  // detected" line updates as the user alt-tabs around. Light cost
  // because each call is just GetForegroundWindow + classify (<2 ms).
  const refreshContext = useCallback(async () => {
    try {
      const probe = (await invoke("get_active_app")) as ActiveAppSnapshot;
      setActiveApp(probe);
      // Use apply_vibe_coding as the classifier (it returns the context
      // alongside the rewritten text; we ignore the text and just take
      // the context).
      const result = (await invoke("apply_vibe_coding", {
        text: "",
        knownSymbols: [],
      })) as VibeCodingResult;
      setContext(result.context);
    } catch {
      setActiveApp(null);
      setContext("None");
    }
  }, []);

  useEffect(() => {
    refreshContext();
    const timer = setInterval(refreshContext, 2000);
    return () => clearInterval(timer);
  }, [refreshContext]);

  const handleToggle = (next: boolean) => {
    updateSetting("vibe_coding_enabled" as never, next as never);
    invoke("change_vibe_coding_enabled_setting", { enabled: next }).catch(
      (err) => {
        console.warn("Failed to persist vibe_coding_enabled:", err);
      },
    );
  };

  const titleLabel =
    activeApp && activeApp.process_name
      ? `${activeApp.process_name}${activeApp.window_title ? " — " + activeApp.window_title : ""}`
      : t("settings.vibeCoding.live.noApp", "No focused app detected");

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title={t("settings.vibeCoding.title", "Vibe Coding")}
        description={t(
          "settings.vibeCoding.description",
          "IDE-aware dictation rewrites — file-tagging in IDEs and coding-agent terminals, identifier backtick-wrapping in IDEs only. Runs only when handy detects the focused app is one of the supported coding contexts.",
        )}
      >
        <div className="p-4">
          <ToggleSwitch
            checked={enabled}
            onChange={handleToggle}
            isUpdating={isUpdating("vibe_coding_enabled")}
            label={t("settings.vibeCoding.toggle.label", "Enable Vibe Coding")}
            description={t(
              "settings.vibeCoding.toggle.description",
              "When off, transcription pastes verbatim regardless of focused app.",
            )}
            descriptionMode="inline"
          />
        </div>
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.vibeCoding.live.title", "Live detection")}
        description={t(
          "settings.vibeCoding.live.subtitle",
          "Refreshes every 2 seconds. Switch to your editor or coding-agent terminal to see the context change.",
        )}
      >
        <div className="p-4 space-y-3">
          <div>
            <div className="text-xs uppercase tracking-wide text-text-muted">
              {t("settings.vibeCoding.live.focusedApp", "Focused app")}
            </div>
            <div className="mt-1 text-sm font-medium text-text break-all">
              {titleLabel}
            </div>
          </div>
          <div>
            <div className="text-xs uppercase tracking-wide text-text-muted">
              {t("settings.vibeCoding.live.context", "Detected context")}
            </div>
            <div className="mt-1 flex items-center gap-2">
              <span
                className={`inline-block w-2 h-2 rounded-full ${
                  context === "None" ? "bg-text-muted/40" : "bg-accent"
                }`}
              />
              <span className="text-sm font-medium text-text">
                {CONTEXT_LABEL[context]}
              </span>
            </div>
            <div className="mt-1 text-xs text-text-muted">
              {CONTEXT_DESCRIPTION[context]}
            </div>
          </div>
        </div>
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.vibeCoding.supported.title", "Supported apps")}
        description={t(
          "settings.vibeCoding.supported.subtitle",
          "handy recognises these by their process name or window title. Add a request on GitHub if your editor is missing.",
        )}
      >
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-px bg-border">
          <SupportedAppRow
            icon={<Code size={14} />}
            name={t("settings.vibeCoding.supported.ides", "IDEs (graphical)")}
            apps="Cursor, VS Code, Windsurf, JetBrains (IntelliJ / PyCharm / WebStorm / GoLand / Rider / RustRover / PhpStorm)"
            features={[
              t(
                "settings.vibeCoding.feature.fileTag",
                "File-tagging (e.g. tag main.py → @main.py)",
              ),
              t(
                "settings.vibeCoding.feature.backtick",
                "Identifier backtick-wrapping (the user settings → `userSettings`)",
              ),
            ]}
          />
          <SupportedAppRow
            icon={<Hash size={14} />}
            name={t(
              "settings.vibeCoding.supported.agents",
              "Coding agents in terminals",
            )}
            apps="Claude Code, Codex, Aider, plus Warp / iTerm2 / Terminal / Windows Terminal / kitty / alacritty / gnome-terminal hosting them"
            features={[
              t(
                "settings.vibeCoding.feature.fileTag",
                "File-tagging (e.g. tag main.py → @main.py)",
              ),
              t(
                "settings.vibeCoding.feature.noBacktick",
                "Backtick-wrapping disabled (shells evaluate backticks)",
              ),
            ]}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup
        title={t(
          "settings.vibeCoding.promptOptimizer.title",
          "Prompt optimizer",
        )}
        description={t(
          "settings.vibeCoding.promptOptimizer.subtitle",
          "Bind a hotkey to rewrite the last dictation as a high-quality LLM prompt. Already shipped as the default 'Prompt Engineer' transform — configure under Transforms.",
        )}
      >
        <div className="p-4 flex items-start gap-3">
          <FileText size={16} className="mt-0.5 shrink-0 text-text-muted" />
          <div className="text-sm text-text-muted">
            {t(
              "settings.vibeCoding.promptOptimizer.body",
              "Dictate freely, then press your Prompt Engineer hotkey — the local LLM rewrites it as a structured, explicit prompt. Open Settings → Transforms to set or change the hotkey.",
            )}
          </div>
        </div>
      </SettingsGroup>
    </div>
  );
};

interface SupportedAppRowProps {
  icon: React.ReactNode;
  name: string;
  apps: string;
  features: string[];
}

const SupportedAppRow: React.FC<SupportedAppRowProps> = ({
  icon,
  name,
  apps,
  features,
}) => (
  <div className="bg-background-elevated p-4 space-y-2">
    <div className="flex items-center gap-2">
      <span className="text-text-muted">{icon}</span>
      <span className="text-sm font-medium text-text">{name}</span>
    </div>
    <div className="text-xs text-text-muted leading-relaxed">{apps}</div>
    <ul className="text-xs text-text-muted space-y-0.5 mt-1">
      {features.map((f) => (
        <li key={f} className="flex items-start gap-1.5">
          <span className="text-accent">•</span>
          <span>{f}</span>
        </li>
      ))}
    </ul>
  </div>
);
