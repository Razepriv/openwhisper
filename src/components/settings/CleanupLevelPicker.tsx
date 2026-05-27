/**
 * 4-tier Auto-Cleanup picker — Phase 1.9 UI.
 *
 * Backend: `settings.rs::CleanupLevel` enum (None / Light / Medium / High)
 * + `system_prompt_for_cleanup_level()` which feeds the LLM. Mirrors
 * Wispr Flow's Auto Cleanup setting.
 *
 * Mounted inside AdvancedSettings alongside the existing post-process
 * toggle. Persists via the same `change_*_setting` Tauri command pattern
 * used by every other settings widget.
 */

import React from "react";
import { useTranslation } from "react-i18next";

import { SettingContainer } from "../ui/SettingContainer";
import { Select } from "../ui/Select";
import { useSettings } from "../../hooks/useSettings";

type CleanupLevel = "none" | "light" | "medium" | "high";

interface CleanupLevelPickerProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  disabled?: boolean;
}

export const CleanupLevelPicker: React.FC<CleanupLevelPickerProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
  disabled = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();

  // `cleanup_level` is added by the OpenWhisper Phase 1.9 commit. If the
  // user is on a settings file that pre-dates it, `serde(default)` lands
  // them on "light".
  //
  // The `as never` cast is needed until `bindings.ts` regenerates on the
  // next `bun run tauri dev`: until then TypeScript doesn't know the
  // new field exists. Runtime works either way because the field IS in
  // the persisted AppSettings JSON.
  const current =
    (getSetting("cleanup_level" as never) as unknown as
      | CleanupLevel
      | undefined) ?? "light";

  const options: Array<{ value: CleanupLevel; label: string }> = [
    { value: "none", label: t("settings.cleanupLevel.none") },
    { value: "light", label: t("settings.cleanupLevel.light") },
    { value: "medium", label: t("settings.cleanupLevel.medium") },
    { value: "high", label: t("settings.cleanupLevel.high") },
  ];

  return (
    <SettingContainer
      title={t("settings.cleanupLevel.title")}
      description={t("settings.cleanupLevel.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      disabled={disabled}
    >
      <Select
        value={current}
        onChange={(value) => {
          if (value !== null) {
            // Same `as never` rationale as the getSetting call above.
            updateSetting("cleanup_level" as never, value as never);
          }
        }}
        options={options}
        disabled={disabled}
      />
    </SettingContainer>
  );
};

export default CleanupLevelPicker;
