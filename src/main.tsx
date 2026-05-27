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
  // Phase Final.Web extreme-diagnostic mode: the user is still seeing
  // white after every prior fix. To prove the HTML→JS→React path:
  // - The splash stays FULL-SCREEN and BRIGHT BLUE forever
  // - On React mount we paint a small GREEN ribbon at the top labeled
  //   "React mounted at <time>"
  // If the user sees blue + green ribbon → HTML + JS + React all work
  // and the React app is rendering, the issue is App.tsx returning
  // null / invisible content. If they see only BLUE → HTML loads but
  // React mount failed. If they see PURE WHITE → HTML never loaded.
  const splash = document.getElementById("boot-splash");
  if (!splash) return;
  const ribbon = document.createElement("div");
  ribbon.style.cssText =
    "position:fixed;top:0;left:0;right:0;background:#0c5;color:#000;font-family:Segoe UI,system-ui,sans-serif;font-size:13px;font-weight:600;padding:6px 12px;z-index:2147483647;pointer-events:none;text-align:center";
  ribbon.textContent = "React mounted at " + new Date().toLocaleTimeString();
  document.body.appendChild(ribbon);
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
