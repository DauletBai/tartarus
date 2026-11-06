use super::*;
use qazna::domain::{AccountKind, NewAccount};
use std::collections::BTreeMap;

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn http_body_boundary_rejects_without_consuming_key_or_changing_balances() {
    let f = Fixture::new().await;
    let token = "body-boundary-operator-token-32-characters";
    let app = api::router(f.ledger.clone(), token.into());
    let payload = serde_json::to_string(&f.transfer("100")).unwrap();
    // Whitespace is valid JSON padding: the domain operation is otherwise valid.
    // Test immediately above the limit first, then reuse exactly the same key.
    for (size, expected, balance) in [
        (16 * 1024 + 1, StatusCode::PAYLOAD_TOO_LARGE, "0"),
        (16 * 1024, StatusCode::OK, "100"),
        (payload.len(), StatusCode::OK, "100"),
    ] {
        let body = format!("{}{}", payload, " ".repeat(size - payload.len()));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/transfers")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("idempotency-key", "body-boundary")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(f.balance(f.resident).await, balance);
        assert_eq!(
            f.transaction_count().await,
            if balance == "0" { 1 } else { 2 }
        );
    }
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL; use scripts/check.py"]
async fn seeded_transfer_histories_match_independent_integer_model() {
    // Independent sequential reference model: no SQL views, production balance
    // helpers or service calls are used to compute expected transitions.
    // This exercises 512 requests, not all possible histories or concurrent schedules.
    for seed in [7u64, 19, 41, 73] {
        let f = Fixture::new().await;
        let mut accounts = vec![f.treasury, f.resident];
        for _ in 0..2 {
            let id = Uuid::new_v4();
            f.ledger
                .create_account(NewAccount {
                    id,
                    kind: AccountKind::Resident,
                })
                .await
                .unwrap();
            accounts.push(id);
        }
        let mut balances = [1000i128, 0, 0, 0];
        let mut accepted = BTreeMap::<String, (usize, usize, i64, Uuid)>::new();
        let mut state = seed;
        let mut random = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for step in 0..128 {
            let choice = random();
            let mut from = (random() % 4) as usize;
            let mut to = (from + 1 + (random() % 3) as usize) % 4;
            let mut amount = (random() % 1400 + 1) as i64;
            let mut key = format!("sequence-{step}");
            if choice % 3 != 0 && !accepted.is_empty() {
                let index = (random() % accepted.len() as u64) as usize;
                let (previous_key, previous) = accepted.iter().nth(index).unwrap();
                key = previous_key.clone();
                (from, to, amount) = (previous.0, previous.1, previous.2);
                if choice % 3 == 2 {
                    amount += 1;
                }
            }
            let result = f
                .ledger
                .transfer(
                    &key,
                    Transfer {
                        from: accounts[from],
                        to: accounts[to],
                        amount: Amount::parse(&amount.to_string()).unwrap(),
                    },
                )
                .await;
            if let Some(previous) = accepted.get(&key) {
                if (from, to, amount) == (previous.0, previous.1, previous.2) {
                    assert_eq!(
                        result.unwrap().transaction_id,
                        previous.3,
                        "seed={seed} step={step}"
                    );
                } else {
                    assert!(
                        matches!(result, Err(LedgerError::IdempotencyConflict)),
                        "seed={seed} step={step}"
                    );
                }
            } else if balances[from] < i128::from(amount) {
                assert!(
                    matches!(result, Err(LedgerError::InsufficientFunds)),
                    "seed={seed} step={step}"
                );
            } else {
                let receipt = result.unwrap();
                balances[from] -= i128::from(amount);
                balances[to] += i128::from(amount);
                accepted.insert(key, (from, to, amount, receipt.transaction_id));
            }
            for (index, account) in accounts.iter().enumerate() {
                assert_eq!(
                    f.balance(*account).await,
                    balances[index].to_string(),
                    "seed={seed} step={step} account={index}"
                );
            }
            assert_eq!(f.transaction_count().await, 1 + accepted.len() as i64);
            assert_eq!(balances.iter().sum::<i128>(), 1000);
        }
    }
}
