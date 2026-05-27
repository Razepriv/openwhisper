/**
 * Transforms settings — hotkey-bound, user-defined post-dictation rewrites.
 *
 * Backend: `transforms.rs` + Tauri commands in `commands/openwhisper.rs`
 * (list_transforms / add_transform / update_transform / delete_transform
 * / preview_transform_prompt).
 *
 * Hotkey strings are interpreted by `shortcut/mod.rs` — same format as
 * the global Push-to-talk shortcut config (e.g. "Ctrl+Alt+1").
 */

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

import { SettingsGroup } from "../../ui/SettingsGroup";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Textarea } from "../../ui/Textarea";

interface TransformBinding {
  id: string;
  name: string;
  prompt: string;
  hotkey: string | null;
}

interface DraftTransform {
  name: string;
  prompt: string;
  hotkey: string;
}

const EMPTY_DRAFT: DraftTransform = {
  name: "",
  prompt: "",
  hotkey: "",
};

export const TransformsSettings: React.FC = () => {
  const { t } = useTranslation();
  const [transforms, setTransforms] = useState<TransformBinding[]>([]);
  const [draft, setDraft] = useState<DraftTransform>(EMPTY_DRAFT);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [preview, setPreview] = useState<string | null>(null);
  const [previewSample, setPreviewSample] = useState(
    "Hello world, this is a test.",
  );

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<TransformBinding[]>("list_transforms");
      setTransforms(list);
    } catch (err) {
      toast.error(t("settings.transforms.errors.load"), {
        description: String(err),
      });
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleSave = async () => {
    const name = draft.name.trim();
    const prompt = draft.prompt.trim();
    if (!name || !prompt) {
      toast.error(t("settings.transforms.errors.required"));
      return;
    }
    const hotkey = draft.hotkey.trim() || null;

    try {
      if (editingId) {
        await invoke("update_transform", {
          id: editingId,
          name,
          prompt,
          hotkey,
        });
      } else {
        const id = `tf_${Date.now().toString(36)}_${Math.random()
          .toString(36)
          .slice(2, 8)}`;
        await invoke("add_transform", { id, name, prompt, hotkey });
      }
      setDraft(EMPTY_DRAFT);
      setEditingId(null);
      await refresh();
    } catch (err) {
      toast.error(t("settings.transforms.errors.save"), {
        description: String(err),
      });
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_transform", { id });
      if (editingId === id) {
        setEditingId(null);
        setDraft(EMPTY_DRAFT);
      }
      await refresh();
    } catch (err) {
      toast.error(t("settings.transforms.errors.delete"), {
        description: String(err),
      });
    }
  };

  const startEdit = (transform: TransformBinding) => {
    setEditingId(transform.id);
    setDraft({
      name: transform.name,
      prompt: transform.prompt,
      hotkey: transform.hotkey ?? "",
    });
  };

  const previewPrompt = async (id: string) => {
    try {
      const rendered = await invoke<string>("preview_transform_prompt", {
        id,
        text: previewSample,
      });
      setPreview(rendered);
    } catch (err) {
      toast.error(t("settings.transforms.errors.preview"), {
        description: String(err),
      });
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title={t("settings.transforms.title")}
        description={t("settings.transforms.description")}
      >
        <div className="p-4 space-y-3">
          <Input
            placeholder={t("settings.transforms.namePlaceholder")}
            value={draft.name}
            onChange={(e) =>
              setDraft({ ...draft, name: e.currentTarget.value })
            }
            className="w-full"
            maxLength={80}
          />
          <Textarea
            placeholder={t("settings.transforms.promptPlaceholder")}
            value={draft.prompt}
            onChange={(e) =>
              setDraft({ ...draft, prompt: e.currentTarget.value })
            }
            className="w-full"
            maxLength={4000}
          />
          <Input
            placeholder={t("settings.transforms.hotkeyPlaceholder")}
            value={draft.hotkey}
            onChange={(e) =>
              setDraft({ ...draft, hotkey: e.currentTarget.value })
            }
            className="w-full"
            maxLength={60}
          />
          <div className="flex gap-2">
            <Button variant="primary" size="sm" onClick={handleSave}>
              {editingId
                ? t("settings.transforms.update")
                : t("settings.transforms.add")}
            </Button>
            {editingId && (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  setEditingId(null);
                  setDraft(EMPTY_DRAFT);
                }}
              >
                {t("common.cancel")}
              </Button>
            )}
          </div>
        </div>
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.transforms.listTitle", {
          count: transforms.length,
        })}
      >
        {loading ? (
          <p className="p-4 text-sm text-mid-gray">{t("common.loading")}</p>
        ) : transforms.length === 0 ? (
          <p className="p-4 text-sm text-mid-gray">
            {t("settings.transforms.empty")}
          </p>
        ) : (
          transforms.map((transform) => (
            <div
              key={transform.id}
              className="flex items-start justify-between gap-3 p-4"
            >
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium">{transform.name}</p>
                <p className="text-xs text-mid-gray mt-1 line-clamp-2">
                  {transform.prompt}
                </p>
                {transform.hotkey && (
                  <p className="text-xs font-mono text-logo-primary mt-1">
                    {transform.hotkey}
                  </p>
                )}
              </div>
              <div className="flex gap-2 shrink-0">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => previewPrompt(transform.id)}
                >
                  {t("settings.transforms.preview")}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => startEdit(transform)}
                >
                  {t("common.edit")}
                </Button>
                <Button
                  variant="danger"
                  size="sm"
                  onClick={() => handleDelete(transform.id)}
                >
                  {t("common.delete")}
                </Button>
              </div>
            </div>
          ))
        )}
      </SettingsGroup>

      {preview !== null && (
        <SettingsGroup title={t("settings.transforms.previewTitle")}>
          <div className="p-4 space-y-3">
            <Input
              placeholder={t("settings.transforms.previewSamplePlaceholder")}
              value={previewSample}
              onChange={(e) => setPreviewSample(e.currentTarget.value)}
              className="w-full"
            />
            <pre className="text-xs whitespace-pre-wrap bg-mid-gray/10 rounded-md p-3 max-h-64 overflow-auto">
              {preview}
            </pre>
            <Button variant="ghost" size="sm" onClick={() => setPreview(null)}>
              {t("common.close")}
            </Button>
          </div>
        </SettingsGroup>
      )}
    </div>
  );
};

export default TransformsSettings;
