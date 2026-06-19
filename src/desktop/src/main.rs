// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent_detection;
mod desktop_detection;
mod onboarding;
mod startkit;
mod tray;

use tauri::{AppHandle, Manager, Runtime};

use startkit::StartkitRunState;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: &'static str,
    os: &'static str,
    arch: &'static str,
}

/// Whether the app is currently in onboarding mode.
pub struct OnboardingActive(pub std::sync::atomic::AtomicBool);

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    }
}

#[tauri::command]
async fn rescan_agent_entries() -> Result<agent_detection::AgentDetectionFile, String> {
    let catalog = agent_detection::source_catalog().map_err(|error| error.to_string())?;
    agent_detection::scan_agents(&catalog)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn rescan_desktop_app_entries() -> Result<desktop_detection::DesktopAppDetectionFile, String>
{
    Ok(desktop_detection::scan_desktop_apps().await)
}

#[tauri::command]
fn get_desktop_app_entries() -> Option<desktop_detection::DesktopAppDetectionFile> {
    None
}

/// Open a trusted HTTP URL in the user's default external browser.
#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    // Minimal guard: only allow http/https schemes. Prevents a rogue
    // caller from asking us to execute `file://` or `javascript:` URIs.
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!("refused to open non-http URL: {url}"));
    }
    open::that(&url)
        .map(|_| ())
        .map_err(|e| format!("failed to open url: {e}"))
}

#[tauri::command]
fn set_ui_locale<R: Runtime>(app: AppHandle<R>, locale: String) -> Result<(), String> {
    tray::set_ui_locale(&app, &locale)
}

fn main() {
    common::logging::init();

    let onboarding_needed = false;

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tracing::info!(
                "[VibeWbz] ⚠️  Another instance tried to start, focusing existing window"
            );
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .manage(OnboardingActive(std::sync::atomic::AtomicBool::new(
            onboarding_needed,
        )))
        .manage(StartkitRunState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            rescan_agent_entries,
            rescan_desktop_app_entries,
            get_desktop_app_entries,
            open_external_url,
            set_ui_locale,
            onboarding::get_settings,
            onboarding::finish_onboarding,
            onboarding::list_agents,
            onboarding::scan_agent_install_status,
            onboarding::check_agent_updates,
            startkit::startkit_manifest,
            startkit::startkit_plan,
            startkit::startkit_scan,
            startkit::start_startkit_install,
            startkit::cancel_startkit_install,
        ])
        .setup(move |app| {
                tray::setup(app)?;

                // Show the window immediately; this build is a local setup UI.
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }

                Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building VibeWbz")
        .run(|_app, event| {
            // On app exit — whether via Cmd-Q, dock quit, window close, or
            // tray Quit — synchronously SIGKILL every registered child
            // process. This is the last line of defense against orphaned
            // plugin/agent processes; the graceful stop paths in
            // RunningDaemon::stop also run but may be skipped entirely on
            // abrupt exit (e.g. signal-driven shutdown before the async
            // runtime has been able to drain its tasks).
            if let tauri::RunEvent::Exit = event {
                common::process::registry::ChildRegistry::global().kill_all();
                common::previews::shutdown_kill_all_ports();
            }
        });
}
