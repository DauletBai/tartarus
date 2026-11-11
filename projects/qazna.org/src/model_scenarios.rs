//! Versioned research models. Quantities and approved simulations are not
//! inventory attestations and cannot authorize monetary postings or issuance.
use crate::{
    capital::CapitalCycle,
    dated::DatedFlow,
    food_routes::{FoodRouteEvidence, FoodRoutes},
    horizon::PeriodPlan,
    ledger::LedgerError,
    materials::{MaterialBasis, MaterialEvidence},
    mathematics::verify_linear,
    rolling::{RollingCapital, RollingSnapshot},
    scenarios::ScenarioSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

pub const MAX_MODEL_REQUEST_BYTES: usize = 1_048_576;
pub const MAX_MODEL_CHECKPOINT_BYTES: usize = 2_097_152;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelResource {
    pub name: String,
    pub unit: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatedBinding {
    pub scenario_id: Uuid,
    /// Ordered mapping from dated locations to model resource indexes.
    pub model_resources: Vec<usize>,
    /// Declares calendar alignment and why these stocks are comparable.
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDefinition {
    pub id: Uuid,
    pub format_version: u32,
    pub resources: Vec<ModelResource>,
    pub period_unit: String,
    pub time_origin: String,
    pub dated_bindings: Vec<DatedBinding>,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelInput {
    pub model: CapitalCycle,
    pub plan: Vec<PeriodPlan>,
    /// First replannable period on the unchanged absolute timeline.
    pub boundary: usize,
    pub material_bases: Vec<MaterialBasis>,
    /// Same order as the immutable definition's dated bindings.
    pub dated_generations: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub food_routes: Option<FoodRoutes>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelProposal {
    pub id: Uuid,
    pub expected_generation: i64,
    pub input: ModelInput,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelCheckpoint {
    pub format_version: u32,
    pub scope: Uuid,
    pub generation: i64,
    pub input: ModelInput,
    pub rolling: RollingSnapshot,
    pub materials: Vec<MaterialEvidence>,
    pub dated: Vec<ScenarioSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub food_routes: Option<FoodRouteEvidence>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelVersion {
    pub definition: ModelDefinition,
    pub generation: i64,
    /// Canonical JSON string preserves exact i128 material arithmetic.
    pub checkpoint: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelDraft {
    pub scenario_id: Uuid,
    pub request: ModelProposal,
    pub candidate: ModelVersion,
    pub state: String,
    pub decision_basis: Option<String>,
}
/// Pure initial research preview. This does not store or approve a generation;
/// dated database anchors must instead be resolved by the operator service.
pub fn preview(
    definition: ModelDefinition,
    input: ModelInput,
) -> Result<ModelVersion, LedgerError> {
    let basis = definition.basis.clone();
    calculate(
        ModelVersion {
            definition,
            generation: 0,
            checkpoint: None,
        },
        ModelProposal {
            id: Uuid::nil(),
            expected_generation: 0,
            input,
            basis,
        },
        vec![],
    )
}

pub(crate) fn text(s: &str, max: usize) -> Result<(), LedgerError> {
    if s.trim().is_empty() || s.len() > max {
        return Err(LedgerError::Invalid("invalid model descriptor or basis"));
    }
    Ok(())
}
impl ModelDefinition {
    pub(crate) fn validate(&self) -> Result<(), LedgerError> {
        text(&self.basis, 1000)?;
        text(&self.period_unit, 128)?;
        text(&self.time_origin, 128)?;
        if self.format_version != 1
            || self.resources.is_empty()
            || self.resources.len() > 32
            || self.dated_bindings.len() > 8
        {
            return Err(LedgerError::Invalid("model version or dimensions"));
        }
        let mut names = BTreeSet::new();
        for r in &self.resources {
            text(&r.name, 128)?;
            text(&r.unit, 128)?;
            if r.name != r.name.trim() || r.unit != r.unit.trim() || !names.insert(&r.name) {
                return Err(LedgerError::Invalid("model resource identities"));
            }
        }
        let mut ids = BTreeSet::new();
        let mut resources = BTreeSet::new();
        for b in &self.dated_bindings {
            text(&b.basis, 1000)?;
            if !ids.insert(b.scenario_id) || b.model_resources.is_empty() {
                return Err(LedgerError::Invalid("duplicate or empty dated binding"));
            }
            for &r in &b.model_resources {
                if r >= self.resources.len() || !resources.insert(r) {
                    return Err(LedgerError::Invalid("overlapping dated stock mappings"));
                }
            }
        }
        Ok(())
    }
}
fn bounded(input: &ModelInput) -> Result<(), LedgerError> {
    let m = &input.model;
    let s = &m.seasonal;
    let h = &s.physical;
    let (r, p, t) = (h.resources.len(), h.processes.len(), h.periods.len());
    if r > 32
        || p > 48
        || t > 120
        || s.land.len() > 16
        || s.crops.len() > 48
        || m.limits.len() > 48
        || m.lives.len() > 32
        || m.stores.len() > 32
        || input.material_bases.len() > 8
        || input.plan.len() != t
        || t * t * p.max(1) * r > 20_000_000
    {
        return Err(LedgerError::Invalid("model calculation budget exceeded"));
    }
    for b in &input.material_bases {
        text(&b.name, 128)?;
        text(&b.unit, 128)?;
        text(&b.source, 1000)?;
    }
    for l in &s.land {
        text(&l.name, 128)?;
    }
    for c in &s.crops {
        text(&c.basis, 1000)?;
    }
    for l in &m.limits {
        text(&l.name, 128)?;
    }
    for l in &m.stores {
        text(&l.name, 128)?;
    }
    Ok(())
}
pub(crate) fn checkpoint(version: &ModelVersion) -> Result<Option<ModelCheckpoint>, LedgerError> {
    let Some(bytes) = &version.checkpoint else {
        return if version.generation == 0 {
            Ok(None)
        } else {
            Err(LedgerError::InvalidStoredResponse)
        };
    };
    if bytes.len() > MAX_MODEL_CHECKPOINT_BYTES {
        return Err(LedgerError::InvalidStoredResponse);
    }
    let c: ModelCheckpoint =
        serde_json::from_str(bytes).map_err(|_| LedgerError::InvalidStoredResponse)?;
    if c.format_version != 1
        || c.scope != version.definition.id
        || c.generation != version.generation
        || serde_json::to_string(&c).map_err(|_| LedgerError::InvalidStoredResponse)? != *bytes
    {
        return Err(LedgerError::InvalidStoredResponse);
    }
    bounded(&c.input)?;
    Ok(Some(c))
}
/// Check independently against both the constraint matrix and chronological
/// resource replay. The previous approved input, never a client checkpoint,
/// freezes every commitment before the proposed boundary.
pub(crate) fn calculate(
    before: ModelVersion,
    request: ModelProposal,
    dated: Vec<ScenarioSnapshot>,
) -> Result<ModelVersion, LedgerError> {
    before.definition.validate()?;
    text(&request.basis, 1000)?;
    if request.expected_generation != before.generation {
        return Err(LedgerError::ScenarioStale);
    }
    if serde_json::to_vec(&request)
        .map_err(|_| LedgerError::InvalidStoredResponse)?
        .len()
        > MAX_MODEL_REQUEST_BYTES
    {
        return Err(LedgerError::Invalid("model request exceeds 1 MiB"));
    }
    bounded(&request.input)?;
    let input = &request.input;
    let h = &input.model.seasonal.physical;
    if h.resources
        != before
            .definition
            .resources
            .iter()
            .map(|r| r.name.clone())
            .collect::<Vec<_>>()
    {
        return Err(LedgerError::Invalid(
            "model resource names or order changed",
        ));
    }
    let invalid = |_| LedgerError::Invalid("infeasible model or rewritten committed history");
    if let Some(old) = checkpoint(&before)? {
        if input.food_routes != old.input.food_routes {
            return Err(LedgerError::Invalid(
                "food identity or shelf assumptions changed",
            ));
        }
        if input.boundary < old.input.boundary
            || serde_json::to_vec(&input.material_bases)
                .map_err(|_| LedgerError::InvalidStoredResponse)?
                != serde_json::to_vec(&old.input.material_bases)
                    .map_err(|_| LedgerError::InvalidStoredResponse)?
        {
            return Err(LedgerError::Invalid("boundary or material basis changed"));
        }
        let prefix = RollingCapital::checkpoint(&old.input.model, &old.input.plan, input.boundary)
            .map_err(invalid)?;
        verify_linear(
            &prefix.compile(&input.model).map_err(invalid)?,
            &h.vector(&input.plan).map_err(invalid)?,
            None,
        )
        .map_err(invalid)?;
        prefix.replay(&input.model, &input.plan).map_err(invalid)?;
    }
    let rolling = RollingCapital::checkpoint(&input.model, &input.plan, input.boundary)
        .map_err(invalid)?
        .snapshot()
        .clone();
    let materials = input
        .material_bases
        .iter()
        .map(|b| b.audit(h, &input.plan).map_err(invalid))
        .collect::<Result<Vec<_>, _>>()?;
    let food_routes = input
        .food_routes
        .as_ref()
        .map(|routes| {
            for &r in &routes.resources {
                if before
                    .definition
                    .resources
                    .get(r)
                    .is_none_or(|v| v.unit != routes.unit)
                {
                    return Err(LedgerError::Invalid("food route unit mismatch"));
                }
            }
            routes.audit(&input.model, &input.plan).map_err(invalid)
        })
        .transpose()?;
    let bindings = &before.definition.dated_bindings;
    if dated.len() != bindings.len() || input.dated_generations.len() != bindings.len() {
        return Err(LedgerError::Invalid("missing dated generations"));
    }
    for (i, (binding, anchor)) in bindings.iter().zip(&dated).enumerate() {
        if anchor.definition.id != binding.scenario_id
            || anchor.generation != input.dated_generations[i]
        {
            return Err(LedgerError::ScenarioStale);
        }
        if binding.model_resources.len() != anchor.definition.resources.len() {
            return Err(LedgerError::Invalid("dated resource dimensions"));
        }
        let bytes = anchor.checkpoint.as_bytes();
        let flow = DatedFlow::restore(
            bytes,
            bytes,
            &anchor.definition.id.to_string(),
            anchor.flow_revision,
        )
        .map_err(invalid)?;
        let balance = flow.balance();
        if balance.period != input.boundary.checked_sub(1).map(|t| t as u32) {
            return Err(LedgerError::Invalid(
                "dated period and model boundary disagree",
            ));
        }
        for (location, &r) in binding.model_resources.iter().enumerate() {
            if before.definition.resources[r].unit != anchor.definition.unit
                || u128::try_from(rolling.opening_stock[r]).ok()
                    != Some(balance.stock_by_resource[location])
            {
                return Err(LedgerError::Invalid(
                    "dated and model stock or unit disagree",
                ));
            }
        }
    }
    let generation = before
        .generation
        .checked_add(1)
        .ok_or(LedgerError::Overflow)?;
    let c = ModelCheckpoint {
        format_version: 1,
        scope: before.definition.id,
        generation,
        input: request.input,
        rolling,
        materials,
        dated,
        food_routes,
    };
    let bytes = serde_json::to_string(&c).map_err(|_| LedgerError::InvalidStoredResponse)?;
    if bytes.len() > MAX_MODEL_CHECKPOINT_BYTES {
        return Err(LedgerError::Invalid("model checkpoint exceeds 2 MiB"));
    }
    Ok(ModelVersion {
        definition: before.definition,
        generation,
        checkpoint: Some(bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (ModelVersion, ModelProposal) {
        let input: ModelInput =
            serde_json::from_str(include_str!("../fixtures/model-scenario.json")).unwrap();
        let definition = ModelDefinition {
            id: Uuid::new_v4(),
            format_version: 1,
            resources: input
                .model
                .seasonal
                .physical
                .resources
                .iter()
                .map(|n| ModelResource {
                    name: n.clone(),
                    unit: "test unit".into(),
                })
                .collect(),
            period_unit: "day".into(),
            time_origin: "synthetic zero".into(),
            dated_bindings: vec![],
            basis: "fixture".into(),
        };
        (
            ModelVersion {
                definition,
                generation: 0,
                checkpoint: None,
            },
            ModelProposal {
                id: Uuid::new_v4(),
                expected_generation: 0,
                input,
                basis: "fixture".into(),
            },
        )
    }
    #[test]
    fn canonical_checkpoint_preserves_constituent_totals_beyond_u64() {
        let (before, mut request) = fixture();
        request.input.material_bases[0].quantity_per_resource_unit = vec![i64::MAX; 3];
        let after = calculate(before, request, vec![]).unwrap();
        let c = checkpoint(&after).unwrap().unwrap();
        assert_eq!(c.materials[0].initial, 102 * i128::from(i64::MAX));
        assert!(c.materials[0].initial > i128::from(u64::MAX));
        assert_eq!(
            serde_json::to_string(&c).unwrap(),
            after.checkpoint.unwrap()
        );
    }
    #[test]
    fn initial_boundary_freezes_schema_but_not_uncommitted_future() {
        let (before, mut request) = fixture();
        request.input.boundary = 0;
        let approved = calculate(before, request.clone(), vec![]).unwrap();
        let c = checkpoint(&approved).unwrap().unwrap();
        assert_eq!(c.rolling.opening_stock, vec![100, 1, 1]);
        assert!(c.rolling.pending.is_empty());
        request.expected_generation = 1;
        request.input.plan[0].discarded[0] = 1;
        for row in &mut request.input.plan {
            row.closing[0] -= 1;
        }
        assert!(calculate(approved.clone(), request.clone(), vec![]).is_ok());
        request.input.model.lives[0].initial_remaining_periods = 4;
        assert!(calculate(approved, request, vec![]).is_err());
    }
    #[test]
    fn oversized_work_and_checkpoint_identity_fail_before_acceptance() {
        let (before, mut request) = fixture();
        request.input.model.seasonal.physical.periods.resize(
            121,
            request.input.model.seasonal.physical.periods[0].clone(),
        );
        assert!(calculate(before, request, vec![]).is_err());
        let (before, request) = fixture();
        let mut approved = calculate(before, request, vec![]).unwrap();
        approved.generation += 1;
        assert!(checkpoint(&approved).is_err());
        approved.generation -= 1;
        approved.definition.id = Uuid::new_v4();
        assert!(checkpoint(&approved).is_err());
    }
}
