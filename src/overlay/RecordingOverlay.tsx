import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  MicrophoneIcon,
  TranscriptionIcon,
  CancelIcon,
} from "../components/icons";
import "./RecordingOverlay.css";
import { commands } from "@/bindings";
import i18n, { syncLanguageFromSettings } from "@/i18n";
import { getLanguageDirection } from "@/lib/utils/rtl";

// OpenWhisper Phase UI.batch: "idle" added so the overlay can stay
// visible as a persistent floating widget (Wispr Flow's Flow Bar
// equivalent). Clicking it triggers the same flow as the push-to-talk
// hotkey via the `trigger_dictation_from_widget` Tauri command.
type OverlayState = "idle" | "recording" | "transcribing" | "processing";

/**
 * Shape of the `overlay-config` event emitted by Rust
 * (`overlay::emit_overlay_config`). The backend always sends both
 * fields; the type is widened with `?` only as a defensive measure for
 * forward-compat with future config additions / older app versions.
 */
interface OverlayConfig {
  floating_widget_enabled?: boolean;
  floating_widget_opacity?: number;
}

/** Defensive clamp matching the backend's [0.2, 1.0] window. */
const clampOpacity = (value: number): number => {
  if (!Number.isFinite(value)) return 0.9;
  if (value < 0.2) return 0.2;
  if (value > 1.0) return 1.0;
  return value;
};

const RecordingOverlay: React.FC = () => {
  const { t } = useTranslation();
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>("recording");
  const [levels, setLevels] = useState<number[]>(Array(16).fill(0));
  // Idle-state opacity from the floating-widget setting. Applied as a
  // CSS custom property (--widget-opacity) consumed by the .idle-clickable
  // rule in RecordingOverlay.css.
  const [widgetOpacity, setWidgetOpacity] = useState<number>(0.9);
  // handy fix: track elapsed time during "transcribing" / "processing"
  // states so the user sees the bar isn't stuck. CPU-only Whisper can
  // take 5-15 s for a short clip; without this the static "Transcribing…"
  // label is indistinguishable from a hang.
  const [elapsedSec, setElapsedSec] = useState<number>(0);
  const elapsedTickRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  const direction = getLanguageDirection(i18n.language);

  // Start / stop the elapsed-seconds counter when the state enters
  // a long-running phase. Recording itself doesn't get a counter
  // (the bars already convey activity); only post-recording stages do.
  useEffect(() => {
    if (state === "transcribing" || state === "processing") {
      setElapsedSec(0);
      const started = Date.now();
      elapsedTickRef.current = setInterval(() => {
        setElapsedSec(Math.floor((Date.now() - started) / 1000));
      }, 250);
      return () => {
        if (elapsedTickRef.current) {
          clearInterval(elapsedTickRef.current);
          elapsedTickRef.current = null;
        }
      };
    }
    // Reset counter when leaving the long-running phases.
    setElapsedSec(0);
    if (elapsedTickRef.current) {
      clearInterval(elapsedTickRef.current);
      elapsedTickRef.current = null;
    }
    return undefined;
  }, [state]);

  useEffect(() => {
    const setupEventListeners = async () => {
      // Listen for show-overlay event from Rust
      const unlistenShow = await listen("show-overlay", async (event) => {
        // Sync language from settings each time overlay is shown
        await syncLanguageFromSettings();
        const overlayState = event.payload as OverlayState;
        setState(overlayState);
        setIsVisible(true);
      });

      // Listen for hide-overlay event from Rust
      const unlistenHide = await listen("hide-overlay", () => {
        setIsVisible(false);
      });

      // Listen for mic-level updates
      const unlistenLevel = await listen<number[]>("mic-level", (event) => {
        const newLevels = event.payload as number[];

        // Apply smoothing to reduce jitter
        const smoothed = smoothedLevelsRef.current.map((prev, i) => {
          const target = newLevels[i] || 0;
          return prev * 0.7 + target * 0.3; // Smooth transition
        });

        smoothedLevelsRef.current = smoothed;
        setLevels(smoothed.slice(0, 9));
      });

      // OpenWhisper Phase UI.batch: receive floating-widget config from
      // Rust (opacity, etc.). The backend pushes this every time it
      // shows the overlay, so the widget always reflects the latest
      // settings without us having to invoke a getter from the webview.
      const unlistenConfig = await listen<OverlayConfig>(
        "overlay-config",
        (event) => {
          const payload = event.payload ?? {};
          if (typeof payload.floating_widget_opacity === "number") {
            setWidgetOpacity(clampOpacity(payload.floating_widget_opacity));
          }
        },
      );

      // Cleanup function
      return () => {
        unlistenShow();
        unlistenHide();
        unlistenLevel();
        unlistenConfig();
      };
    };

    setupEventListeners();
  }, []);

  const getIcon = () => {
    if (state === "recording" || state === "idle") {
      return <MicrophoneIcon />;
    } else {
      return <TranscriptionIcon />;
    }
  };

  const handleIdleClick = () => {
    if (state !== "idle") return;
    // `trigger_dictation_from_widget` is registered in
    // src-tauri/src/lib.rs::collect_commands and reuses the existing
    // signal_handle::send_transcription_input pipeline so the click is
    // indistinguishable from the configured push-to-talk hotkey.
    invoke("trigger_dictation_from_widget").catch((err: unknown) => {
      console.error("widget click failed to trigger dictation:", err);
    });
  };

  return (
    <div
      dir={direction}
      role={state === "idle" ? "button" : undefined}
      tabIndex={state === "idle" ? 0 : undefined}
      onClick={state === "idle" ? handleIdleClick : undefined}
      onKeyDown={
        state === "idle"
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleIdleClick();
              }
            }
          : undefined
      }
      title={state === "idle" ? t("overlay.clickToDictate") : undefined}
      className={`recording-overlay ${isVisible ? "fade-in" : ""} ${
        state === "idle" ? "idle-clickable" : ""
      }`}
      // CSS custom property consumed by `.recording-overlay.idle-clickable`
      // in RecordingOverlay.css. Cast through Record<string,string> because
      // React's CSSProperties type rejects unknown `--*` keys.
      style={
        {
          "--widget-opacity": widgetOpacity.toString(),
        } as React.CSSProperties
      }
    >
      <div className="overlay-left">{getIcon()}</div>

      <div className="overlay-middle">
        {state === "idle" && (
          <div className="transcribing-text">{t("overlay.idle")}</div>
        )}
        {state === "recording" && (
          <div className="bars-container">
            {levels.map((v, i) => (
              <div
                key={i}
                className="bar"
                style={{
                  height: `${Math.min(20, 4 + Math.pow(v, 0.7) * 16)}px`, // Cap at 20px max height
                  transition: "height 60ms ease-out, opacity 120ms ease-out",
                  opacity: Math.max(0.2, v * 1.7), // Minimum opacity for visibility
                }}
              />
            ))}
          </div>
        )}
        {state === "transcribing" && (
          <div className="transcribing-text">
            {t("overlay.transcribing")}
            {elapsedSec > 0 && (
              <span style={{ marginLeft: 6, opacity: 0.7 }}>{elapsedSec}s</span>
            )}
          </div>
        )}
        {state === "processing" && (
          <div className="transcribing-text">
            {t("overlay.processing")}
            {elapsedSec > 0 && (
              <span style={{ marginLeft: 6, opacity: 0.7 }}>{elapsedSec}s</span>
            )}
          </div>
        )}
      </div>

      <div className="overlay-right">
        {state === "recording" && (
          <div
            className="cancel-button"
            onClick={(e) => {
              e.stopPropagation();
              commands.cancelOperation();
            }}
          >
            <CancelIcon />
          </div>
        )}
      </div>
    </div>
  );
};

export default RecordingOverlay;
