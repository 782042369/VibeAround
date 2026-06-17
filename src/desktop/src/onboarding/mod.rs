//! Onboarding: first-run setup wizard.
//! Checks whether settings.json has `"onboarded": true`; exposes Tauri IPC
//! commands so the desktop-ui frontend can read/write settings and signal completion.

mod agent_integrations;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Output, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::io::AsyncReadExt;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::{agent_detection, restart_daemon, OnboardingActive};
use common::config;

use crate::startkit::{StartkitChoices, StartkitItemReport, StartkitItemStatus};

const AGENT_UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Shared state types
// ---------------------------------------------------------------------------

pub struct OnboardingGate {
    pub notify: Arc<Notify>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUpdateCheckRequest {
    pub agent_ids: Vec<String>,
    pub choices: StartkitChoices,
}

// ---------------------------------------------------------------------------
// Settings helpers
// ---------------------------------------------------------------------------

fn settings_path() -> PathBuf {
    config::data_dir().join("settings.json")
}

fn read_settings_value() -> Value {
    let path = settings_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn write_settings_value(val: &Value) -> Result<(), String> {
    // settings.json holds bot tokens, webhook secrets, and tunnel credentials
    // in plain text (by design — the user edits this file directly). Ensure
    // other local users cannot read it. No-op on Windows.
    config::write_settings_json(val)
}

// ---------------------------------------------------------------------------
// Onboarding gate
// ---------------------------------------------------------------------------

/// Read current settings (exposed for startup integration sync).
#[allow(dead_code)]
pub fn get_settings_value() -> serde_json::Value {
    read_settings_value()
}

pub fn needs_onboarding() -> bool {
    let val = read_settings_value();
    !val.get("onboarded")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Resource summary types exposed to the desktop onboarding UI.
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct AgentSummary {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub install_type: Option<String>,
    pub pty_command: String,
    pub direct_only: bool,
    pub download_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Tauri commands — settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_settings() -> Result<Value, String> {
    Ok(read_settings_value())
}

#[tauri::command]
pub fn save_settings<R: Runtime>(app: AppHandle<R>, settings: Value) -> Result<(), String> {
    write_settings_value(&settings)?;
    let _ = app.emit(crate::tray::LAUNCH_CONFIG_CHANGED_EVENT, ());
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands — resource queries
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_agents() -> Vec<AgentSummary> {
    common::resources::AGENTS
        .iter()
        .filter(|a| a.supports_current_platform())
        .map(|a| AgentSummary {
            id: a.id.clone(),
            display_name: a.display_name.clone(),
            description: a.description.clone(),
            install_type: a.install.as_ref().map(|i| i.install_type.clone()),
            pty_command: a.pty_command_for_current_platform().to_string(),
            direct_only: a.direct_only,
            download_url: download_url_for_current_platform(a),
        })
        .collect()
}

fn download_url_for_current_platform(agent: &common::resources::AgentDef) -> Option<String> {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            return agent
                .download_urls
                .windows_arm64
                .clone()
                .or_else(|| agent.download_urls.windows_x64.clone());
        }
        return agent.download_urls.windows_x64.clone();
    }
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            return agent
                .download_urls
                .macos_aarch64
                .clone()
                .or_else(|| agent.download_urls.macos_x64.clone());
        }
        return agent.download_urls.macos_x64.clone();
    }
    None
}

#[tauri::command]
pub async fn scan_agent_install_status(
    settings: Value,
    choices: StartkitChoices,
) -> Result<Vec<StartkitItemReport>, String> {
    Ok(agent_cli_reports(&settings, &choices, &choices.agents)
        .await
        .map_err(|error| error.to_string())?)
}

async fn agent_cli_reports(
    settings: &Value,
    choices: &StartkitChoices,
    agent_ids: &[String],
) -> anyhow::Result<Vec<StartkitItemReport>> {
    let startkit_reports = crate::startkit::scan_agent_cli_reports(settings, choices, agent_ids)
        .await?
        .into_iter()
        .map(|report| (report.id.clone(), report))
        .collect::<HashMap<_, _>>();
    let mut reports = Vec::new();

    for agent_id in agent_ids {
        let report_id = format!("agents.{agent_id}.cli");
        if let Some(report) = startkit_reports.get(&report_id) {
            reports.push(report.clone());
            continue;
        }
        if let Some(agent) = common::resources::agent_by_id(agent_id) {
            reports.push(agent_install_report(agent.clone()).await);
        }
    }

    Ok(reports)
}

#[tauri::command]
pub async fn check_agent_updates(
    request: AgentUpdateCheckRequest,
) -> Result<Vec<StartkitItemReport>, String> {
    let mut tasks = JoinSet::new();

    for agent_id in request.agent_ids {
        let choices = request.choices.clone();
        tasks.spawn(async move { agent_update_report(agent_id, choices).await });
    }

    let mut reports = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if let Some(report) = result.map_err(|error| error.to_string())? {
            reports.push(report);
        }
    }
    Ok(reports)
}

async fn agent_install_report(agent: common::resources::AgentDef) -> StartkitItemReport {
    let agent_id = agent.id.clone();
    let report_id = format!("agents.{}.cli", agent.id);

    let candidate = if let Some(candidate) =
        agent_detection::configured_candidate_with_version(&agent_id).await
    {
        Some(candidate)
    } else {
        agent_detection::scan_agent_and_persist(&agent_id)
            .await
            .ok()
            .and_then(|detection| detection.system_selected_candidate())
    };

    if let Some(candidate) = candidate {
        return StartkitItemReport {
            id: report_id,
            label: agent.display_name,
            group: "agents".to_string(),
            category: "agents".to_string(),
            status: StartkitItemStatus::Ok,
            severity: None,
            version: candidate.version,
            latest_version: None,
            path: Some(candidate.path),
            message: Some(format!("已找到 {}", candidate.source_label)),
            actions: Vec::new(),
            manual_command: None,
            manual_url: None,
            secret: false,
            settings_key: None,
        };
    }

    let program = program_from_command(agent.pty_command_for_current_platform())
        .unwrap_or_else(|| agent.acp.program.clone());
    StartkitItemReport {
        id: report_id,
        label: agent.display_name.clone(),
        group: "agents".to_string(),
        category: "agents".to_string(),
        status: StartkitItemStatus::Blocked,
        severity: None,
        version: None,
        latest_version: None,
        path: None,
        message: Some(format!("没有找到 {program}。请先安装它，然后重新检查。")),
        actions: Vec::new(),
        manual_command: None,
        manual_url: None,
        secret: false,
        settings_key: None,
    }
}

async fn agent_update_report(
    agent_id: String,
    choices: StartkitChoices,
) -> Option<StartkitItemReport> {
    let agent = common::resources::agent_by_id(&agent_id)?;
    let candidate = if let Some(candidate) =
        agent_detection::configured_candidate_with_version(&agent_id).await
    {
        candidate
    } else {
        agent_detection::scan_agent_and_persist(&agent_id)
            .await
            .ok()
            .and_then(|detection| {
                agent_detection::preferred_startkit_candidate(
                    &agent_id,
                    &detection,
                    &choices.toolchain_mode,
                )
            })
            .or_else(|| {
                agent_detection::startkit_candidate_for_mode(&agent_id, &choices.toolchain_mode)
            })?
    };
    let source = candidate.source.clone();
    let local_version = candidate.version.as_deref().and_then(extract_semver);
    let mut report = StartkitItemReport {
        id: format!("agents.{agent_id}.cli"),
        label: agent.display_name.clone(),
        group: "agents".to_string(),
        category: "agents".to_string(),
        status: StartkitItemStatus::Ok,
        severity: None,
        version: candidate.version.clone(),
        latest_version: None,
        path: Some(candidate.path.clone()),
        message: None,
        actions: Vec::new(),
        manual_command: None,
        manual_url: None,
        secret: false,
        settings_key: None,
    };
    let Some(local_version) = local_version else {
        report.message = Some("暂时查不到新版本".to_string());
        return Some(report);
    };

    let latest = match tokio::time::timeout(
        AGENT_UPDATE_CHECK_TIMEOUT,
        latest_version_for_agent_source(&agent_id, &source, &choices),
    )
    .await
    {
        Ok(Ok(Some(version))) => version,
        Ok(_) => {
            report.message = Some("暂时查不到新版本".to_string());
            return Some(report);
        }
        Err(_) => {
            report.message = Some("查新版本超时了".to_string());
            return Some(report);
        }
    };

    report.label = agent.display_name.clone();
    report.id = format!("agents.{agent_id}.cli");
    report.latest_version = Some(latest.clone());

    if local_version != latest {
        report.message = Some(format!("可以手动更新到 {latest}"));
    } else {
        report.message = Some("已是最新版本".to_string());
    }
    Some(report)
}

async fn latest_version_for_agent_source(
    agent_id: &str,
    source: &str,
    choices: &StartkitChoices,
) -> anyhow::Result<Option<String>> {
    if let Some(package) = agent_detection::source_package(agent_id, source) {
        return npm_latest_version(&package, &choices.source).await;
    }
    if source == "homebrew_formula" || source == "homebrew_cask" {
        return homebrew_latest_version(agent_id, source).await;
    }
    Ok(None)
}

async fn homebrew_latest_version(agent_id: &str, source: &str) -> anyhow::Result<Option<String>> {
    let Some(template) = agent_detection::source_command_template(agent_id, source, "upgrade")
    else {
        return Ok(None);
    };
    let Some(token) = template.split_whitespace().last() else {
        return Ok(None);
    };
    let kind = if source == "homebrew_cask" {
        "--cask"
    } else {
        "--formula"
    };
    let mut command = tokio::process::Command::new("brew");
    command.args(["info", "--json=v2", kind, token]);
    let output = command_output_with_timeout(command, AGENT_UPDATE_CHECK_TIMEOUT)
        .await
        .map_err(anyhow::Error::msg)?;
    if !output.status.success() {
        return Ok(None);
    }
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parse brew info")?;
    if source == "homebrew_cask" {
        Ok(value
            .get("casks")
            .and_then(|items| items.as_array())
            .and_then(|items| items.first())
            .and_then(|item| item.get("version"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string))
    } else {
        Ok(value
            .get("formulae")
            .and_then(|items| items.as_array())
            .and_then(|items| items.first())
            .and_then(|item| item.get("versions"))
            .and_then(|versions| versions.get("stable"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string))
    }
}

fn program_from_command(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .next()
        .map(|program| program.trim_matches(['"', '\'']).to_string())
        .filter(|program| !program.is_empty())
}

async fn command_output_with_timeout(
    mut command: tokio::process::Command,
    max_duration: Duration,
) -> Result<Output, String> {
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.kill_on_drop(true);

    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "stdout was not captured".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "stderr was not captured".to_string())?;

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).await.map(|_| buf)
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).await.map(|_| buf)
    });

    let started = Instant::now();
    let status = loop {
        if started.elapsed() >= max_duration {
            let _ = child.kill().await;
            return Err("version check timed out".to_string());
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        sleep(Duration::from_millis(50)).await;
    };

    let stdout = stdout_task
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let stderr = stderr_task
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

async fn npm_latest_version(package: &str, source: &str) -> anyhow::Result<Option<String>> {
    if let Some(version) = requested_package_version(package) {
        return Ok(Some(version));
    }

    let package_name = npm_package_name(package);
    let encoded = encode_npm_package_for_url(&package_name);
    let registry = npm_registry_for_source(source);
    let url = format!("{}/{}", registry.trim_end_matches('/'), encoded);
    let client = reqwest::Client::builder()
        .timeout(AGENT_UPDATE_CHECK_TIMEOUT)
        .build()
        .context("build npm metadata client")?;
    let value: serde_json::Value = client
        .get(url)
        .header("accept", "application/vnd.npm.install-v1+json")
        .send()
        .await
        .context("fetch npm package metadata")?
        .error_for_status()
        .context("npm package metadata status")?
        .json()
        .await
        .context("parse npm package metadata")?;
    Ok(value
        .get("dist-tags")
        .and_then(|tags| tags.get("latest"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string))
}

fn npm_registry_for_source(source: &str) -> &'static str {
    match source {
        "cn" => "https://registry.npmmirror.com",
        _ => "https://registry.npmjs.org",
    }
}

fn npm_package_name(package: &str) -> String {
    if let Some(rest) = package.strip_prefix('@') {
        if let Some((scope, name_and_version)) = rest.split_once('/') {
            let name = name_and_version
                .rsplit_once('@')
                .map(|(name, _)| name)
                .unwrap_or(name_and_version);
            return format!("@{scope}/{name}");
        }
    }
    package
        .rsplit_once('@')
        .map(|(name, _)| name)
        .unwrap_or(package)
        .to_string()
}

fn requested_package_version(package: &str) -> Option<String> {
    if let Some(rest) = package.strip_prefix('@') {
        let (_, name_and_version) = rest.split_once('/')?;
        return name_and_version
            .rsplit_once('@')
            .and_then(|(_, version)| (!version.is_empty()).then(|| version.to_string()));
    }
    package
        .rsplit_once('@')
        .and_then(|(_, version)| (!version.is_empty()).then(|| version.to_string()))
}

fn encode_npm_package_for_url(package: &str) -> String {
    package.replace('@', "%40").replace('/', "%2F")
}

fn extract_semver(value: &str) -> Option<String> {
    for token in value.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')) {
        let token = token.trim_start_matches('v');
        let mut parts = token.split('.');
        let major = parts.next()?;
        let minor = parts.next()?;
        let patch = parts.next()?;
        if major.chars().all(|ch| ch.is_ascii_digit())
            && minor.chars().all(|ch| ch.is_ascii_digit())
            && patch.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        {
            return Some(token.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tauri commands — onboarding flow
// ---------------------------------------------------------------------------

/// Marks onboarding complete and signals the daemon gate.
#[tauri::command]
pub async fn finish_onboarding<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let mut settings = read_settings_value();
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("onboarded".into(), serde_json::json!(true));
    }
    write_settings_value(&settings)?;

    let _ = app.emit("onboarding-complete", ());

    if let Some(active) = app.try_state::<OnboardingActive>() {
        let was_onboarding = active.0.swap(false, Ordering::Relaxed);
        if was_onboarding {
            if let Some(gate) = app.try_state::<OnboardingGate>() {
                gate.notify.notify_one();
            }
        } else {
            restart_daemon(&app).await?;
        }
    }

    Ok(())
}
