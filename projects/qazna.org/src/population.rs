//! Bounded cohort-component research inputs. No identity registry, medical norms,
//! entitlement changes, observed headcounts or monetary authority are implied.
use crate::mathematics::MathError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, MathError>;
pub const LAST_AGE: usize = 1560; // Open-ended 130+ bin, never forced death.
const MAX_PEOPLE: u64 = 20_000_000_000;
fn invalid(s: &'static str) -> MathError {
    MathError::Invalid(s)
}
fn text(s: &str, max: usize) -> Result<()> {
    if s.trim().is_empty() || s.len() > max || s != s.trim() {
        return Err(invalid("population descriptor"));
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Ratio {
    pub numerator: u64,
    pub denominator: u64,
}
impl Ratio {
    fn validate(self) -> Result<()> {
        if self.denominator == 0
            || self.denominator > 1_000_000_000
            || self.numerator > 1_000_000_000_000
        {
            return Err(invalid("population ratio bounds"));
        }
        Ok(())
    }
    fn le(self, other: Self) -> bool {
        u128::from(self.numerator) * u128::from(other.denominator)
            <= u128::from(other.numerator) * u128::from(self.denominator)
    }
    fn integer(self) -> Result<u64> {
        if !self.numerator.is_multiple_of(self.denominator) {
            return Err(invalid("fractional person count"));
        }
        Ok(self.numerator / self.denominator)
    }
    pub(crate) fn ceil_product(self, count: u64) -> Result<u64> {
        let n = u128::from(count) * u128::from(self.numerator);
        let d = u128::from(self.denominator);
        u64::try_from(n / d + u128::from(!n.is_multiple_of(d))).map_err(|_| MathError::Overflow)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Estimate {
    pub lower: Ratio,
    pub central: Ratio,
    pub upper: Ratio,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Synthetic,
    SourceAttributed,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub id: String,
    pub unit: String,
    pub region: String,
    pub reference_period: String,
    pub recorded_on: String,
    pub source: String,
    pub source_version: String,
    pub basis: String,
    pub kind: EvidenceKind,
    pub estimate: Option<Estimate>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    Illustrative,
    SourceAttributed,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub version: String,
    pub mode: InputMode,
    pub parameters: Vec<Parameter>,
}
fn days(year: u32, month: u32) -> Result<u32> {
    if !(1900..=2400).contains(&year) || !(1..=12).contains(&month) {
        return Err(invalid("population calendar"));
    }
    Ok(match month {
        2 => {
            if year.is_multiple_of(400) || year.is_multiple_of(4) && !year.is_multiple_of(100) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    })
}
fn date(s: &str) -> Result<()> {
    if s.len() != 10
        || s.bytes().enumerate().any(|(i, b)| {
            if i == 4 || i == 7 {
                b != b'-'
            } else {
                !b.is_ascii_digit()
            }
        })
    {
        return Err(invalid("registry date"));
    }
    let p: Vec<_> = s.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 {
        return Err(invalid("registry date"));
    }
    let n = p
        .iter()
        .map(|s| s.parse::<u32>().map_err(|_| invalid("registry date")))
        .collect::<Result<Vec<_>>>()?;
    if n[2] == 0 || n[2] > days(n[0], n[1])? {
        return Err(invalid("registry date"));
    }
    Ok(())
}
impl Registry {
    pub fn validate(&self) -> Result<()> {
        text(&self.version, 128)?;
        if self.parameters.is_empty() || self.parameters.len() > 512 {
            return Err(invalid("registry size"));
        }
        let mut ids = BTreeSet::new();
        for p in &self.parameters {
            for s in [
                &p.id,
                &p.unit,
                &p.region,
                &p.reference_period,
                &p.source_version,
            ] {
                text(s, 128)?;
            }
            text(&p.source, 1000)?;
            text(&p.basis, 1000)?;
            date(&p.recorded_on)?;
            if !ids.insert(&p.id)
                || (self.mode == InputMode::SourceAttributed && p.kind == EvidenceKind::Synthetic)
            {
                return Err(invalid(
                    "duplicate parameter or synthetic source in attributed mode",
                ));
            }
            if p.kind == EvidenceKind::SourceAttributed
                && !(p.source.starts_with("https://") || p.source.starts_with("http://"))
            {
                return Err(invalid("attributed source locator"));
            }
            match (&p.estimate, p.kind) {
                (None, EvidenceKind::Unknown) => {}
                (Some(e), k) if k != EvidenceKind::Unknown => {
                    for v in [e.lower, e.central, e.upper] {
                        v.validate()?;
                    }
                    if !e.lower.le(e.central) || !e.central.le(e.upper) {
                        return Err(invalid("unordered uncertainty interval"));
                    }
                }
                _ => return Err(invalid("unknown evidence must remain unknown")),
            }
        }
        Ok(())
    }
    pub(crate) fn get(&self, id: &str, unit: &str) -> Result<&Parameter> {
        let p = self
            .parameters
            .iter()
            .find(|p| p.id == id)
            .ok_or(invalid("unknown population parameter"))?;
        if p.unit != unit {
            return Err(invalid("population parameter unit mismatch"));
        }
        Ok(p)
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    Reference,
    LowerPopulation,
    HigherPopulation,
}
fn selected(e: &Estimate, v: Variant, inverse: bool) -> Ratio {
    match (v, inverse) {
        (Variant::Reference, _) => e.central,
        (Variant::LowerPopulation, false) | (Variant::HigherPopulation, true) => e.lower,
        _ => e.upper,
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InitialCohort {
    pub age_months: usize,
    pub people_parameter: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgeGroup {
    pub id: String,
    pub start_month: usize,
    pub end_month_exclusive: usize,
    pub mortality_parameter: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Migration {
    pub age_months: usize,
    pub arrivals_parameter: String,
    pub departures_parameter: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonthInput {
    pub births_parameter: String,
    pub migration: Vec<Migration>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NeedItem {
    pub id: String,
    pub unit: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NeedRule {
    pub group: String,
    pub item: String,
    pub parameter: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationStudy {
    pub registry: Registry,
    pub variant: Variant,
    pub start_year: u32,
    pub start_month: u32,
    pub initial: Vec<InitialCohort>,
    pub groups: Vec<AgeGroup>,
    pub months: Vec<MonthInput>,
    pub items: Vec<NeedItem>,
    pub need_rules: Vec<NeedRule>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Demand {
    pub item: String,
    pub unit: String,
    pub known_lower: String,
    pub central: Option<String>,
    pub upper: Option<String>,
    pub unknown_parameters: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationMonth {
    pub calendar: String,
    pub days: u32,
    pub opening: String,
    pub group_people: Vec<String>,
    pub births: String,
    pub deaths: String,
    pub arrivals: String,
    pub departures: String,
    pub closing: String,
    pub mortality_remainders: Vec<String>,
    pub demands: Vec<Demand>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationEvidence {
    pub registry_version: String,
    pub mode: InputMode,
    pub variant: Variant,
    pub initial: String,
    pub final_population: String,
    pub months: Vec<PopulationMonth>,
    pub final_age_counts: Vec<String>,
    pub all_selected_needs_known: bool,
}
impl PopulationStudy {
    fn value(&self, id: &str, unit: &str, inverse: bool) -> Result<Ratio> {
        let p = self.registry.get(id, unit)?;
        Ok(selected(
            p.estimate
                .as_ref()
                .ok_or(invalid("demographic parameter is unknown"))?,
            self.variant,
            inverse,
        ))
    }
    pub fn project(&self) -> Result<PopulationEvidence> {
        self.registry.validate()?;
        days(self.start_year, self.start_month)?;
        if self.initial.len() > LAST_AGE + 1
            || self.initial.is_empty()
            || self.groups.is_empty()
            || self.groups.len() > 32
            || self.months.is_empty()
            || self.months.len() > 120
            || self.items.is_empty()
            || self.items.len() > 32
            || self.need_rules.len() != self.groups.len() * self.items.len()
        {
            return Err(invalid("population dimensions"));
        }
        let mut stock = vec![0u64; LAST_AGE + 1];
        let mut seen = BTreeSet::new();
        for c in &self.initial {
            if c.age_months > LAST_AGE || !seen.insert(c.age_months) {
                return Err(invalid("initial age bins"));
            }
            let p = self.registry.get(&c.people_parameter, "person")?;
            let e = p
                .estimate
                .as_ref()
                .ok_or(invalid("unknown initial population"))?;
            // A fixed common opening population is required across demographic variants.
            if e.lower != e.central || e.central != e.upper {
                return Err(invalid(
                    "initial population must have an explicit fixed count",
                ));
            }
            stock[c.age_months] = e.central.integer()?;
        }
        let initial = total(&stock)?;
        if initial > MAX_PEOPLE {
            return Err(invalid("population exceeds research bound"));
        }
        let mut end = 0;
        let mut gids = BTreeSet::new();
        for g in &self.groups {
            text(&g.id, 128)?;
            if !gids.insert(&g.id)
                || g.start_month != end
                || g.end_month_exclusive <= end
                || g.end_month_exclusive > LAST_AGE + 1
            {
                return Err(invalid("age partition must be complete and nonoverlapping"));
            }
            end = g.end_month_exclusive;
            let p = self
                .registry
                .get(&g.mortality_parameter, "death/person/month")?;
            let e = p.estimate.as_ref().ok_or(invalid("unknown mortality"))?;
            if !e.upper.le(Ratio {
                numerator: 1,
                denominator: 1,
            }) {
                return Err(invalid("mortality exceeds one"));
            }
        }
        if end != LAST_AGE + 1 {
            return Err(invalid("missing oldest population group"));
        }
        let mut items = BTreeSet::new();
        let mut rulemap = BTreeMap::new();
        for item in &self.items {
            text(&item.id, 128)?;
            text(&item.unit, 128)?;
            if !items.insert(&item.id) {
                return Err(invalid("duplicate need item"));
            }
        }
        for r in &self.need_rules {
            if !gids.contains(&r.group)
                || !items.contains(&r.item)
                || rulemap.insert((&r.group, &r.item), &r.parameter).is_some()
            {
                return Err(invalid("need matrix coverage"));
            }
            let item = self.items.iter().find(|i| i.id == r.item).unwrap();
            self.registry
                .get(&r.parameter, &format!("{}/person/day", item.unit))?;
        }
        let mut remainders = vec![0u64; self.groups.len()];
        let mut results = vec![];
        let mut all_known = true;
        for (t, m) in self.months.iter().enumerate() {
            if m.migration.len() > LAST_AGE + 1 {
                return Err(invalid("migration dimensions"));
            }
            let offset =
                self.start_month - 1 + u32::try_from(t).map_err(|_| MathError::Overflow)?;
            let year = self.start_year + offset / 12;
            let month = offset % 12 + 1;
            let ndays = days(year, month)?;
            let opening = total(&stock)?;
            let counts = self
                .groups
                .iter()
                .map(|g| total(&stock[g.start_month..g.end_month_exclusive]))
                .collect::<Result<Vec<_>>>()?;
            let mut demands = vec![];
            for item in &self.items {
                let (mut lower, mut central, mut upper) = (0u64, 0u64, 0u64);
                let mut unknown = BTreeSet::new();
                for (g, &n) in self.groups.iter().zip(&counts) {
                    let id = rulemap[&(&g.id, &item.id)];
                    let p = self
                        .registry
                        .get(id, &format!("{}/person/day", item.unit))?;
                    if n == 0 {
                        continue;
                    }
                    if let Some(e) = &p.estimate {
                        let exposure =
                            n.checked_mul(u64::from(ndays)).ok_or(MathError::Overflow)?;
                        lower = lower
                            .checked_add(e.lower.ceil_product(exposure)?)
                            .ok_or(MathError::Overflow)?;
                        central = central
                            .checked_add(e.central.ceil_product(exposure)?)
                            .ok_or(MathError::Overflow)?;
                        upper = upper
                            .checked_add(e.upper.ceil_product(exposure)?)
                            .ok_or(MathError::Overflow)?;
                    } else {
                        unknown.insert(id.clone());
                    }
                }
                let known = unknown.is_empty();
                all_known &= known;
                demands.push(Demand {
                    item: item.id.clone(),
                    unit: item.unit.clone(),
                    known_lower: lower.to_string(),
                    central: known.then(|| central.to_string()),
                    upper: known.then(|| upper.to_string()),
                    unknown_parameters: unknown.into_iter().collect(),
                });
            }
            let mut deaths = 0u64;
            for (gi, g) in self.groups.iter().enumerate() {
                let rate = self.value(&g.mortality_parameter, "death/person/month", true)?;
                let n = counts[gi];
                let numerator =
                    u128::from(n) * u128::from(rate.numerator) + u128::from(remainders[gi]);
                let target = u64::try_from(numerator / u128::from(rate.denominator))
                    .map_err(|_| MathError::Overflow)?;
                remainders[gi] = u64::try_from(numerator % u128::from(rate.denominator)).unwrap();
                // Hamilton apportionment: no dependence on the number of nonempty age bins.
                let mut allocated = 0u64;
                let mut residuals = vec![];
                if n > 0 {
                    for (age, count) in stock
                        .iter_mut()
                        .enumerate()
                        .take(g.end_month_exclusive)
                        .skip(g.start_month)
                    {
                        let q = u128::from(*count) * u128::from(target);
                        let base = u64::try_from(q / u128::from(n)).unwrap();
                        residuals.push((q % u128::from(n), age));
                        *count -= base;
                        allocated += base;
                    }
                    residuals.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                    for &(_, age) in residuals.iter().take((target - allocated) as usize) {
                        stock[age] -= 1;
                    }
                }
                deaths = deaths.checked_add(target).ok_or(MathError::Overflow)?;
            }
            let mut arrived = 0u64;
            let mut departed = 0u64;
            let mut ages = BTreeSet::new();
            let mut incoming = vec![];
            for flow in &m.migration {
                if flow.age_months > LAST_AGE || !ages.insert(flow.age_months) {
                    return Err(invalid("migration ages"));
                }
                let a = self
                    .value(&flow.arrivals_parameter, "person/month", false)?
                    .integer()?;
                let d = self
                    .value(&flow.departures_parameter, "person/month", true)?
                    .integer()?;
                stock[flow.age_months] = stock[flow.age_months]
                    .checked_sub(d)
                    .ok_or(invalid("departures exceed surviving cohort"))?;
                arrived = arrived.checked_add(a).ok_or(MathError::Overflow)?;
                departed = departed.checked_add(d).ok_or(MathError::Overflow)?;
                incoming.push((flow.age_months, a));
            }
            let births = self
                .value(&m.births_parameter, "person/month", false)?
                .integer()?;
            let mut next = vec![0u64; LAST_AGE + 1];
            next[0] = births;
            for (age, &n) in stock.iter().enumerate() {
                let dest = (age + 1).min(LAST_AGE);
                next[dest] = next[dest].checked_add(n).ok_or(MathError::Overflow)?;
            }
            // Inbound age is age at the end boundary; births enter age zero then.
            for (age, n) in incoming {
                next[age] = next[age].checked_add(n).ok_or(MathError::Overflow)?;
            }
            let closing = total(&next)?;
            if closing > MAX_PEOPLE {
                return Err(invalid("population exceeds research bound"));
            }
            if u128::from(opening) + u128::from(births) + u128::from(arrived)
                != u128::from(closing) + u128::from(deaths) + u128::from(departed)
            {
                return Err(invalid("population conservation"));
            }
            results.push(PopulationMonth {
                calendar: format!("{year:04}-{month:02}"),
                days: ndays,
                opening: opening.to_string(),
                group_people: counts.iter().map(ToString::to_string).collect(),
                births: births.to_string(),
                deaths: deaths.to_string(),
                arrivals: arrived.to_string(),
                departures: departed.to_string(),
                closing: closing.to_string(),
                mortality_remainders: remainders.iter().map(ToString::to_string).collect(),
                demands,
            });
            stock = next;
        }
        Ok(PopulationEvidence {
            registry_version: self.registry.version.clone(),
            mode: self.registry.mode,
            variant: self.variant,
            initial: initial.to_string(),
            final_population: total(&stock)?.to_string(),
            months: results,
            final_age_counts: stock.iter().map(ToString::to_string).collect(),
            all_selected_needs_known: all_known,
        })
    }
}
fn total(values: &[u64]) -> Result<u64> {
    values
        .iter()
        .try_fold(0u64, |a, &b| a.checked_add(b).ok_or(MathError::Overflow))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NeedMapping {
    pub item: String,
    pub resource: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationBinding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles: Option<crate::need_profiles::ProfilePlan>,
    pub study: PopulationStudy,
    pub mappings: Vec<NeedMapping>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeedCoverage {
    pub period: usize,
    pub item: String,
    pub resource: usize,
    pub required: String,
    pub planned_delivery: String,
    pub unmet: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationAudit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles: Option<crate::need_profiles::ProfileEvidence>,
    pub projection: PopulationEvidence,
    pub coverage: Vec<NeedCoverage>,
    pub unmapped_items: Vec<String>,
}
impl PopulationBinding {
    pub fn audit(
        &self,
        definition: &crate::model_scenarios::ModelDefinition,
        model: &crate::capital::CapitalCycle,
        plan: &[crate::horizon::PeriodPlan],
    ) -> Result<PopulationAudit> {
        let h = &model.seasonal.physical;
        crate::mathematics::verify_linear(&h.compile()?, &h.vector(plan)?, None)?;
        let p = self.study.project()?;
        if definition.period_unit != "month"
            || definition.time_origin
                != format!("{:04}-{:02}", self.study.start_year, self.study.start_month)
            || p.months.len() != h.periods.len()
            || self.mappings.is_empty()
            || self.mappings.len() > 32
        {
            return Err(invalid(
                "population/physical calendar or mapping dimensions",
            ));
        }
        let mut profiles = self
            .profiles
            .as_ref()
            .map(|profiles| profiles.project(&self.study))
            .transpose()?;
        let mut items = BTreeSet::new();
        let mut resources = BTreeSet::new();
        let mut coverage = vec![];
        for m in &self.mappings {
            if !items.insert(&m.item)
                || !resources.insert(m.resource)
                || m.resource >= h.resources.len()
            {
                return Err(invalid("overlapping population mapping"));
            }
            let unit = &definition
                .resources
                .get(m.resource)
                .ok_or(invalid("population resource"))?
                .unit;
            for (t, row) in p.months.iter().enumerate() {
                let demands = profiles
                    .as_ref()
                    .map(|e| &e.months[t].total)
                    .unwrap_or(&row.demands);
                let d = demands
                    .iter()
                    .find(|d| d.item == m.item)
                    .ok_or(invalid("unknown mapped need"))?;
                if &d.unit != unit {
                    return Err(invalid("population/physical unit mismatch"));
                }
                let required = d
                    .central
                    .as_ref()
                    .ok_or(invalid("unknown need cannot become physical zero"))?;
                let n = required.parse::<i64>().map_err(|_| MathError::Overflow)?;
                if h.periods[t].needs[m.resource] != n {
                    return Err(invalid("physical needs differ from population projection"));
                }
                let delivered = plan[t].deliveries[m.resource];
                let gap = n
                    .checked_sub(delivered)
                    .filter(|x| *x >= 0)
                    .ok_or(invalid("provision exceeds mapped need"))?;
                coverage.push(NeedCoverage {
                    period: t,
                    item: m.item.clone(),
                    resource: m.resource,
                    required: required.clone(),
                    planned_delivery: delivered.to_string(),
                    unmet: gap.to_string(),
                });
            }
        }
        let unmapped = self
            .study
            .items
            .iter()
            .filter(|i| !items.contains(&i.id))
            .map(|i| i.id.clone())
            .collect();
        if let (Some(inputs), Some(evidence)) = (&self.profiles, &mut profiles) {
            inputs.reconcile_deliveries(evidence, &self.mappings, plan)?;
        }
        Ok(PopulationAudit {
            profiles,
            projection: p,
            coverage,
            unmapped_items: unmapped,
        })
    }
}
