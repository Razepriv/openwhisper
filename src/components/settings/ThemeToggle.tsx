import React from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";

interface ThemeToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

type ThemePreference = "system" | "light" | "dark";

/**
 * Theme picker — system / light / dark.
 *
 * Phase Final.UI of the handy redesign. The selection is persisted into
 * `AppSettings.theme` (Rust side) and ALSO applied immediately to the
 * `<html data-theme="...">` attribute so the UI re-themes without a
 * round-trip through the backend. `main.tsx` re-applies on next boot.
 *
 * Uses `as never` casts because the new `theme` field isn't yet present
 * in the auto-generated `bindings.ts` — same workaround we used for the
 * floating-widget settings (the field exists at runtime; bindings refresh
 * on the next `tauri dev` run).
 */
export const ThemeToggle: React.FC<ThemeToggleProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const current = ((getSetting("theme" as never) as
      | ThemePreference
      | undefined) ?? "system") as ThemePreference;

    const options = [
      { value: "system", label: t("settings.advanced.theme.options.system") },
      { value: "light", label: t("settings.advanced.theme.options.light") },
      { value: "dark", label: t("settings.advanced.theme.options.dark") },
    ];

    const handleChange = (value: string): void => {
      const next = value as ThemePreference;
      // Apply immediately for snappy UX — backend round-trip catches up.
      if (next === "system") {
        document.documentElement.removeAttribute("data-theme");
      } else {
        document.documentElement.setAttribute("data-theme", next);
      }
      // Optimistically update the local store so other consumers (and
      // the dropdown's own selected value) reflect the change without
      // waiting for the backend.
      updateSetting("theme" as never, next as never);
      // Persist via the Tauri command directly — `bindings.ts` doesn't
      // expose `changeThemeSetting` yet (auto-regenerated on next
      // `tauri dev`). Errors are non-fatal: the local override still
      // applies for this session.
      invoke("change_theme_setting", { theme: next }).catch((err) => {
        console.warn("Failed to persist theme preference:", err);
      });
    };

    return (
      <SettingContainer
        title={t("settings.advanced.theme.title")}
        description={t("settings.advanced.theme.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <Dropdown
          options={options}
          selectedValue={current}
          onSelect={handleChange}
          disabled={isUpdating("theme")}
        />
      </SettingContainer>
    );
  },
);
