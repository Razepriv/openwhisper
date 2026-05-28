/* ═══════════════════════════════════════════════════════════════════════════
 * handy — landing page progressive enhancement
 *
 * Five jobs, no framework:
 *   1. Theme cycle (system → light → dark) persisted to localStorage
 *   2. OS detection — drives the hero download CTA + the inline section IV
 *      copy + the highlighted row in the all-installers table
 *   3. Footer year
 *   4. Scroll-triggered reveals via IntersectionObserver (.reveal class)
 *   5. Widget mockups — state machine cycling idle → listening → processing
 *
 * Everything is wrapped in feature checks. With JS disabled, the page still
 * renders: hero CTA falls back to the generic Windows download link, the
 * .reveal sections stay invisible-then-fade-in is replaced by always-visible
 * (we apply `is-visible` to all elements as the very first thing), and the
 * theme stays in @media (prefers-color-scheme).
 *
 * Reduced motion: scroll smoothing is disabled, reveal classes apply
 * instantly, and the widget mockup is held in a static "listening" state
 * (the most representative single frame).
 * ═══════════════════════════════════════════════════════════════════════ */

(function () {
  "use strict";

  const REDUCED_MOTION =
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /** ------------------------------------------------------------------ */
  /** 1. Theme cycle                                                      */
  /** ------------------------------------------------------------------ */

  /** @type {Array<"system" | "light" | "dark">} */
  const THEME_CYCLE = ["system", "light", "dark"];
  const THEME_STORAGE_KEY = "handy-landing-theme";

  /** @returns {"system" | "light" | "dark"} */
  function readTheme() {
    try {
      const stored = localStorage.getItem(THEME_STORAGE_KEY);
      if (stored === "light" || stored === "dark" || stored === "system") {
        return stored;
      }
    } catch (_) {
      /* localStorage blocked — fall through */
    }
    return "system";
  }

  /** @param {"system" | "light" | "dark"} theme */
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

  const THEME_ICONS = {
    system:
      '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>',
    light:
      '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/></svg>',
    dark: '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>',
  };

  const THEME_LABELS = {
    system: "System",
    light: "Light",
    dark: "Dark",
  };

  /** @param {"system" | "light" | "dark"} theme */
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
  /** 2. OS detection                                                     */
  /** ------------------------------------------------------------------ */

  /** @typedef {"windows" | "macos" | "linux" | null} OS */

  /** @returns {OS} */
  function detectOS() {
    const uaData = /** @type {any} */ (navigator).userAgentData;
    if (uaData && typeof uaData.platform === "string") {
      const p = uaData.platform.toLowerCase();
      if (p.includes("win")) return "windows";
      if (p.includes("mac")) return "macos";
      if (p.includes("linux")) return "linux";
    }
    const ua = (navigator.userAgent || "").toLowerCase();
    if (ua.includes("win")) return "windows";
    if (ua.includes("mac")) return "macos";
    if (ua.includes("linux") || ua.includes("x11")) return "linux";
    return null;
  }

  /**
   * Per-OS download metadata for the hero card. URLs point directly at
   * the release-asset path so `download` attribute fires immediately
   * instead of bouncing through the GitHub release page UI.
   */
  const VERSION = "0.2.0";
  const RELEASE_BASE = `https://github.com/Razepriv/openwhisper/releases/download/v${VERSION}`;

  /**
   * @type {Record<NonNullable<OS>, {label: string, file: string, size: string, sub: string}>}
   */
  const OS_DOWNLOADS = {
    windows: {
      label: "Download for Windows",
      file: `handy_${VERSION}_x64-setup.exe`,
      size: "~210 MB",
      sub: "Windows 10 / 11 (x64). SmartScreen → \"More info\" → \"Run anyway\" on first launch.",
    },
    macos: {
      label: "Download for macOS",
      file: `handy_${VERSION}_aarch64.dmg`,
      size: "~190 MB",
      sub: "Apple Silicon (M-series). Gatekeeper → System Settings → \"Open Anyway\".",
    },
    linux: {
      label: "Download for Linux",
      file: `handy_${VERSION}_amd64.AppImage`,
      size: "~205 MB",
      sub: "Portable AppImage — just chmod +x and run.",
    },
  };

  function applyOSPersonalisation() {
    const os = detectOS();

    // Hero card
    const heroEyebrow = document.querySelector("[data-os-eyebrow]");
    const heroLabel = document.querySelector("[data-os-label]");
    const heroSub = document.querySelector("[data-os-sub]");
    const primaryBtn = document.querySelector("[data-primary-download]");
    const primaryLabel = document.querySelector("[data-primary-label]");
    const primaryMeta = document.querySelector("[data-primary-meta]");

    if (os && OS_DOWNLOADS[os]) {
      const meta = OS_DOWNLOADS[os];
      if (heroEyebrow) heroEyebrow.textContent = `For your ${osDisplay(os)}`;
      if (heroLabel) heroLabel.textContent = `handy v${VERSION} for ${osDisplay(os)}`;
      if (heroSub) heroSub.textContent = meta.sub;
      if (primaryBtn) primaryBtn.setAttribute("href", `${RELEASE_BASE}/${meta.file}`);
      if (primaryLabel) primaryLabel.textContent = meta.label;
      if (primaryMeta) primaryMeta.textContent = `${meta.file} · ${meta.size}`;

      // Highlight the matching row in the all-installers table
      const card = document.querySelector(`.download-row[data-os="${os}"]`);
      if (card) card.setAttribute("data-os-active", "true");
    } else if (heroEyebrow) {
      heroEyebrow.textContent = "Pick your platform";
    }
  }

  /** @param {NonNullable<OS>} os */
  function osDisplay(os) {
    return os === "macos" ? "Mac" : os.charAt(0).toUpperCase() + os.slice(1);
  }

  /** ------------------------------------------------------------------ */
  /** 3. Year                                                             */
  /** ------------------------------------------------------------------ */

  function setYear() {
    const el = document.getElementById("year");
    if (el) el.textContent = String(new Date().getFullYear());
  }

  /** ------------------------------------------------------------------ */
  /** 4. Scroll reveals                                                   */
  /** ------------------------------------------------------------------ */

  function setupReveals() {
    const targets = document.querySelectorAll(".reveal");
    if (!targets.length) return;

    // With reduced motion, drop the reveal animation entirely — show
    // everything immediately. The CSS rule for prefers-reduced-motion
    // also handles this if JS is disabled.
    if (REDUCED_MOTION || typeof IntersectionObserver !== "function") {
      targets.forEach((t) => t.classList.add("is-visible"));
      return;
    }

    const observer = new IntersectionObserver(
      (entries, obs) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-visible");
            obs.unobserve(entry.target);
          }
        });
      },
      {
        // Reveal a little before the element enters fully — feels less
        // like a wall of pop-ins on long scrolls.
        rootMargin: "0px 0px -10% 0px",
        threshold: 0.05,
      },
    );

    targets.forEach((t) => observer.observe(t));
  }

  /** ------------------------------------------------------------------ */
  /** 5. Widget mockup state machine                                      */
  /** ------------------------------------------------------------------ */

  /**
   * Cycle the widget through its three real states. Timings chosen to
   * feel like a typical dictation: long-enough idle that you can read
   * the label, long-enough listening that the elapsed counter is
   * visibly counting, short processing burst.
   *
   * The label flips between "Ready · ⌃Space" (idle), the running
   * elapsed counter (listening), and "Transcribing…" (processing).
   *
   * Two synced widgets on the page: the small inline one in the lead
   * (Fig. 1) and the larger demo in Section II (Fig. 2). They run off
   * the same state so the animations feel intentional rather than
   * each strobing independently.
   */
  function setupWidgetCycle() {
    const small = document.getElementById("widgetMockup");
    const large = document.getElementById("widgetMockupLarge");
    if (!small && !large) return;

    if (REDUCED_MOTION) {
      // Hold a single representative frame — "listening", because the
      // pulse ring conveys the most about the product in one glance.
      [small, large].forEach((el) => {
        if (!el) return;
        el.setAttribute("data-state", "listening");
        const label = el.querySelector("[data-widget-label], [data-widget-label-lg]");
        if (label) label.textContent = "Listening…";
        const elapsed = el.querySelector("[data-widget-elapsed]");
        if (elapsed) elapsed.textContent = "0:02";
      });
      return;
    }

    const PHASES = [
      { state: "idle", label: "Ready · ⌃Space", duration: 2200 },
      { state: "listening", label: null /* generated */, duration: 3200 },
      { state: "processing", label: "Transcribing…", duration: 1700 },
    ];

    let phaseIdx = 0;
    let phaseStart = performance.now();
    let lastElapsedSecond = -1;

    /** @param {Element} el  @param {string} text */
    function setLabel(el, text) {
      const label = el.querySelector("[data-widget-label]") ||
        el.querySelector("[data-widget-label-lg]");
      if (label) label.textContent = text;
    }

    /** @param {Element} el  @param {string|null} text */
    function setElapsed(el, text) {
      const e = el.querySelector("[data-widget-elapsed]");
      if (e) e.textContent = text || "";
    }

    function applyPhase(idx) {
      const phase = PHASES[idx];
      [small, large].forEach((el) => {
        if (!el) return;
        el.setAttribute("data-state", phase.state);
        if (phase.state === "listening") {
          setLabel(el, "Listening");
          setElapsed(el, "0:00");
        } else {
          setLabel(el, phase.label || "");
          setElapsed(el, "");
        }
      });
      phaseStart = performance.now();
      lastElapsedSecond = 0;
    }

    function tick(now) {
      const phase = PHASES[phaseIdx];
      const elapsed = now - phaseStart;

      if (phase.state === "listening") {
        const secs = Math.floor(elapsed / 1000);
        if (secs !== lastElapsedSecond) {
          lastElapsedSecond = secs;
          const mm = Math.floor(secs / 60);
          const ss = String(secs % 60).padStart(2, "0");
          [small, large].forEach((el) => {
            if (el) setElapsed(el, `${mm}:${ss}`);
          });
        }
      }

      if (elapsed >= phase.duration) {
        phaseIdx = (phaseIdx + 1) % PHASES.length;
        applyPhase(phaseIdx);
      }

      requestAnimationFrame(tick);
    }

    applyPhase(0);
    requestAnimationFrame(tick);
  }

  /** ------------------------------------------------------------------ */
  /** Boot                                                                */
  /** ------------------------------------------------------------------ */

  function init() {
    applyTheme(readTheme());
    applyOSPersonalisation();
    setYear();
    setupReveals();
    setupWidgetCycle();

    const toggle = document.getElementById("themeToggle");
    if (toggle) toggle.addEventListener("click", cycleTheme);

    // Keep the toggle's label in sync if the user is in "system" mode
    // and the OS theme flips while the page is open.
    if (window.matchMedia) {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = () => {
        if (readTheme() === "system") updateThemeButton("system");
      };
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
