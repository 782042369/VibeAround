//! Bridge profile rendering for route-aware launches.

use anyhow::{anyhow, bail};
use serde_json::{json, Map, Value};

use super::catalog;
use super::codex_metadata::{self, CodexModelCatalogSpec};
use super::connections::ProfileBridgeModelRoute;
use super::render::{ConfigEnvTarget, RenderedProfile, RenderedSettingsFile};
use super::schema::{AuthMode, ProfileDef};
use crate::config;

const CODEX_DEFAULT_REASONING_EFFORT: &str = "high";
const CODEX_DEFAULT_CONTEXT_WINDOW: u64 = 400_000;
const CODEX_DEFAULT_AUTO_COMPACT_TOKEN_LIMIT: u64 = 228_000;
const CODEX_MEMORIES_CONSOLIDATION_MODEL: &str = "gpt-5.5";
const CODEX_MEMORIES_EXTRACT_MODEL: &str = "gpt-5.5";
const CODEX_MEMORIES_MAX_RAW_FOR_CONSOLIDATION: u64 = 320;
const CODEX_MEMORIES_MAX_ROLLOUT_AGE_DAYS: u64 = 60;

pub(super) fn render_bridge_launch(
    profile: &ProfileDef,
    launch_target: &str,
    launch_id: &str,
    client_api_type: &str,
    target_api_type: &str,
    upstream_model: Option<&str>,
    fake_model_id: Option<&str>,
    bridge_models: &[ProfileBridgeModelRoute],
) -> anyhow::Result<RenderedProfile> {
    let mut settings = resolve_bridge_settings(
        profile,
        target_api_type,
        upstream_model,
        fake_model_id,
        bridge_models,
    )?;
    let scope_agent_id = if launch_target == "claude-desktop" {
        "claude"
    } else {
        launch_target
    };
    settings.scope = format!("{scope_agent_id}-{client_api_type}");
    match launch_target {
        "claude" | "claude-desktop" => {
            Ok(render_claude_bridge_profile(profile, launch_id, settings))
        }
        "codex" | "codex-desktop" => Ok(render_codex_bridge_profile(profile, launch_id, settings)),
        "gemini" => Ok(render_gemini_bridge_profile(profile, settings)),
        "opencode" => Ok(render_opencode_bridge_profile(
            profile,
            launch_id,
            client_api_type,
            settings,
        )),
        "pi" => render_pi_bridge_profile(profile, client_api_type, settings),
        other => bail!("bridge launch is not wired for '{}'", other),
    }
}

struct BridgeLaunchSettings {
    target_api_type: String,
    scope: String,
    provider_label: String,
    api_key: String,
    models: Vec<BridgeModelSettings>,
    reasoning_effort: String,
}

#[derive(Debug, Clone)]
struct BridgeModelSettings {
    agent_model: String,
    model_context_window: Option<u64>,
    model_capabilities: catalog::ContentCapabilities,
}

impl BridgeLaunchSettings {
    fn default_model(&self) -> &BridgeModelSettings {
        self.models
            .first()
            .expect("bridge settings must contain at least one model")
    }
}

fn resolve_bridge_settings(
    profile: &ProfileDef,
    target_api_type: &str,
    upstream_model: Option<&str>,
    fake_model_id: Option<&str>,
    bridge_models: &[ProfileBridgeModelRoute],
) -> anyhow::Result<BridgeLaunchSettings> {
    let provider = catalog::get(&profile.provider)
        .ok_or_else(|| anyhow!("unknown provider '{}'", profile.provider))?;
    if !profile
        .api_types
        .iter()
        .any(|api_type| api_type == target_api_type)
    {
        bail!(
            "profile '{}' does not expose bridge target '{}'",
            profile.id,
            target_api_type
        );
    }

    let endpoint_id = profile
        .overrides
        .get(target_api_type)
        .and_then(|overrides| overrides.endpoint_id.as_deref());
    let endpoint =
        catalog::find_endpoint(provider, target_api_type, endpoint_id).ok_or_else(|| {
            let suffix = endpoint_id
                .map(|id| format!(" endpoint_id '{id}'"))
                .unwrap_or_default();
            anyhow!(
                "provider '{}' does not expose bridge target '{}'{}",
                profile.provider,
                target_api_type,
                suffix
            )
        })?;
    let api_key = profile
        .credentials
        .get("api_key")
        .filter(|value| !value.is_empty())
        .cloned()
        .or_else(|| {
            matches!(
                profile.auth_mode,
                AuthMode::OauthViaCli | AuthMode::GoogleOauth
            )
            .then(|| "vibewbz-local-bridge".to_string())
        })
        .ok_or_else(|| anyhow!("profile '{}' has no api_key credential", profile.id))?;
    let profile_model = profile
        .overrides
        .get(target_api_type)
        .and_then(|overrides| overrides.model.clone())
        .or_else(|| endpoint.models.first().map(|model| model.id.clone()))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!(
                "profile '{}' has no model configured for bridge target '{}'",
                profile.id,
                target_api_type
            )
        })?;
    let requested_upstream_model = upstream_model
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(profile_model);
    let routes = if bridge_models.is_empty() {
        vec![ProfileBridgeModelRoute {
            upstream_model: requested_upstream_model.clone(),
            agent_model: fake_model_id
                .map(str::trim)
                .filter(|model| !model.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| requested_upstream_model.clone()),
            capabilities: catalog::ContentCapabilities::default(),
        }]
    } else {
        bridge_models.to_vec()
    };
    let models = routes
        .into_iter()
        .filter_map(|route| bridge_model_settings(endpoint, route))
        .collect::<Vec<_>>();
    if models.is_empty() {
        bail!(
            "profile '{}' has no models configured for bridge target '{}'",
            profile.id,
            target_api_type
        );
    }
    let reasoning_effort = profile
        .overrides
        .get(target_api_type)
        .and_then(|overrides| overrides.reasoning_effort.clone())
        .unwrap_or_else(|| CODEX_DEFAULT_REASONING_EFFORT.to_string());

    Ok(BridgeLaunchSettings {
        target_api_type: target_api_type.to_string(),
        scope: String::new(),
        provider_label: provider.label.clone(),
        api_key,
        models,
        reasoning_effort,
    })
}

fn bridge_model_settings(
    endpoint: &catalog::EndpointDef,
    route: ProfileBridgeModelRoute,
) -> Option<BridgeModelSettings> {
    let upstream_model = route.upstream_model.trim().to_string();
    let agent_model = route.agent_model.trim().to_string();
    if upstream_model.is_empty() || agent_model.is_empty() {
        return None;
    }
    let upstream_model =
        catalog::canonical_model_id(endpoint, &upstream_model).unwrap_or(upstream_model);
    let model_def = catalog::find_model(endpoint, &upstream_model);
    let model_context_window = model_def.and_then(|model_def| model_def.context_window);
    let model_capabilities = model_def
        .map(|model_def| endpoint.capabilities.content.merge(&model_def.capabilities))
        .unwrap_or_else(|| endpoint.capabilities.content.clone())
        .merge(&route.capabilities);
    Some(BridgeModelSettings {
        agent_model,
        model_context_window,
        model_capabilities,
    })
}

fn render_claude_bridge_profile(
    profile: &ProfileDef,
    _launch_id: &str,
    settings: BridgeLaunchSettings,
) -> RenderedProfile {
    let bridge_base_url = format!(
        "http://127.0.0.1:{}/va/local-api/{}/{}/{}",
        config::DEFAULT_PORT,
        profile.id,
        settings.scope,
        settings.target_api_type
    );
    RenderedProfile {
        env: claude_env(settings.api_key.clone(), bridge_base_url, &settings),
        settings_files: Vec::new(),
        command_args: Vec::new(),
        config_env: None,
    }
}

fn claude_env(
    api_key: String,
    base_url: String,
    settings: &BridgeLaunchSettings,
) -> Vec<(String, String)> {
    let default_model = settings.default_model();
    let mut env = vec![
        ("ANTHROPIC_API_KEY".to_string(), api_key.clone()),
        ("ANTHROPIC_AUTH_TOKEN".to_string(), api_key.clone()),
        ("ANTHROPIC_BASE_URL".to_string(), base_url),
        (
            "ANTHROPIC_MODEL".to_string(),
            default_model.agent_model.clone(),
        ),
        (
            "CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY".to_string(),
            "1".to_string(),
        ),
    ];
    if !is_claude_discoverable_model(&default_model.agent_model) {
        env.push((
            "ANTHROPIC_CUSTOM_MODEL_OPTION".to_string(),
            default_model.agent_model.clone(),
        ));
        env.push((
            "ANTHROPIC_CUSTOM_MODEL_OPTION_NAME".to_string(),
            default_model.agent_model.clone(),
        ));
        env.push((
            "ANTHROPIC_CUSTOM_MODEL_OPTION_DESCRIPTION".to_string(),
            format!("{} {}", settings.provider_label, default_model.agent_model),
        ));
    }
    if let Some(model_context_window) = default_model.model_context_window {
        env.push((
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW".to_string(),
            model_context_window.to_string(),
        ));
    }
    env
}

fn is_claude_discoverable_model(model: &str) -> bool {
    let model = model.trim().to_ascii_lowercase();
    model.starts_with("claude") || model.starts_with("anthropic")
}

fn render_codex_bridge_profile(
    profile: &ProfileDef,
    launch_id: &str,
    settings: BridgeLaunchSettings,
) -> RenderedProfile {
    let bridge_base_url = format!(
        "http://127.0.0.1:{}/va/local-api/{}/{}/{}/v1",
        config::DEFAULT_PORT,
        profile.id,
        settings.scope,
        settings.target_api_type
    );
    let provider_key = format!("model_providers.{}", profile.provider);
    let mut command_args = Vec::new();
    let default_model = settings.default_model();

    push_config_arg(
        &mut command_args,
        "model",
        &toml_string(&default_model.agent_model),
    );
    push_config_arg(
        &mut command_args,
        "model_provider",
        &toml_string(&profile.provider),
    );
    push_config_arg(
        &mut command_args,
        "model_reasoning_effort",
        &toml_string(&settings.reasoning_effort),
    );
    let mut settings_files = Vec::new();
    let context_window = default_model
        .model_context_window
        .unwrap_or(CODEX_DEFAULT_CONTEXT_WINDOW);
    push_config_arg(
        &mut command_args,
        "model_context_window",
        &context_window.to_string(),
    );
    push_config_arg(
        &mut command_args,
        "model_auto_compact_token_limit",
        &CODEX_DEFAULT_AUTO_COMPACT_TOKEN_LIMIT.to_string(),
    );
    push_config_arg(&mut command_args, "features.memories", "true");
    push_config_arg(
        &mut command_args,
        "memories.consolidation_model",
        &toml_string(CODEX_MEMORIES_CONSOLIDATION_MODEL),
    );
    push_config_arg(
        &mut command_args,
        "memories.extract_model",
        &toml_string(CODEX_MEMORIES_EXTRACT_MODEL),
    );
    push_config_arg(
        &mut command_args,
        "memories.max_raw_memories_for_consolidation",
        &CODEX_MEMORIES_MAX_RAW_FOR_CONSOLIDATION.to_string(),
    );
    push_config_arg(
        &mut command_args,
        "memories.max_rollout_age_days",
        &CODEX_MEMORIES_MAX_ROLLOUT_AGE_DAYS.to_string(),
    );
    let specs: Vec<_> = settings
        .models
        .iter()
        .map(|model| CodexModelCatalogSpec {
            model: model.agent_model.as_str(),
            provider_label: &settings.provider_label,
            context_window: model.model_context_window,
            capabilities: &model.model_capabilities,
        })
        .collect();
    if let Some(model_catalog_json) = codex_metadata::build_model_catalog_json(&specs) {
        let rel_path = format!("codex-model-catalog-{launch_id}.json");
        let catalog_path = super::runtime::profile_state_dir(&profile.id).join(&rel_path);
        let catalog_path = catalog_path.to_string_lossy();
        push_config_arg(
            &mut command_args,
            "model_catalog_json",
            &toml_string(catalog_path.as_ref()),
        );
        settings_files.push(RenderedSettingsFile {
            rel_path,
            contents: model_catalog_json,
        });
    }
    push_provider_config_arg(
        &mut command_args,
        &provider_key,
        "name",
        &toml_string(&settings.provider_label),
    );
    push_provider_config_arg(
        &mut command_args,
        &provider_key,
        "base_url",
        &toml_string(&bridge_base_url),
    );
    push_provider_config_arg(
        &mut command_args,
        &provider_key,
        "wire_api",
        &toml_string("responses"),
    );
    push_provider_config_arg(
        &mut command_args,
        &provider_key,
        "env_key",
        &toml_string("OPENAI_API_KEY"),
    );
    push_provider_config_arg(
        &mut command_args,
        &provider_key,
        "requires_openai_auth",
        "true",
    );

    RenderedProfile {
        env: vec![("OPENAI_API_KEY".to_string(), settings.api_key)],
        settings_files,
        command_args,
        config_env: None,
    }
}

fn render_opencode_bridge_profile(
    profile: &ProfileDef,
    _launch_id: &str,
    client_api_type: &str,
    settings: BridgeLaunchSettings,
) -> RenderedProfile {
    let bridge_base_url = opencode_bridge_base_url(profile, &settings, client_api_type);
    let npm = match client_api_type {
        "anthropic" => "@ai-sdk/anthropic",
        "openai-chat" => "@ai-sdk/openai-compatible",
        _ => "@ai-sdk/openai",
    };
    let provider_id = profile.provider.clone();
    let model = settings.default_model().agent_model.clone();
    let mut models = Map::new();
    for model in &settings.models {
        models.insert(
            model.agent_model.clone(),
            json!({ "name": model.agent_model.as_str() }),
        );
    }
    let mut providers = Map::new();
    providers.insert(
        provider_id.clone(),
        json!({
            "npm": npm,
            "name": settings.provider_label,
            "options": {
                "baseURL": bridge_base_url,
                "apiKey": "{env:VIBEWBZ_OPENCODE_API_KEY}",
                "setCacheKey": true
            },
            "models": Value::Object(models)
        }),
    );
    let config = json!({
        "$schema": "https://opencode.ai/config.json",
        "model": format!("{}/{}", provider_id, model),
        "provider": Value::Object(providers)
    });

    RenderedProfile {
        env: vec![("VIBEWBZ_OPENCODE_API_KEY".to_string(), settings.api_key)],
        settings_files: vec![RenderedSettingsFile {
            rel_path: "opencode.json".to_string(),
            contents: serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string()),
        }],
        command_args: Vec::new(),
        config_env: Some(ConfigEnvTarget::File {
            env: "OPENCODE_CONFIG",
            rel_path: "opencode.json",
        }),
    }
}

fn render_gemini_bridge_profile(
    profile: &ProfileDef,
    settings: BridgeLaunchSettings,
) -> RenderedProfile {
    let bridge_base_url = format!(
        "http://127.0.0.1:{}/va/local-api/{}/{}/{}",
        config::DEFAULT_PORT,
        profile.id,
        settings.scope,
        settings.target_api_type
    );
    RenderedProfile {
        env: vec![
            (
                "GEMINI_API_KEY".to_string(),
                "vibewbz-local-bridge".to_string(),
            ),
            (
                "GOOGLE_API_KEY".to_string(),
                "vibewbz-local-bridge".to_string(),
            ),
            (
                "GEMINI_DEFAULT_AUTH_TYPE".to_string(),
                "gemini-api-key".to_string(),
            ),
            ("GOOGLE_GEMINI_BASE_URL".to_string(), bridge_base_url),
            (
                "GEMINI_MODEL".to_string(),
                settings.default_model().agent_model.clone(),
            ),
        ],
        settings_files: Vec::new(),
        command_args: Vec::new(),
        config_env: None,
    }
}

fn render_pi_bridge_profile(
    profile: &ProfileDef,
    client_api_type: &str,
    settings: BridgeLaunchSettings,
) -> anyhow::Result<RenderedProfile> {
    let bridge_base_url = super::pi_launch::bridge_base_url(
        &profile.id,
        &settings.scope,
        &settings.target_api_type,
        client_api_type,
    );
    let provider_id = super::pi_launch::provider_id(
        &profile.id,
        &format!("bridge-{}-{}", client_api_type, settings.target_api_type),
    );
    let default_model = settings.default_model().clone();
    super::pi_launch::render_pi_provider(super::pi_launch::PiProviderLaunchConfig {
        profile_id: &profile.id,
        provider_id: provider_id.clone(),
        provider_label: &settings.provider_label,
        api_type: client_api_type,
        api_key: settings.api_key,
        base_url: bridge_base_url,
        model: default_model.agent_model,
        model_context_window: default_model.model_context_window,
        model_capabilities: default_model.model_capabilities,
        reasoning: client_api_type == "openai-responses",
        headers: Default::default(),
        auth_header: false,
        file_stem: provider_id,
    })
}

fn opencode_bridge_base_url(
    profile: &ProfileDef,
    settings: &BridgeLaunchSettings,
    client_api_type: &str,
) -> String {
    let suffix = if client_api_type == "anthropic" {
        ""
    } else {
        "/v1"
    };
    format!(
        "http://127.0.0.1:{}/va/local-api/{}/{}/{}{}",
        config::DEFAULT_PORT,
        profile.id,
        settings.scope,
        settings.target_api_type,
        suffix
    )
}

fn push_config_arg(args: &mut Vec<String>, key: &str, value: &str) {
    args.push("-c".to_string());
    args.push(format!("{key}={value}"));
}

fn push_provider_config_arg(args: &mut Vec<String>, provider_key: &str, field: &str, value: &str) {
    push_config_arg(args, &format!("{provider_key}.{field}"), value);
}

fn toml_string(s: &str) -> String {
    if s.contains('\'') {
        let mut out = String::with_capacity(s.len() + 2);
        out.push('"');
        for ch in s.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                other => out.push(other),
            }
        }
        out.push('"');
        out
    } else {
        let mut out = String::with_capacity(s.len() + 2);
        out.push('\'');
        out.push_str(s);
        out.push('\'');
        out
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::profiles::schema::{ApiTypeOverrides, AuthMode, ProfileDef};
    use serde_json::Value;

    use super::*;

    #[test]
    fn codex_bridge_launch_uses_gateway_responses_route() {
        let profile = gateway_profile("openai-responses", "gpt-5.5");

        let rendered = render_bridge_launch(
            &profile,
            "codex",
            "launch-test",
            "openai-responses",
            "openai-responses",
            None,
            None,
            &[],
        )
        .expect("codex bridge launch renders");

        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model='gpt-5.5'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model_reasoning_effort='high'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model_context_window=400000"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model_auto_compact_token_limit=228000"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "features.memories=true"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "memories.consolidation_model='gpt-5.5'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "memories.extract_model='gpt-5.5'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "memories.max_raw_memories_for_consolidation=320"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "memories.max_rollout_age_days=60"));
        assert!(rendered.command_args.iter().any(|arg| {
            arg == "model_providers.custom.base_url='http://127.0.0.1:12358/va/local-api/gateway-test/codex-openai-responses/openai-responses/v1'"
        }));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model_providers.custom.wire_api='responses'"));
        assert!(rendered
            .env
            .contains(&("OPENAI_API_KEY".to_string(), "test-key".to_string())));
        assert!(rendered
            .settings_files
            .iter()
            .any(|settings_file| settings_file.rel_path == "codex-model-catalog-launch-test.json"));
        assert!(rendered.config_env.is_none());
    }

    #[test]
    fn codex_bridge_launch_keeps_explicit_agent_model_routes() {
        let profile = gateway_profile("openai-responses", "gpt-5.5");
        let rendered = render_bridge_launch(
            &profile,
            "codex",
            "launch-test",
            "openai-responses",
            "openai-responses",
            None,
            None,
            &[ProfileBridgeModelRoute {
                upstream_model: "upstream-model".to_string(),
                agent_model: "agent-visible-model".to_string(),
                capabilities: catalog::ContentCapabilities {
                    image_input: true,
                    file_input: true,
                },
            }],
        )
        .expect("codex bridge launch renders");

        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model='agent-visible-model'"));
        let catalog_file = rendered
            .settings_files
            .iter()
            .find(|settings_file| settings_file.rel_path == "codex-model-catalog-launch-test.json")
            .expect("codex model catalog file");
        let catalog: Value =
            serde_json::from_str(&catalog_file.contents).expect("catalog json parses");
        let model = &catalog["models"][0];
        assert_eq!(model["slug"], "agent-visible-model");
        assert_eq!(
            model["input_modalities"],
            serde_json::json!(["text", "image", "file"])
        );
    }

    #[test]
    fn claude_bridge_launch_uses_gateway_env_shape() {
        let profile = gateway_profile("openai-chat", "gpt-5.5");
        let rendered = render_bridge_launch(
            &profile,
            "claude",
            "launch-test",
            "anthropic",
            "openai-chat",
            None,
            Some("claude-sonnet-4-5"),
            &[],
        )
        .expect("claude bridge launch renders");

        let keys: Vec<_> = rendered.env.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_AUTH_TOKEN",
                "ANTHROPIC_BASE_URL",
                "ANTHROPIC_MODEL",
                "CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY",
            ]
        );
        assert_eq!(
            rendered
                .env
                .iter()
                .find(|(key, _)| key == "ANTHROPIC_BASE_URL")
                .map(|(_, value)| value.as_str()),
            Some("http://127.0.0.1:12358/va/local-api/gateway-test/claude-anthropic/openai-chat")
        );
        assert_eq!(
            rendered
                .env
                .iter()
                .find(|(key, _)| key == "ANTHROPIC_MODEL")
                .map(|(_, value)| value.as_str()),
            Some("claude-sonnet-4-5")
        );
        assert!(rendered.settings_files.is_empty());
        assert!(rendered.command_args.is_empty());
        assert!(rendered.config_env.is_none());
    }

    #[test]
    fn claude_desktop_bridge_launch_reuses_claude_scope() {
        let profile = gateway_profile("openai-chat", "gpt-5.5");
        let rendered = render_bridge_launch(
            &profile,
            "claude-desktop",
            "launch-test",
            "anthropic",
            "openai-chat",
            None,
            Some("claude-sonnet-4-5"),
            &[],
        )
        .expect("claude desktop bridge launch renders");

        assert_eq!(
            rendered
                .env
                .iter()
                .find(|(key, _)| key == "ANTHROPIC_BASE_URL")
                .map(|(_, value)| value.as_str()),
            Some("http://127.0.0.1:12358/va/local-api/gateway-test/claude-anthropic/openai-chat")
        );
    }

    fn gateway_profile(api_type: &str, model: &str) -> ProfileDef {
        let mut credentials = BTreeMap::new();
        credentials.insert("api_key".to_string(), "test-key".to_string());

        let mut overrides = BTreeMap::new();
        overrides.insert(
            api_type.to_string(),
            ApiTypeOverrides {
                endpoint_id: None,
                base_url: Some("http://ai.939593.xyz".to_string()),
                model: Some(model.to_string()),
                reasoning_effort: None,
                ..Default::default()
            },
        );

        ProfileDef {
            id: "gateway-test".to_string(),
            label: "VibeWbz Gateway Test".to_string(),
            provider: "custom".to_string(),
            auth_mode: AuthMode::ApiKey,
            api_types: vec![api_type.to_string()],
            credentials,
            overrides,
            use_settings_proxy: false,
            provider_settings: Default::default(),
        }
    }
}
