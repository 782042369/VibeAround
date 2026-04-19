//! Wire-facing and dashboard-facing views of an `ACPPod`.

use serde::Serialize;

use agent_client_protocol as acp;

use crate::acp::routing::RouteKey;

/// Mutable runtime fields of a pod. Consumers (dashboard, TUI, CLI) that
/// want a consistent view of the pod's current state call
/// `ACPPod::state()` and get a clone of this struct.
///
/// Immutable fields (`route`, `started_at`, `bot_identity`) live directly
/// on `ACPPod` and are read without going through the state snapshot.
#[derive(Debug, Clone, Default)]
pub struct PodState {
    pub cli_kind: Option<String>,
    pub profile: Option<String>,
    pub session_id: Option<String>,
    pub workspace: Option<String>,
    pub busy: bool,
    pub failed: Option<String>,
    pub initialize: Option<acp::InitializeResponse>,
}

/// Legacy serializable snapshot — kept while `runtime_status` still
/// projects `SystemEvent::SnapshotChanged { snapshot }`. Will be removed
/// in the next commit when `runtime_status` goes away and consumers read
/// pods directly via `acp_hub.list().await` + `pod.state().await`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PodSnapshot {
    pub route: RouteKey,
    pub bot_identity: Option<String>,
    pub session_id: Option<String>,
    pub cli_kind: Option<String>,
    pub profile: Option<String>,
    pub workspace: Option<String>,
    pub busy: bool,
    pub failed: Option<String>,
    pub started_at: u64,
    pub initialize: Option<acp::InitializeResponse>,
}

impl PodSnapshot {
    pub fn service_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.route.channel_kind,
            self.route.chat_id,
            self.profile
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            self.cli_kind
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        )
    }
}
