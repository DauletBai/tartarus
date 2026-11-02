use crate::{
    domain::Amount,
    needs::Need,
    production::{InputSelection, PlanRequest, ProductionAssessment},
    provisioning::Category,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedNorm {
    pub id: Uuid,
    pub code: String,
    pub revision: i32,
    pub supersedes: Option<Uuid>,
    pub category: Category,
    pub unit: String,
    pub quantity: Amount,
    pub applicability: String,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormNeedRequest {
    pub id: Uuid,
    pub resident: Uuid,
    pub assigned_unit: Uuid,
    pub period: String,
    pub norm: Uuid,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormNeed {
    pub need: Need,
    pub norm: NeedNorm,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessOption {
    pub recipe: Uuid,
    pub capacity: Uuid,
    pub inputs: Vec<InputSelection>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedOptions {
    pub need: Uuid,
    pub resources: Vec<Uuid>,
    pub production: Vec<ProcessOption>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningRequest {
    pub period: String,
    pub needs: Vec<NeedOptions>,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSuggestion {
    pub resource: Uuid,
    pub quantity: String,
    pub required_measure: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSuggestion {
    pub request: PlanRequest,
    pub expected_for_need: String,
    pub surplus: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEvaluation {
    pub recipe: Uuid,
    pub capacity: Uuid,
    pub assessment: ProductionAssessment,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedPlan {
    pub need: Uuid,
    pub category: String,
    pub unit: String,
    pub uncovered: String,
    pub stock_selected: String,
    pub expected_from_production: String,
    pub residual: String,
    pub stock: Vec<StockSuggestion>,
    pub production: Vec<ProcessSuggestion>,
    pub evaluations: Vec<ProcessEvaluation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountingGap {
    pub resident: Uuid,
    pub available: String,
    pub required_for_selected_stock: String,
    pub shortage: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisioningProposal {
    pub advisory: bool,
    pub policy: String,
    pub observed_at: String,
    pub period: String,
    pub basis: String,
    pub needs: Vec<NeedPlan>,
    pub accounting: Vec<AccountingGap>,
}
