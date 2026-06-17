//! Launcher preference summary and validation.

use std::collections::{BTreeMap, HashSet};

use common::agent_state;
use common::profiles::{normalize_legacy_profile_and_persist, schema};
use common::{config, resources};
use serde::Serialize;

use super::connections::{merged_profile_connections, profile_can_launch_agent};
use super::terminal;
use super::workspace::{canonical_agent_id, resolve_agent_workspace_preference, WorkspaceOption};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherPreferences {
    /// Resolved cwd used for profile/direct launches.
    pub workspace: String,
    /// Suggested cwd choices surfaced in the Launch header.
    pub workspace_options: Vec<WorkspaceOption>,
    /// Canonical agent id selected in the Launch tab.
    pub selected_agent: String,
    /// Per-agent launch choices stored in `~/.vibewbz/agents.json`.
    pub agent_preferences: BTreeMap<String, AgentLaunchPreferenceSummary>,
    /// VibeWbz-wide default agent for tray quick launch and IM startup.
    pub default_agent: String,
    /// Optional profile paired with the VibeWbz-wide default agent.
    pub default_profile_id: Option<String>,
    /// Agent ids enabled by onboarding/settings.json.
    pub enabled_agents: Vec<String>,
    /// Back-compat alias for older UI code. New writes go to agents.json.
    pub default_profiles: BTreeMap<String, String>,
    /// Per-profile connection choices for launch targets that can run via
    /// the local API bridge.
    pub profile_connections: agent_state::ProfileConnectionPreferences,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLaunchPreferenceSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<AgentExecutablePreferenceSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExecutablePreferenceSummary {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realpath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub source: String,
    pub source_label: String,
    pub rank: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

pub(super) fn launcher_preferences() -> LauncherPreferences {
    let cfg = config::ensure_loaded();
    let agent_prefs = agent_state::read_prefs();
    let selected_agent = agent_state::resolve_selected_agent(&agent_prefs, &cfg);
    let default_agent = agent_state::resolve_default_agent(&agent_prefs, &cfg);
    let default_profile_id =
        agent_state::resolve_default_profile(&agent_prefs, &cfg, &default_agent);
    let workspace = resolve_agent_workspace_preference(&selected_agent, &agent_prefs)
        .unwrap_or_else(|_| terminal::launch_home_dir().unwrap_or_else(|_| config::data_dir()))
        .to_string_lossy()
        .to_string();
    let agent_preferences = summarize_agent_preferences(&agent_prefs, &cfg);
    let default_profiles = agent_preferences
        .iter()
        .filter_map(|(agent_id, preference)| {
            preference
                .profile_id
                .as_ref()
                .map(|profile_id| (agent_id.clone(), profile_id.clone()))
        })
        .collect();

    LauncherPreferences {
        workspace,
        workspace_options: Vec::new(),
        selected_agent,
        agent_preferences,
        default_agent,
        default_profile_id,
        enabled_agents: cfg.enabled_agents.clone(),
        default_profiles,
        profile_connections: merged_profile_connections(&agent_prefs),
    }
}

pub(super) fn validate_agent_profile_selection(
    agent_id: &str,
    profile_id: Option<String>,
) -> Result<(String, Option<String>), String> {
    let agent_id = resources::agent_by_alias(agent_id)
        .map(|def| def.id.clone())
        .ok_or_else(|| format!("unknown agent: '{agent_id}'"))?;
    let profile_id = profile_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());

    if let Some(profile_id) = &profile_id {
        let profile = schema::load(profile_id)
            .map(normalize_legacy_profile_and_persist)
            .ok_or_else(|| format!("profile '{profile_id}' not found"))?;
        if !profile_can_launch_agent(&profile, &agent_id) {
            return Err(format!("profile '{profile_id}' cannot launch '{agent_id}'"));
        }
    }

    Ok((agent_id, profile_id))
}

fn summarize_agent_preferences(
    agent_prefs: &agent_state::AgentsPrefsFile,
    cfg: &config::Config,
) -> BTreeMap<String, AgentLaunchPreferenceSummary> {
    let mut agent_ids: HashSet<String> = cfg
        .enabled_agents
        .iter()
        .map(|id| canonical_agent_id(id))
        .collect();
    agent_ids.extend(agent_prefs.agents.keys().map(|id| canonical_agent_id(id)));
    agent_ids.extend(cfg.default_profiles.keys().map(|id| canonical_agent_id(id)));

    let mut out = BTreeMap::new();
    for agent_id in agent_ids {
        let stored = agent_prefs.agents.get(&agent_id);
        let profile_id = stored
            .and_then(|preference| preference.profile_id.clone())
            .or_else(|| cfg.default_profiles.get(&agent_id).cloned());
        let workspace = stored
            .and_then(|preference| preference.workspace.as_ref())
            .map(|path| path.to_string_lossy().to_string());
        let executable = stored
            .and_then(|_| agent_state::resolve_agent_executable(agent_prefs, &agent_id))
            .map(executable_summary);
        let executable_path = executable
            .as_ref()
            .map(|executable| executable.path.clone());
        if profile_id.is_some() || workspace.is_some() || executable_path.is_some() {
            out.insert(
                agent_id,
                AgentLaunchPreferenceSummary {
                    profile_id,
                    workspace,
                    executable_path,
                    executable,
                },
            );
        }
    }
    out
}

fn executable_summary(
    executable: agent_state::AgentExecutablePreference,
) -> AgentExecutablePreferenceSummary {
    AgentExecutablePreferenceSummary {
        path: executable.path.to_string_lossy().to_string(),
        realpath: executable
            .realpath
            .map(|path| path.to_string_lossy().to_string()),
        version: executable.version,
        source: executable.source,
        source_label: executable.source_label,
        rank: executable.rank,
        package: executable.package,
    }
}
