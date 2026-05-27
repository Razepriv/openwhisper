/**
 * HotkeyConfigStep — Phase Finalize.E onboarding polish.
 *
 * Surfaces the current push-to-talk hotkey during first-run so the
 * user knows what to press AND can change it before they ever try.
 * Skipping the test = accept the default (Ctrl+Space on Win/Linux,
 * Option+Space on macOS).
 *
 * We deliberately don't validate that the user actually picked
 * something usable — the existing ShortcutInput already handles the
 * parsing + collision detection. We just provide the surface.
 */

import React from "react";
import { useTranslation } from "react-i18next";
import HandyTextLogo from "../icons/HandyTextLogo";
import { ShortcutInput } from "../settings/ShortcutInput";
import { useSettings } from "../../hooks/useSettings";

interface HotkeyConfigStepProps {
  onComplete: () => void;
}

const HotkeyConfigStep: React.FC<HotkeyConfigStepProps> = ({ onComplete }) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const bindings = getSetting("bindings") ?? {};
  const hasTranscribe = Boolean(bindings["transcribe"]);

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-8 bg-background text-text">
      <div className="w-full max-w-xl flex flex-col items-center gap-6">
        <HandyTextLogo />
        <h2 className="text-2xl font-semibold text-center">
          {t("onboarding.hotkeyConfig.title")}
        </h2>
        <p className="text-mid-gray text-center">
          {t("onboarding.hotkeyConfig.description")}
        </p>

        <div className="w-full bg-background-ui rounded-lg p-6">
          {hasTranscribe ? (
            <ShortcutInput shortcutId="transcribe" descriptionMode="tooltip" />
          ) : (
            <p className="text-sm text-mid-gray">
              {t("onboarding.hotkeyConfig.loading")}
            </p>
          )}
        </div>

        <p className="text-xs text-mid-gray text-center max-w-md">
          {t("onboarding.hotkeyConfig.hint")}
        </p>

        <button
          type="button"
          onClick={onComplete}
          className="px-6 py-2 rounded-md bg-logo-primary text-white font-medium"
        >
          {t("onboarding.hotkeyConfig.continueButton")}
        </button>
      </div>
    </div>
  );
};

export default HotkeyConfigStep;
