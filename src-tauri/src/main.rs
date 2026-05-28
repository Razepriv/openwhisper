// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use openwhisper_lib::CliArgs;

fn main() {
    let cli_args = CliArgs::parse();

    #[cfg(target_os = "linux")]
    {
        // DMABUF renderer causes crashes on various GPU/display server configurations
        // See: https://github.com/tauri-apps/tauri/issues/9394
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // OpenWhisper Phase Final.Web — workaround for WebView2 white-screen bug.
    //
    // On certain AMD GPU + driver combinations (and reportedly some Intel iGPU
    // configurations), WebView2's DirectComposition path fails to composite
    // the rendered HTML to the visible window, leaving the user with a pure
    // white screen. We confirmed this experimentally: the HTML loads, JS
    // executes, React mounts — but `PrintWindow(SW_RENDERFULLCONTENT)` captures
    // pure white, matching what users see. The chrome_debug.log shows:
    //   ERROR ui/gl/direct_composition_support.cc:617
    //     AMD VideoProcessorGetOutputExtension failed: Unspecified error
    //
    // The reliable fix is to disable GPU compositing for WebView2. Performance
    // cost is negligible for a settings UI that's mostly static — and a working
    // app beats a fast invisible one. If a future WebView2 release fixes the
    // underlying composition issue, removing this is a one-line revert.
    //
    // Users who know their GPU works fine can override by setting their own
    // `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` env var before launching;
    // we only set ours when no override is present.
    #[cfg(target_os = "windows")]
    {
        const ENV_KEY: &str = "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS";
        // GPU compositing off (root cause of the white-screen bug).
        // ANGLE disabled too — on Intel/AMD iGPUs the ANGLE-on-D3D11 path
        // hits the same composition crash via a different route.
        const ARGS: &str = "--disable-gpu --disable-features=UseAngle";
        match std::env::var_os(ENV_KEY) {
            Some(existing) if !existing.is_empty() => {
                // Respect user-supplied args verbatim — don't append, since
                // some flags conflict (e.g. user might explicitly want GPU on).
            }
            _ => {
                std::env::set_var(ENV_KEY, ARGS);
            }
        }
    }

    openwhisper_lib::run(cli_args)
}
