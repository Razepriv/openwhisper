import React from "react";
import ReactDOM from "react-dom/client";
import { platform } from "@tauri-apps/plugin-os";
import { commands } from "@/bindings";
import App from "./App";

/**
 * handy Phase Final.UI — apply the persisted theme preference BEFORE
 * React mounts so the initial paint already uses the correct palette
 * (avoids a flash of white when the user prefers dark mode).
 *
 * We set `<html data-theme="light"|"dark">`. When `theme === "system"`
 * we remove the attribute entirely so the @media (prefers-color-scheme)
 * rule in App.css takes over.
 */
async function applyPersistedTheme(): Promise<void> {
  try {
    const result = await commands.getAppSettings();
    if (result.status !== "ok") return;
    // `theme` is a new field; cast through `unknown` because bindings.ts
    // hasn't been regenerated yet (it's stale until the next `tauri dev`).
    const pref = (result.data as unknown as { theme?: string }).theme;
    if (pref === "light" || pref === "dark") {
      document.documentElement.setAttribute("data-theme", pref);
    } else {
      // "system" or missing — strip the override and let CSS handle it.
      document.documentElement.removeAttribute("data-theme");
    }
  } catch {
    // Settings backend might not be wired yet on a fresh install; the
    // CSS fallback (prefers-color-scheme) covers us.
  }
}

/**
 * Helper to update the diagnostic boot splash injected by index.html.
 * Safe to call when the splash has already been removed — does nothing.
 */
function setBootStatus(text: string): void {
  const el = document.getElementById("boot-status");
  if (el) el.textContent = text;
}

function showBootError(msg: string): void {
  const status = document.getElementById("boot-status");
  if (status) status.textContent = "Failed to start — see error below";
  const err = document.getElementById("boot-error");
  if (err) {
    err.style.display = "block";
    err.textContent = (err.textContent || "") + msg + "\n";
  }
}

function removeBootSplash(): void {
  // Remove the boot splash now that React has taken over rendering.
  // The error sink wired up in index.html stays armed regardless —
  // any later runtime error still surfaces via the global event
  // listeners painting into a re-anchored #boot-error box.
  document.getElementById("boot-splash")?.remove();
}

try {
  setBootStatus("Setting up platform...");
  // Set platform before render so CSS can scope per-platform (e.g. scrollbar styles)
  document.documentElement.dataset.platform = platform();

  setBootStatus("Loading i18n...");
  // Initialize i18n (dynamic import deferred to top-level for tree-shaking,
  // but we still wrap the global side-effect imports in try/catch via
  // outer scope to catch any unhandled boot errors).
  // eslint-disable-next-line @typescript-eslint/no-require-imports
} catch (e) {
  showBootError("Platform / i18n setup crashed:\n" + String(e));
}

// Initialize i18n (must be a static import for Vite tree-shaking + side effects)
import "./i18n";

// Initialize model store (loads models and sets up event listeners)
import { useModelStore } from "./stores/modelStore";

try {
  setBootStatus("Initializing model store...");
  useModelStore.getState().initialize();
} catch (e) {
  showBootError("Model store init crashed:\n" + String(e));
}

// Kick off theme application in the background — don't block React mount
// on it. The CSS fallback (prefers-color-scheme) means the user sees a
// sensible theme even before this resolves.
applyPersistedTheme();

try {
  setBootStatus("Mounting React...");
  const root = document.getElementById("root");
  if (!root) {
    showBootError("React root element #root was not found in the DOM.");
  } else {
    ReactDOM.createRoot(root).render(
      <React.StrictMode>
        <App />
      </React.StrictMode>,
    );
    // Give React a tick to render, then remove the diagnostic splash.
    setTimeout(removeBootSplash, 100);
  }
} catch (e) {
  showBootError("React mount crashed:\n" + String(e));
}
