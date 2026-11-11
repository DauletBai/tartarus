use qazna::dated::DatedFlow;
use serde_json::{Value, json};
fn new() -> DatedFlow {
    DatedFlow::new("food/v1", "kg", 3).unwrap()
}
fn day(f: &mut DatedFlow, t: u32) {
    f.advance(t).unwrap();
    f.introduce(u64::from(t) + 1, 0, 10, t, t + 6, "opening/harvest")
        .unwrap();
    f.introduce(u64::from(t) + 100, 2, 3, t + 3, t + 8, "committed harvest")
        .unwrap();
    f.transfer(0, 1, 10, t + 1).unwrap();
    if t == 0 {
        f.damage(0, 1).unwrap();
    }
    if t > 0 {
        f.transfer(1, 2, 9, t + 1).unwrap();
    }
    if t > 1 {
        f.withdraw(2, 5).unwrap();
    }
}
#[test]
fn restore_at_every_boundary_matches_uninterrupted_multileg_flow() {
    let mut base = new();
    let mut snapshots = vec![];
    for t in 0..30 {
        day(&mut base, t);
        snapshots.push((base.checkpoint().unwrap(), base.balance().revision));
    }
    for (t, (bytes, revision)) in snapshots.iter().enumerate() {
        let mut restored = DatedFlow::restore(bytes, bytes, "food/v1", *revision).unwrap();
        for day_number in t + 1..30 {
            day(&mut restored, day_number as u32);
        }
        assert_eq!(restored, base);
    }
    let b = base.balance();
    assert_eq!(
        b.released,
        b.withdrawn
            + b.expired
            + b.damaged
            + b.in_transit
            + b.stock_by_resource.iter().sum::<u128>()
    );
    assert!(b.expired > 0 && b.damaged == 1 && b.committed_births > 0);
}
#[test]
fn fefo_keeps_provenance_across_stages_and_does_not_renew_expiry() {
    let mut f = new();
    f.advance(0).unwrap();
    f.introduce(1, 0, 10, 0, 6, "longer lot").unwrap();
    f.introduce(2, 0, 4, 0, 3, "earlier lot").unwrap();
    f.transfer(0, 1, 6, 1).unwrap();
    f.advance(1).unwrap();
    f.transfer(1, 2, 6, 2).unwrap();
    let bytes = f.checkpoint().unwrap();
    let data: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(data["transit"]["2"]["origin"], 2);
    assert_eq!(data["origins"]["2"]["expires"], 3);
    let mut resumed = DatedFlow::restore(&bytes, &bytes, "food/v1", f.balance().revision).unwrap();
    resumed.advance(2).unwrap();
    resumed.withdraw(2, 3).unwrap();
    resumed.advance(3).unwrap();
    assert_eq!(resumed.balance().expired, 1);
    assert_eq!(resumed.balance().stock_by_resource[2], 2);
    assert!(resumed.introduce(2, 0, 1, 3, 10, "reused id").is_err());
}
#[test]
fn birth_in_progress_is_not_stock_and_is_released_exactly_once() {
    let mut f = new();
    f.advance(0).unwrap();
    f.introduce(42, 0, 7, 2, 5, "crop started earlier").unwrap();
    assert_eq!(f.balance().committed_births, 7);
    assert_eq!(f.balance().released, 0);
    assert!(f.withdraw(0, 1).is_err());
    let bytes = f.checkpoint().unwrap();
    f = DatedFlow::restore(&bytes, &bytes, "food/v1", f.balance().revision).unwrap();
    f.advance(1).unwrap();
    f.advance(2).unwrap();
    assert_eq!(f.balance().stock_by_resource[0], 7);
    assert!(f.advance(2).is_err());
    assert!(f.introduce(42, 0, 7, 2, 5, "duplicate").is_err());
    assert_eq!(f.balance().released, 7);
}
#[test]
fn failures_are_atomic_including_partial_fefo_and_counter_overflow() {
    let mut f = new();
    f.advance(0).unwrap();
    f.introduce(1, 0, 4, 0, 2, "test").unwrap();
    let before = f.clone();
    assert!(f.transfer(0, 1, 5, 1).is_err());
    assert_eq!(f, before);
    assert!(f.transfer(0, 1, 4, 2).is_err());
    assert_eq!(f, before);
    assert!(f.introduce(2, 0, 3, 0, 4, "").is_err());
    assert_eq!(f, before);
    let text = String::from_utf8(f.checkpoint().unwrap()).unwrap();
    let key = format!("\"revision\":{}", f.balance().revision);
    let changed = text.replacen(&key, &format!("\"revision\":{}", u64::MAX), 1);
    let mut full =
        DatedFlow::restore(changed.as_bytes(), changed.as_bytes(), "food/v1", u64::MAX).unwrap();
    let before = full.clone();
    assert!(matches!(
        full.withdraw(0, 1),
        Err(qazna::mathematics::MathError::Overflow)
    ));
    assert_eq!(full, before);
}
#[test]
fn tampering_staleness_and_foreign_scope_are_rejected() {
    let mut f = new();
    day(&mut f, 0);
    let anchor = f.checkpoint().unwrap();
    let rev = f.balance().revision;
    assert!(DatedFlow::restore(&anchor, &anchor, "other", rev).is_err());
    assert!(DatedFlow::restore(&anchor, &anchor, "food/v1", rev + 1).is_err());
    let mut v: Value = serde_json::from_slice(&anchor).unwrap();
    v["origins"]["1"]["expires"] = json!(999);
    let forged = serde_json::to_vec(&v).unwrap();
    assert!(DatedFlow::restore(&forged, &anchor, "food/v1", rev).is_err());
    day(&mut f, 1);
    assert!(
        DatedFlow::restore(
            &anchor,
            &f.checkpoint().unwrap(),
            "food/v1",
            f.balance().revision
        )
        .is_err()
    );
}
#[test]
fn malformed_import_does_not_pass_merely_by_matching_its_anchor() {
    let mut f = new();
    day(&mut f, 0);
    let anchor = f.checkpoint().unwrap();
    let text = String::from_utf8(anchor).unwrap();
    for s in [
        text.replacen("\"version\":1", "\"version\":2", 1),
        text.replacen("\"quantity\":10", "\"quantity\":11", 1),
        text.replacen("\"origin\":1", "\"origin\":999", 1),
        text.replacen("\"due\":1", "\"due\":0", 1),
        text.replacen("\"withdrawn\":0", "\"withdrawn\":1", 1),
    ] {
        assert_ne!(s, text);
        assert!(
            DatedFlow::restore(s.as_bytes(), s.as_bytes(), "food/v1", f.balance().revision)
                .is_err()
        );
    }
}
#[test]
fn excessive_or_invalid_state_is_rejected() {
    assert!(DatedFlow::new("", "kg", 3).is_err());
    assert!(DatedFlow::new("x", "kg", 65).is_err());
    let huge = vec![b' '; 8_000_001];
    assert!(DatedFlow::restore(&huge, &huge, "food/v1", 0).is_err());
    let mut f = new();
    assert!(f.introduce(1, 0, 2, 0, 3, "test").is_err());
    f.advance(0).unwrap();
    assert!(f.introduce(1, 3, 2, 0, 3, "test").is_err());
    assert!(f.introduce(1, 0, 0, 0, 3, "test").is_err());
    assert!(f.introduce(1, 0, 2, 3, 3, "test").is_err());
}
#[test]
fn original_expiry_and_totals_match_unit_oracle_for_192_schedules() {
    for life in 2..=7u32 {
        for mask in 0..32u32 {
            let mut f = new();
            let mut expired = 0u128;
            for t in 0..10u32 {
                f.advance(t).unwrap();
                if t < 5 && mask & (1 << t) != 0 {
                    f.introduce(u64::from(t), 0, 1, t, t + life, "unit oracle")
                        .unwrap();
                }
                if t >= life && t - life < 5 && mask & (1 << (t - life)) != 0 {
                    expired += 1;
                }
                let live = (0..5u32)
                    .filter(|&s| mask & (1 << s) != 0 && s <= t && t < s + life)
                    .count() as u128;
                assert_eq!(f.balance().expired, expired);
                assert_eq!(f.balance().stock_by_resource[0], live);
                let b = f.checkpoint().unwrap();
                f = DatedFlow::restore(&b, &b, "food/v1", f.balance().revision).unwrap();
            }
        }
    }
}

#[test]
fn intermediate_arrival_does_not_certify_the_following_leg() {
    let mut f = new();
    f.advance(0).unwrap();
    f.introduce(1, 0, 10, 0, 2, "short life").unwrap();
    f.transfer(0, 1, 10, 1).unwrap();
    f.advance(1).unwrap();
    let before = f.clone();
    assert!(f.transfer(1, 2, 10, 2).is_err());
    assert_eq!(f, before);
    f.advance(2).unwrap();
    assert_eq!(f.balance().expired, 10);
    assert_eq!(f.balance().withdrawn, 0);
}
