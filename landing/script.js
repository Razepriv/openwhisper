/* ---------------------------------------------------------------------------
 * handy landing — tiny progressive-enhancement layer
 *
 * Three jobs, no framework:
 *   1. Theme toggle (system → light → dark → system)
 *   2. OS detection for the hero "Download for your OS" CTA
 *   3. Year in the footer
 *
 * Everything is wrapped in feature checks so the page still renders fine if
 * JS is disabled — the hero button just falls back to the generic Releases
 * page, the theme stays in `prefers-color-scheme`, and the year shows the
 * server-rendered "2026" string.
 * ------------------------------------------------------------------------- */

(function () {
  "use strict";

  /** ------------------------------------------------------------------ */
  /** Theme toggle: system → light → dark → system                       */
  /** ------------------------------------------------------------------ */

  /** @type {Array<"system" | "light" | "dark">} */
  const THEME_CYCLE = ["system", "light", "dark"];
  const THEME_STORAGE_KEY = "handy-landing-theme";

  /**
   * Read stored preference, defaulting to "system".
   * @returns {"system" | "light" | "dark"}
   */
  function readTheme() {
    try {
      const stored = localStorage.getItem(THEME_STORAGE_KEY);
      if (stored === "light" || stored === "dark" || stored === "system") {
        return stored;
      }
    } catch (_) {
      // localStorage blocked (private browsing, file://, etc.) — fall through
    }
    return "system";
  }

  /**
   * Persist preference + apply it to <html>.
   * - "system" → remove data-theme attribute, let CSS @media handle it
   * - "light" / "dark" → set data-theme attribute, overriding @media
   *
   * @param {"system" | "light" | "dark"} theme
   */
  function applyTheme(theme) {
    const root = document.documentElement;
    if (theme === "system") {
      root.removeAttribute("data-theme");
    } else {
      root.setAttribute("data-theme", theme);
    }
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch (_) {
      /* ignore */
    }
    updateThemeButton(theme);
  }

  /** Inline SVG icons keyed by mode — small enough to ship without bloat. */
  const THEME_ICONS = {
    system:
      '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>',
    light:
      '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/></svg>',
    dark: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>',
  };

  const THEME_LABELS = {
    system: "System",
    light: "Light",
    dark: "Dark",
  };

  /**
   * Sync the toggle button's icon + label to the current mode.
   * @param {"system" | "light" | "dark"} theme
   */
  function updateThemeButton(theme) {
    const iconEl = document.querySelector("[data-theme-icon]");
    const labelEl = document.querySelector("[data-theme-label]");
    if (iconEl) iconEl.innerHTML = THEME_ICONS[theme] || THEME_ICONS.system;
    if (labelEl) labelEl.textContent = THEME_LABELS[theme] || "System";
  }

  function cycleTheme() {
    const current = readTheme();
    const idx = THEME_CYCLE.indexOf(current);
    const next = THEME_CYCLE[(idx + 1) % THEME_CYCLE.length];
    applyTheme(next);
  }

  /** ------------------------------------------------------------------ */
  /** OS detection — used only to relabel the primary CTA                */
  /** ------------------------------------------------------------------ */

  /** @returns {"windows" | "macos" | "linux" | null} */
  function detectOS() {
    // navigator.userAgentData is the modern, narrow surface; fall back to
    // userAgent regex matching for browsers that don't expose it yet.
    const uaData = /** @type {any} */ (navigator).userAgentData;
    if (uaData && typeof uaData.platform === "string") {
      const p = uaData.platform.toLowerCase();
      if (p.includes("win")) return "windows";
      if (p.includes("mac")) return "macos";
      if (p.includes("linux")) return "linux";
    }

    const ua = (navigator.userAgent || "").toLowerCase();
    // iPad on iPadOS 13+ identifies as Mac — leave it as macOS, the user
    // will see the warning that desktop builds aren't iPad-compatible.
    if (ua.includes("win")) return "windows";
    if (ua.includes("mac")) return "macos";
    if (ua.includes("linux") || ua.includes("x11")) return "linux";
    return null;
  }

  /** Apply the OS-aware label to the hero primary download button. */
  function applyOSLabel() {
    const os = detectOS();
    const labelEl = document.querySelector("[data-download-label]");
    if (!labelEl || !os) return;

    const LABELS = {
      windows: "Download for Windows",
      macos: "Download for macOS",
      linux: "Download for Linux",
    };
    labelEl.textContent = LABELS[os];

    // Highlight the matching download card further down the page so the
    // user can find the exact asset for their system without scanning.
    const card = document.querySelector(`.download-card[data-os="${os}"]`);
    if (card) card.setAttribute("data-os-active", "true");
  }

  /** ------------------------------------------------------------------ */
  /** Year                                                                */
  /** ------------------------------------------------------------------ */

  function setYear() {
    const el = document.getElementById("year");
    if (el) el.textContent = String(new Date().getFullYear());
  }

  /** ------------------------------------------------------------------ */
  /** Boot                                                                */
  /** ------------------------------------------------------------------ */

  function init() {
    applyTheme(readTheme());
    applyOSLabel();
    setYear();

    const toggle = document.getElementById("themeToggle");
    if (toggle) toggle.addEventListener("click", cycleTheme);

    // If the user is in "system" mode, react to OS-level dark-mode changes
    // so the toggle label stays accurate. We don't override `data-theme`
    // here — CSS handles the swap via `@media` automatically.
    if (window.matchMedia) {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = () => {
        if (readTheme() === "system") updateThemeButton("system");
      };
      // Older Safari uses addListener / removeListener.
      if (typeof mq.addEventListener === "function") {
        mq.addEventListener("change", handler);
      } else if (typeof mq.addListener === "function") {
        mq.addListener(handler);
      }
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
