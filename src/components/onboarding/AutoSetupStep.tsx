/**
 * AutoSetupStep — first-run zero-touch setup screen.
 *
 * Phase 1.5b of the OpenWhisper roadmap. Consumes the auto_provisioner
 * backend (see src-tauri/src/auto_provisioner.rs) to deliver the promise
 * in research/08-decisions.md decision B6: user grants 2 permissions →
 * sees "we picked X for your machine, downloading ~Y MB" → can dictate
 * immediately when the download completes.
 *
 * ## Lifecycle
 *
 * 1. On mount: call `get_recommended_stack` → display the stack to the user.
 * 2. Auto-start: invoke `start_auto_provisioning` immediately (no extra
 *    click — that's the "zero-touch" part).
 * 3. Listen to `model-download-progress` events from the bundled
 *    ModelManager to drive a progress bar.
 * 4. On `auto-provision-completed`: call `onComplete()`.
 * 5. On `auto-provision-failed`: surface error + offer the advanced manual
 *    picker as an escape hatch.
 *
 * Why `invoke` directly instead of typed `bindings.ts`? The new
 * `get_recommended_stack` / `start_auto_provisioning` commands aren't in
 * `bindings.ts` yet — that file regenerates only when `bun run tauri dev`
 * actually launches. Using `invoke` directly with local type aliases keeps
 * us unblocked without running the full dev server. The next real `tauri
 * dev` run regenerates bindings cleanly.
 */

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";

import HandyTextLogo from "../icons/HandyTextLogo";
import { ProgressBar } from "../shared";

/**
 * Mirrors backend_resolver::Stack. Keep in sync until bindings.ts is
 * regenerated. The summary string is already localised on the Rust side
 * (model + backend names are stable identifiers).
 */
interface RecommendedStack {
  tier: "S" | "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "Z";
  stt_model_id: string;
  cleanup_backend:
    | "AppleFM"
    | "PhiSilica"
    | "Ollama"
    | "LlamaSidecar"
    | "None";
  estimated_download_mb: number;
  summary: string;
}

/**
 * Mirrors managers::model::DownloadProgress emitted by the existing
 * ModelManager. The shape is `{ model_id, downloaded, total, percentage }`.
 */
interface DownloadProgress {
  model_id: string;
  downloaded: number;
  total: number;
  percentage: number;
}

interface AutoSetupStepProps {
  /** Called when the recommended model is downloaded + activated. */
  onComplete: () => void;
  /**
   * Called when the user clicks "Choose another model" — caller should
   * route to the manual `Onboarding` picker. Provides the escape hatch
   * required by decision B11 (custom user models).
   */
  onShowAdvanced: () => void;
}

const AutoSetupStep: React.FC<AutoSetupStepProps> = ({
  onComplete,
  onShowAdvanced,
}) => {
  const { t } = useTranslation();
  const [stack, setStack] = useState<RecommendedStack | null>(null);
  const [progress, setProgress] = useState<number>(0);
  const [phase, setPhase] = useState<
    "probing" | "downloading" | "finalizing" | "done" | "error"
  >("probing");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  /**
   * Step 1 — probe + resolve. Runs once on mount.
   *
   * Splitting probe-then-provision from the provision call lets us show
   * the user *what* we're about to do before doing it — better than a
   * generic spinner.
   */
  const probeAndStart = useCallback(async () => {
    try {
      const resolved = await invoke<RecommendedStack>(
        "get_recommended_stack",
        { hasCapableGpu: false, ollamaDetected: false },
      );
      setStack(resolved);
      setPhase("downloading");

      // Step 2 — kick off the provisioner. Don't await: completion
      // is signalled by the auto-provision-completed event listener below.
      // Errors thrown from invoke() are caught by the .catch().
      invoke("start_auto_provisioning", {
        hasCapableGpu: false,
        ollamaDetected: false,
      }).catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        setErrorMessage(msg);
        setPhase("error");
      });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(msg);
      setPhase("error");
    }
  }, []);

  useEffect(() => {
    probeAndStart();
  }, [probeAndStart]);

  /**
   * Listen for download progress on the recommended model so we can drive
   * a visible progress bar.
   */
  useEffect(() => {
    if (!stack) return;
    const unlistenPromise = listen<DownloadProgress>(
      "model-download-progress",
      (event) => {
        if (event.payload.model_id === stack.stt_model_id) {
          setProgress(Math.min(event.payload.percentage, 100));
        }
      },
    );
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, [stack]);

  /**
   * Listen for the verification + extraction phases that come AFTER
   * the download completes. Some models are .tar.gz that take a few
   * extra seconds to set up; the user should see meaningful state
   * during that time, not a stuck "100%" bar.
   */
  useEffect(() => {
    if (!stack) return;
    const unlistenVerify = listen<string>(
      "model-verification-started",
      (event) => {
        if (event.payload === stack.stt_model_id) setPhase("finalizing");
      },
    );
    const unlistenExtract = listen<string>(
      "model-extraction-started",
      (event) => {
        if (event.payload === stack.stt_model_id) setPhase("finalizing");
      },
    );
    return () => {
      unlistenVerify.then((fn) => fn());
      unlistenExtract.then((fn) => fn());
    };
  }, [stack]);

  /**
   * Terminal events from auto_provisioner. On completion we hand off
   * to the parent state machine; on failure we display an error + the
   * "advanced picker" escape hatch.
   */
  useEffect(() => {
    const unlistenCompleted = listen<{ active_model_id: string }>(
      "auto-provision-completed",
      () => {
        setPhase("done");
        setProgress(100);
        // Small delay so the user actually sees the "done" state before
        // the screen disappears.
        setTimeout(() => onComplete(), 700);
      },
    );
    const unlistenFailed = listen<{ error: string }>(
      "auto-provision-failed",
      (event) => {
        setPhase("error");
        setErrorMessage(event.payload.error);
        toast.error(t("onboarding.autoSetup.errors.provisioning"), {
          description: event.payload.error,
        });
      },
    );
    return () => {
      unlistenCompleted.then((fn) => fn());
      unlistenFailed.then((fn) => fn());
    };
  }, [onComplete, t]);

  const headlineKey = `onboarding.autoSetup.phase.${phase}`;

  return (
    <div className="h-screen w-screen flex flex-col p-6 gap-6 inset-0">
      <div className="flex flex-col items-center gap-2 shrink-0">
        <HandyTextLogo width={200} />
        <p className="text-text/70 max-w-md font-medium mx-auto text-center">
          {t("onboarding.autoSetup.subtitle")}
        </p>
      </div>

      <div className="max-w-[600px] w-full mx-auto flex-1 flex flex-col min-h-0 justify-center gap-6">
        <h2 className="text-xl font-medium text-center">{t(headlineKey)}</h2>

        {stack ? (
          <div className="bg-background-secondary border border-mid-gray/20 rounded-lg p-4 space-y-2">
            <div className="flex items-baseline justify-between gap-4">
              <span className="text-sm text-text/70">
                {t("onboarding.autoSetup.recommended")}
              </span>
              <span className="text-xs font-mono text-text/50">
                {t("onboarding.autoSetup.tier", { tier: stack.tier })}
              </span>
            </div>
            <p className="font-medium">{stack.summary}</p>
            <div className="flex items-baseline justify-between gap-4 text-sm text-text/70">
              <span>{t("onboarding.autoSetup.downloadSize")}</span>
              <span className="font-mono tabular-nums">
                {stack.estimated_download_mb >= 1000
                  ? `${(stack.estimated_download_mb / 1024).toFixed(1)} GB`
                  : `${stack.estimated_download_mb} MB`}
              </span>
            </div>
          </div>
        ) : (
          <p className="text-text/50 text-center">
            {t("onboarding.autoSetup.probing")}
          </p>
        )}

        {(phase === "downloading" || phase === "finalizing") && stack && (
          <div className="space-y-2">
            <ProgressBar
              progress={[
                {
                  id: stack.stt_model_id,
                  percentage: phase === "finalizing" ? 100 : progress,
                },
              ]}
              size="large"
            />
            <p className="text-sm text-text/70 text-center tabular-nums">
              {phase === "downloading"
                ? t("onboarding.autoSetup.downloadingPct", {
                    pct: Math.round(progress),
                  })
                : t("onboarding.autoSetup.finalizing")}
            </p>
          </div>
        )}

        {phase === "error" && errorMessage && (
          <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-4 space-y-2">
            <p className="font-medium text-red-500">
              {t("onboarding.autoSetup.errorTitle")}
            </p>
            <p className="text-sm text-text/70 break-words">{errorMessage}</p>
          </div>
        )}

        <button
          type="button"
          onClick={onShowAdvanced}
          className="text-sm text-text/50 hover:text-text/80 transition-colors mx-auto"
        >
          {t("onboarding.autoSetup.chooseManually")}
        </button>
      </div>
    </div>
  );
};

export default AutoSetupStep;
