//! Startkit: manifest-driven environment doctor and repair runner.
//!
//! The manifest and scripts live under `src/resources/startkit/`. This module
//! keeps the engine generic: resolve the item graph from user choices, execute
//! platform scripts with a stable environment, and normalize all output into
//! structured item reports for the onboarding UI.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::agent_detection;

const SETTINGS_TOML: &str = include_str!("../../../resources/startkit/settings.toml");
const STARTKIT_PROGRESS_EVENT: &str = "startkit-progress";
const STARTKIT_COMPLETE_EVENT: &str = "startkit-complete";
pub struct StartkitRunState {
    cancelled: Arc<AtomicBool>,
}

impl Default for StartkitRunState {
    fn default() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub startkit: StartkitInfo,
    #[serde(default)]
    pub runner: RunnerConfig,
    #[serde(default)]
    pub sources: HashMap<String, SourceConfig>,
    #[serde(default, rename = "items")]
    pub items: Vec<StartkitItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartkitInfo {
    pub id: String,
    pub name: String,
    pub schema: u32,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunnerConfig {
    #[serde(default = "default_timeout_secs")]
    pub default_timeout_secs: u64,
    #[serde(default)]
    pub log_redact_keys: Vec<String>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            default_timeout_secs: default_timeout_secs(),
            log_redact_keys: Vec::new(),
        }
    }
}

fn default_timeout_secs() -> u64 {
    600
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceConfig {
    pub label: String,
    pub node_index: String,
    pub node_dist: String,
    pub npm_registry: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartkitItem {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub include_if: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub managed: bool,
    #[serde(default)]
    pub plugin_dependency: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub version_arg: Option<String>,
    #[serde(default)]
    pub npm_package: Option<String>,
    #[serde(default)]
    pub settings_key: Option<String>,
    #[serde(default)]
    pub secret: bool,
    #[serde(default)]
    pub detect: Option<PlatformScript>,
    #[serde(default)]
    pub install: Option<PlatformScript>,
    #[serde(default)]
    pub repair: Option<PlatformScript>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformScript {
    #[serde(default)]
    pub macos: Option<String>,
    #[serde(default)]
    pub windows: Option<String>,
    #[serde(default)]
    pub linux: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

impl PlatformScript {
    fn for_platform(&self, platform: &str) -> Option<&str> {
        match platform {
            "macos" => self.macos.as_deref(),
            "windows" => self.windows.as_deref(),
            "linux" => self.linux.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartkitChoices {
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default = "default_toolchain_mode")]
    pub toolchain_mode: String,
    #[serde(default)]
    pub shell_path: bool,
}

impl Default for StartkitChoices {
    fn default() -> Self {
        Self {
            agents: Vec::new(),
            source: default_source(),
            toolchain_mode: default_toolchain_mode(),
            shell_path: false,
        }
    }
}

fn default_source() -> String {
    "global".to_string()
}

fn default_toolchain_mode() -> String {
    "system".to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartkitManifestSummary {
    pub id: String,
    pub name: String,
    pub schema: u32,
    pub version: String,
    pub sources: HashMap<String, SourceConfig>,
    pub items: Vec<StartkitItemSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartkitItemSummary {
    pub id: String,
    pub label: String,
    pub group: String,
    pub category: String,
    pub description: Option<String>,
    pub severity: Option<String>,
    pub kind: Option<String>,
    pub managed: bool,
    pub has_repair: bool,
    pub secret: bool,
    pub settings_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartkitPlan {
    pub platform: String,
    pub source: String,
    pub item_ids: Vec<String>,
    pub items: Vec<StartkitItemSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartkitItemStatus {
    Pending,
    Running,
    Ok,
    Missing,
    Outdated,
    Broken,
    NeedsConfig,
    Blocked,
    Error,
    Skipped,
}

impl StartkitItemStatus {
    fn from_script(value: &str) -> Self {
        match value {
            "ok" => Self::Ok,
            "missing" => Self::Missing,
            "outdated" => Self::Outdated,
            "broken" => Self::Broken,
            "needs_config" => Self::NeedsConfig,
            "blocked" => Self::Blocked,
            "skipped" => Self::Skipped,
            _ => Self::Error,
        }
    }

    fn needs_install(&self) -> bool {
        matches!(self, Self::Missing | Self::Outdated | Self::Broken)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartkitItemReport {
    pub id: String,
    pub label: String,
    pub group: String,
    pub category: String,
    pub status: StartkitItemStatus,
    pub severity: Option<String>,
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub path: Option<String>,
    pub message: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual_url: Option<String>,
    #[serde(default)]
    pub secret: bool,
    #[serde(default)]
    pub settings_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartkitScanReport {
    pub plan: StartkitPlan,
    pub reports: Vec<StartkitItemReport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartkitProgressEvent {
    pub id: String,
    pub label: String,
    pub status: StartkitItemStatus,
    pub message: Option<String>,
    pub report: Option<StartkitItemReport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartkitCompleteEvent {
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ScriptOutput {
    status: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    latest_version: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    actions: Vec<String>,
    #[serde(default)]
    manual_command: Option<String>,
    #[serde(default)]
    manual_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StartkitPaths {
    pub root: PathBuf,
    pub home: PathBuf,
    pub cache_dir: PathBuf,
}

impl StartkitPaths {
    pub fn new(root: PathBuf) -> Self {
        let home = common::config::data_dir();
        Self {
            root,
            cache_dir: home.join("cache").join("startkit"),
            home,
        }
    }
}

pub fn load_manifest() -> anyhow::Result<Manifest> {
    toml::from_str(SETTINGS_TOML).context("parsing startkit/settings.toml")
}

#[tauri::command]
pub fn startkit_manifest() -> Result<StartkitManifestSummary, String> {
    manifest_summary().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn startkit_plan(choices: StartkitChoices) -> Result<StartkitPlan, String> {
    plan(&choices, None).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn startkit_scan<R: Runtime>(
    app: AppHandle<R>,
    settings: Value,
    choices: StartkitChoices,
) -> Result<StartkitScanReport, String> {
    scan_with_progress(Some(&app), &settings, &choices, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_startkit_install<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, StartkitRunState>,
    settings: Value,
    choices: StartkitChoices,
) -> Result<(), String> {
    state.cancelled.store(false, Ordering::Relaxed);
    common::config::write_settings_json(&settings).map_err(|e| e.to_string())?;

    let cancelled = Arc::clone(&state.cancelled);
    tauri::async_runtime::spawn(async move {
        let status = match run_startkit_install(app.clone(), settings, choices, cancelled).await {
            Ok(status) => status,
            Err(err) => {
                let _ = app.emit(
                    STARTKIT_PROGRESS_EVENT,
                    StartkitProgressEvent {
                        id: "startkit".to_string(),
                        label: "Startkit".to_string(),
                        status: StartkitItemStatus::Error,
                        message: Some(err.to_string()),
                        report: None,
                    },
                );
                "error".to_string()
            }
        };
        let _ = app.emit(STARTKIT_COMPLETE_EVENT, StartkitCompleteEvent { status });
    });

    Ok(())
}

#[tauri::command]
pub async fn cancel_startkit_install(state: State<'_, StartkitRunState>) -> Result<(), String> {
    state.cancelled.store(true, Ordering::Relaxed);
    Ok(())
}

pub fn manifest_summary() -> anyhow::Result<StartkitManifestSummary> {
    let manifest = load_manifest()?;
    Ok(StartkitManifestSummary {
        id: manifest.startkit.id,
        name: manifest.startkit.name,
        schema: manifest.startkit.schema,
        version: manifest.startkit.version,
        sources: manifest.sources,
        items: manifest.items.iter().map(item_summary).collect(),
    })
}

pub fn plan(choices: &StartkitChoices, platform: Option<&str>) -> anyhow::Result<StartkitPlan> {
    let manifest = load_manifest()?;
    plan_from_manifest(&manifest, choices, platform.unwrap_or(current_platform()))
}

#[allow(dead_code)]
pub async fn scan(
    settings: &Value,
    choices: &StartkitChoices,
    platform: Option<&str>,
) -> anyhow::Result<StartkitScanReport> {
    scan_with_progress::<tauri::Wry>(None, settings, choices, platform).await
}

pub(crate) async fn scan_agent_cli_reports(
    settings: &Value,
    choices: &StartkitChoices,
    agent_ids: &[String],
) -> anyhow::Result<Vec<StartkitItemReport>> {
    let _ = settings;
    let manifest = load_manifest()?;
    let mut tasks = JoinSet::new();

    for (index, agent_id) in agent_ids.iter().enumerate() {
        let item_id = format!("agents.{agent_id}.cli");
        let Ok(item) = find_item(&manifest, &item_id).cloned() else {
            continue;
        };
        let agent_id = agent_id.clone();
        let choices = choices.clone();
        tasks.spawn(async move {
            let report = scan_agent_cli_item(&item, &agent_id, &choices).await;
            (index, report)
        });
    }

    let mut reports = Vec::new();
    while let Some(result) = tasks.join_next().await {
        reports.push(result?);
    }
    reports.sort_by_key(|(index, _)| *index);
    Ok(reports.into_iter().map(|(_, report)| report).collect())
}

async fn scan_with_progress<R: Runtime>(
    app: Option<&AppHandle<R>>,
    settings: &Value,
    choices: &StartkitChoices,
    platform: Option<&str>,
) -> anyhow::Result<StartkitScanReport> {
    let manifest = load_manifest()?;
    let platform = platform.unwrap_or(current_platform());
    let plan = plan_from_manifest(&manifest, choices, platform)?;
    let paths = StartkitPaths::new(startkit_root());
    let mut reports = Vec::new();

    for item_id in &plan.item_ids {
        let item = find_item(&manifest, item_id)?;
        if let Some(app) = app {
            emit_progress(
                app,
                item,
                StartkitItemStatus::Running,
                Some("Checking".to_string()),
                None,
            );
        }
        let report = scan_item(&manifest, &paths, item, settings, choices, platform).await;
        if let Some(app) = app {
            emit_progress(
                app,
                item,
                report.status.clone(),
                report.message.clone(),
                Some(report.clone()),
            );
        }
        reports.push(report);
    }

    Ok(StartkitScanReport { plan, reports })
}

#[allow(dead_code)]
pub async fn execute_item(
    settings: &Value,
    choices: &StartkitChoices,
    item_id: &str,
) -> anyhow::Result<StartkitItemReport> {
    execute_item_with_cancel(settings, choices, item_id, None, None).await
}

async fn execute_item_with_cancel(
    settings: &Value,
    choices: &StartkitChoices,
    item_id: &str,
    cancelled: Option<&Arc<AtomicBool>>,
    progress: Option<&(dyn Fn(&StartkitItem, StartkitItemStatus, Option<String>) + Sync)>,
) -> anyhow::Result<StartkitItemReport> {
    let manifest = load_manifest()?;
    let platform = current_platform();
    let paths = StartkitPaths::new(startkit_root());
    let item = find_item(&manifest, item_id)?;
    if let Some(agent_id) = agent_id_from_cli_item(&item.id) {
        return execute_agent_cli_item(item, agent_id, choices, cancelled, progress).await;
    }
    let before = scan_item(&manifest, &paths, item, settings, choices, platform).await;

    if !before.status.needs_install() {
        return Ok(before);
    }

    let Some(script) = &item.install else {
        return Ok(StartkitItemReport {
            status: StartkitItemStatus::Blocked,
            message: Some("这个工具暂时不能自动安装，请手动安装后再检查".to_string()),
            ..base_report(item)
        });
    };

    let Some(script_path) = script.for_platform(platform) else {
        return Ok(StartkitItemReport {
            status: StartkitItemStatus::Blocked,
            message: Some(format!("当前系统暂不支持自动安装：{platform}")),
            ..base_report(item)
        });
    };

    if let Some(progress) = progress {
        progress(
            item,
            StartkitItemStatus::Running,
            Some(install_phase_message(item)),
        );
    }

    match run_script(
        &manifest,
        &paths,
        item,
        choices,
        platform,
        script_path,
        script,
        cancelled,
    )
    .await
    {
        Ok(report) => report_from_script(item, report),
        Err(err) => StartkitItemReport {
            status: StartkitItemStatus::Error,
            message: Some(err.to_string()),
            ..base_report(item)
        },
    }
    .pipe(Ok)
}

async fn run_startkit_install<R: Runtime>(
    app: AppHandle<R>,
    settings: Value,
    choices: StartkitChoices,
    cancelled: Arc<AtomicBool>,
) -> anyhow::Result<String> {
    let manifest = load_manifest()?;
    let platform = current_platform();
    let plan = plan_from_manifest(&manifest, &choices, platform)?;
    let mut had_error = false;
    let mut needs_input = false;
    let mut blocked_item_ids = HashSet::<String>::new();

    for item_id in &plan.item_ids {
        if cancelled.load(Ordering::Relaxed) {
            return Ok("cancelled".to_string());
        }

        let item = find_item(&manifest, item_id)?;
        if effective_item_dependencies(item)
            .iter()
            .any(|dependency| blocked_item_ids.contains(*dependency))
        {
            blocked_item_ids.insert(item.id.clone());
            emit_progress(
                &app,
                item,
                StartkitItemStatus::Skipped,
                Some("前面有东西没装好，这项先跳过".to_string()),
                Some(StartkitItemReport {
                    status: StartkitItemStatus::Skipped,
                    message: Some("前面有东西没装好，这项先跳过".to_string()),
                    ..base_report(item)
                }),
            );
            continue;
        }

        let report = if item.kind.as_deref() == Some("managed_npm_package") {
            run_managed_npm_package_item(&app, item, &cancelled).await
        } else {
            let progress =
                |item: &StartkitItem, status: StartkitItemStatus, message: Option<String>| {
                    emit_progress(&app, item, status, message, None);
                };
            execute_item_with_cancel(
                &settings,
                &choices,
                item_id,
                Some(&cancelled),
                Some(&progress),
            )
            .await
        };

        match report {
            Ok(report) => {
                if matches!(
                    report.status,
                    StartkitItemStatus::Error | StartkitItemStatus::Blocked
                ) {
                    had_error = true;
                    blocked_item_ids.insert(item.id.clone());
                }
                if matches!(report.status, StartkitItemStatus::NeedsConfig) {
                    needs_input = true;
                }
                emit_progress(
                    &app,
                    item,
                    report.status.clone(),
                    report.message.clone(),
                    Some(report),
                );
            }
            Err(err) => {
                had_error = true;
                blocked_item_ids.insert(item.id.clone());
                emit_progress(
                    &app,
                    item,
                    StartkitItemStatus::Error,
                    Some(err.to_string()),
                    None,
                );
            }
        }
    }

    Ok(if cancelled.load(Ordering::Relaxed) {
        "cancelled"
    } else if had_error {
        "error"
    } else if needs_input {
        "needs_input"
    } else {
        "complete"
    }
    .to_string())
}

async fn execute_agent_cli_item(
    item: &StartkitItem,
    agent_id: &str,
    choices: &StartkitChoices,
    cancelled: Option<&Arc<AtomicBool>>,
    progress: Option<&(dyn Fn(&StartkitItem, StartkitItemStatus, Option<String>) + Sync)>,
) -> anyhow::Result<StartkitItemReport> {
    let before = scan_agent_cli_item(item, agent_id, choices).await;
    if !before.status.needs_install() {
        return Ok(before);
    }

    let Some(package) = agent_cli_npm_install_package(agent_id) else {
        return Ok(StartkitItemReport {
            status: StartkitItemStatus::Blocked,
            message: Some("这个工具暂时不能自动安装，请手动安装后再检查".to_string()),
            ..base_report(item)
        });
    };

    if let Some(progress) = progress {
        progress(
            item,
            StartkitItemStatus::Running,
            Some(format!("正在安装 {}", item.label)),
        );
    }

    let log_progress = |line| {
        if let Some(progress) = progress {
            progress(item, StartkitItemStatus::Running, Some(line));
        }
    };
    let is_cancelled = || {
        cancelled
            .map(|flag| flag.load(Ordering::Relaxed))
            .unwrap_or(false)
    };

    let result = if choices.toolchain_mode == "managed" {
        let install_dir = common::process::env::acp_agents_dir();
        common::agent::auto_install_npm_package_in_dir_with_progress_and_cancel(
            &package,
            &install_dir,
            log_progress,
            is_cancelled,
        )
        .await
    } else {
        common::agent::auto_install_npm_global_package_with_progress_and_cancel(
            &package,
            log_progress,
            is_cancelled,
        )
        .await
    };

    if let Err(error) = result {
        return Ok(StartkitItemReport {
            status: StartkitItemStatus::Error,
            message: Some(error.to_string()),
            ..base_report(item)
        });
    }

    let after = scan_agent_cli_item(item, agent_id, choices).await;
    if matches!(after.status, StartkitItemStatus::Ok) {
        Ok(after)
    } else {
        Ok(StartkitItemReport {
            status: StartkitItemStatus::Error,
            message: Some(format!(
                "{} 安装结束了，但现在还没检测到{}",
                item.label,
                after
                    .message
                    .as_deref()
                    .map(|message| format!(": {message}"))
                    .unwrap_or_default()
            )),
            ..base_report(item)
        })
    }
}

async fn run_managed_npm_package_item<R: Runtime>(
    app: &AppHandle<R>,
    item: &StartkitItem,
    cancelled: &Arc<AtomicBool>,
) -> anyhow::Result<StartkitItemReport> {
    let before = scan_managed_npm_package_item(item);
    if !before.status.needs_install() {
        return Ok(before);
    }

    let Some(package) = item.npm_package.as_deref() else {
        return Ok(StartkitItemReport {
            status: StartkitItemStatus::Blocked,
            message: Some("没有配置要安装的 npm 包".to_string()),
            ..base_report(item)
        });
    };

    let install_dir = managed_item_dependency_dir(item)?;
    emit_progress(
        app,
        item,
        StartkitItemStatus::Running,
        Some(format!("正在安装 {}", item.label)),
        None,
    );

    common::agent::auto_install_npm_package_in_dir_with_progress_and_cancel(
        package,
        &install_dir,
        |line| {
            emit_progress(app, item, StartkitItemStatus::Running, Some(line), None);
        },
        || cancelled.load(Ordering::Relaxed),
    )
    .await?;

    let after = scan_managed_npm_package_item(item);
    if matches!(after.status, StartkitItemStatus::Ok) {
        Ok(after)
    } else {
        Ok(StartkitItemReport {
            status: StartkitItemStatus::Error,
            message: Some(format!("{} 安装结束了，但现在还不能用", item.label)),
            ..base_report(item)
        })
    }
}

fn install_phase_message(item: &StartkitItem) -> String {
    match item.id.as_str() {
        "essentials.node" => "正在下载 Node.js".to_string(),
        _ => format!("正在安装 {}", item.label),
    }
}

fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    item: &StartkitItem,
    status: StartkitItemStatus,
    message: Option<String>,
    report: Option<StartkitItemReport>,
) {
    emit_progress_event(
        app,
        item.id.clone(),
        item.label.clone(),
        status,
        message,
        report,
    );
}

fn emit_progress_event<R: Runtime>(
    app: &AppHandle<R>,
    id: String,
    label: String,
    status: StartkitItemStatus,
    message: Option<String>,
    report: Option<StartkitItemReport>,
) {
    let _ = app.emit(
        STARTKIT_PROGRESS_EVENT,
        StartkitProgressEvent {
            id,
            label,
            status,
            message,
            report,
        },
    );
}

async fn scan_item(
    manifest: &Manifest,
    paths: &StartkitPaths,
    item: &StartkitItem,
    settings: &Value,
    choices: &StartkitChoices,
    platform: &str,
) -> StartkitItemReport {
    if let Some(agent_id) = agent_id_from_cli_item(&item.id) {
        return scan_agent_cli_item(item, agent_id, choices).await;
    }

    if item.kind.as_deref() == Some("config") {
        return scan_config_item(item, settings);
    }

    if item.kind.as_deref() == Some("managed_npm_package") {
        return scan_managed_npm_package_item(item);
    }

    let Some(detect) = &item.detect else {
        return StartkitItemReport {
            status: StartkitItemStatus::Pending,
            message: Some("这个项目还没有配置检测方式".to_string()),
            ..base_report(item)
        };
    };

    let Some(script_path) = detect.for_platform(platform) else {
        return StartkitItemReport {
            status: StartkitItemStatus::Blocked,
            message: Some(format!("当前系统暂不支持自动检测：{platform}")),
            ..base_report(item)
        };
    };

    match run_script(
        manifest,
        paths,
        item,
        choices,
        platform,
        script_path,
        detect,
        None,
    )
    .await
    {
        Ok(output) => apply_manual_guidance(report_from_script(item, output), item, choices),
        Err(err) => StartkitItemReport {
            status: StartkitItemStatus::Error,
            message: Some(err.to_string()),
            ..base_report(item)
        },
    }
}

fn scan_managed_npm_package_item(item: &StartkitItem) -> StartkitItemReport {
    let Some(package) = item.npm_package.as_deref() else {
        return StartkitItemReport {
            status: StartkitItemStatus::Blocked,
            message: Some("没有配置要安装的 npm 包".to_string()),
            ..base_report(item)
        };
    };
    let bin_name = item
        .program
        .as_deref()
        .map(str::to_string)
        .unwrap_or_else(|| common::agent::npm_package_bin_name(package));
    let Ok(install_dir) = managed_item_dependency_dir(item) else {
        return StartkitItemReport {
            status: StartkitItemStatus::Blocked,
            message: Some("没有找到安装目录".to_string()),
            ..base_report(item)
        };
    };

    if common::agent::npm_package_installed_in_dir(package, &bin_name, &install_dir) {
        let bin_path = common::process::env::resolve_npm_bin_in_dir(&install_dir, &bin_name)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
        return StartkitItemReport {
            status: StartkitItemStatus::Ok,
            path: bin_path,
            message: Some(format!("{} 已准备好", item.label)),
            actions: Vec::new(),
            ..base_report(item)
        };
    }

    StartkitItemReport {
        status: StartkitItemStatus::Missing,
        message: Some(format!("将安装 {}", item.label)),
        actions: vec!["install".to_string()],
        ..base_report(item)
    }
}

fn managed_item_dependency_dir(item: &StartkitItem) -> anyhow::Result<PathBuf> {
    let dependency_id = item
        .plugin_dependency
        .as_deref()
        .ok_or_else(|| anyhow!("managed item '{}' has no dependency id", item.id))?;
    Ok(common::plugins::user_plugin_dependency_dir(dependency_id))
}

fn agent_cli_npm_install_package(agent_id: &str) -> Option<String> {
    if !agent_detection::agent_uses_npm_install(agent_id) {
        return None;
    }
    agent_detection::source_package(agent_id, "npm_global")
}

async fn scan_agent_cli_item(
    item: &StartkitItem,
    agent_id: &str,
    choices: &StartkitChoices,
) -> StartkitItemReport {
    let selected = if let Some(candidate) =
        agent_detection::configured_candidate_with_version(agent_id).await
    {
        Some(candidate)
    } else {
        agent_detection::scan_agent_and_persist(agent_id)
            .await
            .ok()
            .and_then(|detection| {
                agent_detection::preferred_startkit_candidate(
                    agent_id,
                    &detection,
                    &choices.toolchain_mode,
                )
            })
    };

    match selected {
        Some(candidate) => StartkitItemReport {
            status: StartkitItemStatus::Ok,
            version: candidate.version,
            path: Some(candidate.path),
            message: Some(format!("已找到 {}", candidate.source_label)),
            actions: Vec::new(),
            ..base_report(item)
        },
        None => {
            if agent_cli_npm_install_package(agent_id).is_some() {
                let target = if choices.toolchain_mode == "managed" {
                    "到 VibeWbz 管理目录"
                } else {
                    "通过 npm"
                };
                return StartkitItemReport {
                    status: StartkitItemStatus::Missing,
                    message: Some(format!("将{target}安装 {}", item.label)),
                    actions: vec!["install".to_string()],
                    ..base_report(item)
                };
            }

            apply_agent_manual_guidance(
                StartkitItemReport {
                    status: StartkitItemStatus::Blocked,
                    message: Some(agent_missing_message(item, &choices.toolchain_mode)),
                    actions: Vec::new(),
                    ..base_report(item)
                },
                agent_id,
            )
        }
    }
}

fn agent_missing_message(item: &StartkitItem, toolchain_mode: &str) -> String {
    if toolchain_mode == "managed" {
        return format!("{} 暂时没有内置安装方式，请手动安装后再检查。", item.label);
    }
    format!(
        "没有在这台电脑上找到 {}。请先安装它，然后重新检查。",
        item.label
    )
}

fn apply_manual_guidance(
    mut report: StartkitItemReport,
    item: &StartkitItem,
    choices: &StartkitChoices,
) -> StartkitItemReport {
    if !matches!(
        report.status,
        StartkitItemStatus::Missing
            | StartkitItemStatus::Outdated
            | StartkitItemStatus::Broken
            | StartkitItemStatus::Blocked
    ) {
        return report;
    }

    let Some(guidance) = manual_guidance_for_item(item, choices) else {
        return report;
    };

    report.status = StartkitItemStatus::Blocked;
    report.message = Some(guidance.message);
    report.actions = vec!["manual".to_string()];
    report.manual_command = guidance.command;
    report.manual_url = guidance.url;
    report
}

fn apply_agent_manual_guidance(
    mut report: StartkitItemReport,
    agent_id: &str,
) -> StartkitItemReport {
    report.actions = vec!["manual".to_string()];
    report.manual_command = agent_detection::source_command_template(agent_id, "native", "install");
    report.manual_url = manual_agent_url(agent_id).map(str::to_string);
    report
}

struct ManualGuidance {
    message: String,
    command: Option<String>,
    url: Option<String>,
}

fn manual_guidance_for_item(
    item: &StartkitItem,
    _choices: &StartkitChoices,
) -> Option<ManualGuidance> {
    let platform = current_platform();
    match item.id.as_str() {
        "essentials.node" => Some(ManualGuidance {
            message: format!(
                "请安装 Node.js {} 或更新版本。Node.js 安装包会自带 npm。装好后重新检查。",
                item.min_version.as_deref().unwrap_or("22.0.0")
            ),
            command: None,
            url: Some("https://nodejs.org/en/download".to_string()),
        }),
        "essentials.git" => {
            let (command, url) = match platform {
                "macos" => (
                    Some("xcode-select --install".to_string()),
                    Some("https://developer.apple.com/documentation/xcode/installing-the-command-line-tools/".to_string()),
                ),
                "windows" => (
                    Some("winget install --id Git.Git -e --source winget".to_string()),
                    Some("https://git-scm.com/download/win".to_string()),
                ),
                _ => (None, Some("https://git-scm.com/downloads".to_string())),
            };
            Some(ManualGuidance {
                message: "请先安装 Git，然后重新检查。".to_string(),
                command,
                url,
            })
        }
        _ => None,
    }
}

fn manual_agent_url(agent_id: &str) -> Option<&'static str> {
    match agent_id {
        _ => None,
    }
}

fn agent_id_from_cli_item(item_id: &str) -> Option<&str> {
    item_id
        .strip_prefix("agents.")
        .and_then(|value| value.strip_suffix(".cli"))
}

fn scan_config_item(item: &StartkitItem, settings: &Value) -> StartkitItemReport {
    let has_value = item
        .settings_key
        .as_deref()
        .and_then(|key| settings_path_value(settings, key))
        .and_then(Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    StartkitItemReport {
        status: if has_value {
            StartkitItemStatus::Ok
        } else {
            StartkitItemStatus::NeedsConfig
        },
        message: Some(if has_value {
            "Configured".to_string()
        } else {
            "Needs configuration".to_string()
        }),
        actions: if has_value {
            Vec::new()
        } else {
            vec!["configure".to_string()]
        },
        ..base_report(item)
    }
}

async fn run_script(
    manifest: &Manifest,
    paths: &StartkitPaths,
    item: &StartkitItem,
    choices: &StartkitChoices,
    platform: &str,
    script_path: &str,
    script: &PlatformScript,
    cancelled: Option<&Arc<AtomicBool>>,
) -> anyhow::Result<ScriptOutput> {
    let full_path = paths.root.join(script_path);
    if !full_path.exists() {
        bail!("script not found: {}", full_path.display());
    }

    let mut command = if platform == "windows" {
        let mut cmd = Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]);
        cmd.arg(&full_path);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg(&full_path);
        cmd
    };

    command.args(&script.args);
    command.env_clear();
    command.envs(common::process::env::enriched_env().clone());
    apply_startkit_env(&mut command, manifest, paths, item, choices)?;
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let output = run_command_with_cancel(
        command,
        Duration::from_secs(manifest.runner.default_timeout_secs),
        cancelled,
    )
    .await
    .context("running startkit script")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| {
            anyhow!(
                "script did not emit JSON{}",
                if stderr.trim().is_empty() {
                    String::new()
                } else {
                    format!(": {}", redact(&stderr, &manifest.runner.log_redact_keys))
                }
            )
        })?;

    let parsed: ScriptOutput =
        serde_json::from_str(line).with_context(|| format!("parsing script JSON: {line}"))?;
    Ok(parsed)
}

async fn run_command_with_cancel(
    mut command: Command,
    max_duration: Duration,
    cancelled: Option<&Arc<AtomicBool>>,
) -> anyhow::Result<std::process::Output> {
    let mut child = command.spawn().context("spawning startkit script")?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("startkit script stdout was not captured"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("startkit script stderr was not captured"))?;

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
        if cancelled
            .map(|flag| flag.load(Ordering::Relaxed))
            .unwrap_or(false)
        {
            let _ = child.kill().await;
            bail!("cancelled");
        }
        if started.elapsed() >= max_duration {
            let _ = child.kill().await;
            bail!("startkit script timed out");
        }
        if let Some(status) = child.try_wait().context("polling startkit script")? {
            break status;
        }
        sleep(Duration::from_millis(200)).await;
    };

    let stdout = stdout_task
        .await
        .context("joining startkit stdout reader")?
        .context("reading startkit stdout")?;
    let stderr = stderr_task
        .await
        .context("joining startkit stderr reader")?
        .context("reading startkit stderr")?;

    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

fn apply_startkit_env(
    command: &mut Command,
    manifest: &Manifest,
    paths: &StartkitPaths,
    item: &StartkitItem,
    choices: &StartkitChoices,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(&paths.cache_dir).ok();

    let source = manifest
        .sources
        .get(&choices.source)
        .or_else(|| manifest.sources.get("global"))
        .ok_or_else(|| anyhow!("startkit source '{}' not found", choices.source))?;

    command.env("STARTKIT_HOME", &paths.home);
    command.env("STARTKIT_ROOT", &paths.root);
    command.env("STARTKIT_CACHE_DIR", &paths.cache_dir);
    command.env("STARTKIT_SOURCE", &choices.source);
    let managed_item_active =
        item_uses_managed_dependency_dir(item) && choices.toolchain_mode == "managed";
    command.env(
        "STARTKIT_ITEM_MANAGED",
        if managed_item_active { "true" } else { "false" },
    );
    command.env("STARTKIT_NPM_REGISTRY", &source.npm_registry);
    command.env("STARTKIT_NODE_INDEX_URL", &source.node_index);
    command.env("STARTKIT_NODE_DIST_BASE", &source.node_dist);
    command.env(
        "STARTKIT_CAN_INSTALL",
        if item.install.is_some() && (!item.managed || managed_item_active) {
            "true"
        } else {
            "false"
        },
    );
    command.env("STARTKIT_ITEM_ID", &item.id);
    if let Some(value) = &item.min_version {
        command.env("STARTKIT_MIN_VERSION", value);
    }
    if let Some(value) = &item.program {
        command.env("STARTKIT_PROGRAM", value);
    }
    if let Some(value) = &item.version_arg {
        command.env("STARTKIT_VERSION_ARG", value);
    }
    if let Some(value) = &item.npm_package {
        command.env("STARTKIT_NPM_PACKAGE", value);
    }
    if let Some(value) = &item.plugin_dependency {
        let plugin_dir = common::plugins::user_plugin_dependency_dir(value);
        let plugin_bin_dir = plugin_dir.join("bin");
        std::fs::create_dir_all(&plugin_bin_dir).ok();
        command.env("STARTKIT_PLUGIN_DIR", plugin_dir);
        command.env("STARTKIT_PLUGIN_BIN_DIR", plugin_bin_dir);
    }

    Ok(())
}

fn item_uses_managed_dependency_dir(item: &StartkitItem) -> bool {
    item.managed && item.plugin_dependency.is_some()
}

fn report_from_script(item: &StartkitItem, output: ScriptOutput) -> StartkitItemReport {
    StartkitItemReport {
        status: StartkitItemStatus::from_script(&output.status),
        version: output.version,
        latest_version: output.latest_version,
        path: output.path,
        message: output.message,
        actions: output.actions,
        manual_command: output.manual_command,
        manual_url: output.manual_url,
        ..base_report(item)
    }
}

fn base_report(item: &StartkitItem) -> StartkitItemReport {
    StartkitItemReport {
        id: item.id.clone(),
        label: item.label.clone(),
        group: item.group.clone(),
        category: item.category.clone(),
        status: StartkitItemStatus::Pending,
        severity: item.severity.clone(),
        version: None,
        latest_version: None,
        path: None,
        message: None,
        actions: Vec::new(),
        manual_command: None,
        manual_url: None,
        secret: item.secret,
        settings_key: item.settings_key.clone(),
    }
}

fn item_summary(item: &StartkitItem) -> StartkitItemSummary {
    StartkitItemSummary {
        id: item.id.clone(),
        label: item.label.clone(),
        group: item.group.clone(),
        category: item.category.clone(),
        description: item.description.clone(),
        severity: item.severity.clone(),
        kind: item.kind.clone(),
        managed: item.managed,
        has_repair: item.repair.is_some(),
        secret: item.secret,
        settings_key: item.settings_key.clone(),
    }
}

fn plan_from_manifest(
    manifest: &Manifest,
    choices: &StartkitChoices,
    platform: &str,
) -> anyhow::Result<StartkitPlan> {
    let by_id: HashMap<&str, &StartkitItem> = manifest
        .items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let mut selected = HashSet::<String>::new();

    for item in &manifest.items {
        if !supports_platform(item, platform) {
            continue;
        }
        if should_include(item, choices) {
            add_with_deps(item, &by_id, platform, &mut selected)?;
        }
    }

    let mut ordered = Vec::new();
    let mut temporary = HashSet::new();
    let mut permanent = HashSet::new();
    for id in selected.iter() {
        visit(
            id,
            &by_id,
            platform,
            &selected,
            &mut temporary,
            &mut permanent,
            &mut ordered,
        )?;
    }

    let items = ordered
        .iter()
        .map(|id| item_summary(find_item(manifest, id).expect("planned item exists")))
        .collect();

    Ok(StartkitPlan {
        platform: platform.to_string(),
        source: choices.source.clone(),
        item_ids: ordered,
        items,
    })
}

fn add_with_deps(
    item: &StartkitItem,
    by_id: &HashMap<&str, &StartkitItem>,
    platform: &str,
    selected: &mut HashSet<String>,
) -> anyhow::Result<()> {
    selected.insert(item.id.clone());
    for dep in effective_item_dependencies(item) {
        let dep_item = by_id
            .get(dep)
            .ok_or_else(|| anyhow!("startkit item '{}' depends on missing '{}'", item.id, dep))?;
        if !supports_platform(dep_item, platform) {
            continue;
        }
        add_with_deps(dep_item, by_id, platform, selected)?;
    }
    Ok(())
}

fn visit(
    id: &str,
    by_id: &HashMap<&str, &StartkitItem>,
    platform: &str,
    selected: &HashSet<String>,
    temporary: &mut HashSet<String>,
    permanent: &mut HashSet<String>,
    ordered: &mut Vec<String>,
) -> anyhow::Result<()> {
    if permanent.contains(id) {
        return Ok(());
    }
    if !temporary.insert(id.to_string()) {
        bail!("cycle in startkit item dependencies at '{id}'");
    }
    let item = by_id
        .get(id)
        .ok_or_else(|| anyhow!("planned startkit item missing: {id}"))?;
    for dep in effective_item_dependencies(item) {
        if selected.contains(dep) {
            let dep_item = by_id.get(dep).ok_or_else(|| {
                anyhow!("startkit item '{}' depends on missing '{}'", item.id, dep)
            })?;
            if supports_platform(dep_item, platform) {
                visit(
                    dep, by_id, platform, selected, temporary, permanent, ordered,
                )?;
            }
        }
    }
    temporary.remove(id);
    permanent.insert(id.to_string());
    ordered.push(id.to_string());
    Ok(())
}

fn effective_item_dependencies(item: &StartkitItem) -> Vec<&str> {
    let mut deps = item
        .depends_on
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some(agent_id) = agent_id_from_cli_item(&item.id) {
        if agent_detection::agent_uses_npm_install(agent_id) && !deps.contains(&"essentials.node") {
            deps.push("essentials.node");
        }
    }
    deps
}

fn should_include(item: &StartkitItem, choices: &StartkitChoices) -> bool {
    item.include_if.iter().any(|rule| match rule.as_str() {
        "always" => true,
        "agent:any" => !choices.agents.is_empty(),
        "shell_path:true" => choices.shell_path,
        "toolchain:system" => choices.toolchain_mode != "managed",
        "toolchain:managed" => choices.toolchain_mode == "managed",
        rule if rule.starts_with("agent:") => {
            let agent = &rule["agent:".len()..];
            choices.agents.iter().any(|id| id == agent)
        }
        _ => false,
    })
}

fn supports_platform(item: &StartkitItem, platform: &str) -> bool {
    item.platforms.is_empty() || item.platforms.iter().any(|p| p == platform)
}

fn find_item<'a>(manifest: &'a Manifest, id: &str) -> anyhow::Result<&'a StartkitItem> {
    manifest
        .items
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| anyhow!("unknown startkit item: {id}"))
}

fn settings_path_value<'a>(settings: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current = settings;
    for part in key.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

pub fn current_platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "windows" => "windows",
        other => other,
    }
}

fn startkit_root() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/startkit")
    }
    #[cfg(not(debug_assertions))]
    {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        for candidate in [
            exe_dir.join("_up_").join("resources").join("startkit"),
            exe_dir
                .join("..")
                .join("Resources")
                .join("_up_")
                .join("resources")
                .join("startkit"),
            exe_dir.join("resources").join("startkit"),
        ] {
            if candidate.exists() {
                return candidate;
            }
        }
        exe_dir.join("_up_").join("resources").join("startkit")
    }
}

fn redact(value: &str, keys: &[String]) -> String {
    let mut out = value.to_string();
    for key in keys {
        if key.is_empty() {
            continue;
        }
        out = out.replace(key, "***");
    }
    out
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(choices: StartkitChoices) -> Vec<String> {
        let manifest = load_manifest().unwrap();
        plan_from_manifest(&manifest, &choices, "macos")
            .unwrap()
            .item_ids
    }

    #[test]
    fn codex_only_plan_only_installs_codex_stack() {
        let item_ids = ids(StartkitChoices {
            agents: vec!["codex".to_string()],
            ..StartkitChoices::default()
        });

        assert!(item_ids.contains(&"essentials.node".to_string()));
        assert!(!item_ids.contains(&"agents.adapters".to_string()));
        assert!(!item_ids.contains(&"essentials.git".to_string()));
        assert!(item_ids.contains(&"agents.codex.cli".to_string()));
        assert!(!item_ids.contains(&"agents.claude.cli".to_string()));
        assert!(!item_ids.contains(&"tunnels.cloudflare.binary".to_string()));
        assert!(!item_ids.contains(&"channels.plugins".to_string()));
    }

    #[test]
    fn npm_cli_agent_depends_on_node_before_cli() {
        let item_ids = ids(StartkitChoices {
            agents: vec!["claude".to_string()],
            ..StartkitChoices::default()
        });

        let node = item_ids
            .iter()
            .position(|id| id == "essentials.node")
            .expect("node is planned");
        let cli = item_ids
            .iter()
            .position(|id| id == "agents.claude.cli")
            .expect("claude cli is planned");
        assert!(node < cli);
        assert_eq!(
            agent_cli_npm_install_package("claude").as_deref(),
            Some("@anthropic-ai/claude-code")
        );
    }

    #[test]
    fn non_npm_essentials_use_manual_guidance() {
        let manifest = load_manifest().unwrap();
        let node = find_item(&manifest, "essentials.node").unwrap();
        let report = apply_manual_guidance(
            StartkitItemReport {
                status: StartkitItemStatus::Missing,
                message: Some("Node.js will be installed".to_string()),
                actions: vec!["install".to_string()],
                ..base_report(node)
            },
            node,
            &StartkitChoices::default(),
        );

        assert_eq!(report.status, StartkitItemStatus::Blocked);
        assert_eq!(report.actions, vec!["manual".to_string()]);
        assert_eq!(
            report.manual_url.as_deref(),
            Some("https://nodejs.org/en/download")
        );
    }

    #[test]
    fn startkit_choices_default_to_system_toolchain() {
        let choices: StartkitChoices = serde_json::from_value(serde_json::json!({
            "agents": ["codex"],
            "source": "global"
        }))
        .unwrap();

        assert_eq!(choices.toolchain_mode, "system");
        assert!(!choices.shell_path);
    }

    #[test]
    fn shell_path_choice_no_longer_adds_environment_item() {
        let item_ids = ids(StartkitChoices {
            agents: vec!["codex".to_string()],
            shell_path: true,
            ..StartkitChoices::default()
        });

        assert!(!item_ids.contains(&"environment.shell_path".to_string()));
    }
}
