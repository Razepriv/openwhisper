/**
 * Scratchpad / Notes — local markdown notes editor.
 *
 * Backend: `managers/notes.rs` + Tauri commands (list_notes / get_note /
 * create_note / update_note / delete_note). Stored as JSON on disk,
 * never synced.
 */

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

import { SettingsGroup } from "../../ui/SettingsGroup";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Textarea } from "../../ui/Textarea";

interface Note {
  id: string;
  title: string;
  body: string;
  created_at: number;
  updated_at: number;
}

interface DraftNote {
  title: string;
  body: string;
}

const EMPTY_DRAFT: DraftNote = { title: "", body: "" };

export const NotesSettings: React.FC = () => {
  const { t } = useTranslation();
  const [notes, setNotes] = useState<Note[]>([]);
  const [draft, setDraft] = useState<DraftNote>(EMPTY_DRAFT);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<Note[]>("list_notes");
      // Newest first by updated_at desc.
      list.sort((a, b) => b.updated_at - a.updated_at);
      setNotes(list);
    } catch (err) {
      toast.error(t("settings.notes.errors.load"), {
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
    const title = draft.title.trim();
    if (!title) {
      toast.error(t("settings.notes.errors.titleRequired"));
      return;
    }
    try {
      if (editingId) {
        await invoke("update_note", {
          id: editingId,
          title,
          body: draft.body,
        });
      } else {
        await invoke("create_note", { title, body: draft.body });
      }
      setDraft(EMPTY_DRAFT);
      setEditingId(null);
      await refresh();
    } catch (err) {
      toast.error(t("settings.notes.errors.save"), {
        description: String(err),
      });
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_note", { id });
      if (editingId === id) {
        setEditingId(null);
        setDraft(EMPTY_DRAFT);
      }
      await refresh();
    } catch (err) {
      toast.error(t("settings.notes.errors.delete"), {
        description: String(err),
      });
    }
  };

  const startEdit = (note: Note) => {
    setEditingId(note.id);
    setDraft({ title: note.title, body: note.body });
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title={t("settings.notes.title")}
        description={t("settings.notes.description")}
      >
        <div className="p-4 space-y-3">
          <Input
            placeholder={t("settings.notes.titlePlaceholder")}
            value={draft.title}
            onChange={(e) =>
              setDraft({ ...draft, title: e.currentTarget.value })
            }
            className="w-full"
          />
          <Textarea
            placeholder={t("settings.notes.bodyPlaceholder")}
            value={draft.body}
            onChange={(e) =>
              setDraft({ ...draft, body: e.currentTarget.value })
            }
            className="w-full min-h-[140px]"
          />
          <div className="flex gap-2">
            <Button variant="primary" size="sm" onClick={handleSave}>
              {editingId
                ? t("settings.notes.update")
                : t("settings.notes.create")}
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
        title={t("settings.notes.listTitle", { count: notes.length })}
      >
        {loading ? (
          <p className="p-4 text-sm text-mid-gray">{t("common.loading")}</p>
        ) : notes.length === 0 ? (
          <p className="p-4 text-sm text-mid-gray">
            {t("settings.notes.empty")}
          </p>
        ) : (
          notes.map((note) => (
            <div
              key={note.id}
              className="flex items-start justify-between gap-3 p-4"
            >
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{note.title}</p>
                <p className="text-xs text-mid-gray mt-1 line-clamp-2 whitespace-pre-wrap">
                  {note.body}
                </p>
                <p className="text-xs text-mid-gray/70 mt-1">
                  {new Date(note.updated_at * 1000).toLocaleString()}
                </p>
              </div>
              <div className="flex gap-2 shrink-0">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => startEdit(note)}
                >
                  {t("common.edit")}
                </Button>
                <Button
                  variant="danger"
                  size="sm"
                  onClick={() => handleDelete(note.id)}
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

export default NotesSettings;
