use crate::planning::{PlanningRequest, ProvisioningProposal};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRequest {
    pub id: Uuid,
    pub plan: PlanningRequest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftAction {
    pub need: Uuid,
    pub kind: String,
    pub reservation: Option<Uuid>,
    pub attempt: Option<Uuid>,
    pub production_plan: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningDraft {
    pub id: Uuid,
    pub format_version: i32,
    pub state: String,
    pub created_at: String,
    pub request: PlanningRequest,
    pub proposal: ProvisioningProposal,
    pub outcome_basis: Option<String>,
    pub actions: Vec<DraftAction>,
}
