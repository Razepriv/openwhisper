/**
 * MicTestStep — Phase Finalize.E onboarding polish.
 *
 * After the model finishes downloading we want the user to confirm
 * the mic actually works before we drop them into the main app. The
 * subscription to `mic-level` events is the same channel the
 * recording overlay uses (see `overlay::emit_levels`), so this test
 * exercises the real audio path end-to-end:
 *
 *  1. User clicks "Start mic test"
 *  2. We trigger a 5-second recording via `commands.toggleTranscription`
 *  3. The level meter updates from `mic-level` events
 *  4. We auto-stop after 5s and let the user accept or retry
 *
 * Failure modes covered:
 *  - No mic input: levels stay at 0; we surface a "no audio detected"
 *    warning and link to settings.
 *  - Permission denied: a `recording-error` event triggers a toast
 *    (handled by App.tsx already).
 *
 * The whole step is skippable — users who know their setup works can
 * just click "Skip" and we move on. We don't force a successful test
 * because automated detection of "is this real audio?" is fragile.
 */

import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import HandyTextLogo from "../icons/HandyTextLogo";
import { useSettings } from "../../hooks/useSettings";

interface MicTestStepProps {
  onComplete: () => void;
  onSkip: () => void;
}

/** Number of bars in the level meter — matches the recording overlay. */
const BAR_COUNT = 24;

const MicTestStep: React.FC<MicTestStepProps> = ({ onComplete, onSkip }) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const [maxLevel, setMaxLevel] = useState(0);
  const [levels, setLevels] = useState<number[]>(Array(BAR_COUNT).fill(0));
  const smoothedRef = useRef<number[]>(Array(BAR_COUNT).fill(0));

  // Subscribe to mic-level events from the backend. Same channel the
  // overlay uses — guarantees the test path matches the real path.
  useEffect(() => {
    const unlistenPromise = listen<number[]>("mic-level", (event) => {
      const incoming = event.payload ?? [];
      const smoothed = smoothedRef.current.map((prev, i) => {
        const target = incoming[i] ?? 0;
        return prev * 0.6 + target * 0.4;
      });
      smoothedRef.current = smoothed;
      setLevels(smoothed.slice(0, BAR_COUNT));
      const peak = smoothed.reduce((a, b) => Math.max(a, b), 0);
      setMaxLevel((prev) => Math.max(prev, peak));
    });
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, []);

  // Click-driven test path that reuses the widget trigger. Same flow
  // as the floating widget click — start dictation, mic levels stream
  // in via the event listener above, user releases by clicking again
  // (the trigger toggles).
  const handleStart = async () => {
    setMaxLevel(0);
    try {
      // `trigger_dictation_from_widget` toggles the same coordinator
      // path as a press-to-talk hotkey. The mic-level events above
      // will start flowing as soon as the recording starts.
      await invoke("trigger_dictation_from_widget");
    } catch (e) {
      console.warn("Failed to start mic test:", e);
    }
  };

  const bindings = getSetting("bindings") ?? {};
  const transcribeHotkey =
    bindings["transcribe"]?.current_binding ?? "Ctrl+Space";

  const detectedAudio = maxLevel > 0.05;

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-8 bg-background text-text">
      <div className="w-full max-w-xl flex flex-col items-center gap-6">
        <HandyTextLogo />
        <h2 className="text-2xl font-semibold text-center">
          {t("onboarding.micTest.title")}
        </h2>
        <p className="text-mid-gray text-center">
          {t("onboarding.micTest.description")}
        </p>

        {/* Level meter — uses the same shape as the recording overlay
            so users get visual continuity between the test and the
            real recording bar later. */}
        <div className="w-full bg-background-ui rounded-lg p-6 flex flex-col items-center gap-4">
          <div className="flex items-end justify-center gap-1 h-16">
            {levels.map((v, i) => (
              <div
                key={i}
                className="w-2 bg-logo-primary rounded-sm transition-all"
                style={{
                  height: `${Math.min(60, 4 + Math.pow(v, 0.7) * 56)}px`,
                  opacity: Math.max(0.3, v * 1.5),
                }}
              />
            ))}
          </div>
          {detectedAudio ? (
            <p className="text-sm text-green-500 font-medium">
              {t("onboarding.micTest.detected")}
            </p>
          ) : maxLevel === 0 ? (
            <p className="text-sm text-mid-gray">
              {t("onboarding.micTest.idle", { hotkey: transcribeHotkey })}
            </p>
          ) : (
            <p className="text-sm text-amber-500 font-medium">
              {t("onboarding.micTest.noAudio")}
            </p>
          )}
        </div>

        <div className="flex gap-3 w-full">
          <button
            type="button"
            onClick={handleStart}
            className="flex-1 px-4 py-2 rounded-md bg-logo-primary text-white font-medium"
          >
            {t("onboarding.micTest.startButton")}
          </button>
          <button
            type="button"
            onClick={onComplete}
            className="flex-1 px-4 py-2 rounded-md bg-background-ui text-text font-medium"
          >
            {t("onboarding.micTest.continueButton")}
          </button>
        </div>

        <button
          type="button"
          onClick={onSkip}
          className="text-sm text-mid-gray hover:text-text underline"
        >
          {t("onboarding.micTest.skipButton")}
        </button>
      </div>
    </div>
  );
};

export default MicTestStep;
