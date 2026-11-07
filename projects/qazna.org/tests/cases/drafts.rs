use super::*;
use qazna::{drafts::*, needs::NeedReservationRequest};

pub(super) async fn scenario(f: &Fixture, funds: bool) -> (Context, Uuid, DraftRequest) {
    let c = setup(f, "20", "10", "20").await;
    let stock = meal_stock(f, &c, "2", "10").await;
    let id = need(f, f.resident, c.unit, Category::Food, "meal", "5").await;
    if funds {
        f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    }
    let request = DraftRequest {
        id: Uuid::new_v4(),
        plan: proposal(vec![NeedOptions {
            need: id,
            resources: vec![stock],
            production: vec![option(&c)],
        }]),
    };
    (c, stock, request)
}
async fn pending(f: &Fixture, id: Uuid) {
    let d = f.ledger.planning_draft(id).await.unwrap();
    assert_eq!(d.state, "pending");
    assert!(d.actions.is_empty());
    for table in [
        "planning_outcomes",
        "planning_actions",
        "reservations",
        "production_plans",
        "need_attempts",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&f.pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table} must roll back");
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn draft_applies_reviewed_stock_and_production_once_without_receipt_or_postings() {
    let f = Fixture::new().await;
    let (c, stock, request) = scenario(&f, true).await;
    let draft = f
        .ledger
        .create_planning_draft("draft", request.clone())
        .await
        .unwrap();
    pending(&f, draft.id).await;
    let count = f.transaction_count().await;
    let applied = f
        .ledger
        .apply_planning_draft("apply", draft.id, reason())
        .await
        .unwrap();
    assert_eq!(applied.state, "applied");
    assert_eq!(applied.actions.len(), 2);
    assert_eq!(applied.actions[0].kind, "stock");
    assert_eq!(applied.actions[1].kind, "production");
    assert_eq!(f.transaction_count().await, count);
    assert_eq!(f.balance(f.resident).await, "100");
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "20");
    assert_eq!(f.ledger.resource(stock).await.unwrap().available, "0");
    assert_eq!(f.ledger.resource(c.grain).await.unwrap().reserved, "2");
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "2");
    let n = f.ledger.need(request.plan.needs[0].need).await.unwrap();
    assert_eq!(
        (
            n.reserved.as_str(),
            n.received.as_str(),
            n.uncovered.as_str(),
            n.unmet.as_str()
        ),
        ("2", "0", "3", "5")
    );
    assert_eq!(
        serde_json::json!(applied),
        serde_json::json!(f.ledger.planning_draft(draft.id).await.unwrap())
    );
    assert_eq!(
        serde_json::json!(applied),
        serde_json::json!(
            f.ledger
                .apply_planning_draft("apply", draft.id, reason())
                .await
                .unwrap()
        )
    );
    assert_eq!(
        serde_json::json!(draft),
        serde_json::json!(
            f.ledger
                .create_planning_draft("draft", request.clone())
                .await
                .unwrap()
        )
    );
    assert!(matches!(
        f.ledger
            .discard_planning_draft("discard", draft.id, reason())
            .await,
        Err(LedgerError::DraftClosed)
    ));
    f.ledger
        .cancel(
            "cancel-stock",
            applied.actions[0].reservation.unwrap(),
            reason(),
        )
        .await
        .unwrap();
    f.ledger
        .cancel_production(
            "cancel-plan",
            applied.actions[1].production_plan.unwrap(),
            reason(),
        )
        .await
        .unwrap();
    assert!(matches!(
        f.ledger
            .apply_planning_draft("again", draft.id, reason())
            .await,
        Err(LedgerError::DraftClosed)
    ));
    let mut changed = request;
    changed.plan.basis = "Changed request".into();
    assert!(matches!(
        f.ledger.create_planning_draft("draft", changed).await,
        Err(LedgerError::IdempotencyConflict)
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn accounting_shortage_is_separate_from_stale_snapshot_and_requires_review() {
    let f = Fixture::new().await;
    let (_, _, request) = scenario(&f, false).await;
    let draft = f
        .ledger
        .create_planning_draft("draft", request.clone())
        .await
        .unwrap();
    assert_eq!(draft.proposal.accounting[0].shortage, "20");
    assert!(matches!(
        f.ledger
            .apply_planning_draft("apply", draft.id, reason())
            .await,
        Err(LedgerError::InsufficientFunds)
    ));
    pending(&f, draft.id).await;
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    assert!(matches!(
        f.ledger
            .apply_planning_draft("apply", draft.id, reason())
            .await,
        Err(LedgerError::DraftStale)
    ));
    pending(&f, draft.id).await;
    let mut fresh = request;
    fresh.id = Uuid::new_v4();
    let fresh = f
        .ledger
        .create_planning_draft("fresh", fresh)
        .await
        .unwrap();
    // Failed commands do not consume the key.
    f.ledger
        .apply_planning_draft("apply", fresh.id, reason())
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn changed_physical_capacity_or_need_cannot_silently_replace_reviewed_plan() {
    for change in ["stock", "capacity", "need"] {
        let f = Fixture::new().await;
        let (c, stock, request) = scenario(&f, true).await;
        let draft = f
            .ledger
            .create_planning_draft("draft", request.clone())
            .await
            .unwrap();
        match change {
            "capacity" => {
                f.ledger
                    .plan_production("other", plan(&c, "1"))
                    .await
                    .unwrap();
            }
            "stock" => {
                f.ledger
                    .reserve(
                        "other",
                        ReservationRequest {
                            resident: f.resident,
                            resource: stock,
                            quantity: amount("1"),
                            basis: "Other request".into(),
                        },
                    )
                    .await
                    .unwrap();
            }
            _ => {
                f.ledger
                    .reserve_need(
                        "other",
                        request.plan.needs[0].need,
                        NeedReservationRequest {
                            resource: stock,
                            quantity: amount("1"),
                            basis: "Other request".into(),
                        },
                    )
                    .await
                    .unwrap();
            }
        }
        assert!(
            matches!(
                f.ledger
                    .apply_planning_draft("apply", draft.id, reason())
                    .await,
                Err(LedgerError::DraftStale)
            ),
            "{change}"
        );
        assert_eq!(
            f.ledger.planning_draft(draft.id).await.unwrap().state,
            "pending"
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM commands WHERE key='apply'")
            .fetch_one(&f.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn empty_draft_can_be_discarded_but_never_applied() {
    let f = Fixture::new().await;
    let (_, _, mut request) = scenario(&f, true).await;
    request.plan.needs[0].resources.clear();
    request.plan.needs[0].production.clear();
    let draft = f
        .ledger
        .create_planning_draft("draft", request)
        .await
        .unwrap();
    assert!(matches!(
        f.ledger
            .apply_planning_draft("apply", draft.id, reason())
            .await,
        Err(LedgerError::DraftEmpty)
    ));
    let closed = f
        .ledger
        .discard_planning_draft("discard", draft.id, reason())
        .await
        .unwrap();
    assert_eq!(closed.state, "discarded");
    assert!(closed.actions.is_empty());
    assert_eq!(
        serde_json::json!(closed),
        serde_json::json!(
            f.ledger
                .discard_planning_draft("discard", draft.id, reason())
                .await
                .unwrap()
        )
    );
    assert!(matches!(
        f.ledger
            .apply_planning_draft("apply", draft.id, reason())
            .await,
        Err(LedgerError::DraftClosed)
    ));
    assert!(matches!(
        f.ledger.planning_draft(Uuid::new_v4()).await,
        Err(LedgerError::DraftNotFound)
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_creators_and_appliers_are_bounded_and_idempotent() {
    let f = Fixture::new().await;
    let (_, _, request) = scenario(&f, true).await;
    let mut jobs = Vec::new();
    for _ in 0..24 {
        let ledger = f.ledger.clone();
        let req = request.clone();
        jobs.push(tokio::spawn(async move {
            ledger.create_planning_draft("draft", req).await
        }));
    }
    for job in jobs {
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(15), job)
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .id,
            request.id
        );
    }
    let (a, b) = tokio::join!(
        f.ledger.apply_planning_draft("apply", request.id, reason()),
        f.ledger.apply_planning_draft("apply", request.id, reason())
    );
    assert_eq!(serde_json::json!(a.unwrap()), serde_json::json!(b.unwrap()));
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "20");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn applying_and_discarding_have_one_winner() {
    let f = Fixture::new().await;
    let (_, _, request) = scenario(&f, true).await;
    let draft = f
        .ledger
        .create_planning_draft("draft", request)
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        f.ledger.apply_planning_draft("apply", draft.id, reason()),
        f.ledger
            .discard_planning_draft("discard", draft.id, reason())
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(LedgerError::DraftClosed)
    ));
    let d = f.ledger.planning_draft(draft.id).await.unwrap();
    assert_eq!(d.actions.len(), if d.state == "applied" { 2 } else { 0 });
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn late_production_failure_rolls_back_stock_outcome_and_command() {
    let f = Fixture::new().await;
    let (_, _, request) = scenario(&f, true).await;
    let draft = f
        .ledger
        .create_planning_draft("draft", request)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_plan() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected failure'; END $$; CREATE TRIGGER fail_plan BEFORE INSERT ON production_plans FOR EACH ROW EXECUTE FUNCTION fail_plan();").execute(&f.pool).await.unwrap();
    assert!(matches!(
        f.ledger
            .apply_planning_draft("apply", draft.id, reason())
            .await,
        Err(LedgerError::Database(_))
    ));
    pending(&f, draft.id).await;
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "0");
    sqlx::query("DROP TRIGGER fail_plan ON production_plans")
        .execute(&f.pool)
        .await
        .unwrap();
    f.ledger
        .apply_planning_draft("apply", draft.id, reason())
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn database_rejects_partial_application_and_history_changes() {
    let f = Fixture::new().await;
    let (_, _, request) = scenario(&f, true).await;
    let draft = f
        .ledger
        .create_planning_draft("draft", request)
        .await
        .unwrap();
    assert!(
        sqlx::query(
            "INSERT INTO planning_outcomes(draft_id,state,basis) VALUES ($1,'applied','No actions')"
        )
        .bind(draft.id)
        .execute(&f.pool)
        .await
        .is_err()
    );
    pending(&f, draft.id).await;
    f.ledger
        .apply_planning_draft("apply", draft.id, reason())
        .await
        .unwrap();
    for table in ["planning_drafts", "planning_outcomes", "planning_actions"] {
        assert!(
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&f.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query(&format!("TRUNCATE {table} CASCADE"))
                .execute(&f.pool)
                .await
                .is_err()
        );
    }
    assert!(
        sqlx::query("UPDATE planning_drafts SET proposal='{}'")
            .execute(&f.pool)
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_drafts_share_resources_without_overcommit() {
    let f = Fixture::new().await;
    let (c, stock, request) = scenario(&f, true).await;
    let mut second = request.clone();
    second.id = Uuid::new_v4();
    let a = f.ledger.create_planning_draft("a", request).await.unwrap();
    let b = f.ledger.create_planning_draft("b", second).await.unwrap();
    let (a, b) = tokio::join!(
        f.ledger.apply_planning_draft("apply-a", a.id, reason()),
        f.ledger.apply_planning_draft("apply-b", b.id, reason())
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert!(matches!(
        a.as_ref().err().or(b.as_ref().err()),
        Some(LedgerError::DraftStale)
    ));
    assert_eq!(f.ledger.resource(stock).await.unwrap().reserved, "2");
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "2");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn upgrade_from_norms_preserves_existing_needs_and_holds() {
    let f = Fixture::new_with_migrations(6).await;
    let (c, stock, request) = scenario(&f, true).await;
    f.ledger
        .plan_production("old", plan(&c, "1"))
        .await
        .unwrap();
    MIGRATOR.run(&f.pool).await.unwrap();
    let before = f
        .ledger
        .propose_provisioning(request.plan.clone())
        .await
        .unwrap();
    MIGRATOR.run(&f.pool).await.unwrap();
    f.ledger.ready().await.unwrap();
    let draft = f
        .ledger
        .create_planning_draft("draft", request)
        .await
        .unwrap();
    assert_eq!(
        serde_json::json!(before.needs),
        serde_json::json!(draft.proposal.needs)
    );
    assert_eq!(f.ledger.resource(stock).await.unwrap().reserved, "0");
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "2");
    f.ledger
        .apply_planning_draft("apply", draft.id, reason())
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn multiple_needs_share_stock_and_capacity_in_reviewed_order() {
    let f = Fixture::new().await;
    let (c, stock, mut request) = scenario(&f, true).await;
    let other = Uuid::new_v4();
    f.ledger
        .create_account(NewAccount {
            id: other,
            kind: AccountKind::Resident,
        })
        .await
        .unwrap();
    let n = need(&f, other, c.unit, Category::Food, "meal", "6").await;
    request.plan.needs.push(NeedOptions {
        need: n,
        resources: vec![stock],
        production: vec![option(&c)],
    });
    let draft = f
        .ledger
        .create_planning_draft("draft", request)
        .await
        .unwrap();
    assert_eq!(draft.proposal.needs[1].stock_selected, "0");
    assert_eq!(draft.proposal.needs[1].expected_from_production, "6");
    let applied = f
        .ledger
        .apply_planning_draft("apply", draft.id, reason())
        .await
        .unwrap();
    assert_eq!(applied.actions.len(), 3);
    assert_eq!(applied.actions[2].need, n);
    assert_eq!(f.ledger.resource(c.grain).await.unwrap().reserved, "6");
    assert_eq!(f.ledger.resource(c.water).await.unwrap().reserved, "15");
    assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "6");
    assert_eq!(f.ledger.need(n).await.unwrap().uncovered, "6");
    assert_eq!(f.balance(other).await, "0");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn database_rejects_retrospective_attribution_of_unlinked_actions() {
    let f = Fixture::new().await;
    let (_, stock, request) = scenario(&f, true).await;
    let n = request.plan.needs[0].need;
    let draft = f
        .ledger
        .create_planning_draft("draft", request)
        .await
        .unwrap();
    let stock = f
        .ledger
        .reserve_need(
            "standalone-stock",
            n,
            NeedReservationRequest {
                resource: stock,
                quantity: amount("2"),
                basis: "Independent reservation".into(),
            },
        )
        .await
        .unwrap();
    let production = f
        .ledger
        .plan_production(
            "standalone-production",
            draft.proposal.needs[0].production[0].request.clone(),
        )
        .await
        .unwrap();
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO planning_outcomes(draft_id,state,basis) VALUES ($1,'applied','Retroactive association')")
        .bind(draft.id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO planning_actions(draft_id,need_position,action_position,kind,need_id,reservation_id,attempt_id) VALUES ($1,0,0,'stock',$2,$3,$4)")
        .bind(draft.id).bind(n).bind(stock.reservation_id).bind(stock.id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO planning_actions(draft_id,need_position,action_position,kind,need_id,production_plan_id) VALUES ($1,0,0,'production',$2,$3)")
        .bind(draft.id).bind(n).bind(production.plan_id).execute(&mut *tx).await.unwrap();
    let error = tx.commit().await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23514")
    );
    assert_eq!(
        f.ledger.planning_draft(draft.id).await.unwrap().state,
        "pending"
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn batch_and_direct_reservation_race_without_partial_batch_or_deadlock() {
    let f = Fixture::new().await;
    let (c, stock, request) = scenario(&f, true).await;
    let draft = f
        .ledger
        .create_planning_draft("draft", request)
        .await
        .unwrap();
    let direct = ReservationRequest {
        resident: f.resident,
        resource: stock,
        quantity: amount("1"),
        basis: "Direct concurrent request".into(),
    };
    let (batch, one) = tokio::join!(
        f.ledger.apply_planning_draft("apply", draft.id, reason()),
        f.ledger.reserve("direct", direct)
    );
    match (batch, one) {
        (Ok(_), Err(LedgerError::InsufficientStock)) => {
            assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "2");
            assert_eq!(f.ledger.resource(stock).await.unwrap().reserved, "2");
        }
        (Err(LedgerError::DraftStale), Ok(_)) => {
            assert_eq!(f.ledger.capacity(c.capacity).await.unwrap().reserved, "0");
            assert_eq!(f.ledger.resource(stock).await.unwrap().reserved, "1");
        }
        other => panic!("unexpected race result: {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn active_forecasts_prevent_duplicate_planning_and_cancel_reopens_gap() {
    let f = Fixture::new().await;
    let (_, _, request) = scenario(&f, true).await;
    let draft = f
        .ledger
        .create_planning_draft("d", request.clone())
        .await
        .unwrap();
    let applied = f
        .ledger
        .apply_planning_draft("a", draft.id, reason())
        .await
        .unwrap();
    let next = f
        .ledger
        .propose_provisioning(request.plan.clone())
        .await
        .unwrap();
    assert_eq!(next.needs[0].uncovered, "3");
    assert_eq!(next.needs[0].committed_production, "3");
    assert_eq!(next.needs[0].expected_from_production, "0");
    assert_eq!(next.needs[0].residual, "0");
    assert!(next.needs[0].production.is_empty());
    let plan = applied
        .actions
        .iter()
        .find_map(|a| a.production_plan)
        .unwrap();
    f.ledger
        .cancel_production("c", plan, reason())
        .await
        .unwrap();
    let next = f.ledger.propose_provisioning(request.plan).await.unwrap();
    assert_eq!(next.needs[0].committed_production, "0");
    assert_eq!(next.needs[0].expected_from_production, "3");
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn forecast_excess_is_visible_and_completion_requires_actual_receipt() {
    let f = Fixture::new().await;
    let (c, _, request) = scenario(&f, true).await;
    let draft = f
        .ledger
        .create_planning_draft("d", request.clone())
        .await
        .unwrap();
    let applied = f
        .ledger
        .apply_planning_draft("a", draft.id, reason())
        .await
        .unwrap();
    let plan = applied
        .actions
        .iter()
        .find_map(|a| a.production_plan)
        .unwrap();
    let extra = meal_stock(&f, &c, "3", "1").await;
    let mut plan_request = request.plan;
    plan_request.needs[0].resources.push(extra);
    // Include a request without any new production candidate: its outstanding
    // commitments must still be observed and locked before application.
    plan_request.needs[0].production.clear();
    let next = f
        .ledger
        .create_planning_draft(
            "next",
            DraftRequest {
                id: Uuid::new_v4(),
                plan: plan_request.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(next.proposal.needs[0].committed_production, "3");
    assert_eq!(next.proposal.needs[0].forecast_excess, "3");
    let producer = executor(&f, c.unit).await;
    let result = f
        .ledger
        .complete_production("finish", plan, completion(producer, "1"))
        .await
        .unwrap();
    assert_eq!(result.output_shortfall, "2");
    assert!(matches!(
        f.ledger
            .apply_planning_draft("stale", next.id, reason())
            .await,
        Err(LedgerError::DraftStale)
    ));
    let next = f.ledger.propose_provisioning(plan_request).await.unwrap();
    assert_eq!(next.needs[0].committed_production, "0");
    let n = f.ledger.need(next.needs[0].need).await.unwrap();
    assert_eq!(n.received, "0");
    assert_eq!(n.unmet, "5");
}
