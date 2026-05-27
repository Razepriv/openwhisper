/**
 * FirstDictationStep — Phase Finalize.E onboarding polish.
 *
 * The "show the user how to actually use this thing" screen. Three
 * cards that match the three first things they'll do:
 *  1. Press-and-hold the configured hotkey to dictate.
 *  2. Click the floating widget if they're using it.
 *  3. Highlight + Cmd/Ctrl+Alt+Space for Command Mode.
 *
 * Pure information, no interactive bits — keeps the path to first
 * dictation short. Users can return to settings for advanced surfaces
 * (Transforms, Dictionary, etc.) once they're past onboarding.
 */

import React from "react";
import { useTranslation } from "react-i18next";
import HandyTextLogo from "../icons/HandyTextLogo";
import { useSettings } from "../../hooks/useSettings";

interface FirstDictationStepProps {
  onComplete: () => void;
}

const FirstDictationStep: React.FC<FirstDictationStepProps> = ({
  onComplete,
}) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const bindings = getSetting("bindings") ?? {};
  const transcribeHotkey =
    bindings["transcribe"]?.current_binding ?? "Ctrl+Space";
  const commandModeHotkey =
    bindings["command_mode"]?.current_binding ?? "Ctrl+Alt+Space";

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-8 bg-background text-text">
      <div className="w-full max-w-2xl flex flex-col items-center gap-6">
        <HandyTextLogo />
        <h2 className="text-2xl font-semibold text-center">
          {t("onboarding.firstDictation.title")}
        </h2>
        <p className="text-mid-gray text-center max-w-md">
          {t("onboarding.firstDictation.description")}
        </p>

        <div className="w-full grid grid-cols-1 md:grid-cols-3 gap-4">
          <TutorialCard
            step="1"
            title={t("onboarding.firstDictation.cards.dictate.title")}
            body={t("onboarding.firstDictation.cards.dictate.body", {
              hotkey: transcribeHotkey,
            })}
          />
          <TutorialCard
            step="2"
            title={t("onboarding.firstDictation.cards.widget.title")}
            body={t("onboarding.firstDictation.cards.widget.body")}
          />
          <TutorialCard
            step="3"
            title={t("onboarding.firstDictation.cards.commandMode.title")}
            body={t("onboarding.firstDictation.cards.commandMode.body", {
              hotkey: commandModeHotkey,
            })}
          />
        </div>

        <button
          type="button"
          onClick={onComplete}
          className="mt-2 px-8 py-2 rounded-md bg-logo-primary text-white font-medium"
        >
          {t("onboarding.firstDictation.continueButton")}
        </button>
      </div>
    </div>
  );
};

interface TutorialCardProps {
  step: string;
  title: string;
  body: string;
}

const TutorialCard: React.FC<TutorialCardProps> = ({ step, title, body }) => {
  const { t } = useTranslation();
  return (
    <div className="bg-background-ui rounded-lg p-4 flex flex-col gap-2 h-full">
      <div className="text-xs font-semibold text-logo-primary">
        {t("onboarding.firstDictation.stepLabel", { step })}
      </div>
      <div className="text-base font-medium text-text">{title}</div>
      <div className="text-sm text-mid-gray">{body}</div>
    </div>
  );
};

export default FirstDictationStep;
