/**
 * Snippets settings — CRUD UI over the local snippets store.
 *
 * Backend: `managers/snippets.rs` + Tauri commands in
 * `commands/openwhisper.rs` (list_snippets / add_snippet / update_snippet
 * / delete_snippet / bulk_set_snippets).
 *
 * Wired into the main sidebar via `components/Sidebar.tsx::SECTIONS_CONFIG`.
 */

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

import { SettingsGroup } from "../../ui/SettingsGroup";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Textarea } from "../../ui/Textarea";

interface Snippet {
  id: string;
  trigger: string;
  expansion: string;
  casing_preserve: boolean;
}

interface DraftSnippet {
  trigger: string;
  expansion: string;
}

const EMPTY_DRAFT: DraftSnippet = { trigger: "", expansion: "" };

export const SnippetsSettings: React.FC = () => {
  const { t } = useTranslation();
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [draft, setDraft] = useState<DraftSnippet>(EMPTY_DRAFT);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<Snippet[]>("list_snippets");
      setSnippets(list);
    } catch (err) {
      toast.error(t("settings.snippets.errors.load"), {
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
    const trigger = draft.trigger.trim();
    const expansion = draft.expansion.trim();
    if (!trigger || !expansion) {
      toast.error(t("settings.snippets.errors.required"));
      return;
    }

    try {
      if (editingId) {
        await invoke("update_snippet", {
          id: editingId,
          trigger,
          expansion,
        });
      } else {
        const id = `snip_${Date.now().toString(36)}_${Math.random()
          .toString(36)
          .slice(2, 8)}`;
        await invoke("add_snippet", { id, trigger, expansion });
      }
      setDraft(EMPTY_DRAFT);
      setEditingId(null);
      await refresh();
    } catch (err) {
      toast.error(t("settings.snippets.errors.save"), {
        description: String(err),
      });
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_snippet", { id });
      if (editingId === id) {
        setEditingId(null);
        setDraft(EMPTY_DRAFT);
      }
      await refresh();
    } catch (err) {
      toast.error(t("settings.snippets.errors.delete"), {
        description: String(err),
      });
    }
  };

  const startEdit = (snippet: Snippet) => {
    setEditingId(snippet.id);
    setDraft({ trigger: snippet.trigger, expansion: snippet.expansion });
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title={t("settings.snippets.title")}
        description={t("settings.snippets.description")}
      >
        <div className="p-4 space-y-3">
          <Input
            placeholder={t("settings.snippets.triggerPlaceholder")}
            value={draft.trigger}
            onChange={(e) =>
              setDraft({ ...draft, trigger: e.currentTarget.value })
            }
            className="w-full"
            maxLength={60}
          />
          <Textarea
            placeholder={t("settings.snippets.expansionPlaceholder")}
            value={draft.expansion}
            onChange={(e) =>
              setDraft({ ...draft, expansion: e.currentTarget.value })
            }
            className="w-full"
            maxLength={4000}
            variant="compact"
          />
          <div className="flex gap-2">
            <Button variant="primary" size="sm" onClick={handleSave}>
              {editingId
                ? t("settings.snippets.update")
                : t("settings.snippets.add")}
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
        title={t("settings.snippets.listTitle", {
          count: snippets.length,
        })}
      >
        {loading ? (
          <p className="p-4 text-sm text-mid-gray">{t("common.loading")}</p>
        ) : snippets.length === 0 ? (
          <p className="p-4 text-sm text-mid-gray">
            {t("settings.snippets.empty")}
          </p>
        ) : (
          snippets.map((snippet) => (
            <div
              key={snippet.id}
              className="flex items-start justify-between gap-3 p-4"
            >
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium font-mono truncate">
                  {snippet.trigger}
                </p>
                <p className="text-xs text-mid-gray mt-1 line-clamp-2">
                  {snippet.expansion}
                </p>
              </div>
              <div className="flex gap-2 shrink-0">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => startEdit(snippet)}
                >
                  {t("common.edit")}
                </Button>
                <Button
                  variant="danger"
                  size="sm"
                  onClick={() => handleDelete(snippet.id)}
                >
                  {t("common.delete")}
                </Button>
              </div>
            </div>
          ))
        )}
      </SettingsGroup>
    </div>
  );
};

export default SnippetsSettings;
