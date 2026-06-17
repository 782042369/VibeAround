//! System tray (Tray-First UX). Tauri 2.10 API.
//! Native menu with quick actions; "Show Window" opens the main webview window.
//! During onboarding, service-related items are disabled until `onboarding-complete` fires.

use std::{collections::BTreeMap, sync::atomic::Ordering};

use serde::{Deserialize, Serialize};
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{TrayIcon, TrayIconBuilder},
    App, AppHandle, Listener, Manager, Runtime,
};

use crate::OnboardingActive;

pub(crate) const LAUNCH_CONFIG_CHANGED_EVENT: &str = "launch-config-changed";

const TRAY_ID: &str = "main";
const MAIN_WINDOW_LABEL: &str = "main";
const MENU_LAUNCH_DEFAULT: &str = "launch_default";
const MENU_SHOW_WINDOW: &str = "show_window";
const MENU_QUIT: &str = "quit";
const MENU_LAUNCH_DIRECT_PREFIX: &str = "launch_direct:";
const MENU_LAUNCH_PROFILE_PREFIX: &str = "launch_profile:";
const SETTINGS_UI_LOCALE: &str = "ui_locale";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UiLocale {
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

impl UiLocale {
    fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en_us" => Some(Self::En),
            "zh" | "zh-cn" | "zh_cn" | "zh-hans" | "zh_hans" => Some(Self::ZhCn),
            _ => None,
        }
    }

    fn as_settings_value(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
        }
    }

    fn text(self, key: TrayText) -> &'static str {
        match self {
            Self::En => key.en(),
            Self::ZhCn => match key {
                TrayText::ShowWindow => "显示窗口",
                TrayText::QuickLaunch => "快速启动",
                TrayText::LaunchWithoutProfile => "不使用 Profile 启动",
                TrayText::Bridge => "代理",
                TrayText::Quit => "退出",
            },
        }
    }
}

#[derive(Clone, Copy)]
enum TrayText {
    ShowWindow,
    QuickLaunch,
    LaunchWithoutProfile,
    Bridge,
    Quit,
}

impl TrayText {
    fn en(self) -> &'static str {
        match self {
            Self::ShowWindow => "Show Window",
            Self::QuickLaunch => "Quick Launch",
            Self::LaunchWithoutProfile => "Launch Without Profile",
            Self::Bridge => "bridge",
            Self::Quit => "Quit",
        }
    }
}

pub fn setup<R: Runtime>(app: &App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let menu = build_menu(&app_handle)?;

    // Embed the tray icon bytes at compile time so the shipped binary
    // doesn't depend on any filesystem path being present at runtime.
    //
    // Using `env!("CARGO_MANIFEST_DIR")` + `Image::from_path` as we did
    // previously bakes the *build machine's* absolute source path into
    // the binary. On every other machine `Image::from_path` returns
    // `Err`, `tray::setup` returns that error, Tauri treats setup failure
    // as fatal, and the app crashes at launch with an "application quit
    // unexpectedly" dialog.
    const TRAY_ICON_PNG: &[u8] = include_bytes!("../icons/32x32.png");
    let icon = Image::from_bytes(TRAY_ICON_PNG)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip("VibeWbz")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            MENU_LAUNCH_DEFAULT => {
                if let Err(e) = crate::profiles::profiles_launch_default() {
                    tracing::warn!("[tray] failed to launch default agent: {}", e);
                }
            }
            MENU_SHOW_WINDOW => {
                if let Some(w) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                    let _ = w.unminimize();
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            MENU_QUIT => {
                app.exit(0);
            }
            id if id.starts_with(MENU_LAUNCH_DIRECT_PREFIX) => {
                if let Err(e) = handle_direct_launch_menu(id) {
                    tracing::warn!("[tray] failed to direct launch menu item '{}': {}", id, e);
                }
            }
            id if id.starts_with(MENU_LAUNCH_PROFILE_PREFIX) => {
                if let Err(e) = handle_profile_launch_menu(id) {
                    tracing::warn!("[tray] failed to launch profile menu item '{}': {}", id, e);
                }
            }
            _ => {}
        })
        .build(app)?;

    // Listen for onboarding-complete -> rebuild menu items with launch enabled.
    let app_handle_onboarding = app_handle.clone();
    app.listen("onboarding-complete", move |_| {
        if let Err(e) = rebuild_menu(&app_handle_onboarding) {
            tracing::warn!("[tray] failed to rebuild after onboarding: {}", e);
        }
    });

    // Profile/default changes alter the launch menu tree.
    let app_handle_launch_config = app_handle.clone();
    app.listen(LAUNCH_CONFIG_CHANGED_EVENT, move |_| {
        if let Err(e) = rebuild_menu(&app_handle_launch_config) {
            tracing::warn!("[tray] failed to rebuild launch menu: {}", e);
        }
    });

    Ok(())
}

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let locale = current_locale();
    let is_onboarding = is_onboarding(app);
    let launch_enabled = !is_onboarding;

    let show_item =
        MenuItemBuilder::with_id(MENU_SHOW_WINDOW, locale.text(TrayText::ShowWindow)).build(app)?;
    let launch_default_item =
        MenuItemBuilder::with_id(MENU_LAUNCH_DEFAULT, locale.text(TrayText::QuickLaunch))
            .enabled(launch_enabled)
            .build(app)?;
    let direct_launch_menu = build_direct_launch_submenu(app, launch_enabled, locale)?;
    let profile_menus = build_agent_profile_submenus(app, launch_enabled, locale)?;
    let quit_item = MenuItemBuilder::with_id(MENU_QUIT, locale.text(TrayText::Quit)).build(app)?;

    let mut builder = MenuBuilder::new(app)
        .item(&show_item)
        .separator()
        .item(&launch_default_item)
        .item(&direct_launch_menu);
    for profile_menu in &profile_menus {
        builder = builder.item(profile_menu);
    }

    builder.separator().item(&quit_item).build()
}

fn build_direct_launch_submenu<R: Runtime>(
    app: &AppHandle<R>,
    launch_enabled: bool,
    locale: UiLocale,
) -> tauri::Result<tauri::menu::Submenu<R>> {
    let mut builder = SubmenuBuilder::with_id(
        app,
        "direct_launch",
        locale.text(TrayText::LaunchWithoutProfile),
    )
    .enabled(launch_enabled);

    for agent in common::resources::AGENTS
        .iter()
        .filter(|agent| agent.supports_current_platform())
    {
        let item = MenuItemBuilder::with_id(
            format!("{}{}", MENU_LAUNCH_DIRECT_PREFIX, agent.id),
            menu_text(&agent.display_name),
        )
        .enabled(launch_enabled)
        .build(app)?;
        builder = builder.item(&item);
    }

    builder.build()
}

#[derive(Debug)]
struct AgentProfileMenuGroup {
    agent_id: &'static str,
    label: &'static str,
    entries: Vec<AgentProfileMenuEntry>,
}

#[derive(Debug)]
struct AgentProfileMenuEntry {
    profile_id: String,
    label: String,
    uses_bridge_label: bool,
}

fn build_agent_profile_submenus<R: Runtime>(
    app: &AppHandle<R>,
    launch_enabled: bool,
    locale: UiLocale,
) -> tauri::Result<Vec<tauri::menu::Submenu<R>>> {
    let profiles = crate::profiles::ordered_profiles();
    let agent_prefs = common::agent_state::read_prefs();
    let profile_connections =
        common::profiles::connections::merged_profile_connections(&agent_prefs);

    let groups = agent_profile_menu_groups(&profiles, &profile_connections);
    let mut out = Vec::new();

    for group in groups {
        let mut agent_builder = SubmenuBuilder::with_id(
            app,
            format!("launch_profile_agent_menu:{}", group.agent_id),
            menu_text(group.label),
        )
        .enabled(launch_enabled);

        for entry in group.entries {
            let menu_id = format!(
                "{}{}:{}",
                MENU_LAUNCH_PROFILE_PREFIX, entry.profile_id, group.agent_id
            );
            let label = profile_entry_menu_label(&entry, locale);
            let item = MenuItemBuilder::with_id(menu_id, menu_text(&label))
                .enabled(launch_enabled)
                .build(app)?;
            agent_builder = agent_builder.item(&item);
        }

        let agent_menu = agent_builder.build()?;
        out.push(agent_menu);
    }

    Ok(out)
}

fn agent_profile_menu_groups(
    profiles: &[common::profiles::ProfileDef],
    connections: &common::agent_state::ProfileConnectionPreferences,
) -> Vec<AgentProfileMenuGroup> {
    if profiles.is_empty() {
        return Vec::new();
    }

    let provider_counts = profile_provider_counts(profiles);
    let mut groups: BTreeMap<&'static str, AgentProfileMenuGroup> = BTreeMap::new();
    for profile in profiles {
        let profile_label = profile_menu_title(profile, &provider_counts);
        let targets = common::profiles::connections::launch_targets_for_profile_with_connections(
            profile,
            connections,
        );
        for target in targets {
            let group = groups
                .entry(target.id)
                .or_insert_with(|| AgentProfileMenuGroup {
                    agent_id: target.id,
                    label: target.label,
                    entries: Vec::new(),
                });
            group.entries.push(AgentProfileMenuEntry {
                profile_id: profile.id.clone(),
                label: profile_label.clone(),
                uses_bridge_label: target_uses_bridge_label(&target),
            });
        }
    }

    ordered_agent_profile_menu_groups(groups)
}

fn ordered_agent_profile_menu_groups(
    mut groups: BTreeMap<&'static str, AgentProfileMenuGroup>,
) -> Vec<AgentProfileMenuGroup> {
    let mut out = Vec::with_capacity(groups.len());
    for agent in common::resources::AGENTS.iter() {
        if let Some(group) = groups.remove(agent.id.as_str()) {
            out.push(group);
        }
    }
    out.extend(groups.into_values());
    out
}

fn target_uses_bridge_label(target: &common::profiles::connections::ProfileLaunchTarget) -> bool {
    target.bridge_target_api_type.is_some()
}

fn profile_entry_menu_label(entry: &AgentProfileMenuEntry, locale: UiLocale) -> String {
    if entry.uses_bridge_label {
        format!("{} ({})", entry.label, locale.text(TrayText::Bridge))
    } else {
        entry.label.clone()
    }
}

fn rebuild_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    if let Some(tray) = tray_icon(app) {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

pub(crate) fn set_ui_locale<R: Runtime>(app: &AppHandle<R>, locale: &str) -> Result<(), String> {
    let locale =
        UiLocale::from_str(locale).ok_or_else(|| format!("unsupported locale: {locale}"))?;
    common::config::update_settings_json(|root| {
        if !root.is_object() {
            *root = serde_json::json!({});
        }
        if let Some(obj) = root.as_object_mut() {
            obj.insert(
                SETTINGS_UI_LOCALE.to_string(),
                serde_json::Value::String(locale.as_settings_value().to_string()),
            );
        }
    })?;
    rebuild_menu(app).map_err(|e| e.to_string())
}

fn tray_icon<R: Runtime>(app: &AppHandle<R>) -> Option<TrayIcon<R>> {
    app.tray_by_id(TRAY_ID)
}

fn is_onboarding<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<OnboardingActive>()
        .map(|s| s.0.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn current_locale() -> UiLocale {
    let path = common::config::data_dir().join("settings.json");
    let Ok(data) = std::fs::read_to_string(path) else {
        return UiLocale::En;
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&data) else {
        return UiLocale::En;
    };
    root.get(SETTINGS_UI_LOCALE)
        .and_then(|value| value.as_str())
        .and_then(UiLocale::from_str)
        .unwrap_or(UiLocale::En)
}

fn handle_profile_launch_menu(menu_id: &str) -> Result<(), String> {
    let payload = menu_id
        .strip_prefix(MENU_LAUNCH_PROFILE_PREFIX)
        .ok_or_else(|| format!("invalid profile launch menu id: {menu_id}"))?;
    let (profile_id, agent_id) = payload
        .split_once(':')
        .ok_or_else(|| format!("invalid profile launch menu id: {menu_id}"))?;
    crate::profiles::profiles_launch(profile_id.to_string(), agent_id.to_string())
}

fn handle_direct_launch_menu(menu_id: &str) -> Result<(), String> {
    let agent_id = menu_id
        .strip_prefix(MENU_LAUNCH_DIRECT_PREFIX)
        .ok_or_else(|| format!("invalid direct launch menu id: {menu_id}"))?;
    crate::profiles::profiles_launch_direct(agent_id.to_string())
}

fn profile_provider_counts(
    profiles: &[common::profiles::ProfileDef],
) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for profile in profiles {
        *counts.entry(profile.provider.clone()).or_insert(0) += 1;
    }
    counts
}

fn profile_menu_title(
    profile: &common::profiles::ProfileDef,
    provider_counts: &std::collections::BTreeMap<String, usize>,
) -> String {
    let provider = common::profiles::catalog::get(&profile.provider);
    let provider_label = provider
        .map(|catalog| catalog.label.as_str())
        .unwrap_or(profile.provider.as_str());
    let needs_profile_label = provider_counts
        .get(&profile.provider)
        .copied()
        .unwrap_or_default()
        > 1
        || profile.provider == "custom";
    if needs_profile_label {
        format!("{} - {}", provider_label, profile.label)
    } else {
        provider_label.to_string()
    }
}

fn menu_text(text: &str) -> String {
    text.replace('&', "&&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use common::profiles::connections::ProfileLaunchTarget;
    use common::profiles::schema::{AuthMode, ProfileDef, ProviderSettings};

    fn profile(id: &str, label: &str, api_types: &[&str]) -> ProfileDef {
        ProfileDef {
            id: id.to_string(),
            label: label.to_string(),
            provider: "custom".to_string(),
            auth_mode: AuthMode::ApiKey,
            api_types: api_types.iter().map(|value| (*value).to_string()).collect(),
            credentials: BTreeMap::new(),
            overrides: BTreeMap::new(),
            use_settings_proxy: false,
            provider_settings: ProviderSettings::default(),
        }
    }

    #[test]
    fn bridge_launch_targets_use_bridge_label() {
        let target = ProfileLaunchTarget {
            id: "codex",
            label: "Codex",
            api_type: "openai-responses".to_string(),
            bridge_target_api_type: Some("anthropic".to_string()),
        };

        assert!(target_uses_bridge_label(&target));
    }

    #[test]
    fn native_launch_targets_do_not_use_bridge_label() {
        let target = ProfileLaunchTarget {
            id: "claude",
            label: "Claude Code",
            api_type: "anthropic".to_string(),
            bridge_target_api_type: None,
        };

        assert!(!target_uses_bridge_label(&target));
    }

    #[test]
    fn profile_entry_label_marks_bridge_routes() {
        let entry = AgentProfileMenuEntry {
            profile_id: "deepseek".to_string(),
            label: "DeepSeek".to_string(),
            uses_bridge_label: true,
        };

        assert_eq!(
            profile_entry_menu_label(&entry, UiLocale::En),
            "DeepSeek (bridge)"
        );
        assert_eq!(
            profile_entry_menu_label(&entry, UiLocale::ZhCn),
            "DeepSeek (代理)"
        );
    }

    #[test]
    fn profile_launch_groups_are_agent_first_with_profile_entries() {
        let profiles = vec![
            profile("anthropic-profile", "Anthropic Profile", &["anthropic"]),
            profile("openai-profile", "OpenAI Profile", &["openai-responses"]),
        ];
        let connections = [(
            "anthropic-profile".to_string(),
            [(
                "codex".to_string(),
                common::agent_state::ProfileConnectionPreference {
                    selected_api_type: Some("openai-responses".to_string()),
                    bridge: [(
                        "openai-responses".to_string(),
                        common::agent_state::ProfileBridgePreference {
                            enabled: true,
                            target_api_type: Some("anthropic".to_string()),
                            ..Default::default()
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect();

        let groups = agent_profile_menu_groups(&profiles, &connections);
        let codex = groups
            .iter()
            .find(|group| group.agent_id == "codex")
            .expect("codex group");

        assert_eq!(codex.label, "Codex");
        assert_eq!(codex.entries.len(), 2);
        assert_eq!(codex.entries[0].profile_id, "anthropic-profile");
        assert!(codex.entries[0].uses_bridge_label);
        assert_eq!(codex.entries[1].profile_id, "openai-profile");
        assert!(!codex.entries[1].uses_bridge_label);
    }
}
