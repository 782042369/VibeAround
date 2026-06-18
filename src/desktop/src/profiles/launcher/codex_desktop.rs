//! Codex Desktop profile overlay.
//!
//! Codex Desktop reads the shared `~/.codex/config.toml`, while the CLI can
//! take profile-specific `-c` args. For desktop profile launches, reconcile our
//! previous marker blocks first, then write a fresh VibeWbz-owned overlay.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ::common::{auth, config, profiles};
use anyhow::Context;
use profiles::{render::RenderedProfile, ProfileDef};

use super::codex;

const MARKER: &str = "VIBEWBZ-CODEX-DESKTOP";
const ROOT_KEYS: &[&str] = &[
    "model",
    "model_provider",
    "model_reasoning_effort",
    "model_context_window",
    "model_auto_compact_token_limit",
    "model_catalog_json",
    "disable_response_storage",
];
const MANAGED_TABLE_KEYS: &[&str] = &[
    "features.memories",
    "memories.consolidation_model",
    "memories.extract_model",
    "memories.max_raw_memories_for_consolidation",
    "memories.max_rollout_age_days",
];
const FEATURE_KEYS: &[&str] = &["memories"];
const MEMORY_KEYS: &[&str] = &[
    "consolidation_model",
    "extract_model",
    "max_raw_memories_for_consolidation",
    "max_rollout_age_days",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayBlock {
    Restore,
    Active,
    Provider,
}

#[derive(Debug)]
struct CodexDesktopOverlay {
    launch_id: String,
    profile_id: String,
    root_entries: Vec<(String, String)>,
    provider_id: String,
    provider_entries: Vec<(String, String)>,
}

pub(super) fn apply_profile_overlay(
    profile: &ProfileDef,
    launch_id: &str,
    rendered: RenderedProfile,
) -> anyhow::Result<()> {
    let env = profiles::runtime::materialize_env(&profile.id, rendered.clone())
        .with_context(|| format!("materialize Codex Desktop profile '{}'", profile.id))?;
    if let Some(token) = env_value(&env, "OPENAI_API_KEY") {
        write_auth_json(&codex_auth_path(), &token)?;
    }
    let overlay = CodexDesktopOverlay::from_rendered(profile, launch_id, &rendered, &env);
    apply_overlay_to_path(&codex_config_path(), &overlay)
}

pub(super) fn cleanup_profile_overlay() -> anyhow::Result<()> {
    cleanup_overlay_at_path(&codex_config_path())
}

fn codex_config_path() -> PathBuf {
    config::home_dir().join(".codex").join("config.toml")
}

fn codex_auth_path() -> PathBuf {
    config::home_dir().join(".codex").join("auth.json")
}

fn write_auth_json(path: &Path, token: &str) -> anyhow::Result<()> {
    let contents = serde_json::to_string_pretty(&serde_json::json!({
        "OPENAI_API_KEY": token,
    }))?;
    write_file_if_changed(path, &(contents + "\n"))
}

fn apply_overlay_to_path(path: &Path, overlay: &CodexDesktopOverlay) -> anyhow::Result<()> {
    let current = std::fs::read_to_string(path).unwrap_or_default();
    let next = apply_overlay_to_string(&current, overlay);
    write_config_if_changed(path, &current, next)
}

fn cleanup_overlay_at_path(path: &Path) -> anyhow::Result<()> {
    let current = match std::fs::read_to_string(path) {
        Ok(current) => current,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("read Codex config {:?}", path)),
    };
    let next = cleanup_vibewbz_blocks(&current);
    write_config_if_changed(path, &current, next)
}

fn write_config_if_changed(path: &Path, current: &str, next: String) -> anyhow::Result<()> {
    if next == current {
        return Ok(());
    }
    write_file_if_changed(path, &next)
}

fn write_file_if_changed(path: &Path, contents: &str) -> anyhow::Result<()> {
    if matches!(std::fs::read_to_string(path), Ok(current) if current == contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create Codex dir {:?}", parent))?;
    }
    let tmp = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("codex-file"),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&tmp, contents).with_context(|| format!("write Codex temp {:?}", tmp))?;
    auth::set_owner_only(&tmp).with_context(|| format!("chmod Codex temp {:?}", tmp))?;
    std::fs::rename(&tmp, path).with_context(|| format!("replace Codex file {:?}", path))?;
    auth::set_owner_only(path).with_context(|| format!("chmod Codex file {:?}", path))?;
    Ok(())
}

fn apply_overlay_to_string(current: &str, overlay: &CodexDesktopOverlay) -> String {
    let cleaned = cleanup_vibewbz_blocks(current);
    let (body, restore_lines) = remove_conflicting_root_keys(&cleaned);
    let mut sections = Vec::new();
    if !restore_lines.is_empty() {
        sections.push(render_restore_block(overlay, &restore_lines));
    }
    sections.push(render_active_block(overlay));
    if !overlay.provider_entries.is_empty() {
        sections.push(render_provider_block(overlay));
    }

    let body = body.trim_start_matches('\n');
    let mut out = sections.join("\n\n");
    if !body.trim().is_empty() {
        out.push_str("\n\n");
        out.push_str(body);
    }
    ensure_trailing_newline(out)
}

fn cleanup_vibewbz_blocks(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some(kind) = begin_block_kind(lines[i]) else {
            out.push(lines[i].to_string());
            i += 1;
            continue;
        };

        let Some(end_index) = find_end_block(&lines, i + 1, kind) else {
            out.push(lines[i].to_string());
            i += 1;
            continue;
        };

        if kind == OverlayBlock::Restore {
            for line in &lines[i + 1..end_index] {
                out.push(uncomment_restore_line(line));
            }
        }
        i = end_index + 1;
    }
    ensure_trailing_newline(out.join("\n"))
}

fn remove_conflicting_root_keys(input: &str) -> (String, Vec<String>) {
    let mut body = Vec::new();
    let mut restore = Vec::new();
    let mut in_root = true;
    let mut section: Option<String> = None;
    let mut restore_features_header = false;
    let mut restore_memories_header = false;

    for line in input.lines() {
        let trimmed = line.trim_start();
        if let Some(next_section) = section_name(trimmed) {
            in_root = false;
            section = Some(next_section.to_string());
        }
        if in_root {
            if let Some(key) = root_key_for_line(trimmed) {
                if ROOT_KEYS.contains(&key) {
                    restore.push(line.to_string());
                    continue;
                }
            }
        } else if let Some(section) = section.as_deref() {
            if let Some(key) = root_key_for_line(trimmed) {
                if section == "features" && FEATURE_KEYS.contains(&key) {
                    if !restore_features_header {
                        restore.push("[features]".to_string());
                        restore_features_header = true;
                    }
                    restore.push(line.to_string());
                    continue;
                }
                if section == "memories" && MEMORY_KEYS.contains(&key) {
                    if !restore_memories_header {
                        restore.push("[memories]".to_string());
                        restore_memories_header = true;
                    }
                    restore.push(line.to_string());
                    continue;
                }
            }
        }
        body.push(line.to_string());
    }

    (ensure_trailing_newline(body.join("\n")), restore)
}

impl CodexDesktopOverlay {
    fn from_rendered(
        profile: &ProfileDef,
        launch_id: &str,
        rendered: &RenderedProfile,
        _env: &[(String, String)],
    ) -> Self {
        let entries = config_entries_from_args(&rendered.command_args);
        let original_provider = entries
            .iter()
            .find(|(key, _)| key == "model_provider")
            .and_then(|(_, value)| parse_toml_string(value))
            .unwrap_or_else(|| profile.provider.clone());
        let provider_id = if profile.provider == "custom" {
            original_provider.clone()
        } else {
            format!("vibewbz_{}", safe_config_key(&profile.id))
        };

        let mut root_entries = Vec::new();
        for (key, value) in &entries {
            if ROOT_KEYS.contains(&key.as_str()) || MANAGED_TABLE_KEYS.contains(&key.as_str()) {
                if key == "model_provider" {
                    root_entries.push((key.clone(), codex::toml_string(&provider_id)));
                } else {
                    root_entries.push((key.clone(), value.clone()));
                }
            }
        }
        if !root_entries.iter().any(|(key, _)| key == "model_provider") {
            root_entries.push((
                "model_provider".to_string(),
                codex::toml_string(&provider_id),
            ));
        }

        let mut provider_entries = BTreeMap::new();
        let prefix = format!("model_providers.{original_provider}.");
        for (key, value) in &entries {
            if let Some(field) = key.strip_prefix(&prefix) {
                provider_entries.insert(field.to_string(), value.clone());
            }
        }

        provider_entries.remove("env_key");
        provider_entries.insert("requires_openai_auth".to_string(), "true".to_string());

        Self {
            launch_id: launch_id.to_string(),
            profile_id: profile.id.clone(),
            root_entries,
            provider_id,
            provider_entries: provider_entries.into_iter().collect(),
        }
    }
}

fn config_entries_from_args(args: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < args.len() {
        if args[i] == "-c" {
            if let Some((key, value)) = args[i + 1].split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if !key.is_empty() && !value.is_empty() {
                    out.push((key.to_string(), value.to_string()));
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

fn env_value(env: &[(String, String)], key: &str) -> Option<String> {
    env.iter()
        .find(|(candidate, value)| candidate == key && !value.is_empty())
        .map(|(_, value)| value.clone())
}

fn render_restore_block(overlay: &CodexDesktopOverlay, restore_lines: &[String]) -> String {
    let mut lines = vec![begin_marker(OverlayBlock::Restore, overlay)];
    lines.extend(restore_lines.iter().map(|line| format!("# {line}")));
    lines.push(end_marker(OverlayBlock::Restore));
    lines.join("\n")
}

fn render_active_block(overlay: &CodexDesktopOverlay) -> String {
    let mut lines = vec![begin_marker(OverlayBlock::Active, overlay)];
    let mut features = Vec::new();
    let mut memories = Vec::new();
    for (key, value) in &overlay.root_entries {
        if let Some(field) = key.strip_prefix("features.") {
            features.push((field, value));
        } else if let Some(field) = key.strip_prefix("memories.") {
            memories.push((field, value));
        } else {
            lines.push(format!("{key} = {value}"));
        }
    }
    if !features.is_empty() {
        lines.push(String::new());
        lines.push("[features]".to_string());
        lines.extend(
            features
                .into_iter()
                .map(|(key, value)| format!("{key} = {value}")),
        );
    }
    if !memories.is_empty() {
        lines.push(String::new());
        lines.push("[memories]".to_string());
        lines.extend(
            memories
                .into_iter()
                .map(|(key, value)| format!("{key} = {value}")),
        );
    }
    lines.push(end_marker(OverlayBlock::Active));
    lines.join("\n")
}

fn render_provider_block(overlay: &CodexDesktopOverlay) -> String {
    let mut lines = vec![begin_marker(OverlayBlock::Provider, overlay)];
    lines.push(format!("[model_providers.{}]", overlay.provider_id));
    lines.extend(
        overlay
            .provider_entries
            .iter()
            .map(|(key, value)| format!("{key} = {value}")),
    );
    lines.push(end_marker(OverlayBlock::Provider));
    lines.join("\n")
}

fn begin_marker(kind: OverlayBlock, overlay: &CodexDesktopOverlay) -> String {
    format!(
        "# {MARKER} BEGIN {} run={} profile={}",
        block_name(kind),
        overlay.launch_id,
        overlay.profile_id
    )
}

fn end_marker(kind: OverlayBlock) -> String {
    format!("# {MARKER} END {}", block_name(kind))
}

fn begin_block_kind(line: &str) -> Option<OverlayBlock> {
    let trimmed = line.trim();
    if !trimmed.starts_with("# ") || !trimmed.contains(MARKER) || !trimmed.contains(" BEGIN ") {
        return None;
    }
    if trimmed.contains(" BEGIN RESTORE") {
        Some(OverlayBlock::Restore)
    } else if trimmed.contains(" BEGIN ACTIVE") {
        Some(OverlayBlock::Active)
    } else if trimmed.contains(" BEGIN PROVIDER") {
        Some(OverlayBlock::Provider)
    } else {
        None
    }
}

fn find_end_block(lines: &[&str], start: usize, kind: OverlayBlock) -> Option<usize> {
    let end = end_marker(kind);
    lines
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, line)| line.trim() == end)
        .map(|(index, _)| index)
}

fn block_name(kind: OverlayBlock) -> &'static str {
    match kind {
        OverlayBlock::Restore => "RESTORE",
        OverlayBlock::Active => "ACTIVE",
        OverlayBlock::Provider => "PROVIDER",
    }
}

fn uncomment_restore_line(line: &str) -> String {
    line.strip_prefix("# ")
        .or_else(|| line.strip_prefix('#'))
        .unwrap_or(line)
        .to_string()
}

fn root_key_for_line(trimmed: &str) -> Option<&str> {
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    if key.contains('.') || key.is_empty() {
        return None;
    }
    Some(key)
}

fn section_name(trimmed: &str) -> Option<&str> {
    let name = trimmed.strip_prefix('[')?.strip_suffix(']')?.trim();
    (!name.is_empty()).then_some(name)
}

fn parse_toml_string(value: &str) -> Option<String> {
    let doc = format!("value = {value}");
    let parsed: toml::Value = toml::from_str(&doc).ok()?;
    parsed.get("value")?.as_str().map(ToOwned::to_owned)
}

fn safe_config_key(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("profile");
    }
    out
}

fn ensure_trailing_newline(mut value: String) -> String {
    if !value.is_empty() && !value.ends_with('\n') {
        value.push('\n');
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::common::profiles::schema::{AuthMode, ProfileDef};

    fn profile() -> ProfileDef {
        ProfileDef {
            id: "deepseek-main".to_string(),
            label: "DeepSeek".to_string(),
            provider: "deepseek".to_string(),
            auth_mode: AuthMode::ApiKey,
            api_types: vec!["openai-responses".to_string()],
            credentials: Default::default(),
            overrides: Default::default(),
            use_settings_proxy: false,
            provider_settings: Default::default(),
        }
    }

    fn overlay() -> CodexDesktopOverlay {
        let rendered = RenderedProfile {
            env: vec![("OPENAI_API_KEY".to_string(), "sk-test".to_string())],
            settings_files: Vec::new(),
            command_args: vec![
                "-c".to_string(),
                "model='deepseek-v4-pro'".to_string(),
                "-c".to_string(),
                "model_provider='deepseek'".to_string(),
                "-c".to_string(),
                "features.memories=true".to_string(),
                "-c".to_string(),
                "memories.consolidation_model='gpt-5.5'".to_string(),
                "-c".to_string(),
                "memories.extract_model='gpt-5.5'".to_string(),
                "-c".to_string(),
                "memories.max_raw_memories_for_consolidation=320".to_string(),
                "-c".to_string(),
                "memories.max_rollout_age_days=60".to_string(),
                "-c".to_string(),
                "model_providers.deepseek.name='DeepSeek'".to_string(),
                "-c".to_string(),
                "model_providers.deepseek.base_url='https://api.deepseek.com/v1'".to_string(),
                "-c".to_string(),
                "model_providers.deepseek.wire_api='responses'".to_string(),
                "-c".to_string(),
                "model_providers.deepseek.env_key='OPENAI_API_KEY'".to_string(),
            ],
            config_env: None,
        };
        CodexDesktopOverlay::from_rendered(&profile(), "launch-123", &rendered, &rendered.env)
    }

    #[test]
    fn overlay_comments_existing_root_keys_and_uses_managed_provider() {
        let current = r#"model = "gpt-5-codex"
model_provider = "openai"

[mcp_servers.local]
url = "http://127.0.0.1:12358/mcp"
"#;

        let next = apply_overlay_to_string(current, &overlay());

        assert!(next.contains("# VIBEWBZ-CODEX-DESKTOP BEGIN RESTORE"));
        assert!(next.contains("# model = \"gpt-5-codex\""));
        assert!(next.contains("model_provider = 'vibewbz_deepseek-main'"));
        assert!(next.contains("[features]\nmemories = true"));
        assert!(
            next.contains("[memories]\nconsolidation_model = 'gpt-5.5'\nextract_model = 'gpt-5.5'")
        );
        assert!(next.contains("max_raw_memories_for_consolidation = 320"));
        assert!(next.contains("max_rollout_age_days = 60"));
        assert!(next.contains("[model_providers.vibewbz_deepseek-main]"));
        assert!(next.contains("requires_openai_auth = true"));
        assert!(!next.contains("env_key = 'OPENAI_API_KEY'"));
        assert!(next.contains("[mcp_servers.local]\nurl = \"http://127.0.0.1:12358/mcp\""));
    }

    #[test]
    fn cleanup_restores_previous_root_keys_and_removes_provider_block() {
        let with_overlay = apply_overlay_to_string("model = \"gpt-5-codex\"\n", &overlay());
        let cleaned = cleanup_vibewbz_blocks(&with_overlay);

        assert!(cleaned.contains("model = \"gpt-5-codex\""));
        assert!(!cleaned.contains(MARKER));
        assert!(!cleaned.contains("[model_providers.vibewbz_deepseek-main]"));
    }

    #[test]
    fn cleanup_restores_previous_memory_settings() {
        let current = r#"[features]
memories = false

[memories]
consolidation_model = "gpt-5"
extract_model = "gpt-5"
max_raw_memories_for_consolidation = 100
max_rollout_age_days = 30
"#;
        let with_overlay = apply_overlay_to_string(current, &overlay());
        let cleaned = cleanup_vibewbz_blocks(&with_overlay);

        assert!(cleaned.contains("[features]\nmemories = false"));
        assert!(cleaned
            .contains("[memories]\nconsolidation_model = \"gpt-5\"\nextract_model = \"gpt-5\""));
        assert!(cleaned.contains("max_raw_memories_for_consolidation = 100"));
        assert!(cleaned.contains("max_rollout_age_days = 30"));
        assert!(!cleaned.contains(MARKER));
    }

    #[test]
    fn overlay_is_idempotent_for_vibewbz_blocks() {
        let first = apply_overlay_to_string("model = \"gpt-5-codex\"\n", &overlay());
        let second = apply_overlay_to_string(&first, &overlay());

        assert_eq!(
            second
                .matches("# VIBEWBZ-CODEX-DESKTOP BEGIN ACTIVE")
                .count(),
            1
        );
        assert_eq!(
            second
                .matches("[model_providers.vibewbz_deepseek-main]")
                .count(),
            1
        );
    }

    #[test]
    fn cleanup_overlay_at_path_restores_config_for_direct_launch() {
        let dir = std::env::temp_dir().join(format!(
            "vibewbz-codex-desktop-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("config.toml");
        let with_overlay = apply_overlay_to_string("model = \"gpt-5-codex\"\n", &overlay());
        std::fs::write(&path, with_overlay).expect("write test config");

        cleanup_overlay_at_path(&path).expect("cleanup overlay");

        let cleaned = std::fs::read_to_string(&path).expect("read cleaned config");
        assert!(cleaned.contains("model = \"gpt-5-codex\""));
        assert!(!cleaned.contains(MARKER));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn auth_json_is_written_next_to_config_toml() {
        let dir =
            std::env::temp_dir().join(format!("vibewbz-codex-auth-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let auth_path = dir.join("auth.json");

        write_auth_json(&auth_path, "sk-test").expect("write auth json");

        let contents = std::fs::read_to_string(&auth_path).expect("read auth json");
        assert_eq!(contents, "{\n  \"OPENAI_API_KEY\": \"sk-test\"\n}\n");
        let _ = std::fs::remove_dir_all(dir);
    }
}
