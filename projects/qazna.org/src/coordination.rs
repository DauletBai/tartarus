use crate::planning::{PlanningRequest, ProvisioningProposal};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorRequest {
    pub id: Uuid,
    pub plan: PlanningRequest,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Monitor {
    pub id: Uuid,
    pub state: String,
    pub plan: PlanningRequest,
    pub queued: i64,
    pub running: i64,
    pub failed: i64,
    pub latest: Option<CoordinationResult>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CoordinationResult {
    pub job: Uuid,
    pub monitor: Uuid,
    pub trigger_command: Option<String>,
    pub engine: String,
    pub proposal: ProvisioningProposal,
    pub draft: Uuid,
    pub changed: bool,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiReport {
    pub summary: String,
    pub findings: Vec<AiFinding>,
    pub questions: Vec<String>,
    pub requires_operator_approval: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiFinding {
    pub signal: String,
    pub subject: Uuid,
    pub quantity: String,
    pub explanation: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRequest {
    pub id: Uuid,
    pub model: String,
    pub report: AiReport,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AiReview {
    pub id: Uuid,
    pub job: Uuid,
    pub provider: String,
    pub model: String,
    pub report: AiReport,
}
