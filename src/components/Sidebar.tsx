import React from "react";
import { useTranslation } from "react-i18next";
import {
  Cog,
  FlaskConical,
  History,
  Info,
  Sparkles,
  Cpu,
  BookText,
  Scissors,
  Wand2,
  BarChart3,
  StickyNote,
} from "lucide-react";
import HandyTextLogo from "./icons/HandyTextLogo";
import HandyHand from "./icons/HandyHand";
import { useSettings } from "../hooks/useSettings";
import {
  GeneralSettings,
  AdvancedSettings,
  HistorySettings,
  DebugSettings,
  AboutSettings,
  PostProcessingSettings,
  ModelsSettings,
  SnippetsSettings,
  DictionarySettings,
  TransformsSettings,
  InsightsSettings,
  NotesSettings,
} from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  [key: string]: any;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: (settings: any) => boolean;
}

export const SECTIONS_CONFIG = {
  general: {
    labelKey: "sidebar.general",
    icon: HandyHand,
    component: GeneralSettings,
    enabled: () => true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
  },
  // OpenWhisper-added sections — backend wired via commands/openwhisper.rs.
  dictionary: {
    labelKey: "sidebar.dictionary",
    icon: BookText,
    component: DictionarySettings,
    enabled: () => true,
  },
  snippets: {
    labelKey: "sidebar.snippets",
    icon: Scissors,
    component: SnippetsSettings,
    enabled: () => true,
  },
  transforms: {
    labelKey: "sidebar.transforms",
    icon: Wand2,
    component: TransformsSettings,
    enabled: () => true,
  },
  insights: {
    labelKey: "sidebar.insights",
    icon: BarChart3,
    component: InsightsSettings,
    enabled: () => true,
  },
  notes: {
    labelKey: "sidebar.notes",
    icon: StickyNote,
    component: NotesSettings,
    enabled: () => true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: Cog,
    component: AdvancedSettings,
    enabled: () => true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  postprocessing: {
    labelKey: "sidebar.postProcessing",
    icon: Sparkles,
    component: PostProcessingSettings,
    enabled: (settings) => settings?.post_process_enabled ?? false,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode ?? false,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const availableSections = Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  return (
    <div className="flex flex-col w-44 h-full border-e border-border bg-background items-stretch px-3 py-4">
      {/* Brand mark — editorial wordmark + subtle separator below */}
      <div className="flex items-center justify-start px-2 pb-4 mb-4 border-b border-border">
        <HandyTextLogo width={70} />
      </div>

      {/* Nav list — each row is a tight pill with a clean active state.
          Active = subtle violet-soft background + left-edge accent bar,
          giving it the "selected article" feel without the heavy fill. */}
      <nav className="flex flex-col gap-px">
        {availableSections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;

          return (
            <button
              key={section.id}
              type="button"
              onClick={() => onSectionChange(section.id)}
              className={`group relative flex items-center gap-2.5 px-2.5 py-1.5 rounded-md text-left transition-all
                ${
                  isActive
                    ? "bg-accent-soft text-accent"
                    : "text-text-muted hover:bg-background-ui hover:text-text"
                }`}
              title={t(section.labelKey)}
            >
              {/* Left accent bar — only renders on the active row */}
              <span
                aria-hidden
                className={`absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-r-full transition-all ${
                  isActive ? "bg-accent" : "bg-transparent"
                }`}
              />
              <Icon width={16} height={16} className="shrink-0" />
              <span className="text-[13px] font-medium tracking-tight truncate">
                {t(section.labelKey)}
              </span>
            </button>
          );
        })}
      </nav>
    </div>
  );
};
