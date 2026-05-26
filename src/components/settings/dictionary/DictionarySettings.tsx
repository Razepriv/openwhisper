/**
 * Personal Dictionary settings — CRUD UI over the structured dictionary.
 *
 * Backend: `managers/dictionary.rs` + Tauri commands in
 * `commands/openwhisper.rs` (list_dictionary_entries / add_dictionary_entry
 * / update_dictionary_entry / delete_dictionary_entry).
 *
 * Entries feed `transcription.rs`'s `initial_prompt` for Whisper biasing;
 * starred entries float to the top of the bias-token budget.
 */

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Star } from "lucide-react";
import { toast } from "sonner";

import { SettingsGroup } from "../../ui/SettingsGroup";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

type DictionarySource = "manual" | "migrated" | "learned" | "imported";

interface DictionaryEntry {
  term: string;
  replacement: string | null;
  starred: boolean;
  source: DictionarySource;
}

interface DraftEntry {
  term: string;
  replacement: string;
  starred: boolean;
}

const EMPTY_DRAFT: DraftEntry = { term: "", replacement: "", starred: false };

export const DictionarySettings: React.FC = () => {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<DictionaryEntry[]>([]);
  const [draft, setDraft] = useState<DraftEntry>(EMPTY_DRAFT);
  const [editingTerm, setEditingTerm] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<DictionaryEntry[]>("list_dictionary_entries");
      setEntries(list);
    } catch (err) {
      toast.error(t("settings.dictionary.errors.load"), {
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
    const term = draft.term.trim();
    if (!term) {
      toast.error(t("settings.dictionary.errors.termRequired"));
      return;
    }
    const replacement = draft.replacement.trim();
    const payload = {
      term,
      replacement: replacement ? replacement : null,
      starred: draft.starred,
    };

    try {
      if (editingTerm) {
        await invoke("update_dictionary_entry", {
          term: editingTerm,
          input: payload,
        });
      } else {
        await invoke("add_dictionary_entry", { input: payload });
      }
      setDraft(EMPTY_DRAFT);
      setEditingTerm(null);
      await refresh();
    } catch (err) {
      toast.error(t("settings.dictionary.errors.save"), {
        description: String(err),
      });
    }
  };

  const handleDelete = async (term: string) => {
    try {
      await invoke("delete_dictionary_entry", { term });
      if (editingTerm === term) {
        setEditingTerm(null);
        setDraft(EMPTY_DRAFT);
      }
      await refresh();
    } catch (err) {
      toast.error(t("settings.dictionary.errors.delete"), {
        description: String(err),
      });
    }
  };

  const startEdit = (entry: DictionaryEntry) => {
    setEditingTerm(entry.term);
    setDraft({
      term: entry.term,
      replacement: entry.replacement ?? "",
      starred: entry.starred,
    });
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title={t("settings.dictionary.title")}
        description={t("settings.dictionary.description")}
      >
        <div className="p-4 space-y-3">
          <Input
            placeholder={t("settings.dictionary.termPlaceholder")}
            value={draft.term}
            onChange={(e) =>
              setDraft({ ...draft, term: e.currentTarget.value })
            }
            className="w-full"
            maxLength={60}
          />
          <Input
            placeholder={t("settings.dictionary.replacementPlaceholder")}
            value={draft.replacement}
            onChange={(e) =>
              setDraft({ ...draft, replacement: e.currentTarget.value })
            }
            className="w-full"
            maxLength={120}
          />
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={draft.starred}
              onChange={(e) =>
                setDraft({ ...draft, starred: e.currentTarget.checked })
              }
            />
            {t("settings.dictionary.starredLabel")}
          </label>
          <div className="flex gap-2">
            <Button variant="primary" size="sm" onClick={handleSave}>
              {editingTerm
                ? t("settings.dictionary.update")
                : t("settings.dictionary.add")}
            </Button>
            {editingTerm && (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  setEditingTerm(null);
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
        title={t("settings.dictionary.listTitle", { count: entries.length })}
      >
        {loading ? (
          <p className="p-4 text-sm text-mid-gray">{t("common.loading")}</p>
        ) : entries.length === 0 ? (
          <p className="p-4 text-sm text-mid-gray">
            {t("settings.dictionary.empty")}
          </p>
        ) : (
          entries.map((entry) => (
            <div
              key={entry.term}
              className="flex items-start justify-between gap-3 p-4"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  {entry.starred && (
                    <Star size={14} className="text-logo-primary shrink-0" />
                  )}
                  <p className="text-sm font-medium truncate">{entry.term}</p>
                </div>
                {entry.replacement && (
                  <p className="text-xs text-mid-gray mt-1">
                    → {entry.replacement}
                  </p>
                )}
                <p className="text-xs text-mid-gray/70 mt-0.5">
                  {t(`settings.dictionary.source.${entry.source}`)}
                </p>
              </div>
              <div className="flex gap-2 shrink-0">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => startEdit(entry)}
                >
                  {t("common.edit")}
                </Button>
                <Button
                  variant="danger"
                  size="sm"
                  onClick={() => handleDelete(entry.term)}
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

export default DictionarySettings;
