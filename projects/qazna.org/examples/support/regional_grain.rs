//! Route national grain withdrawals to regional depots within each monthly deadline.
use super::calendar;
use serde_json::{Value, json};
use std::collections::BTreeMap;

type Grain = [i64; 2];
type Regions = BTreeMap<String, Grain>;

fn bookings(need: Grain, payload: i64) -> i64 {
    assert!(payload > 0 && need.iter().all(|&kg| kg >= 0));
    let kg = need.iter().sum::<i64>();
    kg / payload + i64::from(kg % payload != 0)
}

// The manifest loads rice first, then oats. A late lot can use spare space
// after the current month's orders. Quantities are whole gross-equivalent kg.
fn load(request: Grain, space: &mut i64) -> Grain {
    request.map(|kg| {
        let loaded = kg.min(*space);
        *space -= loaded;
        loaded
    })
}

fn replay(
    needs: &[Regions],
    deliveries: &[Grain],
    payload: i64,
    cancellation: Option<(usize, &str)>,
) -> Value {
    assert_eq!(needs.len(), deliveries.len());
    let mut held = Regions::new();
    let mut peak = 0;
    let mut first_peak = String::new();
    let mut trips_used = 0;
    let mut total_missed = [0; 2];
    let mut total_late = [0; 2];
    let mut months = Vec::new();
    let mut affected = Vec::new();
    for (t, (regions, inbound)) in needs.iter().zip(deliveries).enumerate() {
        let month = calendar(t + 12);
        let mut receipts = BTreeMap::new();
        let mut monthly_trips = 0;
        for (j, &received) in inbound.iter().enumerate() {
            assert_eq!(regions.values().map(|q| q[j]).sum::<i64>(), received);
        }
        if t > 0 {
            assert!(held.keys().eq(regions.keys()), "Regional geography changed");
        }
        for (region, &need) in regions {
            let planned = bookings(need, payload);
            let cancelled = cancellation == Some((t, region.as_str()));
            assert!(!cancelled || planned > 0);
            let trips = planned - i64::from(cancelled);
            monthly_trips += trips;
            let mut space = trips * payload;
            let opening = *held.entry(region.clone()).or_default();
            let on_time = load(need, &mut space);
            let late = load(opening, &mut space);
            let missed: Grain = std::array::from_fn(|j| need[j] - on_time[j]);
            let closing: Grain = std::array::from_fn(|j| opening[j] - late[j] + missed[j]);
            for j in 0..2 {
                assert_eq!(opening[j] + need[j], on_time[j] + late[j] + closing[j]);
                total_missed[j] += missed[j];
                total_late[j] += late[j];
            }
            held.insert(region.clone(), closing);
            if opening != [0; 2] || closing != [0; 2] {
                affected.push(json!({"month":month, "region":region, "on_time_kg":on_time, "missed_deadline_kg":missed,
                    "late_receipt_kg":late, "closing_hub_kg":closing, "trips":trips}));
            }
            receipts.insert(region, json!({"trips":trips, "on_time_kg":on_time}));
        }
        if monthly_trips > peak {
            peak = monthly_trips;
            first_peak.clone_from(&month);
        }
        trips_used += monthly_trips;
        if cancellation.is_none() {
            months.push(json!({"month":month, "hub_inbound_kg":inbound, "receipts":receipts}));
        }
    }
    json!({"monthly":months, "affected_receipts":affected, "missed_deadline_kg":total_missed,
        "late_receipt_kg":total_late, "closing_hub_kg_by_region":held, "completed_trips":trips_used,
        "peak_monthly_trips":peak, "first_peak_month":first_peak})
}

pub(super) fn study(needs: &[Regions], deliveries: &[Grain], payload: i64) -> Value {
    let normal = replay(needs, deliveries, payload, None);
    let disrupted = replay(needs, deliveries, payload, Some((12, "south")));
    assert_eq!(normal["missed_deadline_kg"], json!([0, 0]));
    assert!(disrupted["missed_deadline_kg"] != json!([0, 0]));
    assert_eq!(
        disrupted["late_receipt_kg"],
        disrupted["missed_deadline_kg"]
    );
    json!({"payload_kg":payload, "booking_rule":"One destination per trip; the two grain lots can share a load. Same-month arrival; no route loss. Required trips are capacity bounds, not evidence of an available fleet.",
        "stress":"One booked southern departure is cancelled in January 2028 without same-month replacement. Current orders load before delayed lots; residual capacity serves the same destination.",
        "baseline":normal, "cancellation":disrupted})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocations_preserve_deadlines_identity_and_unresolved_final_stock() {
        let orders = Regions::from([("north".to_string(), [1, 0]), ("south".to_string(), [4, 7])]);
        let (needs, inbound) = (vec![orders; 2], [[5, 7]; 2]);
        let normal = replay(&needs, &inbound, 10, None);
        // National pooling alone would claim two trips and silently mix destinations.
        assert_eq!(normal["peak_monthly_trips"], 3);
        let shock = replay(&needs, &inbound, 10, Some((0, "south")));
        assert_eq!(shock["missed_deadline_kg"], json!([0, 1]));
        assert_eq!(shock["late_receipt_kg"], json!([0, 1]));
        let terminal = replay(&needs, &inbound, 10, Some((1, "south")));
        assert_eq!(terminal["late_receipt_kg"], json!([0, 0]));
        assert_eq!(terminal["closing_hub_kg_by_region"]["south"], json!([0, 1]));
    }
}
