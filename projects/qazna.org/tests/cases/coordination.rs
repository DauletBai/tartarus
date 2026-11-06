use super::drafts::scenario;
use super::*;
use qazna::coordination::*;

async fn monitor(f: &Fixture, funds: bool) -> (MonitorRequest, Uuid) {
    let (_, stock, request) = scenario(f, funds).await;
    // Old committed commands are dispatched before registering this scope.
    assert!(f.ledger.coordinate_once().await.unwrap().is_none());
    let monitor = MonitorRequest {
        id: Uuid::new_v4(),
        plan: request.plan,
    };
    f.ledger
        .create_monitor("monitor", monitor.clone())
        .await
        .unwrap();
    (monitor, stock)
}
async fn count(f: &Fixture, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(&f.pool)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn automatic_recalculation_keeps_operator_authority_and_deduplicates_drafts() {
    let f = Fixture::new().await;
    let (m, stock) = monitor(&f, false).await;
    let first = f.ledger.coordinate_once().await.unwrap().unwrap();
    assert!(first.changed);
    assert_eq!(first.proposal.accounting[0].shortage, "20");
    assert_eq!(
        f.ledger.planning_draft(first.draft).await.unwrap().state,
        "pending"
    );
    assert_eq!(f.ledger.resource(stock).await.unwrap().reserved, "0");
    assert_eq!(count(&f, "production_plans").await, 0);
    assert!(f.ledger.coordinate_once().await.unwrap().is_none());
    // A norm declaration does not change the selected existing needs.
    f.ledger
        .create_need_norm("unrelated", norm("7"))
        .await
        .unwrap();
    let same = f.ledger.coordinate_once().await.unwrap().unwrap();
    assert!(!same.changed);
    assert_eq!(same.draft, first.draft);
    assert_eq!(count(&f, "planning_drafts").await, 1);
    f.ledger.transfer("fund", f.transfer("100")).await.unwrap();
    let funded = f.ledger.coordinate_once().await.unwrap().unwrap();
    assert!(funded.changed);
    assert_ne!(funded.draft, first.draft);
    assert_eq!(funded.proposal.accounting[0].shortage, "0");
    let current = f.ledger.monitor(m.id).await.unwrap();
    assert_eq!(current.latest.unwrap().job, funded.job);
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "0");
    assert_eq!(count(&f, "coordination_reviews").await, 0);
    // Only the explicit operator command applies the generated draft.
    f.ledger
        .apply_planning_draft("operator-apply", funded.draft, reason())
        .await
        .unwrap();
    assert_eq!(f.ledger.account(f.resident).await.unwrap().reserved, "20");
    // A complete recalculation/application needs no model review or credentials.
    assert_eq!(count(&f, "coordination_reviews").await, 0);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn rolled_back_and_replayed_commands_do_not_duplicate_events() {
    let f = Fixture::new().await;
    let (m, _) = monitor(&f, true).await;
    f.ledger.coordinate_once().await.unwrap();
    let before = count(&f, "coordination_events").await;
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO commands(key,request,response) VALUES ('aborted','{}','{}')")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(count(&f, "coordination_events").await, before);
    let req = f.transfer("1");
    f.ledger.transfer("event", req.clone()).await.unwrap();
    f.ledger.transfer("event", req).await.unwrap();
    assert_eq!(count(&f, "coordination_events").await, before + 1);
    f.ledger.coordinate_once().await.unwrap();
    assert_eq!(f.ledger.monitor(m.id).await.unwrap().queued, 0);
    assert_eq!(count(&f, "coordination_jobs").await, 2);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn late_commit_is_dispatched_even_after_a_newer_event() {
    let f = Fixture::new().await;
    monitor(&f, true).await;
    f.ledger.coordinate_once().await.unwrap();
    let mut slow = f.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO commands(key,request,response) VALUES ('slow','{}','{}')")
        .execute(&mut *slow)
        .await
        .unwrap();
    sqlx::query("INSERT INTO commands(key,request,response) VALUES ('fast','{}','{}')")
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        f.ledger
            .coordinate_once()
            .await
            .unwrap()
            .unwrap()
            .trigger_command
            .as_deref(),
        Some("fast")
    );
    slow.commit().await.unwrap();
    assert_eq!(
        f.ledger
            .coordinate_once()
            .await
            .unwrap()
            .unwrap()
            .trigger_command
            .as_deref(),
        Some("slow")
    );
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn concurrent_workers_complete_each_job_once() {
    let f = Fixture::new().await;
    monitor(&f, true).await;
    let mut workers = Vec::new();
    for _ in 0..12 {
        let ledger = f.ledger.clone();
        workers.push(tokio::spawn(async move { ledger.coordinate_once().await }));
    }
    let mut completed = 0;
    for worker in workers {
        if worker.await.unwrap().unwrap().is_some() {
            completed += 1;
        }
    }
    assert_eq!(completed, 1);
    assert_eq!(count(&f, "coordination_results").await, 1);
    assert_eq!(count(&f, "planning_drafts").await, 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn expired_worker_lease_is_recovered_but_retries_are_bounded() {
    let f = Fixture::new().await;
    let (m, _) = monitor(&f, true).await;
    sqlx::query("UPDATE coordination_jobs SET state='running',attempts=1,lease_token=$1,lease_until=clock_timestamp()-interval '1 second' WHERE monitor_id=$2")
        .bind(Uuid::new_v4()).bind(m.id).execute(&f.pool).await.unwrap();
    f.ledger.coordinate_once().await.unwrap().unwrap();
    let attempts: i32 =
        sqlx::query_scalar("SELECT attempts FROM coordination_jobs WHERE monitor_id=$1")
            .bind(m.id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(attempts, 2);
    f.ledger.transfer("signal", f.transfer("1")).await.unwrap();
    // Dispatch a new job, but simulate its three crashed workers before another pass.
    sqlx::query("INSERT INTO coordination_jobs(monitor_id,command_key,state,attempts,lease_token,lease_until) VALUES ($1,'signal','running',3,$2,clock_timestamp()-interval '1 second')")
        .bind(m.id).bind(Uuid::new_v4()).execute(&f.pool).await.unwrap();
    assert!(f.ledger.coordinate_once().await.unwrap().is_none());
    assert_eq!(f.ledger.monitor(m.id).await.unwrap().failed, 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn stop_cancels_pending_and_running_work_and_preserves_history() {
    let f = Fixture::new().await;
    let (m, _) = monitor(&f, true).await;
    sqlx::query("UPDATE coordination_jobs SET state='running',attempts=1,lease_token=$1,lease_until=clock_timestamp()+interval '30 seconds' WHERE monitor_id=$2")
        .bind(Uuid::new_v4()).bind(m.id).execute(&f.pool).await.unwrap();
    let stopped = f.ledger.stop_monitor("stop", m.id, reason()).await.unwrap();
    assert_eq!(stopped.state, "stopped");
    assert_eq!(stopped.running, 0);
    assert_eq!(
        serde_json::json!(stopped),
        serde_json::json!(f.ledger.stop_monitor("stop", m.id, reason()).await.unwrap())
    );
    f.ledger.transfer("later", f.transfer("1")).await.unwrap();
    assert!(f.ledger.coordinate_once().await.unwrap().is_none());
    assert_eq!(count(&f, "coordination_jobs").await, 1);
    assert_eq!(count(&f, "planning_drafts").await, 0);
    assert!(matches!(
        f.ledger.stop_monitor("again", m.id, reason()).await,
        Err(LedgerError::MonitorStopped)
    ));
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn failed_result_write_rolls_back_draft_and_can_be_recovered() {
    let f = Fixture::new().await;
    let (m, _) = monitor(&f, true).await;
    sqlx::raw_sql("CREATE FUNCTION fail_result() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected failure'; END $$; CREATE TRIGGER fail_result BEFORE INSERT ON coordination_results FOR EACH ROW EXECUTE FUNCTION fail_result();").execute(&f.pool).await.unwrap();
    assert!(f.ledger.coordinate_once().await.is_err());
    assert_eq!(count(&f, "planning_drafts").await, 0);
    assert_eq!(count(&f, "coordination_results").await, 0);
    sqlx::query("DROP TRIGGER fail_result ON coordination_results")
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE coordination_jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE monitor_id=$1").bind(m.id).execute(&f.pool).await.unwrap();
    assert!(f.ledger.coordinate_once().await.unwrap().is_some());
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn ai_report_cannot_invent_evidence_bypass_approval_or_apply_actions() {
    let f = Fixture::new().await;
    monitor(&f, false).await;
    let result = f.ledger.coordinate_once().await.unwrap().unwrap();
    let report = AiReport {
        summary: "Accounting is insufficient; production is a forecast.".into(),
        findings: vec![AiFinding {
            signal: "accounting_shortage".into(),
            subject: f.resident,
            quantity: "20".into(),
            explanation: "Matches the recorded gap.".into(),
        }],
        questions: vec!["What explicit allocation should the operator review?".into()],
        requires_operator_approval: true,
    };
    for defect in ["amount", "subject", "signal", "approval", "duplicate"] {
        let mut bad = report.clone();
        match defect {
            "amount" => bad.findings[0].quantity = "999".into(),
            "subject" => bad.findings[0].subject = Uuid::new_v4(),
            "signal" => bad.findings[0].signal = "issue_money".into(),
            "approval" => bad.requires_operator_approval = false,
            _ => bad.findings.push(bad.findings[0].clone()),
        }
        assert!(
            matches!(
                f.ledger
                    .record_ai_review(
                        "review",
                        result.job,
                        ReviewRequest {
                            id: Uuid::new_v4(),
                            model: "synthetic-test".into(),
                            report: bad
                        }
                    )
                    .await,
                Err(LedgerError::Invalid(_))
            ),
            "{defect}"
        );
    }
    let id = Uuid::new_v4();
    let accepted = f
        .ledger
        .record_ai_review(
            "review",
            result.job,
            ReviewRequest {
                id,
                model: "synthetic-test".into(),
                report: report.clone(),
            },
        )
        .await
        .unwrap();
    let replay = f
        .ledger
        .record_ai_review(
            "review",
            result.job,
            ReviewRequest {
                id,
                model: "synthetic-test".into(),
                report,
            },
        )
        .await
        .unwrap();
    assert_eq!(serde_json::json!(accepted), serde_json::json!(replay));
    assert_eq!(f.ledger.ai_reviews(result.job).await.unwrap().len(), 1);
    assert_eq!(count(&f, "reservations").await, 0);
    assert_eq!(count(&f, "production_plans").await, 0);
    assert!(f.ledger.coordinate_once().await.unwrap().is_none());
    for table in [
        "coordination_monitors",
        "coordination_stops",
        "coordination_results",
        "coordination_reviews",
    ] {
        assert!(
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&f.pool)
                .await
                .is_err()
        );
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn migration_preserves_reviewed_drafts_and_does_not_backfill_old_events() {
    let f = Fixture::new_with_migrations(7).await;
    let (c, stock, request) = scenario(&f, true).await;
    // Historical v1 JSON can be read after upgrade, but requires a fresh review.
    let process = plan(&c, "1");
    let assessment = f.ledger.assess_production(process.clone()).await.unwrap();
    let old = serde_json::json!({"advisory":true,"policy":"listed_need_order_stock_then_production",
        "observed_at":"2026-11-02T00:00:00Z","period":request.plan.period,"basis":request.plan.basis,
        "needs":[{"need":request.plan.needs[0].need,"category":"food","unit":"meal","uncovered":"5","stock_selected":"2","expected_from_production":"3","residual":"0",
          "stock":[{"resource":stock,"quantity":"2","required_measure":"20"}],
          "production":[{"request":process,"expected_for_need":"3","surplus":"0"}],
          "evaluations":[{"recipe":c.recipe.id,"capacity":c.capacity,"assessment":assessment}]}],
        "accounting":[{"resident":f.resident,"available":"100","required_for_selected_stock":"20","shortage":"0"}]});
    sqlx::query("INSERT INTO planning_drafts(id,request,proposal) VALUES ($1,$2,$3)")
        .bind(request.id)
        .bind(serde_json::json!(request.plan))
        .bind(&old)
        .execute(&f.pool)
        .await
        .unwrap();
    MIGRATOR.run(&f.pool).await.unwrap();
    assert_eq!(count(&f, "coordination_events").await, 0);
    let saved = f.ledger.planning_draft(request.id).await.unwrap();
    assert_eq!(saved.proposal.needs[0].committed_production, "0");
    assert_eq!(saved.proposal.needs[0].forecast_excess, "0");
    let raw: serde_json::Value =
        sqlx::query_scalar("SELECT proposal FROM planning_drafts WHERE id=$1")
            .bind(request.id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(raw, old);
    assert!(matches!(
        f.ledger
            .apply_planning_draft("old-apply", request.id, reason())
            .await,
        Err(LedgerError::DraftStale)
    ));
    let fresh = f
        .ledger
        .create_planning_draft(
            "fresh",
            qazna::drafts::DraftRequest {
                id: Uuid::new_v4(),
                plan: request.plan,
            },
        )
        .await
        .unwrap();
    f.ledger
        .apply_planning_draft("apply", fresh.id, reason())
        .await
        .unwrap();
    assert_eq!(count(&f, "coordination_events").await, 1);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn expired_or_stopped_worker_cannot_publish_a_late_result() {
    for mode in ["reclaim", "stop"] {
        let f = Fixture::new().await;
        let (m, _) = monitor(&f, true).await;
        let mut blocker = f.pool.begin().await.unwrap();
        sqlx::query("LOCK TABLE needs IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *blocker)
            .await
            .unwrap();
        let old = f.ledger.clone();
        let first = tokio::spawn(async move { old.coordinate_once().await });
        async fn wait_attempt(f: &Fixture, attempt: i32) {
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    let actual: i32 =
                        sqlx::query_scalar("SELECT attempts FROM coordination_jobs LIMIT 1")
                            .fetch_one(&f.pool)
                            .await
                            .unwrap();
                    if actual == attempt {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
        }
        wait_attempt(&f, 1).await;
        let second = if mode == "reclaim" {
            sqlx::query(
                "UPDATE coordination_jobs SET lease_until=clock_timestamp()-interval '1 second'",
            )
            .execute(&f.pool)
            .await
            .unwrap();
            let new = f.ledger.clone();
            let worker = tokio::spawn(async move { new.coordinate_once().await });
            wait_attempt(&f, 2).await;
            Some(worker)
        } else {
            f.ledger.stop_monitor("stop", m.id, reason()).await.unwrap();
            None
        };
        blocker.commit().await.unwrap();
        assert!(matches!(
            first.await.unwrap(),
            Err(LedgerError::CalculationLeaseLost)
        ));
        if let Some(worker) = second {
            assert!(worker.await.unwrap().unwrap().is_some());
        }
        assert_eq!(
            count(&f, "coordination_results").await,
            if mode == "reclaim" { 1 } else { 0 }
        );
        assert_eq!(
            count(&f, "planning_drafts").await,
            if mode == "reclaim" { 1 } else { 0 }
        );
    }
}
