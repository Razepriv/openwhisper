import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { Slider } from "../ui/Slider";
import { useSettings } from "../../hooks/useSettings";

interface FloatingWidgetProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * OpenWhisper Phase UI.batch — settings surface for the persistent
 * floating widget (Wispr Flow's "Flow Bar" equivalent). When enabled,
 * the overlay stays visible at all times and acts as a click target
 * for dictation in addition to its mid-recording role.
 *
 * Implementation notes:
 * - `floating_widget_enabled` / `floating_widget_opacity` are new
 *   fields on `AppSettings`. They exist on the Rust side and are
 *   persisted, but the auto-generated `bindings.ts` only refreshes when
 *   `tauri dev` runs — so we cast through `as never` to silence the
 *   stale type without disabling type-checking on the rest of the file.
 * - Opacity is clamped to [0.2, 1.0] backend-side; the slider matches
 *   so the UI never lets a user set something the backend would reject.
 */
export const FloatingWidget: React.FC<FloatingWidgetProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    // Cast through `as never` so we can read fields that exist in the
    // Rust AppSettings but aren't yet reflected in `bindings.ts`. Safe
    // because the field is always present at runtime — serde default
    // kicks in for legacy settings files that predate the addition.
    const widgetEnabled =
      (getSetting("floating_widget_enabled" as never) as
        | boolean
        | undefined) ?? true;
    const widgetOpacity =
      (getSetting("floating_widget_opacity" as never) as
        | number
        | undefined) ?? 0.9;

    return (
      <div className="flex flex-col gap-2">
        <ToggleSwitch
          checked={widgetEnabled}
          onChange={(enabled) =>
            updateSetting(
              "floating_widget_enabled" as never,
              enabled as never,
            )
          }
          isUpdating={isUpdating("floating_widget_enabled")}
          label={t("settings.advanced.floatingWidget.title")}
          description={t("settings.advanced.floatingWidget.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
        {widgetEnabled && (
          <Slider
            value={widgetOpacity}
            onChange={(value: number) =>
              updateSetting(
                "floating_widget_opacity" as never,
                value as never,
              )
            }
            min={0.2}
            max={1.0}
            step={0.05}
            label={t("settings.advanced.floatingWidget.opacity.title")}
            description={t(
              "settings.advanced.floatingWidget.opacity.description",
            )}
            descriptionMode={descriptionMode}
            grouped={grouped}
            formatValue={(value) => `${Math.round(value * 100)}%`}
          />
        )}
      </div>
    );
  },
);
