use crate::{domain::Amount, provisioning::Category};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityRequest {
    pub id: Uuid,
    pub unit: Uuid,
    pub period: String,
    pub capacity_unit: String,
    pub quantity: Amount,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capacity {
    pub declaration: CapacityRequest,
    pub reserved: String,
    #[serde(default = "zero")]
    pub consumed: String,
    pub available: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeInput {
    pub slot: String,
    pub category: Category,
    pub unit: String,
    pub quantity_per_batch: Amount,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeRequest {
    pub id: Uuid,
    pub unit: Uuid,
    pub name: String,
    pub capacity_unit: String,
    pub capacity_per_batch: Amount,
    pub output_category: Category,
    pub output_unit: String,
    pub output_per_batch: Amount,
    pub inputs: Vec<RecipeInput>,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSelection {
    pub slot: String,
    pub resource: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRequest {
    pub recipe: Uuid,
    pub capacity: Uuid,
    pub batches: Amount,
    pub inputs: Vec<InputSelection>,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAssessment {
    pub slot: String,
    pub resource: Uuid,
    pub supplying_unit: Uuid,
    pub category: String,
    pub unit: String,
    pub required: String,
    pub available: String,
    pub shortage: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionAssessment {
    pub feasible: bool,
    pub max_batches: String,
    pub planned_output: String,
    pub capacity_required: String,
    pub capacity_available: String,
    pub reasons: Vec<String>,
    pub inputs: Vec<InputAssessment>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAttempt {
    pub status: String,
    pub plan_id: Option<Uuid>,
    pub assessment: ProductionAssessment,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionPlan {
    pub id: Uuid,
    pub state: String,
    pub request: PlanRequest,
    pub planned_output: String,
}

fn zero() -> String {
    "0".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionOutput {
    pub id: Uuid,
    pub producer: Uuid,
    pub quantity: Amount,
    pub unit_measure: Amount,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputLoss {
    pub slot: String,
    pub quantity: Amount,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionRequest {
    pub output: Option<ProductionOutput>,
    pub losses: Vec<InputLoss>,
    pub basis: String,
    pub shortfall_basis: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedInput {
    pub slot: String,
    pub resource: Uuid,
    pub category: String,
    pub unit: String,
    pub withdrawn: String,
    pub loss: String,
    pub used: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionCompletion {
    pub plan: Uuid,
    pub recipe: Uuid,
    pub output_resource: Option<Uuid>,
    pub planned_output: String,
    pub actual_output: String,
    pub output_shortfall: String,
    pub output_category: String,
    pub output_unit: String,
    pub capacity_consumed: String,
    pub basis: String,
    pub shortfall_basis: Option<String>,
    pub inputs: Vec<CompletedInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceProvenance {
    pub resource: Uuid,
    pub origin: String,
    pub declaration_basis: String,
    pub completion: Option<ProductionCompletion>,
}
