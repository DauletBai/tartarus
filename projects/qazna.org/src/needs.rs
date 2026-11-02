use crate::{domain::Amount, provisioning::Category};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitRequest {
    pub id: Uuid,
    pub parent: Option<Uuid>,
    pub name: String,
    pub function: String,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: Uuid,
    pub parent: Option<Uuid>,
    pub name: String,
    pub function: String,
    pub children: Vec<Uuid>,
    pub providers: Vec<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRequest {
    pub producer: Uuid,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLink {
    pub unit: Uuid,
    pub producer: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedRequest {
    pub id: Uuid,
    pub resident: Uuid,
    pub assigned_unit: Uuid,
    pub period: String,
    pub category: Category,
    pub unit: String,
    pub quantity: Amount,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Need {
    pub id: Uuid,
    pub resident: Uuid,
    pub assigned_unit: Uuid,
    pub period: String,
    pub category: String,
    pub unit: String,
    pub requested: String,
    pub reserved: String,
    pub received: String,
    pub uncovered: String,
    pub unmet: String,
    pub last_attempt_id: Option<Uuid>,
    pub last_attempt_reasons: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedReservationRequest {
    pub resource: Uuid,
    pub quantity: Amount,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptSnapshot {
    pub uncovered_before: String,
    pub resource_available: String,
    pub accounting_available: String,
    pub required_measure: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedAttempt {
    pub id: Uuid,
    pub need: Uuid,
    pub resource: Uuid,
    pub quantity: String,
    pub status: String,
    pub reservation_id: Option<Uuid>,
    pub reasons: Vec<String>,
    pub snapshot: AttemptSnapshot,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeriodQuery {
    pub period: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CoverageLine {
    pub category: String,
    pub unit: String,
    pub needs: i64,
    pub requested: String,
    pub received: String,
    pub reserved: String,
    pub uncovered: String,
    pub unmet: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Coverage {
    pub unit: Uuid,
    pub period: String,
    pub lines: Vec<CoverageLine>,
}
