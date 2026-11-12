//! Disjoint person-day profiles and local provision; no personal records.
use crate::{mathematics::MathError, population::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, MathError>;
fn invalid(s: &'static str) -> MathError {
    MathError::Invalid(s)
}
fn label(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 128 || s.trim() != s {
        return Err(invalid("profile identifier"));
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExposureSource {
    pub source: String,
    pub source_version: String,
    pub recorded_on: String,
    pub kind: EvidenceKind,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LifeStage {
    General,
    PregnancyFirst,
    PregnancySecond,
    PregnancyThird,
    Postpartum,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Support {
    Standard,
    Assisted,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub stage: LifeStage,
    pub support: Support,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExposureCell {
    pub region: String,
    pub group: String,
    pub profile: String,
    pub person_days: Vec<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Norm {
    Inherit {},
    Replace { parameter: String },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProfileRule {
    pub cell: usize,
    pub item: String,
    pub norm: Norm,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocalDelivery {
    pub region: String,
    pub item: String,
    pub quantities: Vec<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProfilePlan {
    pub registry: Registry,
    pub exposure_source: ExposureSource,
    pub regions: Vec<String>,
    pub profiles: Vec<Profile>,
    pub cells: Vec<ExposureCell>,
    pub rules: Vec<ProfileRule>,
    pub local_delivery: Vec<LocalDelivery>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionalNeeds {
    pub region: String,
    pub demands: Vec<Demand>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileMonth {
    pub calendar: String,
    pub regions: Vec<RegionalNeeds>,
    pub total: Vec<Demand>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalCoverage {
    pub period: usize,
    pub region: String,
    pub item: String,
    pub required: String,
    pub delivered: String,
    pub unmet: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileEvidence {
    pub months: Vec<ProfileMonth>,
    pub local_coverage: Vec<LocalCoverage>,
    pub all_selected_needs_known: bool,
}
fn sum_demands(item: &NeedItem, rows: &[Demand]) -> Result<Demand> {
    let mut nums = [0u64; 3];
    let mut missing = BTreeSet::new();
    for r in rows {
        for (i, v) in [Some(&r.known_lower), r.central.as_ref(), r.upper.as_ref()]
            .iter()
            .enumerate()
        {
            if let Some(v) = v {
                nums[i] = nums[i]
                    .checked_add(v.parse::<u64>().map_err(|_| MathError::Overflow)?)
                    .ok_or(MathError::Overflow)?;
            }
        }
        missing.extend(r.unknown_parameters.iter().cloned());
    }
    Ok(Demand {
        item: item.id.clone(),
        unit: item.unit.clone(),
        known_lower: nums[0].to_string(),
        central: missing.is_empty().then(|| nums[1].to_string()),
        upper: missing.is_empty().then(|| nums[2].to_string()),
        unknown_parameters: missing.into_iter().collect(),
    })
}
impl ProfilePlan {
    pub fn project(&self, study: &PopulationStudy) -> Result<ProfileEvidence> {
        let population = study.project()?;
        self.registry.validate()?;
        // Validate source metadata using the same rules as parameter provenance.
        let source = Registry {
            version: "exposure-source".into(),
            mode: self.registry.mode,
            parameters: vec![Parameter {
                id: "exposure".into(),
                unit: "person-day".into(),
                region: "all".into(),
                reference_period: "projection calendar".into(),
                recorded_on: self.exposure_source.recorded_on.clone(),
                source: self.exposure_source.source.clone(),
                source_version: self.exposure_source.source_version.clone(),
                basis: "Aggregate disjoint exposure schedule".into(),
                kind: self.exposure_source.kind,
                estimate: Some(Estimate {
                    lower: Ratio {
                        numerator: 1,
                        denominator: 1,
                    },
                    central: Ratio {
                        numerator: 1,
                        denominator: 1,
                    },
                    upper: Ratio {
                        numerator: 1,
                        denominator: 1,
                    },
                }),
            }],
        };
        source.validate()?;
        if study.registry.mode != self.registry.mode {
            return Err(invalid("profile evidence mode mismatch"));
        }
        if self.regions.is_empty()
            || self.regions.len() > 16
            || self.profiles.is_empty()
            || self.profiles.len() > 16
            || self.cells.is_empty()
            || self.cells.len() > 256
            || self.rules.len() != self.cells.len() * study.items.len()
            || population.months.len() != study.months.len()
        {
            return Err(invalid("profile dimensions"));
        }
        let mut regions = BTreeSet::new();
        let mut profiles = BTreeSet::new();
        let mut states = BTreeSet::new();
        for r in &self.regions {
            label(r)?;
            if !regions.insert(r) {
                return Err(invalid("duplicate region"));
            }
        }
        for p in &self.profiles {
            label(&p.id)?;
            if !profiles.insert(&p.id) || !states.insert((p.stage, p.support)) {
                return Err(invalid("duplicate joint profile"));
            }
        }
        let mut keys = BTreeSet::new();
        let mut used_regions = BTreeSet::new();
        let mut used_profiles = BTreeSet::new();
        for c in &self.cells {
            if !regions.contains(&c.region)
                || !profiles.contains(&c.profile)
                || !study.groups.iter().any(|g| g.id == c.group)
                || c.person_days.len() != study.months.len()
                || !keys.insert((&c.region, &c.group, &c.profile))
            {
                return Err(invalid("invalid or overlapping profile cell"));
            }
            used_regions.insert(&c.region);
            used_profiles.insert(&c.profile);
        }
        if regions != used_regions || profiles != used_profiles {
            return Err(invalid("unused region or profile"));
        }
        let mut norms = BTreeMap::new();
        for r in &self.rules {
            let item = study
                .items
                .iter()
                .find(|i| i.id == r.item)
                .ok_or(invalid("unknown profile item"))?;
            if r.cell >= self.cells.len() || norms.insert((r.cell, &r.item), &r.norm).is_some() {
                return Err(invalid("profile norm matrix"));
            }
            if let Norm::Replace { parameter } = &r.norm {
                self.registry
                    .get(parameter, &format!("{}/person/day", item.unit))?;
            }
        }
        let mut months = vec![];
        let mut all_known = true;
        for (t, m) in population.months.iter().enumerate() {
            for (gi, g) in study.groups.iter().enumerate() {
                let expected = m.group_people[gi]
                    .parse::<u64>()
                    .map_err(|_| MathError::Overflow)?
                    .checked_mul(u64::from(m.days))
                    .ok_or(MathError::Overflow)?;
                let actual = self
                    .cells
                    .iter()
                    .filter(|c| c.group == g.id)
                    .try_fold(0u64, |a, c| {
                        a.checked_add(c.person_days[t]).ok_or(MathError::Overflow)
                    })?;
                if expected != actual {
                    return Err(invalid("profile exposure does not partition population"));
                }
            }
            let mut regional = vec![];
            for region in &self.regions {
                let mut demands = vec![];
                for item in &study.items {
                    // Aggregate equal norms before integer rounding: splitting a cell
                    // into identical profiles must not create demand from rounding.
                    let mut buckets: BTreeMap<String, (&Parameter, u64)> = BTreeMap::new();
                    for (ci, c) in self
                        .cells
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| &c.region == region)
                    {
                        let (key, p) = match norms[&(ci, &item.id)] {
                            Norm::Inherit {} => {
                                let rule = study
                                    .need_rules
                                    .iter()
                                    .find(|r| r.group == c.group && r.item == item.id)
                                    .ok_or(invalid("base norm missing"))?;
                                (
                                    format!("base:{}", rule.parameter),
                                    study.registry.get(
                                        &rule.parameter,
                                        &format!("{}/person/day", item.unit),
                                    )?,
                                )
                            }
                            Norm::Replace { parameter } => (
                                format!("profile:{parameter}"),
                                self.registry
                                    .get(parameter, &format!("{}/person/day", item.unit))?,
                            ),
                        };
                        let b = buckets.entry(key).or_insert((p, 0));
                        b.1 =
                            b.1.checked_add(c.person_days[t])
                                .ok_or(MathError::Overflow)?;
                    }
                    let mut parts = vec![];
                    for (key, (p, n)) in buckets {
                        let d = if n == 0 {
                            Demand {
                                item: item.id.clone(),
                                unit: item.unit.clone(),
                                known_lower: "0".into(),
                                central: Some("0".into()),
                                upper: Some("0".into()),
                                unknown_parameters: vec![],
                            }
                        } else if let Some(e) = &p.estimate {
                            Demand {
                                item: item.id.clone(),
                                unit: item.unit.clone(),
                                known_lower: e.lower.ceil_product(n)?.to_string(),
                                central: Some(e.central.ceil_product(n)?.to_string()),
                                upper: Some(e.upper.ceil_product(n)?.to_string()),
                                unknown_parameters: vec![],
                            }
                        } else {
                            Demand {
                                item: item.id.clone(),
                                unit: item.unit.clone(),
                                known_lower: "0".into(),
                                central: None,
                                upper: None,
                                unknown_parameters: vec![key],
                            }
                        };
                        parts.push(d);
                    }
                    let d = sum_demands(item, &parts)?;
                    all_known &= d.central.is_some();
                    demands.push(d);
                }
                regional.push(RegionalNeeds {
                    region: region.clone(),
                    demands,
                });
            }
            let total = study
                .items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    sum_demands(
                        item,
                        &regional
                            .iter()
                            .map(|r| r.demands[i].clone())
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            months.push(ProfileMonth {
                calendar: m.calendar.clone(),
                regions: regional,
                total,
            });
        }
        Ok(ProfileEvidence {
            months,
            local_coverage: vec![],
            all_selected_needs_known: all_known,
        })
    }
    pub fn reconcile_deliveries(
        &self,
        evidence: &mut ProfileEvidence,
        mappings: &[NeedMapping],
        plan: &[crate::horizon::PeriodPlan],
    ) -> Result<()> {
        if self.local_delivery.len() != self.regions.len() * mappings.len() {
            return Err(invalid("local delivery matrix"));
        }
        let mut rows = BTreeMap::new();
        for d in &self.local_delivery {
            if !self.regions.contains(&d.region)
                || !mappings.iter().any(|m| m.item == d.item)
                || d.quantities.len() != plan.len()
                || rows.insert((&d.region, &d.item), d).is_some()
            {
                return Err(invalid("invalid local delivery"));
            }
        }
        for (t, month) in evidence.months.iter().enumerate() {
            for map in mappings {
                let mut delivered = 0u64;
                for region in &month.regions {
                    let d = &region
                        .demands
                        .iter()
                        .find(|d| d.item == map.item)
                        .ok_or(invalid("local mapped item"))?;
                    let need = d
                        .central
                        .as_ref()
                        .ok_or(invalid("unknown local need"))?
                        .parse::<u64>()
                        .map_err(|_| MathError::Overflow)?;
                    let amount = rows[&(&region.region, &map.item)].quantities[t];
                    delivered = delivered.checked_add(amount).ok_or(MathError::Overflow)?;
                    evidence.local_coverage.push(LocalCoverage {
                        period: t,
                        region: region.region.clone(),
                        item: map.item.clone(),
                        required: need.to_string(),
                        delivered: amount.to_string(),
                        unmet: need.saturating_sub(amount).to_string(),
                    });
                }
                if u64::try_from(plan[t].deliveries[map.resource])
                    .map_err(|_| invalid("negative local provision"))?
                    != delivered
                {
                    return Err(invalid("local provision differs from physical delivery"));
                }
            }
        }
        Ok(())
    }
}
