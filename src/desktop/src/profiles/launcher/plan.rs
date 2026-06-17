//! Launch plan builder.
//!
//! This module decides what should be launched. Platform modules only execute
//! the final plan, which keeps terminal-specific code away from profile,
//! bridge, and launch routing decisions.

#[cfg(not(test))]
use std::borrow::Cow;

use ::common::{agent as agent_integrations, profiles, resources};
use anyhow::{anyhow, Context};
use profiles::ProfileDef;

use super::common::LaunchPlan;
use super::{bridge, claude_desktop, codex_desktop};

const LOCAL_BRIDGE_NO_PROXY: &str = "localhost,127.0.0.1,::1,0.0.0.0,127.0.0.0/8";
const LOCAL_BRIDGE_PROXY_ENV_KEYS: &[&str] = &[
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];
const VIBEWBZ_LAUNCH_ID_ENV: &str = "VIBEWBZ_LAUNCH_ID";
const VIBEWBZ_LAUNCH_TARGET_ENV: &str = "VIBEWBZ_LAUNCH_TARGET";

enum LaunchTarget<'a> {
    Profile {
        profile: &'a ProfileDef,
        launch_target: &'a str,
    },
    Direct {
        agent_id: &'a str,
    },
}

pub(super) struct LaunchPlanBuilder<'a> {
    launch_id: String,
    target: Option<LaunchTarget<'a>>,
}

impl<'a> LaunchPlanBuilder<'a> {
    pub(super) fn new() -> Self {
        Self {
            launch_id: uuid::Uuid::new_v4().to_string(),
            target: None,
        }
    }

    pub(super) fn profile(mut self, profile: &'a ProfileDef, launch_target: &'a str) -> Self {
        self.target = Some(LaunchTarget::Profile {
            profile,
            launch_target,
        });
        self
    }

    pub(super) fn direct(mut self, agent_id: &'a str) -> Self {
        self.target = Some(LaunchTarget::Direct { agent_id });
        self
    }

    pub(super) fn build(self) -> anyhow::Result<LaunchPlan> {
        match self
            .target
            .as_ref()
            .ok_or_else(|| anyhow!("launch target is required"))?
        {
            LaunchTarget::Profile {
                profile,
                launch_target,
            } => self.build_profile_plan(profile, launch_target),
            LaunchTarget::Direct { agent_id } => self.build_direct_plan(agent_id),
        }
    }

    fn build_profile_plan(
        &self,
        profile: &ProfileDef,
        launch_target: &str,
    ) -> anyhow::Result<LaunchPlan> {
        let rendered = bridge::render_for_launch(profile, launch_target, &self.launch_id)?;

        self.build_rendered_profile_plan(profile, launch_target, rendered)
    }

    fn build_direct_plan(&self, agent_id: &str) -> anyhow::Result<LaunchPlan> {
        let agent = resources::agent_by_id(agent_id)
            .ok_or_else(|| anyhow!("agent '{}' not found in agents.json", agent_id))?;
        let workspace = crate::profiles::resolve_launch_workspace(agent_id)?;
        install_project_integrations_for_launch(agent_id, &workspace)?;

        if agent_id == "codex-desktop" {
            codex_desktop::cleanup_profile_overlay()
                .context("restore Codex Desktop config before direct launch")?;
        } else if agent_id == "claude-desktop" {
            claude_desktop::cleanup_profile_config()
                .context("restore Claude Desktop config before direct launch")?;
        }
        Ok(LaunchPlan {
            env: Vec::new(),
            command: direct_launch_command_for_agent(
                agent_id,
                &agent,
                agent.pty_command_for_current_platform(),
            ),
            args: Vec::new(),
            window_label: format!("{} (direct)", agent.display_name),
            workspace,
            macos_app_probe: macos_app_probe_for_direct_agent(agent_id, &agent),
            windows_process_probe: windows_process_probe_for_direct_agent(&agent),
            windows_executable_path: windows_executable_path_for_agent(agent_id),
        })
    }

    fn build_rendered_profile_plan(
        &self,
        profile: &ProfileDef,
        launch_target: &str,
        rendered: profiles::render::RenderedProfile,
    ) -> anyhow::Result<LaunchPlan> {
        let agent_id = profiles::runtime::agent_id_for(launch_target)?;
        let agent = resources::agent_by_id(agent_id)
            .ok_or_else(|| anyhow!("agent '{}' not found in agents.json", agent_id))?;
        let workspace = crate::profiles::resolve_launch_workspace(agent_id)?;
        install_project_integrations_for_launch(agent_id, &workspace)?;
        if agent_id == "codex-desktop" {
            let mut env = Vec::new();
            let mut args = Vec::new();
            append_vibewbz_launch_context_env(&mut env, profile, launch_target, &self.launch_id);
            if bridge::launch_uses_local_bridge(profile, launch_target)? {
                append_local_bridge_proxy_bypass_env(&mut env);
                args.extend(codex_desktop_local_bridge_args());
            }
            codex_desktop::apply_profile_overlay(profile, &self.launch_id, rendered)
                .with_context(|| format!("prepare Codex Desktop profile '{}'", profile.id))?;
            return Ok(LaunchPlan {
                env,
                command: direct_launch_command_for_agent(
                    agent_id,
                    &agent,
                    agent.pty_command_for_current_platform(),
                ),
                args,
                window_label: profile.label.clone(),
                workspace,
                macos_app_probe: macos_app_probe_for_direct_agent(agent_id, &agent),
                windows_process_probe: windows_process_probe_for_direct_agent(&agent),
                windows_executable_path: windows_executable_path_for_agent(agent_id),
            });
        }
        if agent_id == "claude-desktop" {
            let _ = rendered;
            claude_desktop::apply_profile_config(profile)
                .with_context(|| format!("prepare Claude Desktop profile '{}'", profile.id))?;
            let mut env = Vec::new();
            append_vibewbz_launch_context_env(&mut env, profile, launch_target, &self.launch_id);
            return Ok(LaunchPlan {
                env,
                command: direct_launch_command_for_agent(
                    agent_id,
                    &agent,
                    agent.pty_command_for_current_platform(),
                ),
                args: Vec::new(),
                window_label: profile.label.clone(),
                workspace,
                macos_app_probe: macos_app_probe_for_direct_agent(agent_id, &agent),
                windows_process_probe: windows_process_probe_for_direct_agent(&agent),
                windows_executable_path: windows_executable_path_for_agent(agent_id),
            });
        }
        let command_args = rendered.command_args.clone();
        let env = materialized_profile_env(profile, launch_target, &self.launch_id, rendered)?;

        Ok(LaunchPlan {
            env,
            command: launch_command_for_agent(agent_id, agent.pty_command_for_current_platform()),
            args: command_args,
            window_label: profile.label.clone(),
            workspace,
            macos_app_probe: None,
            windows_process_probe: None,
            windows_executable_path: None,
        })
    }
}

fn macos_app_probe_for_direct_agent(agent_id: &str, agent: &resources::AgentDef) -> Option<String> {
    if !cfg!(target_os = "macos") || !agent.direct_only {
        return None;
    }
    macos_configured_app_name(agent_id)
        .or_else(|| open_app_name(agent.pty_command_for_current_platform()))
}

fn open_app_name(command: &str) -> Option<String> {
    command
        .trim()
        .strip_prefix("open -a ")
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.trim_matches('"').to_string())
}

fn windows_process_probe_for_direct_agent(agent: &resources::AgentDef) -> Option<String> {
    if !cfg!(target_os = "windows") || !agent.direct_only {
        return None;
    }
    start_process_name(agent.pty_command_for_current_platform())
}

#[cfg(not(test))]
fn windows_executable_path_for_agent(agent_id: &str) -> Option<std::path::PathBuf> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    let prefs = ::common::agent_state::read_prefs();
    ::common::agent_state::resolve_agent_executable_path(&prefs, agent_id)
}

#[cfg(test)]
fn windows_executable_path_for_agent(_agent_id: &str) -> Option<std::path::PathBuf> {
    None
}

fn direct_launch_command_for_agent(
    agent_id: &str,
    agent: &resources::AgentDef,
    fallback_command: &str,
) -> String {
    if agent.direct_only {
        if let Some(command) = macos_configured_app_launch_command(agent_id) {
            return command;
        }
    }
    launch_command_for_agent(agent_id, fallback_command)
}

#[cfg(not(test))]
fn macos_configured_app_launch_command(agent_id: &str) -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let prefs = ::common::agent_state::read_prefs();
    let path = ::common::agent_state::resolve_agent_executable_path(&prefs, agent_id)?;
    if !path.exists() {
        return None;
    }
    let path = path.to_string_lossy();
    let escaped = shell_escape::unix::escape(Cow::Borrowed(path.as_ref()));
    Some(format!("open {escaped}"))
}

#[cfg(test)]
fn macos_configured_app_launch_command(_agent_id: &str) -> Option<String> {
    None
}

#[cfg(not(test))]
fn macos_configured_app_name(agent_id: &str) -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let prefs = ::common::agent_state::read_prefs();
    let path = ::common::agent_state::resolve_agent_executable_path(&prefs, agent_id)?;
    if path.extension().and_then(|extension| extension.to_str()) != Some("app") {
        return None;
    }
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
fn macos_configured_app_name(_agent_id: &str) -> Option<String> {
    None
}

fn start_process_name(command: &str) -> Option<String> {
    command
        .trim()
        .strip_prefix("Start-Process ")
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.trim_matches('"').to_string())
}

#[cfg(not(test))]
fn launch_command_for_agent(agent_id: &str, fallback_command: &str) -> String {
    ::common::agent_detection::resolve_agent_command(agent_id, fallback_command)
}

#[cfg(test)]
fn launch_command_for_agent(_agent_id: &str, fallback_command: &str) -> String {
    fallback_command.to_string()
}

fn materialized_profile_env(
    profile: &ProfileDef,
    launch_target: &str,
    launch_id: &str,
    rendered: profiles::render::RenderedProfile,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut env = profiles::runtime::materialize_env(&profile.id, rendered)?;
    if bridge::launch_uses_local_bridge(profile, launch_target)? {
        append_local_bridge_proxy_bypass_env(&mut env);
    } else {
        profiles::runtime::append_settings_proxy_env(profile, &mut env)?;
    }
    append_vibewbz_launch_context_env(&mut env, profile, launch_target, launch_id);
    Ok(env)
}

fn append_vibewbz_launch_context_env(
    env: &mut Vec<(String, String)>,
    profile: &ProfileDef,
    launch_target: &str,
    launch_id: &str,
) {
    env.retain(|(key, _)| {
        key != VIBEWBZ_LAUNCH_ID_ENV
            && key != agent_integrations::launch::VIBEWBZ_PROFILE_ID_ENV
            && key != VIBEWBZ_LAUNCH_TARGET_ENV
    });
    env.push((VIBEWBZ_LAUNCH_ID_ENV.to_string(), launch_id.to_string()));
    env.push((
        agent_integrations::launch::VIBEWBZ_PROFILE_ID_ENV.to_string(),
        profile.id.clone(),
    ));
    env.push((
        VIBEWBZ_LAUNCH_TARGET_ENV.to_string(),
        launch_target.to_string(),
    ));
}

fn install_project_integrations_for_launch(
    agent_id: &str,
    workspace: &std::path::Path,
) -> anyhow::Result<()> {
    let integration_agent_id = project_integration_agent_id(agent_id);
    agent_integrations::auto_install_project_integrations(integration_agent_id, workspace)
        .with_context(|| format!("install project integrations for {}", integration_agent_id))
}

fn project_integration_agent_id(agent_id: &str) -> &str {
    match agent_id {
        "claude-desktop" => "claude",
        "codex-desktop" => "codex",
        other => other,
    }
}

fn append_local_bridge_proxy_bypass_env(env: &mut Vec<(String, String)>) {
    env.retain(|(key, _)| !LOCAL_BRIDGE_PROXY_ENV_KEYS.contains(&key.as_str()));
    env.extend([
        ("HTTP_PROXY".to_string(), String::new()),
        ("HTTPS_PROXY".to_string(), String::new()),
        ("ALL_PROXY".to_string(), String::new()),
        ("http_proxy".to_string(), String::new()),
        ("https_proxy".to_string(), String::new()),
        ("all_proxy".to_string(), String::new()),
        ("NO_PROXY".to_string(), LOCAL_BRIDGE_NO_PROXY.to_string()),
        ("no_proxy".to_string(), LOCAL_BRIDGE_NO_PROXY.to_string()),
    ]);
}

fn codex_desktop_local_bridge_args() -> Vec<String> {
    if cfg!(target_os = "macos") {
        vec!["--args".to_string(), "--no-proxy-server".to_string()]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ::common::profiles::schema::{ApiTypeOverrides, AuthMode, ProfileDef, ProviderSettings};

    use super::*;

    impl<'a> LaunchPlanBuilder<'a> {
        fn with_launch_id(launch_id: &str) -> Self {
            Self {
                launch_id: launch_id.to_string(),
                target: None,
            }
        }
    }

    fn gateway_anthropic_profile() -> ProfileDef {
        ProfileDef {
            id: "gateway-test".to_string(),
            label: "VibeWbz Gateway Test".to_string(),
            provider: "custom".to_string(),
            auth_mode: AuthMode::ApiKey,
            api_types: vec!["anthropic".to_string()],
            credentials: [("api_key".to_string(), "test-key".to_string())]
                .into_iter()
                .collect(),
            overrides: [(
                "anthropic".to_string(),
                ApiTypeOverrides {
                    base_url: Some("http://ai.939593.xyz".to_string()),
                    model: Some("claude-sonnet-4-5".to_string()),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
            use_settings_proxy: false,
            provider_settings: ProviderSettings::default(),
        }
    }

    fn env_value<'a>(env: &'a [(String, String)], key: &str) -> Option<&'a str> {
        env.iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn local_bridge_proxy_bypass_env_clears_proxy_and_sets_no_proxy() {
        let mut env = vec![
            (
                "HTTP_PROXY".to_string(),
                "http://127.0.0.1:7897".to_string(),
            ),
            ("NO_PROXY".to_string(), "old.example".to_string()),
            ("OPENAI_API_KEY".to_string(), "test-key".to_string()),
        ];

        append_local_bridge_proxy_bypass_env(&mut env);

        assert_eq!(env_value(&env, "HTTP_PROXY"), Some(""));
        assert_eq!(env_value(&env, "HTTPS_PROXY"), Some(""));
        assert_eq!(env_value(&env, "ALL_PROXY"), Some(""));
        assert_eq!(env_value(&env, "http_proxy"), Some(""));
        assert_eq!(env_value(&env, "https_proxy"), Some(""));
        assert_eq!(env_value(&env, "all_proxy"), Some(""));
        assert_eq!(env_value(&env, "NO_PROXY"), Some(LOCAL_BRIDGE_NO_PROXY));
        assert_eq!(env_value(&env, "no_proxy"), Some(LOCAL_BRIDGE_NO_PROXY));
        assert_eq!(env_value(&env, "OPENAI_API_KEY"), Some("test-key"));
        for key in LOCAL_BRIDGE_PROXY_ENV_KEYS {
            assert_eq!(
                env.iter().filter(|(candidate, _)| candidate == key).count(),
                1,
                "{key} should be present exactly once"
            );
        }
    }

    #[test]
    fn codex_desktop_local_bridge_args_disable_macos_proxy() {
        let args = codex_desktop_local_bridge_args();
        if cfg!(target_os = "macos") {
            assert_eq!(args, vec!["--args", "--no-proxy-server"]);
        } else {
            assert!(args.is_empty());
        }
    }

    #[test]
    fn desktop_launches_install_companion_cli_integrations() {
        assert_eq!(project_integration_agent_id("codex-desktop"), "codex");
        assert_eq!(project_integration_agent_id("claude-desktop"), "claude");
    }

    #[test]
    fn direct_launch_plan_has_no_profile_env() {
        let plan = LaunchPlanBuilder::with_launch_id("launch-123")
            .direct("claude")
            .build()
            .expect("direct plan");

        assert_eq!(plan.command, "claude code --permission-mode acceptEdits");
        assert!(plan.args.is_empty());
        assert!(plan.env.is_empty());
        assert_eq!(plan.window_label, "Claude Code (direct)");
    }

    #[test]
    fn profile_launch_plan_adds_vibewbz_identity_env() {
        let profile = gateway_anthropic_profile();
        let plan = LaunchPlanBuilder::with_launch_id("launch-123")
            .profile(&profile, "claude")
            .build()
            .expect("profile plan");

        assert_eq!(plan.command, "claude code --permission-mode acceptEdits");
        assert_eq!(plan.window_label, "VibeWbz Gateway Test");
        assert!(plan
            .env
            .contains(&("VIBEWBZ_LAUNCH_ID".to_string(), "launch-123".to_string())));
        assert!(plan
            .env
            .contains(&("VIBEWBZ_PROFILE_ID".to_string(), "gateway-test".to_string())));
        assert!(plan
            .env
            .contains(&("VIBEWBZ_LAUNCH_TARGET".to_string(), "claude".to_string())));
    }

    #[test]
    fn claude_desktop_profile_plan_uses_3p_config_without_terminal_args() {
        let profile = gateway_anthropic_profile();
        let root = std::env::temp_dir().join(format!(
            "vibewbz-claude-desktop-plan-{}",
            uuid::Uuid::new_v4()
        ));
        let _guard = claude_desktop::set_test_user_data_dir(root.clone());
        let plan = LaunchPlanBuilder::with_launch_id("launch-123")
            .profile(&profile, "claude-desktop")
            .build()
            .expect("claude desktop profile plan");

        if cfg!(target_os = "windows") {
            assert_eq!(plan.command, "Start-Process Claude");
        } else {
            assert_eq!(plan.command, "open -a Claude");
        }
        assert!(plan.args.is_empty());
        assert_eq!(plan.window_label, "VibeWbz Gateway Test");
        assert!(plan
            .env
            .contains(&("VIBEWBZ_LAUNCH_ID".to_string(), "launch-123".to_string())));
        assert!(plan
            .env
            .contains(&("VIBEWBZ_PROFILE_ID".to_string(), "gateway-test".to_string())));
        assert!(plan.env.contains(&(
            "VIBEWBZ_LAUNCH_TARGET".to_string(),
            "claude-desktop".to_string()
        )));
        let meta_path = root.join("configLibrary").join("_meta.json");
        let meta: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&meta_path).expect("read Claude Desktop meta"),
        )
        .expect("parse Claude Desktop meta");
        let applied_id = meta
            .get("appliedId")
            .and_then(serde_json::Value::as_str)
            .expect("applied id");
        let applied: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                root.join("configLibrary")
                    .join(format!("{applied_id}.json")),
            )
            .expect("read Claude Desktop applied profile"),
        )
        .expect("parse Claude Desktop applied profile");
        assert_eq!(
            applied
                .get("inferenceProvider")
                .and_then(serde_json::Value::as_str),
            Some("gateway")
        );
        assert_eq!(
            applied
                .get("inferenceGatewayBaseUrl")
                .and_then(serde_json::Value::as_str),
            Some("http://127.0.0.1:12358/va/local-api/gateway-test/claude-anthropic/anthropic")
        );
        if cfg!(target_os = "macos") {
            assert_eq!(plan.macos_app_probe.as_deref(), Some("Claude"));
        }
        if cfg!(target_os = "windows") {
            assert_eq!(plan.windows_process_probe.as_deref(), Some("Claude"));
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn claude_desktop_direct_plan_restores_3p_config() {
        let profile = gateway_anthropic_profile();
        let root = std::env::temp_dir().join(format!(
            "vibewbz-claude-desktop-direct-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("configLibrary")).expect("create Claude config library");
        std::fs::write(
            root.join("configLibrary").join("_meta.json"),
            r#"{"appliedId":"default-id","entries":[{"id":"default-id","name":"Default"}]}"#,
        )
        .expect("write Claude meta");
        std::fs::write(
            root.join("claude_desktop_config.json"),
            r#"{"deploymentMode":"1p"}"#,
        )
        .expect("write Claude deployment config");

        let _guard = claude_desktop::set_test_user_data_dir(root.clone());
        LaunchPlanBuilder::with_launch_id("launch-123")
            .profile(&profile, "claude-desktop")
            .build()
            .expect("Claude Desktop profile plan");
        LaunchPlanBuilder::with_launch_id("launch-456")
            .direct("claude-desktop")
            .build()
            .expect("Claude Desktop direct plan");

        let meta: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("configLibrary").join("_meta.json"))
                .expect("read restored Claude meta"),
        )
        .expect("parse restored Claude meta");
        assert_eq!(
            meta.get("appliedId").and_then(serde_json::Value::as_str),
            Some("default-id")
        );
        assert_eq!(
            meta.get("entries")
                .and_then(serde_json::Value::as_array)
                .expect("entries")
                .len(),
            1
        );
        let deployment: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("claude_desktop_config.json"))
                .expect("read restored deployment config"),
        )
        .expect("parse restored deployment config");
        assert_eq!(
            deployment
                .get("deploymentMode")
                .and_then(serde_json::Value::as_str),
            Some("1p")
        );
        assert!(!root.join(".vibewbz-claude-desktop-state.json").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
