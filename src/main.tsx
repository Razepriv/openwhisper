import React from "react";
import ReactDOM from "react-dom/client";
import { platform } from "@tauri-apps/plugin-os";
import App from "./App";

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
