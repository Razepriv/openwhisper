/**
 * Voice Profile dashboard — analytics over local transcription history.
 *
 * Backend: `managers/insights.rs` + Tauri command `get_voice_profile` in
 * `commands/openwhisper.rs`. 100 % local — never aggregates to a server.
 */

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

import { SettingsGroup } from "../../ui/SettingsGroup";
import { Button } from "../../ui/Button";

interface TopWord {
  word: string;
  count: number;
}

interface DailyWordCount {
  date: string;
  words: number;
}

interface HourlyWordCount {
  hour: number;
  words: number;
}

interface VoiceProfile {
  total_words: number;
  total_sessions: number;
  estimated_wpm: number | null;
  top_words: TopWord[];
  daily_word_counts: DailyWordCount[];
  hourly_word_counts: HourlyWordCount[];
  current_streak_days: number;
  longest_streak_days: number;
}

export const InsightsSettings: React.FC = () => {
  const { t } = useTranslation();
  const [profile, setProfile] = useState<VoiceProfile | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invoke<VoiceProfile>("get_voice_profile");
      setProfile(data);
    } catch (err) {
      toast.error(t("settings.insights.errors.load"), {
        description: String(err),
      });
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const maxDaily = Math.max(1, ...(profile?.daily_word_counts.map((d) => d.words) ?? [0]));
  const maxHourly = Math.max(1, ...(profile?.hourly_word_counts.map((h) => h.words) ?? [0]));

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title={t("settings.insights.title")}
        description={t("settings.insights.description")}
      >
        <div className="p-4">
          <Button variant="secondary" size="sm" onClick={refresh}>
            {t("settings.insights.refresh")}
          </Button>
        </div>
      </SettingsGroup>

      {loading && (
        <p className="text-sm text-mid-gray text-center">{t("common.loading")}</p>
      )}

      {!loading && profile && (
        <>
          <SettingsGroup title={t("settings.insights.summary")}>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 p-4">
              <Stat
                label={t("settings.insights.totalWords")}
                value={profile.total_words.toLocaleString()}
              />
              <Stat
                label={t("settings.insights.totalSessions")}
                value={profile.total_sessions.toLocaleString()}
              />
              <Stat
                label={t("settings.insights.wpm")}
                value={profile.estimated_wpm?.toString() ?? "—"}
              />
              <Stat
                label={t("settings.insights.currentStreak", {
                  count: profile.current_streak_days,
                })}
                value={profile.current_streak_days.toString()}
              />
            </div>
            <div className="px-4 pb-4 text-xs text-mid-gray">
              {t("settings.insights.longestStreak", {
                count: profile.longest_streak_days,
              })}
            </div>
          </SettingsGroup>

          {profile.top_words.length > 0 && (
            <SettingsGroup title={t("settings.insights.topWordsTitle")}>
              <div className="p-4 space-y-2">
                {profile.top_words.map((tw) => (
                  <div
                    key={tw.word}
                    className="flex items-center justify-between text-sm"
                  >
                    <span className="font-mono">{tw.word}</span>
                    <span className="text-mid-gray">{tw.count}</span>
                  </div>
                ))}
              </div>
            </SettingsGroup>
          )}

          <SettingsGroup title={t("settings.insights.last30DaysTitle")}>
            <div className="p-4">
              <div className="flex items-end gap-[2px] h-24">
                {profile.daily_word_counts
                  .slice()
                  .reverse()
                  .map((day) => (
                    <div
                      key={day.date}
                      title={`${day.date}: ${day.words}`}
                      className="flex-1 bg-logo-primary/60 hover:bg-logo-primary rounded-sm transition-colors"
                      style={{
                        height: `${(day.words / maxDaily) * 100}%`,
                        minHeight: day.words > 0 ? "2px" : "0",
                      }}
                    />
                  ))}
              </div>
              <p className="text-xs text-mid-gray mt-2">
                {t("settings.insights.dailyHint")}
              </p>
            </div>
          </SettingsGroup>

          <SettingsGroup title={t("settings.insights.hourlyTitle")}>
            <div className="p-4">
              <div className="flex items-end gap-[2px] h-24">
                {profile.hourly_word_counts.map((hr) => (
                  <div
                    key={hr.hour}
                    title={`${hr.hour}:00 — ${hr.words}`}
                    className="flex-1 bg-logo-primary/60 hover:bg-logo-primary rounded-sm transition-colors"
                    style={{
                      height: `${(hr.words / maxHourly) * 100}%`,
                      minHeight: hr.words > 0 ? "2px" : "0",
                    }}
                  />
                ))}
              </div>
              <p className="text-xs text-mid-gray mt-2">
                {t("settings.insights.hourlyHint")}
              </p>
            </div>
          </SettingsGroup>
        </>
      )}
    </div>
  );
};

interface StatProps {
  label: string;
  value: string;
}

function Stat({ label, value }: StatProps) {
  return (
    <div className="rounded-md bg-mid-gray/10 p-3 text-center">
      <p className="text-2xl font-semibold tabular-nums">{value}</p>
      <p className="text-xs text-mid-gray mt-1">{label}</p>
    </div>
  );
}

export default InsightsSettings;
