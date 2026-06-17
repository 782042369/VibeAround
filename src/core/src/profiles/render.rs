//! Render orchestrator — resolve a profile against a provider API kind and
//! CLI launch target, then produce the env vars + optional settings files
//! the launcher will hand to the spawned terminal.
//!
//! The mustache-lite engine is intentionally tiny: it supports `{{name}}`
//! substitution against a flat string context and nothing else (no pipes,
//! no conditionals, no escaping). Catalog templates that need richer logic
//! should pre-shape the data instead. Empty resolved env values are
//! dropped so a missing-but-not-required field doesn't end up exporting
//! `KEY=""`.

use std::collections::BTreeMap;

use anyhow::{anyhow, bail};

use super::catalog::{
    self, AuthModeDef, ContentCapabilities, EndpointDef, ProviderCatalog, RenderRules,
    SettingsFileTemplate,
};
use super::codex_metadata::{self, CodexModelCatalogSpec};
use super::schema::{ApiTypeOverrides, AuthMode, ProfileDef};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RenderedProfile {
    pub env: Vec<(String, String)>,
    pub settings_files: Vec<RenderedSettingsFile>,
    pub command_args: Vec<String>,
    /// Which env var should point at profile-local rendered config once
    /// the launcher materializes any settings files. We avoid overriding
    /// agent home dirs such as CODEX_HOME or CLAUDE_CONFIG_DIR so those CLIs
    /// keep loading the user's own sessions, plugins, and skills.
    pub config_env: Option<ConfigEnvTarget>,
}

#[derive(Debug, Clone)]
pub struct RenderedSettingsFile {
    pub rel_path: String,
    pub contents: String,
}

#[derive(Debug, Clone)]
pub enum ConfigEnvTarget {
    Directory(&'static str),
    File {
        env: &'static str,
        rel_path: &'static str,
    },
}

// ---------------------------------------------------------------------------
// Public entry
// ---------------------------------------------------------------------------

pub fn render(
    profile: &ProfileDef,
    api_type: &str,
    launch_target: &str,
    catalog: &ProviderCatalog,
) -> anyhow::Result<RenderedProfile> {
    let endpoint = pick_endpoint(profile, catalog, api_type)?;
    let auth = pick_auth_mode(endpoint, &profile.auth_mode)?;
    let context = build_context(profile, api_type, launch_target, endpoint, catalog);
    if launch_target == "pi" {
        return render_pi_profile(profile, api_type, endpoint, catalog, &context);
    }

    let opencode_rules;
    let render_rules = if launch_target == "opencode" {
        opencode_rules = opencode_render_rules(api_type)?;
        &opencode_rules
    } else {
        auth.render
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "auth mode '{}' has no render rules (only oauth flows skip rendering, which v1 doesn't expose)",
                    auth.mode
                )
            })?
    };

    // Env vars — drop entries whose substituted value is empty so we don't
    // end up exporting blank keys (e.g. `ANTHROPIC_MODEL=""` when the user
    // didn't pick a model override).
    let mut env: Vec<(String, String)> = Vec::new();
    for (k, tmpl) in &render_rules.env {
        if !is_valid_env_key(k) {
            bail!("invalid env key in render rules: '{}'", k);
        }
        let v = substitute(tmpl, &context);
        if !v.is_empty() {
            env.push((k.clone(), v));
        }
    }
    if is_claude_launch_target(launch_target) && api_type == "anthropic" {
        normalize_claude_env(&mut env, &context);
    }

    // Settings files — substitute against the same context, validate each path.
    let mut settings_files: Vec<RenderedSettingsFile> = Vec::new();
    for sf in &render_rules.settings_files {
        validate_rel_path(&sf.rel_path)?;
        settings_files.push(RenderedSettingsFile {
            rel_path: sf.rel_path.clone(),
            contents: substitute(&sf.template, &context),
        });
    }

    let mut command_args = command_args_for(launch_target, &context);
    if let Some(metadata) = selected_model_metadata(&context, endpoint) {
        add_codex_model_catalog(
            profile,
            launch_target,
            &context,
            &metadata,
            &mut settings_files,
            &mut command_args,
        )?;
    }

    let config_env = config_env_for_rendered_files(launch_target, &settings_files);

    Ok(RenderedProfile {
        env,
        settings_files,
        command_args,
        config_env,
    })
}

// ---------------------------------------------------------------------------
// Lookups
// ---------------------------------------------------------------------------

fn pick_endpoint<'a>(
    profile: &ProfileDef,
    catalog: &'a ProviderCatalog,
    api_type: &str,
) -> anyhow::Result<&'a EndpointDef> {
    let endpoint_id = profile
        .overrides
        .get(api_type)
        .and_then(|overrides| overrides.endpoint_id.as_deref());
    catalog::find_endpoint(catalog, api_type, endpoint_id).ok_or_else(|| {
        let suffix = endpoint_id
            .map(|id| format!(" endpoint_id '{id}'"))
            .unwrap_or_default();
        anyhow!(
            "provider '{}' has no endpoint for api_type '{}'{}",
            catalog.id,
            api_type,
            suffix
        )
    })
}

fn pick_auth_mode<'a>(
    endpoint: &'a EndpointDef,
    auth_mode: &AuthMode,
) -> anyhow::Result<&'a AuthModeDef> {
    let needle = match auth_mode {
        AuthMode::ApiKey => "api_key",
        AuthMode::OauthViaCli => "oauth_via_cli",
        AuthMode::GoogleOauth => "google_oauth",
    };
    endpoint
        .auth_modes
        .iter()
        .find(|a| a.mode == needle)
        .ok_or_else(|| {
            anyhow!(
                "endpoint '{}' has no auth mode '{}'",
                endpoint.api_type,
                needle
            )
        })
}

fn config_env_for(launch_target: &str) -> Option<ConfigEnvTarget> {
    match launch_target {
        "opencode" => Some(ConfigEnvTarget::File {
            env: "OPENCODE_CONFIG",
            rel_path: "opencode.json",
        }),
        _ => None,
    }
}

fn config_env_for_rendered_files(
    launch_target: &str,
    settings_files: &[RenderedSettingsFile],
) -> Option<ConfigEnvTarget> {
    if settings_files
        .iter()
        .all(|settings_file| settings_file.rel_path.starts_with("codex-model-catalog-"))
    {
        return None;
    }
    config_env_for(launch_target)
}

fn opencode_render_rules(api_type: &str) -> anyhow::Result<RenderRules> {
    match api_type {
        "openai-responses" => Ok(RenderRules {
            env: [(
                "VIBEWBZ_OPENCODE_API_KEY".to_string(),
                "{{api_key}}".to_string(),
            )]
            .into_iter()
            .collect(),
            settings_files: vec![SettingsFileTemplate {
                rel_path: "opencode.json".to_string(),
                template: "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"model\": \"{{provider_id}}/{{model|json}}\",\n  \"provider\": {\n    \"{{provider_id}}\": {\n      \"npm\": \"@ai-sdk/openai\",\n      \"name\": \"{{provider_label|json}}\",\n      \"options\": {\n        \"baseURL\": \"{{base_url|json}}\",\n        \"apiKey\": \"{env:VIBEWBZ_OPENCODE_API_KEY}\",\n        \"setCacheKey\": true\n      },\n      \"models\": {\n        \"{{model|json}}\": { \"name\": \"{{model|json}}\" }\n      }\n    }\n  }\n}\n".to_string(),
            }],
        }),
        "openai-chat" => Ok(RenderRules {
            env: [(
                "VIBEWBZ_OPENCODE_API_KEY".to_string(),
                "{{api_key}}".to_string(),
            )]
            .into_iter()
            .collect(),
            settings_files: vec![SettingsFileTemplate {
                rel_path: "opencode.json".to_string(),
                template: "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"model\": \"{{provider_id}}/{{model|json}}\",\n  \"provider\": {\n    \"{{provider_id}}\": {\n      \"npm\": \"@ai-sdk/openai-compatible\",\n      \"name\": \"{{provider_label|json}}\",\n      \"options\": {\n        \"baseURL\": \"{{base_url|json}}\",\n        \"apiKey\": \"{env:VIBEWBZ_OPENCODE_API_KEY}\",\n        \"setCacheKey\": true\n      },\n      \"models\": {\n        \"{{model|json}}\": { \"name\": \"{{model|json}}\" }\n      }\n    }\n  }\n}\n".to_string(),
            }],
        }),
        "anthropic" => Ok(RenderRules {
            env: [(
                "VIBEWBZ_OPENCODE_API_KEY".to_string(),
                "{{api_key}}".to_string(),
            )]
            .into_iter()
            .collect(),
            settings_files: vec![SettingsFileTemplate {
                rel_path: "opencode.json".to_string(),
                template: "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"model\": \"{{provider_id}}/{{model|json}}\",\n  \"provider\": {\n    \"{{provider_id}}\": {\n      \"npm\": \"@ai-sdk/anthropic\",\n      \"name\": \"{{provider_label|json}}\",\n      \"options\": {\n        \"baseURL\": \"{{base_url|json}}\",\n        \"apiKey\": \"{env:VIBEWBZ_OPENCODE_API_KEY}\"\n      },\n      \"models\": {\n        \"{{model|json}}\": { \"name\": \"{{model|json}}\" }\n      }\n    }\n  }\n}\n".to_string(),
            }],
        }),
        other => bail!("opencode launch is not wired for api kind '{}'", other),
    }
}

fn render_pi_profile(
    profile: &ProfileDef,
    api_type: &str,
    endpoint: &EndpointDef,
    catalog: &ProviderCatalog,
    context: &BTreeMap<String, String>,
) -> anyhow::Result<RenderedProfile> {
    let model = context
        .get("model")
        .map(String::as_str)
        .filter(|value| !value.is_empty());
    let model_def = model.and_then(|model| catalog::find_model(endpoint, model));
    let model_capabilities = model_def
        .map(|model_def| endpoint.capabilities.content.merge(&model_def.capabilities))
        .unwrap_or_else(|| endpoint.capabilities.content.clone());
    let provider_id = super::pi_launch::provider_id(&profile.id, api_type);
    super::pi_launch::render_pi_provider(super::pi_launch::PiProviderLaunchConfig {
        profile_id: &profile.id,
        provider_id: provider_id.clone(),
        provider_label: &catalog.label,
        api_type,
        api_key: context.get("api_key").cloned().unwrap_or_default(),
        base_url: context.get("base_url").cloned().unwrap_or_default(),
        model: context.get("model").cloned().unwrap_or_default(),
        model_context_window: model_def.and_then(|model_def| model_def.context_window),
        model_capabilities,
        reasoning: endpoint.capabilities.reasoning_effort,
        headers: endpoint.headers.clone(),
        auth_header: endpoint.auth_header,
        file_stem: provider_id,
    })
}

fn command_args_for(launch_target: &str, ctx: &BTreeMap<String, String>) -> Vec<String> {
    if !is_codex_launch_target(launch_target) {
        return Vec::new();
    }

    let mut args = Vec::new();
    let mut push_config = |key: &str, value: String| {
        args.push("-c".to_string());
        args.push(format!("{key}={value}"));
    };
    if let Some(model) = ctx.get("model").filter(|v| !v.is_empty()) {
        push_config("model", toml_string(model));
    }
    if let Some(provider_id) = ctx.get("provider_id").filter(|v| !v.is_empty()) {
        push_config("model_provider", toml_string(provider_id));
    }
    if let Some(reasoning_effort) = ctx.get("reasoning_effort").filter(|v| !v.is_empty()) {
        push_config("model_reasoning_effort", toml_string(reasoning_effort));
    }
    if let Some(context_window) = ctx.get("model_context_window").filter(|v| !v.is_empty()) {
        push_config("model_context_window", context_window.clone());
    }

    let Some(provider_id) = ctx.get("provider_id").filter(|v| !v.is_empty()) else {
        return args;
    };
    let provider_key = format!("model_providers.{}", toml_key(provider_id));
    if let Some(provider_label) = ctx.get("provider_label").filter(|v| !v.is_empty()) {
        push_config(&format!("{provider_key}.name"), toml_string(provider_label));
    }
    if let Some(base_url) = ctx.get("base_url").filter(|v| !v.is_empty()) {
        let base_url = codex_provider_base_url(
            ctx.get("api_type").map(String::as_str).unwrap_or_default(),
            base_url,
        );
        push_config(&format!("{provider_key}.base_url"), toml_string(&base_url));
    }
    if let Some(api_type) = ctx.get("api_type").filter(|v| !v.is_empty()) {
        let wire_api = if api_type == "openai-chat" {
            "chat"
        } else {
            "responses"
        };
        push_config(&format!("{provider_key}.wire_api"), toml_string(wire_api));
    }
    if ctx.get("codex_auth_json").map(String::as_str) == Some("true") {
        push_config(
            &format!("{provider_key}.requires_openai_auth"),
            "true".to_string(),
        );
    } else {
        push_config(
            &format!("{provider_key}.env_key"),
            toml_string(codex_provider_env_key(provider_id)),
        );
    }
    args
}

#[derive(Debug)]
struct SelectedModelMetadata {
    context_window: u64,
    capabilities: ContentCapabilities,
}

fn selected_model_metadata(
    ctx: &BTreeMap<String, String>,
    endpoint: &EndpointDef,
) -> Option<SelectedModelMetadata> {
    let model = ctx.get("model").filter(|value| !value.is_empty())?;
    let model_def = catalog::find_model(endpoint, model)?;
    Some(SelectedModelMetadata {
        context_window: model_def.context_window?,
        capabilities: endpoint.capabilities.content.merge(&model_def.capabilities),
    })
}

fn add_codex_model_catalog(
    profile: &ProfileDef,
    launch_target: &str,
    ctx: &BTreeMap<String, String>,
    metadata: &SelectedModelMetadata,
    settings_files: &mut Vec<RenderedSettingsFile>,
    command_args: &mut Vec<String>,
) -> anyhow::Result<()> {
    if !is_codex_launch_target(launch_target) {
        return Ok(());
    }
    let Some(model) = ctx.get("model").filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let Some(provider_label) = ctx.get("provider_label").filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let spec = CodexModelCatalogSpec {
        model,
        provider_label,
        context_window: Some(metadata.context_window),
        capabilities: &metadata.capabilities,
    };
    let Some(model_catalog_json) = codex_metadata::build_model_catalog_json(&[spec]) else {
        return Ok(());
    };

    let rel_path = codex_model_catalog_rel_path(model);
    validate_rel_path(&rel_path)?;
    let catalog_path = super::runtime::profile_state_dir(&profile.id).join(&rel_path);
    let catalog_path = catalog_path.to_string_lossy();
    command_args.push("-c".to_string());
    command_args.push(format!(
        "model_catalog_json={}",
        toml_string(catalog_path.as_ref())
    ));
    settings_files.push(RenderedSettingsFile {
        rel_path,
        contents: model_catalog_json,
    });

    Ok(())
}

fn codex_model_catalog_rel_path(model: &str) -> String {
    let mut slug = String::with_capacity(model.len());
    for ch in model.chars().take(96) {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            slug.push(ch);
        } else {
            slug.push('_');
        }
    }
    if slug.is_empty() {
        slug.push_str("model");
    }
    format!("codex-model-catalog-{slug}.json")
}

fn codex_provider_base_url(api_type: &str, base_url: &str) -> String {
    if api_type != "openai-responses" && api_type != "openai-chat" {
        return base_url.to_string();
    }

    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

fn normalize_claude_env(env: &mut Vec<(String, String)>, ctx: &BTreeMap<String, String>) {
    let api_key = first_env_value(env, &["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"])
        .or_else(|| ctx.get("api_key").cloned())
        .unwrap_or_default();
    let base_url = first_env_value(env, &["ANTHROPIC_BASE_URL"])
        .or_else(|| ctx.get("base_url").cloned())
        .unwrap_or_default();
    let model = first_env_value(env, &["ANTHROPIC_MODEL"])
        .or_else(|| ctx.get("model").cloned())
        .unwrap_or_default();
    let auto_compact_window = ctx
        .get("model_context_window")
        .filter(|value| !value.is_empty())
        .cloned();

    env.retain(|(key, _)| !is_standardized_claude_env_key(key));
    push_env_if_nonempty(env, "ANTHROPIC_API_KEY", api_key.clone());
    push_env_if_nonempty(env, "ANTHROPIC_AUTH_TOKEN", api_key);
    push_env_if_nonempty(env, "ANTHROPIC_BASE_URL", base_url);
    push_env_if_nonempty(env, "ANTHROPIC_MODEL", model);
    push_env_if_configured(
        env,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        ctx.get("claude_default_haiku_model"),
    );
    push_env_if_configured(
        env,
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        ctx.get("claude_default_sonnet_model"),
    );
    push_env_if_configured(
        env,
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        ctx.get("claude_default_opus_model"),
    );
    if let Some(auto_compact_window) = auto_compact_window {
        push_env_if_nonempty(env, "CLAUDE_CODE_AUTO_COMPACT_WINDOW", auto_compact_window);
    }
}

fn first_env_value(env: &[(String, String)], keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        env.iter()
            .find(|(candidate, value)| candidate == key && !value.is_empty())
            .map(|(_, value)| value.clone())
    })
}

fn push_env_if_nonempty(env: &mut Vec<(String, String)>, key: &str, value: String) {
    if !value.is_empty() {
        env.push((key.to_string(), value));
    }
}

fn push_env_if_configured(env: &mut Vec<(String, String)>, key: &str, value: Option<&String>) {
    if let Some(value) = value {
        env.push((key.to_string(), value.clone()));
    }
}

fn is_standardized_claude_env_key(key: &str) -> bool {
    matches!(
        key,
        "ANTHROPIC_API_KEY"
            | "ANTHROPIC_AUTH_TOKEN"
            | "ANTHROPIC_BASE_URL"
            | "ANTHROPIC_MODEL"
            | "ANTHROPIC_DEFAULT_OPUS_MODEL"
            | "ANTHROPIC_DEFAULT_SONNET_MODEL"
            | "ANTHROPIC_DEFAULT_HAIKU_MODEL"
            | "CLAUDE_CODE_AUTO_COMPACT_WINDOW"
            | "CLAUDE_CODE_SUBAGENT_MODEL"
            | "CLAUDE_CODE_EFFORT_LEVEL"
    )
}

fn codex_provider_env_key(provider_id: &str) -> &'static str {
    match provider_id {
        "azure" => "AZURE_OPENAI_API_KEY",
        _ => "OPENAI_API_KEY",
    }
}

fn is_codex_launch_target(launch_target: &str) -> bool {
    matches!(launch_target, "codex" | "codex-desktop")
}

fn is_claude_launch_target(launch_target: &str) -> bool {
    matches!(launch_target, "claude" | "claude-desktop")
}

/// Wraps a value as a TOML literal string (`'...'`).  Literal strings have no
/// escape sequences so they never contain `"` or `\` delimiters.  This is
/// important on Windows where PowerShell 5.1 mangles native-command arguments
/// that contain `"` characters.
///
/// Falls back to a basic (double-quoted) string when the value contains `'`,
/// which is the only character forbidden inside TOML literal strings.
fn toml_string(s: &str) -> String {
    if s.contains('\'') {
        // Fallback: TOML basic string with standard escaping.
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

// ---------------------------------------------------------------------------
// Context builder
// ---------------------------------------------------------------------------

fn build_context(
    profile: &ProfileDef,
    api_type: &str,
    launch_target: &str,
    endpoint: &EndpointDef,
    catalog: &ProviderCatalog,
) -> BTreeMap<String, String> {
    let overrides = profile
        .overrides
        .get(api_type)
        .cloned()
        .unwrap_or_else(ApiTypeOverrides::default);

    let mut ctx: BTreeMap<String, String> = BTreeMap::new();
    let vibewbz_codex = is_vibewbz_codex_profile(profile, api_type);
    let provider_id = if vibewbz_codex {
        codex_provider_id_from_label(&profile.label)
    } else {
        profile.provider.clone()
    };
    ctx.insert("provider_id".to_string(), provider_id.clone());
    ctx.insert("provider_table_key".to_string(), toml_key(&provider_id));
    ctx.insert(
        "provider_label".to_string(),
        if vibewbz_codex {
            profile.label.clone()
        } else {
            catalog.label.clone()
        },
    );
    if vibewbz_codex {
        ctx.insert("codex_auth_json".to_string(), "true".to_string());
    }
    ctx.insert("api_type".to_string(), api_type.to_string());
    let base_url = overrides
        .base_url
        .unwrap_or_else(|| endpoint.default_base_url.clone());
    let base_url = if is_codex_launch_target(launch_target) {
        codex_provider_base_url(api_type, &base_url)
    } else {
        base_url
    };
    ctx.insert("base_url".to_string(), base_url);
    let requested_model = overrides
        .model
        .filter(|model| !model.trim().is_empty())
        .or_else(|| endpoint.models.first().map(|model| model.id.clone()))
        .unwrap_or_default();
    if let Some(model) = overrides.claude_default_haiku_model {
        ctx.insert("claude_default_haiku_model".to_string(), model);
    }
    if let Some(model) = overrides.claude_default_sonnet_model {
        ctx.insert("claude_default_sonnet_model".to_string(), model);
    }
    if let Some(model) = overrides.claude_default_opus_model {
        ctx.insert("claude_default_opus_model".to_string(), model);
    }
    let model_def = catalog::find_model(endpoint, &requested_model);
    let model = model_def
        .map(|model_def| model_def.id.clone())
        .unwrap_or(requested_model);
    if let Some(context_window) = model_def.and_then(|model_def| model_def.context_window) {
        ctx.insert(
            "model_context_window".to_string(),
            context_window.to_string(),
        );
    }
    ctx.insert("model".to_string(), model);
    ctx.insert(
        "reasoning_effort".to_string(),
        overrides
            .reasoning_effort
            .unwrap_or_else(|| "medium".to_string()),
    );

    // Credentials are flattened in last so a (hypothetical) catalog field
    // named "model" doesn't shadow the explicitly-resolved override above.
    // In practice fields are domain-specific (e.g. `api_key`); the ordering
    // is defensive.
    for (k, v) in &profile.credentials {
        if k == "base_url" || k == "model" {
            continue;
        }
        ctx.insert(k.clone(), v.clone());
    }
    ctx
}

fn is_vibewbz_codex_profile(profile: &ProfileDef, api_type: &str) -> bool {
    profile.provider == "custom" && api_type == "openai-responses"
}

fn codex_provider_id_from_label(label: &str) -> String {
    let label = label.trim();
    if label.is_empty() {
        "VibeWbz Gateway".to_string()
    } else {
        label.to_string()
    }
}

fn toml_key(key: &str) -> String {
    if is_bare_toml_key(key) {
        key.to_string()
    } else {
        json_string(key)
    }
}

fn is_bare_toml_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    out.push_str(&json_escape(s));
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// Mustache-lite
// ---------------------------------------------------------------------------

fn substitute(template: &str, ctx: &BTreeMap<String, String>) -> String {
    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // Find the closing `}}`.
            let after_open = i + 2;
            if let Some(close_rel) = template[after_open..].find("}}") {
                let raw = template[after_open..after_open + close_rel].trim();
                // `{{name|filter}}` runs the named value through a filter
                // before substitution. Used to JSON-escape secrets that
                // get spliced into auth.json templates — without this an
                // api_key containing `"` or `\` would corrupt the file.
                let (name, filter) = match raw.split_once('|') {
                    Some((n, f)) => (n.trim(), Some(f.trim())),
                    None => (raw, None),
                };
                if let Some(v) = ctx.get(name) {
                    let rendered = match filter {
                        Some("json") => json_escape(v),
                        Some(other) => {
                            tracing::warn!(
                                "[profiles] unknown template filter '{}' on '{{{{ {} | {} }}}}'; \
                                 substituting raw value",
                                other,
                                name,
                                other
                            );
                            v.clone()
                        }
                        None => v.clone(),
                    };
                    out.push_str(&rendered);
                }
                i = after_open + close_rel + 2;
                continue;
            }
            // No closing — treat as literal and bail out of the scan.
            out.push_str(&template[i..]);
            break;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// JSON-escape the *contents* of a string literal — the caller is
/// responsible for the surrounding `"`. This intentionally does NOT add
/// the outer quotes so catalog templates can keep the JSON shape
/// human-readable (`"OPENAI_API_KEY": "{{api_key|json}}"`).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Validators
// ---------------------------------------------------------------------------

pub fn validate_rel_path(rel: &str) -> anyhow::Result<()> {
    if rel.is_empty() {
        bail!("rel_path is empty");
    }
    if rel.starts_with('/') || rel.starts_with('\\') {
        bail!("rel_path must not be absolute: '{}'", rel);
    }
    for component in rel.split(['/', '\\']) {
        if component == ".." {
            bail!("rel_path must not contain '..': '{}'", rel);
        }
    }
    Ok(())
}

fn is_valid_env_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::profiles::schema::{ApiTypeOverrides, AuthMode, ProfileDef};

    use super::*;

    #[test]
    fn claude_launch_env_uses_gateway_anthropic_shape() {
        let profile = gateway_profile("anthropic", "claude-sonnet-4-5");
        let provider = catalog::custom();
        let rendered =
            render(&profile, "anthropic", "claude", provider).expect("claude profile renders");
        let keys: Vec<_> = rendered.env.iter().map(|(key, _)| key.as_str()).collect();

        assert_eq!(
            keys,
            vec![
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_AUTH_TOKEN",
                "ANTHROPIC_BASE_URL",
                "ANTHROPIC_MODEL",
            ]
        );
        assert_eq!(
            env_value(&rendered.env, "ANTHROPIC_API_KEY"),
            Some("test-key")
        );
        assert_eq!(
            env_value(&rendered.env, "ANTHROPIC_BASE_URL"),
            Some("http://ai.939593.xyz")
        );
        assert_eq!(
            env_value(&rendered.env, "ANTHROPIC_MODEL"),
            Some("claude-sonnet-4-5")
        );
        assert!(rendered.command_args.is_empty());
        assert!(rendered.settings_files.is_empty());
        assert!(rendered.config_env.is_none());
    }

    #[test]
    fn claude_desktop_launch_uses_claude_env_shape() {
        let profile = gateway_profile("anthropic", "claude-sonnet-4-5");
        let provider = catalog::custom();

        let cli =
            render(&profile, "anthropic", "claude", provider).expect("claude profile renders");
        let desktop = render(&profile, "anthropic", "claude-desktop", provider)
            .expect("claude desktop profile renders");

        assert_eq!(desktop.env, cli.env);
        assert!(desktop.command_args.is_empty());
        assert!(desktop.settings_files.is_empty());
        assert!(desktop.config_env.is_none());
    }

    #[test]
    fn claude_launch_env_includes_default_model_pins() {
        let mut profile = gateway_profile("anthropic", "claude-sonnet-4-5");
        let overrides = profile
            .overrides
            .get_mut("anthropic")
            .expect("anthropic overrides");
        overrides.claude_default_haiku_model = Some("claude-haiku-test".to_string());
        overrides.claude_default_sonnet_model = Some("claude-sonnet-test".to_string());
        overrides.claude_default_opus_model = Some("claude-opus-test".to_string());
        let provider = catalog::custom();

        let rendered =
            render(&profile, "anthropic", "claude", provider).expect("claude profile renders");

        assert_eq!(
            env_value(&rendered.env, "ANTHROPIC_DEFAULT_HAIKU_MODEL"),
            Some("claude-haiku-test")
        );
        assert_eq!(
            env_value(&rendered.env, "ANTHROPIC_DEFAULT_SONNET_MODEL"),
            Some("claude-sonnet-test")
        );
        assert_eq!(
            env_value(&rendered.env, "ANTHROPIC_DEFAULT_OPUS_MODEL"),
            Some("claude-opus-test")
        );
    }

    #[test]
    fn codex_launch_uses_gateway_responses_config_args() {
        let profile = gateway_profile("openai-responses", "gpt-5.5");
        let provider = catalog::custom();

        let rendered =
            render(&profile, "openai-responses", "codex", provider).expect("codex profile renders");

        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model='gpt-5.5'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model_provider='VibeWbz Gateway Test'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg
                == "model_providers.\"VibeWbz Gateway Test\".base_url='http://ai.939593.xyz/v1'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model_providers.\"VibeWbz Gateway Test\".wire_api='responses'"));
        assert!(
            rendered
                .command_args
                .iter()
                .any(|arg| arg
                    == "model_providers.\"VibeWbz Gateway Test\".requires_openai_auth=true")
        );
        assert!(rendered
            .env
            .contains(&("OPENAI_API_KEY".to_string(), "test-key".to_string())));
        assert!(rendered.config_env.is_none());
    }

    #[test]
    fn codex_desktop_launch_uses_codex_config_args() {
        let profile = gateway_profile("openai-responses", "gpt-5.5");
        let provider = catalog::custom();

        let rendered = render(&profile, "openai-responses", "codex-desktop", provider)
            .expect("codex desktop profile renders");

        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model='gpt-5.5'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model_provider='VibeWbz Gateway Test'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg == "model_providers.\"VibeWbz Gateway Test\".wire_api='responses'"));
        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg
                == "model_providers.\"VibeWbz Gateway Test\".base_url='http://ai.939593.xyz/v1'"));
        assert!(rendered.config_env.is_none());
    }

    #[test]
    fn codex_launch_does_not_duplicate_existing_v1_base_url() {
        let mut profile = gateway_profile("openai-responses", "gpt-5.5");
        profile
            .overrides
            .get_mut("openai-responses")
            .expect("openai responses overrides")
            .base_url = Some("http://ai.939593.xyz/v1/".to_string());
        let provider = catalog::custom();

        let rendered =
            render(&profile, "openai-responses", "codex", provider).expect("codex profile renders");

        assert!(rendered
            .command_args
            .iter()
            .any(|arg| arg
                == "model_providers.\"VibeWbz Gateway Test\".base_url='http://ai.939593.xyz/v1'"));
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
                reasoning_effort: Some("medium".to_string()),
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

    fn env_value<'a>(env: &'a [(String, String)], key: &str) -> Option<&'a str> {
        env.iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value.as_str())
    }
}
