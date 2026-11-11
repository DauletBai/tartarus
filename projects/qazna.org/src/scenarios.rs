//! Bounded research scenario commands. No inventory, posting or issuance authority.
use crate::{dated::DatedFlow, mathematics::MathError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_CHECKPOINT_BYTES: usize = 262_144;
pub const MAX_COMMANDS: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDefinition {
    pub id: Uuid,
    pub format_version: u32,
    pub unit: String,
    /// Ordered, immutable names for the single-unit locations/stages.
    pub resources: Vec<String>,
    pub basis: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DatedCommand {
    Advance {
        period: u32,
    },
    Introduce {
        id: u64,
        resource: usize,
        quantity: u64,
        available: u32,
        expires: u32,
        basis: String,
    },
    Transfer {
        source: usize,
        target: usize,
        quantity: u64,
        due: u32,
    },
    Withdraw {
        resource: usize,
        quantity: u64,
    },
    Damage {
        movement: u64,
        quantity: u64,
    },
}
impl DatedCommand {
    pub(crate) fn execute(&self, flow: &mut DatedFlow) -> Result<(), MathError> {
        match self {
            Self::Advance { period } => flow.advance(*period),
            Self::Introduce {
                id,
                resource,
                quantity,
                available,
                expires,
                basis,
            } => flow.introduce(*id, *resource, *quantity, *available, *expires, basis),
            Self::Transfer {
                source,
                target,
                quantity,
                due,
            } => flow.transfer(*source, *target, *quantity, *due),
            Self::Withdraw { resource, quantity } => flow.withdraw(*resource, *quantity),
            Self::Damage { movement, quantity } => flow.damage(*movement, *quantity),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioProposal {
    pub id: Uuid,
    pub expected_generation: i64,
    pub commands: Vec<DatedCommand>,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDecision {
    pub apply: bool,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioSnapshot {
    pub definition: ScenarioDefinition,
    /// Store generation is separate from the per-command mathematical revision.
    pub generation: i64,
    pub flow_revision: u64,
    /// Canonical JSON bytes encoded as a string, not a client-authoritative import.
    pub checkpoint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioDraft {
    pub scenario_id: Uuid,
    pub request: ScenarioProposal,
    pub candidate: ScenarioSnapshot,
    pub state: String,
    pub decision_basis: Option<String>,
}
