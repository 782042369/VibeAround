use std::collections::BTreeMap;

use super::*;
use crate::profiles::schema::{AuthMode, ProviderSettings};

fn profile(api_types: &[&str]) -> ProfileDef {
    ProfileDef {
        id: "profile-test".to_string(),
        label: "Profile Test".to_string(),
        provider: "custom".to_string(),
        auth_mode: AuthMode::ApiKey,
        api_types: api_types.iter().map(|value| (*value).to_string()).collect(),
        credentials: BTreeMap::new(),
        overrides: BTreeMap::new(),
        use_settings_proxy: false,
        provider_settings: ProviderSettings::default(),
    }
}

fn connections(
    profile_id: &str,
    agent_id: &str,
    preference: agent_state::ProfileConnectionPreference,
) -> agent_state::ProfileConnectionPreferences {
    [(
        profile_id.to_string(),
        [(agent_id.to_string(), preference)].into_iter().collect(),
    )]
    .into_iter()
    .collect()
}

#[test]
fn codex_uses_openai_responses_natively() {
    let profile = profile(&["openai-responses"]);
    let route = resolve_profile_agent_route_with_connections(&profile, "codex", &BTreeMap::new())
        .expect("codex route");

    assert_eq!(route.client_api_type, "openai-responses");
    assert_eq!(route.bridge_target_api_type, None);
}

#[test]
fn claude_uses_anthropic_natively() {
    let profile = profile(&["anthropic"]);
    let route = resolve_profile_agent_route_with_connections(&profile, "claude", &BTreeMap::new())
        .expect("claude route");

    assert_eq!(route.client_api_type, "anthropic");
    assert_eq!(route.bridge_target_api_type, None);
}

#[test]
fn codex_can_bridge_to_anthropic_gateway_profile() {
    let profile = profile(&["anthropic"]);
    let prefs = connections(
        &profile.id,
        "codex",
        agent_state::ProfileConnectionPreference {
            selected_api_type: Some("openai-responses".to_string()),
            bridge: [(
                "openai-responses".to_string(),
                agent_state::ProfileBridgePreference {
                    enabled: true,
                    target_api_type: Some("anthropic".to_string()),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        },
    );
    let route = resolve_profile_agent_route_with_connections(&profile, "codex", &prefs)
        .expect("codex bridge route");

    assert_eq!(route.client_api_type, "openai-responses");
    assert_eq!(route.bridge_target_api_type.as_deref(), Some("anthropic"));
}

#[test]
fn claude_can_bridge_to_openai_responses_gateway_profile() {
    let profile = profile(&["openai-responses"]);
    let prefs = connections(
        &profile.id,
        "claude",
        agent_state::ProfileConnectionPreference {
            selected_api_type: Some("anthropic".to_string()),
            bridge: [(
                "anthropic".to_string(),
                agent_state::ProfileBridgePreference {
                    enabled: true,
                    target_api_type: Some("openai-responses".to_string()),
                    upstream_model: Some("gpt-5.5".to_string()),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        },
    );
    let route = resolve_profile_agent_route_with_connections(&profile, "claude", &prefs)
        .expect("claude bridge route");

    assert_eq!(route.client_api_type, "anthropic");
    assert_eq!(
        route.bridge_target_api_type.as_deref(),
        Some("openai-responses")
    );
    assert_eq!(route.bridge_upstream_model.as_deref(), Some("gpt-5.5"));
}

#[test]
fn desktop_targets_reuse_cli_bridge_connections() {
    let profile = profile(&["openai-responses"]);
    let prefs = connections(
        &profile.id,
        "claude",
        agent_state::ProfileConnectionPreference {
            selected_api_type: Some("anthropic".to_string()),
            bridge: [(
                "anthropic".to_string(),
                agent_state::ProfileBridgePreference {
                    enabled: true,
                    target_api_type: Some("openai-responses".to_string()),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        },
    );

    let route = resolve_profile_agent_route_with_connections(&profile, "claude-desktop", &prefs)
        .expect("claude desktop bridge route");
    let targets = launch_targets_for_profile_with_connections(&profile, &prefs);

    assert_eq!(route.client_api_type, "anthropic");
    assert_eq!(
        route.bridge_target_api_type.as_deref(),
        Some("openai-responses")
    );
    assert!(targets.iter().any(|target| {
        target.id == "claude-desktop"
            && target.api_type == "anthropic"
            && target.bridge_target_api_type.as_deref() == Some("openai-responses")
    }));
}

#[test]
fn launch_targets_are_limited_to_claude_and_codex() {
    let profile = profile(&["anthropic", "openai-responses"]);
    let ids: Vec<_> = launch_targets_for_profile_with_connections(&profile, &BTreeMap::new())
        .into_iter()
        .map(|target| target.id)
        .collect();

    assert_eq!(
        ids,
        vec!["claude", "claude-desktop", "codex", "codex-desktop"]
    );
}

#[test]
fn bridge_route_carries_model_list() {
    let profile = profile(&["anthropic"]);
    let prefs = connections(
        &profile.id,
        "codex",
        agent_state::ProfileConnectionPreference {
            selected_api_type: Some("openai-responses".to_string()),
            bridge: [(
                "openai-responses".to_string(),
                agent_state::ProfileBridgePreference {
                    enabled: true,
                    target_api_type: Some("anthropic".to_string()),
                    models: vec![
                        agent_state::ProfileBridgeModelPreference {
                            upstream_model: Some("claude-real".to_string()),
                            fake_model_id: Some("claude-fake".to_string()),
                            capabilities: Default::default(),
                        },
                        agent_state::ProfileBridgeModelPreference {
                            upstream_model: Some("provider-extra".to_string()),
                            fake_model_id: None,
                            capabilities: Default::default(),
                        },
                    ],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        },
    );

    let route = resolve_profile_agent_route_with_connections(&profile, "codex", &prefs)
        .expect("codex bridge route");

    assert_eq!(
        route.bridge_models,
        vec![
            ProfileBridgeModelRoute {
                upstream_model: "claude-real".to_string(),
                agent_model: "claude-fake".to_string(),
                capabilities: Default::default(),
            },
            ProfileBridgeModelRoute {
                upstream_model: "provider-extra".to_string(),
                agent_model: "provider-extra".to_string(),
                capabilities: Default::default(),
            },
        ]
    );
}
