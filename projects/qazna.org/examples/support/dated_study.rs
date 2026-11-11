//! Identity-preserving checkpoints for the existing fixed-yield food routes.
use qazna::{capital::CapitalCycle, dated::DatedFlow, horizon::PeriodPlan, mathematics::MathError};
use serde_json::{Value, json};
const SCOPE: &str = "upstream-food-v1/identity-routes/18-month-assumption";
fn step(
    flow: &mut DatedFlow,
    m: &CapitalCycle,
    plan: &[PeriodPlan],
    t: usize,
) -> Result<(), MathError> {
    let h = &m.seasonal.physical;
    let p = &plan[t];
    let month = u32::try_from(t).map_err(|_| MathError::Overflow)?;
    flow.advance(month)?;
    if t == 0 {
        for k in 0..9 {
            if h.initial[k] > 0 {
                flow.introduce(
                    k as u64,
                    k,
                    h.initial[k] as u64,
                    0,
                    18,
                    "declared opening preserved stock",
                )?;
            }
        }
    }
    for j in 0..15 {
        let a = u64::try_from(p.activities[j])
            .map_err(|_| MathError::Invalid("negative crop activity"))?;
        if a == 0 {
            continue;
        }
        let k = j % 3;
        let recipe = &h.processes[j];
        let used = a
            .checked_mul(recipe.inputs[k] as u64)
            .ok_or(MathError::Overflow)?;
        if used > 0 {
            flow.withdraw(k, used)?;
        }
        let arrival = t
            .checked_add(recipe.lead_periods)
            .ok_or(MathError::Overflow)?;
        let available = u32::try_from(arrival).map_err(|_| MathError::Overflow)?;
        let quantity = a
            .checked_mul(recipe.outputs[k] as u64)
            .ok_or(MathError::Overflow)?;
        flow.introduce(
            1000 + (t as u64) * 128 + j as u64,
            k,
            quantity,
            available,
            available.checked_add(18).ok_or(MathError::Overflow)?,
            &format!("harvest from crop process {j} started at absolute month {t}"),
        )?;
    }
    for k in 0..3 {
        let j = 15 + 4 * k;
        for (from, to, q) in [
            (
                k,
                3 + k,
                p.activities[j]
                    .checked_add(p.activities[j + 1])
                    .ok_or(MathError::Overflow)?,
            ),
            (
                3 + k,
                6 + k,
                p.activities[j + 2]
                    .checked_add(p.activities[j + 3])
                    .ok_or(MathError::Overflow)?,
            ),
        ] {
            if q > 0 {
                flow.transfer(from, to, q as u64, month + 1)?;
            }
        }
        if p.deliveries[6 + k] > 0 {
            flow.withdraw(6 + k, p.deliveries[6 + k] as u64)?;
        }
    }
    let b = flow.balance();
    if b.expired > 0 {
        return Err(MathError::Infeasible(format!(
            "dated checkpoint expiry:{t}"
        )));
    }
    for k in 0..9 {
        if b.stock_by_resource[k] != p.closing[k] as u128 {
            return Err(MathError::Infeasible(format!(
                "dated checkpoint stock:{t}:{k}"
            )));
        }
    }
    Ok(())
}
pub fn study(upstream: &Value) -> Value {
    let c = &upstream["cases"][1];
    let m: CapitalCycle = serde_json::from_value(c["model"].clone()).unwrap();
    let p: Vec<PeriodPlan> = serde_json::from_value(c["candidate"].clone()).unwrap();
    m.seasonal.physical.vector(&p).unwrap();
    let mut f = DatedFlow::new(SCOPE, "gross-equivalent kg", 9).unwrap();
    let mut saved = vec![];
    let mut views = vec![];
    for t in 0..p.len() {
        step(&mut f, &m, &p, t).unwrap();
        let bytes = f.checkpoint().unwrap();
        let b = f.balance();
        if [4, 18, 30, 42, 60].contains(&(t + 1)) {
            views.push(json!({"next_period":t+1,"checkpoint_bytes":bytes.len(),"balance":b}));
        }
        saved.push((bytes, b.revision));
    }
    let mut matches = 0;
    for (cut, (bytes, revision)) in saved.iter().enumerate() {
        let mut resumed = DatedFlow::restore(bytes, bytes, SCOPE, *revision).unwrap();
        for t in cut + 1..p.len() {
            step(&mut resumed, &m, &p, t).unwrap();
        }
        assert_eq!(resumed, f);
        assert_eq!(resumed.checkpoint().unwrap(), f.checkpoint().unwrap());
        matches += 1;
    }
    let (anchor, revision) = &saved[17];
    let mut edited: Value = serde_json::from_slice(anchor).unwrap();
    edited["origins"]["0"]["expires"] = json!(9999);
    let payload = serde_json::to_vec(&edited).unwrap();
    let altered_rejected = DatedFlow::restore(&payload, anchor, SCOPE, *revision).is_err();
    let stale_rejected = DatedFlow::restore(&saved[3].0, &saved[3].0, SCOPE, *revision).is_err();
    assert!(altered_rejected && stale_rejected);
    json!({"kind":"dated_food_identity_checkpoint_continuity","source_case":c["name"],"months":60,"model_calls":0,
        "checkpoint_format":1,"boundaries_verified":matches,"all_suffix_states_byte_identical":true,
        "altered_expiry_rejected":altered_rejected,"stale_revision_rejected":stale_rejected,
        "checkpoints":views,"sample_checkpoint_at_next_period_4":serde_json::from_slice::<Value>(&saved[3].0).unwrap(),
        "final_balance":f.balance(),"max_checkpoint_bytes":saved.iter().map(|x|x.0.len()).max().unwrap(),
        "scope":["Independent origin-ID replay matches the same nine food stocks as the full upstream plan; original DatedStock FEFO companion is preserved",
        "All 60 end-period boundaries are serialized and restored, then replayed to the same final byte-identical state",
        "Identity processing and transport preserve original expiry and origin; only declared new harvest creates a new origin and age",
        "Committed harvest is not released stock; source seed withdrawals and resident consumption share a conservation counter but are not equated as human provision",
        "Trusted expected bytes, scope and current revision must be obtained independently. Equality is not a signature, proof of execution, or implemented trusted database store",
        "Single physical unit, 18-month synthetic shelf assumption and one-to-one routes; no certified food safety, conversion retention, temperature history, quarantine or recall workflow",
        "Active stock and movements are compact; origin tombstones and counters are retained to reject ID reuse, bounded by explicit item and serialized-byte limits",
        "No database migrations, endpoints, monetary authority, automatic postings or AI calls"]})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upstream_food_resumes_from_all_60_boundaries() {
        let u = super::super::upstream_study::study(&super::super::automation_study::study(
            &super::super::seasonal_study::study(&super::super::nutrition_study::study()),
        ));
        let r = study(&u);
        assert_eq!(r["boundaries_verified"], 60);
        assert_eq!(r["all_suffix_states_byte_identical"], true);
        assert_eq!(r["final_balance"]["expired"], 0);
        assert_eq!(r["final_balance"]["committed_births"], 0);
        assert!(
            r["checkpoints"][0]["balance"]["committed_births"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(
            r["checkpoints"][0]["balance"]["in_transit"]
                .as_u64()
                .unwrap()
                > 0
        );
    }
}
