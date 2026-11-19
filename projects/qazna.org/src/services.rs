//! Non-storable regional services use available capital and shared work time.
use crate::{
    capital::CapitalCycle,
    horizon::PeriodPlan,
    mathematics::MathError,
    model_scenarios::ModelDefinition,
    population::{PopulationAudit, Ratio, Registry},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, MathError>;
fn bad(s: &'static str) -> MathError {
    MathError::Invalid(s)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServicePool {
    pub id: String,
    pub region: String,
    pub resource: usize,
    pub unit: String,
    pub rate_parameter: String,
    pub working_days: Vec<u32>,
    pub availability_bps: Vec<u16>,
    /// Per active process unit per month, through the month before completion.
    pub process_load_parameters: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PoolRequirement {
    pub pool: String,
    pub parameter: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatingRequirement {
    pub resource: usize,
    pub parameter: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceRule {
    pub region: String,
    pub item: String,
    pub pools: Vec<PoolRequirement>,
    pub operating: Vec<OperatingRequirement>,
    pub delivered: Vec<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServicePlan {
    /// Protected free capacity in the pool's own unit; never extra inventory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reserves: Vec<PoolReserve>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revisions: Vec<ServiceRevision>,
    pub registry: Registry,
    pub pools: Vec<ServicePool>,
    pub rules: Vec<ServiceRule>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PoolReserve {
    pub pool: String,
    pub minimum: Vec<u64>,
    pub basis: String,
    pub source_reference: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReserveBalance {
    pub period: usize,
    pub pool: String,
    pub required: String,
    pub free_capacity: String,
    pub shortfall: String,
}
/// One declared change of future availability, retained with the approved input.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AvailabilityChange {
    pub pool: String,
    pub period: usize,
    pub previous_bps: u16,
    pub available_bps: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceRevision {
    pub boundary: usize,
    pub basis: String,
    pub source_reference: String,
    pub changes: Vec<AvailabilityChange>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceCoverage {
    pub period: usize,
    pub region: String,
    pub item: String,
    pub required: String,
    pub delivered: String,
    pub unmet: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolBalance {
    pub period: usize,
    pub pool: String,
    pub active_stock: String,
    pub available: String,
    pub service_use: String,
    pub pipeline_use: String,
    pub shortage: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatingBalance {
    pub period: usize,
    pub resource: usize,
    pub required: String,
    pub physical_delivery: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceEvidence {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reserves: Vec<ReserveBalance>,
    pub coverage: Vec<ServiceCoverage>,
    pub pools: Vec<PoolBalance>,
    pub operating: Vec<OperatingBalance>,
    pub all_selected_services_feasible: bool,
    pub unbound_items: Vec<String>,
}
fn number(n: &str) -> Result<u64> {
    n.parse().map_err(|_| MathError::Overflow)
}
fn add(a: &mut u64, b: u64) -> Result<()> {
    *a = a.checked_add(b).ok_or(MathError::Overflow)?;
    Ok(())
}
impl ServicePlan {
    /// Validate a declaration against this exact base without requiring feasibility.
    pub fn with_revision(&self, revision: ServiceRevision) -> Result<Self> {
        let mut next = self.clone();
        for change in &revision.changes {
            let pool = next
                .pools
                .iter_mut()
                .find(|p| p.id == change.pool)
                .ok_or(bad("unknown service pool"))?;
            *pool
                .availability_bps
                .get_mut(change.period)
                .ok_or(bad("service period out of bounds"))? = change.available_bps;
        }
        let boundary = revision.boundary;
        next.revisions.push(revision);
        next.verify_continuation(self, boundary)?;
        Ok(next)
    }

    /// Only availability after the rolling boundary may change. Norms, need and
    /// service schedules remain fixed; physical work is checked by RollingCapital.
    pub fn verify_continuation(&self, old: &Self, boundary: usize) -> Result<()> {
        if self.pools.len() != old.pools.len() || self.revisions.len() > 128 {
            return Err(bad("service revision dimensions"));
        }
        let mut normalized = self.clone();
        normalized.revisions = old.revisions.clone();
        let mut changes = Vec::new();
        for (next, prior) in normalized.pools.iter_mut().zip(&old.pools) {
            if next.availability_bps.len() != prior.availability_bps.len()
                || boundary > prior.availability_bps.len()
            {
                return Err(bad("service revision calendar"));
            }
            for (t, (&after, &before)) in next
                .availability_bps
                .iter()
                .zip(&prior.availability_bps)
                .enumerate()
            {
                if after > 10000 || (t < boundary && after != before) {
                    return Err(bad("service availability history or range"));
                }
                if after != before {
                    changes.push(AvailabilityChange {
                        pool: prior.id.clone(),
                        period: t,
                        previous_bps: before,
                        available_bps: after,
                    });
                }
            }
            next.availability_bps = prior.availability_bps.clone();
        }
        if normalized != *old {
            return Err(bad("service assumptions changed"));
        }
        if changes.is_empty() {
            return if self.revisions == old.revisions {
                Ok(())
            } else {
                Err(bad("service revision without availability change"))
            };
        }
        if changes.len() > 4096
            || self.revisions.len() != old.revisions.len() + 1
            || self.revisions[..old.revisions.len()] != old.revisions
        {
            return Err(bad("service revision history changed"));
        }
        let revision = self
            .revisions
            .last()
            .ok_or(bad("missing service revision"))?;
        if revision.boundary != boundary
            || revision.changes != changes
            || revision.basis.trim().is_empty()
            || revision.basis.len() > 1000
            || revision.source_reference.trim().is_empty()
            || revision.source_reference.len() > 512
        {
            return Err(bad("service revision does not explain exact change"));
        }
        Ok(())
    }
    fn rate(&self, id: &str, unit: &str) -> Result<Ratio> {
        Ok(self
            .registry
            .get(id, unit)?
            .estimate
            .as_ref()
            .ok_or(bad("unknown service coefficient"))?
            .central)
    }
    pub fn audit(
        &self,
        definition: &ModelDefinition,
        model: &CapitalCycle,
        plan: &[PeriodPlan],
        population: &PopulationAudit,
    ) -> Result<ServiceEvidence> {
        self.registry.validate()?;
        let p = population
            .profiles
            .as_ref()
            .ok_or(bad("services require regional need profiles"))?;
        if self.registry.mode != population.projection.mode
            || self.pools.is_empty()
            || self.pools.len() > 32
            || self.rules.is_empty()
            || self.rules.len() > 128
        {
            return Err(bad("service dimensions or evidence mode"));
        }
        let h = &model.seasonal.physical;
        if p.months.is_empty()
            || plan.len() != p.months.len()
            || population.projection.months.len() != plan.len()
            || definition.resources.len() != h.resources.len()
        {
            return Err(bad("service calendar or resources"));
        }
        let capital = model.replay(plan)?;
        let mut pool_ids = BTreeMap::new();
        let mut resources = BTreeSet::new();
        for (i, pool) in self.pools.iter().enumerate() {
            if pool.id.trim().is_empty()
                || pool.id.len() > 128
                || pool.unit.trim().is_empty()
                || pool.unit.len() > 128
                || !p.months[0].regions.iter().any(|r| r.region == pool.region)
                || pool_ids.insert(&pool.id, i).is_some()
                || !resources.insert(pool.resource)
                || pool.resource >= h.resources.len()
                || !model.lives.iter().any(|l| l.resource == pool.resource)
                || model.limits.iter().any(|l| l.resource == pool.resource)
                || model
                    .stores
                    .iter()
                    .any(|s| s.asset_resource == pool.resource)
                || pool.working_days.len() != plan.len()
                || pool.availability_bps.len() != plan.len()
                || pool.process_load_parameters.len() != h.processes.len()
            {
                return Err(bad("service pool identity or dimensions"));
            }
            let unit = format!(
                "{}/{}/day",
                pool.unit, definition.resources[pool.resource].unit
            );
            if self.rate(&pool.rate_parameter, &unit)?.numerator == 0 {
                return Err(bad("zero service pool rate"));
            }
            for (t, days) in pool.working_days.iter().enumerate() {
                if *days > population.projection.months[t].days || pool.availability_bps[t] > 10000
                {
                    return Err(bad("service work calendar"));
                }
            }
            for id in &pool.process_load_parameters {
                self.rate(id, &format!("{}/activity/month", pool.unit))?;
            }
        }
        let mut keys = BTreeSet::new();
        let mut reserve_floors = BTreeMap::new();
        if self.reserves.len() > self.pools.len() {
            return Err(bad("too many service reserves"));
        }
        for reserve in &self.reserves {
            if !pool_ids.contains_key(&reserve.pool)
                || reserve_floors
                    .insert(&reserve.pool, &reserve.minimum)
                    .is_some()
                || reserve.minimum.len() != plan.len()
                || reserve.basis.trim().is_empty()
                || reserve.basis.len() > 1000
                || reserve.source_reference.trim().is_empty()
                || reserve.source_reference.len() > 512
            {
                return Err(bad("invalid service reserve identity, calendar or source"));
            }
        }
        let mut selected = BTreeSet::new();
        let mut operating_resources = BTreeSet::new();
        let mut used_pools = BTreeSet::new();
        for rule in &self.rules {
            let region = p.months[0]
                .regions
                .iter()
                .find(|r| r.region == rule.region)
                .ok_or(bad("unknown service region"))?;
            let need = region
                .demands
                .iter()
                .find(|d| d.item == rule.item)
                .ok_or(bad("unknown service item"))?;
            if !keys.insert((&rule.region, &rule.item))
                || population.coverage.iter().any(|c| c.item == rule.item)
                || rule.pools.is_empty()
                || rule.pools.len() > self.pools.len()
                || rule.delivered.len() != plan.len()
                || rule.operating.len() > h.resources.len()
            {
                return Err(bad("service rule coverage"));
            }
            selected.insert(&rule.item);
            let mut used = BTreeSet::new();
            for u in &rule.pools {
                let &i = pool_ids.get(&u.pool).ok_or(bad("unknown service pool"))?;
                let pool = &self.pools[i];
                if pool.region != rule.region
                    || !used.insert(i)
                    || self
                        .rate(&u.parameter, &format!("{}/{}", pool.unit, need.unit))?
                        .numerator
                        == 0
                {
                    return Err(bad("cross-region or duplicate service capacity"));
                }
                used_pools.insert(i);
            }
            let mut ops = BTreeSet::new();
            for op in &rule.operating {
                if op.resource >= h.resources.len()
                    || !ops.insert(op.resource)
                    || resources.contains(&op.resource)
                    || population
                        .coverage
                        .iter()
                        .any(|c| c.resource == op.resource)
                {
                    return Err(bad("operating resource overlap"));
                }
                self.rate(
                    &op.parameter,
                    &format!("{}/{}", definition.resources[op.resource].unit, need.unit),
                )?;
                operating_resources.insert(op.resource);
            }
        }
        for (i, pool) in self.pools.iter().enumerate() {
            if pool.process_load_parameters.iter().any(|id| {
                self.rate(id, &format!("{}/activity/month", pool.unit))
                    .is_ok_and(|r| r.numerator > 0)
            }) {
                used_pools.insert(i);
            }
        }
        if used_pools.len() != self.pools.len()
            || keys.len() != p.months[0].regions.len() * selected.len()
        {
            return Err(bad("incomplete regional service matrix"));
        }
        let mut evidence = ServiceEvidence {
            reserves: vec![],
            coverage: vec![],
            pools: vec![],
            operating: vec![],
            all_selected_services_feasible: true,
            unbound_items: p.months[0]
                .total
                .iter()
                .filter(|d| {
                    !selected.contains(&d.item)
                        && !population.coverage.iter().any(|c| c.item == d.item)
                })
                .map(|d| d.item.clone())
                .collect(),
        };
        for (t, month) in p.months.iter().enumerate() {
            let mut use_buckets: BTreeMap<(usize, &String), u64> = BTreeMap::new();
            let mut op_buckets: BTreeMap<(usize, &String), u64> = BTreeMap::new();
            for rule in &self.rules {
                let d = month
                    .regions
                    .iter()
                    .find(|r| r.region == rule.region)
                    .ok_or(bad("missing regional service evidence"))?
                    .demands
                    .iter()
                    .find(|d| d.item == rule.item)
                    .ok_or(bad("missing service demand evidence"))?;
                let required = number(d.central.as_ref().ok_or(bad("unknown service demand"))?)?;
                let delivered = rule.delivered[t];
                if delivered > required {
                    return Err(bad("service delivery exceeds need"));
                }
                evidence.all_selected_services_feasible &= delivered == required;
                evidence.coverage.push(ServiceCoverage {
                    period: t,
                    region: rule.region.clone(),
                    item: rule.item.clone(),
                    required: required.to_string(),
                    delivered: delivered.to_string(),
                    unmet: (required - delivered).to_string(),
                });
                for u in &rule.pools {
                    add(
                        use_buckets
                            .entry((pool_ids[&u.pool], &u.parameter))
                            .or_default(),
                        delivered,
                    )?;
                }
                for op in &rule.operating {
                    add(
                        op_buckets.entry((op.resource, &op.parameter)).or_default(),
                        delivered,
                    )?;
                }
            }
            let mut service_use = vec![0u64; self.pools.len()];
            for ((pool, id), n) in use_buckets {
                let parameter = self
                    .registry
                    .parameters
                    .iter()
                    .find(|p| &p.id == id)
                    .unwrap();
                add(
                    &mut service_use[pool],
                    parameter
                        .estimate
                        .as_ref()
                        .unwrap()
                        .central
                        .ceil_product(n)?,
                )?;
            }
            for (i, pool) in self.pools.iter().enumerate() {
                let active =
                    u64::try_from(capital.available_opening_stock_by_period[t][pool.resource])
                        .map_err(|_| bad("negative service stock"))?;
                let r = self.rate(
                    &pool.rate_parameter,
                    &format!(
                        "{}/{}/day",
                        pool.unit, definition.resources[pool.resource].unit
                    ),
                )?;
                let capacity = u128::from(active)
                    .checked_mul(u128::from(pool.working_days[t]))
                    .and_then(|n| n.checked_mul(u128::from(r.numerator)))
                    .and_then(|n| n.checked_mul(u128::from(pool.availability_bps[t])))
                    .ok_or(MathError::Overflow)?
                    / (u128::from(r.denominator) * 10000);
                let available = u64::try_from(capacity).map_err(|_| MathError::Overflow)?;
                let mut loads: BTreeMap<&String, u64> = BTreeMap::new();
                for (j, process) in h.processes.iter().enumerate() {
                    let start = (t + 1).saturating_sub(process.lead_periods);
                    let n = plan[start..=t].iter().try_fold(0u64, |sum, row| {
                        sum.checked_add(
                            u64::try_from(row.activities[j])
                                .map_err(|_| bad("negative pipeline activity"))?,
                        )
                        .ok_or(MathError::Overflow)
                    })?;
                    add(
                        loads.entry(&pool.process_load_parameters[j]).or_default(),
                        n,
                    )?;
                }
                let mut pipeline = 0;
                for (id, n) in loads {
                    add(
                        &mut pipeline,
                        self.rate(id, &format!("{}/activity/month", pool.unit))?
                            .ceil_product(n)?,
                    )?;
                }
                let total = service_use[i]
                    .checked_add(pipeline)
                    .ok_or(MathError::Overflow)?;
                let shortage = total.saturating_sub(available);
                evidence.all_selected_services_feasible &= shortage == 0;
                if let Some(floors) = reserve_floors.get(&pool.id) {
                    let free = available.saturating_sub(total);
                    let shortfall = floors[t].saturating_sub(free);
                    evidence.all_selected_services_feasible &= shortfall == 0;
                    evidence.reserves.push(ReserveBalance {
                        period: t,
                        pool: pool.id.clone(),
                        required: floors[t].to_string(),
                        free_capacity: free.to_string(),
                        shortfall: shortfall.to_string(),
                    });
                }
                evidence.pools.push(PoolBalance {
                    period: t,
                    pool: pool.id.clone(),
                    active_stock: active.to_string(),
                    available: available.to_string(),
                    service_use: service_use[i].to_string(),
                    pipeline_use: pipeline.to_string(),
                    shortage: shortage.to_string(),
                });
            }
            let mut ops: BTreeMap<usize, u64> =
                operating_resources.iter().map(|&k| (k, 0)).collect();
            for ((resource, id), n) in op_buckets {
                let parameter = self
                    .registry
                    .parameters
                    .iter()
                    .find(|p| &p.id == id)
                    .unwrap();
                add(
                    ops.get_mut(&resource).unwrap(),
                    parameter
                        .estimate
                        .as_ref()
                        .unwrap()
                        .central
                        .ceil_product(n)?,
                )?;
            }
            for (resource, n) in ops {
                if i64::try_from(n).map_err(|_| MathError::Overflow)?
                    != h.periods[t].needs[resource]
                    || h.periods[t].needs[resource] != plan[t].deliveries[resource]
                {
                    return Err(bad(
                        "service operating inputs differ from physical provision",
                    ));
                }
                evidence.operating.push(OperatingBalance {
                    period: t,
                    resource,
                    required: n.to_string(),
                    physical_delivery: plan[t].deliveries[resource].to_string(),
                });
            }
        }
        Ok(evidence)
    }
}
