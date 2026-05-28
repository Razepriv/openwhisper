import { type FC, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../ui/SettingContainer";
import { Dropdown, type DropdownOption } from "../ui/Dropdown";
import { useSettings } from "../../hooks/useSettings";
import { commands } from "@/bindings";
import type {
  WhisperAcceleratorSetting,
  OrtAcceleratorSetting,
} from "@/bindings";

const ORT_LABELS: Record<OrtAcceleratorSetting, string> = {
  auto: "Auto",
  cpu: "CPU",
  cuda: "CUDA",
  directml: "DirectML",
  rocm: "ROCm",
};

interface AccelerationSelectorProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

/**
 * Whisper dropdown encodes accelerator + device in a single value:
 *   "auto"   → accelerator=auto,  gpu_device=-1
 *   "cpu"    → accelerator=cpu,   gpu_device=-1
 *   "gpu:0"  → accelerator=gpu,   gpu_device=0
 *   "gpu:1"  → accelerator=gpu,   gpu_device=1
 */
function encodeWhisperValue(
  accelerator: WhisperAcceleratorSetting,
  gpuDevice: number,
): string {
  if (accelerator === "cpu") return "cpu";
  if (accelerator === "gpu" && gpuDevice >= 0) return `gpu:${gpuDevice}`;
  return "auto";
}

function decodeWhisperValue(value: string): {
  accelerator: WhisperAcceleratorSetting;
  gpuDevice: number;
} {
  if (value === "cpu") return { accelerator: "cpu", gpuDevice: -1 };
  if (value.startsWith("gpu:")) {
    const id = parseInt(value.slice(4), 10);
    return { accelerator: "gpu", gpuDevice: id };
  }
  return { accelerator: "auto", gpuDevice: -1 };
}

export const AccelerationSelector: FC<AccelerationSelectorProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const [whisperOptions, setWhisperOptions] = useState<DropdownOption[]>([]);
  const [ortOptions, setOrtOptions] = useState<DropdownOption[]>([]);

  useEffect(() => {
    commands.getAvailableAccelerators().then((available) => {
      // Build combined Whisper options: Auto, [GPU devices, dedicated first], CPU.
      //
      // Sort order: dedicated GPUs first (descending VRAM), integrated GPUs after.
      // Reason: ggml enumerates Vulkan devices in driver-reported order, which on
      // most laptops puts the integrated iGPU at index 0 even when a discrete
      // dGPU is present. Showing the dedicated GPU first matches Wispr Flow's
      // behaviour and makes the "right" choice the first one the user sees.
      const devices = [...(available.gpu_devices ?? [])].sort((a, b) => {
        const kindRank = (k?: string) => (k === "dedicated" ? 0 : 1);
        const byKind = kindRank(a.kind) - kindRank(b.kind);
        if (byKind !== 0) return byKind;
        return (b.total_vram_mb ?? 0) - (a.total_vram_mb ?? 0);
      });

      const hasDedicated = devices.some((d) => d.kind === "dedicated");

      const opts: DropdownOption[] = [
        {
          value: "auto",
          // When a dedicated GPU is present, make it clear in the label that
          // Auto will pick it. When only integrated/CPU is available, keep
          // the plain "Auto" label.
          label: hasDedicated
            ? t(
                "settings.advanced.acceleration.gpuDevice.autoDedicated",
                "Auto (Dedicated GPU)",
              )
            : t("settings.advanced.acceleration.gpuDevice.auto"),
        },
      ];

      for (const dev of devices) {
        const vramLabel =
          dev.total_vram_mb >= 1024
            ? `${(dev.total_vram_mb / 1024).toFixed(1)} GB`
            : `${dev.total_vram_mb} MB`;
        // Append (Dedicated) / (Integrated) tag so multi-GPU users can tell
        // them apart even when both have similar names (some AMD laptops show
        // "AMD Radeon" for both the iGPU and the dGPU).
        const kindTag =
          dev.kind === "dedicated"
            ? ` — ${t("settings.advanced.acceleration.gpuDevice.dedicated", "Dedicated")}`
            : dev.kind === "integrated"
              ? ` — ${t("settings.advanced.acceleration.gpuDevice.integrated", "Integrated")}`
              : "";
        opts.push({
          value: `gpu:${dev.id}`,
          label: `${dev.name} (${vramLabel})${kindTag}`,
        });
      }

      opts.push({ value: "cpu", label: "CPU" });
      setWhisperOptions(opts);

      // ORT options (unchanged)
      const ortVals = available.ort.includes("auto")
        ? available.ort
        : ["auto", ...available.ort];
      setOrtOptions(
        ortVals.map((v) => ({
          value: v,
          label: ORT_LABELS[v as OrtAcceleratorSetting] ?? v,
        })),
      );
    });
  }, [t]);

  const currentAccelerator = getSetting("whisper_accelerator") ?? "auto";
  const currentGpuDevice = getSetting("whisper_gpu_device") ?? -1;
  const currentWhisper = encodeWhisperValue(
    currentAccelerator as WhisperAcceleratorSetting,
    currentGpuDevice as number,
  );
  const currentOrt = getSetting("ort_accelerator") ?? "auto";

  const handleWhisperChange = async (value: string) => {
    const { accelerator, gpuDevice } = decodeWhisperValue(value);
    await updateSetting("whisper_accelerator", accelerator);
    await updateSetting("whisper_gpu_device", gpuDevice);
  };

  return (
    <>
      <SettingContainer
        title={t("settings.advanced.acceleration.whisper.title")}
        description={t("settings.advanced.acceleration.whisper.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        layout="horizontal"
      >
        <Dropdown
          options={whisperOptions}
          selectedValue={currentWhisper}
          onSelect={handleWhisperChange}
          disabled={
            isUpdating("whisper_accelerator") ||
            isUpdating("whisper_gpu_device")
          }
        />
      </SettingContainer>
      {ortOptions.length > 2 && (
        <SettingContainer
          title={t("settings.advanced.acceleration.ort.title")}
          description={t("settings.advanced.acceleration.ort.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
          layout="horizontal"
        >
          <Dropdown
            options={ortOptions}
            selectedValue={currentOrt}
            onSelect={(value) =>
              updateSetting("ort_accelerator", value as OrtAcceleratorSetting)
            }
            disabled={isUpdating("ort_accelerator")}
          />
        </SettingContainer>
      )}
    </>
  );
};
