use crate::domain::{Account, NewAccount, Receipt, Transfer};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
pub(crate) type Tx<'a> = Transaction<'a, Postgres>;

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("invalid request: {0}")]
    Invalid(&'static str),
    #[error("account not found")]
    NotFound,
    #[error("account already exists")]
    AccountConflict,
    #[error("idempotency key belongs to another request")]
    IdempotencyConflict,
    #[error("insufficient available funds")]
    InsufficientFunds,
    #[error("balance limit exceeded")]
    Overflow,
    #[error("allocation already exists for resident, period and category")]
    AllocationConflict,
    #[error("resource not found")]
    ResourceNotFound,
    #[error("resource already exists")]
    ResourceConflict,
    #[error("insufficient physical stock")]
    InsufficientStock,
    #[error("reservation not found")]
    ReservationNotFound,
    #[error("reservation already resolved")]
    ReservationClosed,
    #[error("provisioning unit not found")]
    UnitNotFound,
    #[error("provisioning unit already exists")]
    UnitConflict,
    #[error("producer already belongs to a unit")]
    ProviderConflict,
    #[error("provider is outside assigned unit hierarchy")]
    ProviderOutOfScope,
    #[error("need not found")]
    NeedNotFound,
    #[error("need already exists for resident, period, category and unit")]
    NeedConflict,
    #[error("production capacity, recipe or plan not found")]
    ProductionNotFound,
    #[error("production definition already exists")]
    ProductionConflict,
    #[error("production plan is already closed")]
    ProductionClosed,
    #[error("need norm not found")]
    NormNotFound,
    #[error("norm revision already exists or branches from a superseded version")]
    NormConflict,
    #[error("planning draft not found")]
    DraftNotFound,
    #[error("planning draft already exists")]
    DraftConflict,
    #[error("planning draft is already closed")]
    DraftClosed,
    #[error("planning observation changed; create and review a new draft")]
    DraftStale,
    #[error("draft has no actions to apply")]
    DraftEmpty,
    #[error("coordination monitor not found")]
    MonitorNotFound,
    #[error("coordination monitor already exists")]
    MonitorConflict,
    #[error("coordination monitor is stopped")]
    MonitorStopped,
    #[error("coordination result not found")]
    CalculationNotFound,
    #[error("calculation failed; inspect monitor status")]
    CalculationFailed,
    #[error("calculation lease expired or monitor stopped")]
    CalculationLeaseLost,
    #[error("AI review already exists")]
    ReviewConflict,
    #[error("stored response is invalid")]
    InvalidStoredResponse,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Clone)]
pub struct Ledger {
    pub(crate) pool: PgPool,
}

impl Ledger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn ready(&self) -> Result<(), LedgerError> {
        sqlx::query("SELECT job_id FROM coordination_results LIMIT 0")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_account(&self, account: NewAccount) -> Result<Account, LedgerError> {
        let result = sqlx::query("INSERT INTO accounts(id, kind) VALUES ($1, $2)")
            .bind(account.id)
            .bind(account.kind.as_str())
            .execute(&self.pool)
            .await;
        if let Err(error) = result {
            if error
                .as_database_error()
                .is_some_and(|e| e.is_unique_violation())
            {
                return Err(LedgerError::AccountConflict);
            }
            return Err(error.into());
        }
        self.account(account.id).await
    }

    pub async fn account(&self, id: Uuid) -> Result<Account, LedgerError> {
        let row = sqlx::query("SELECT kind, balance::text, reserved::text, (balance-reserved)::text AS available FROM account_totals WHERE id = $1")
            .bind(id).fetch_optional(&self.pool).await?.ok_or(LedgerError::NotFound)?;
        Ok(Account {
            id,
            kind: row.try_get("kind")?,
            balance: row.try_get("balance")?,
            reserved: row.try_get("reserved")?,
            available: row.try_get("available")?,
        })
    }

    pub async fn transfer(&self, key: &str, request: Transfer) -> Result<Receipt, LedgerError> {
        if request.from == request.to {
            return Err(LedgerError::Invalid("source and destination must differ"));
        }
        let payload = serde_json::json!({"version": 1, "transfer": request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let id = Uuid::new_v4();
        post(&mut tx, id, key, "transfer", &payload, &request).await?;
        finish(
            tx,
            key,
            &payload,
            Receipt {
                transaction_id: id,
                transfer: request,
            },
        )
        .await
    }

    /// Serialize a command key across all routes, including operations without postings.
    pub(crate) async fn start(
        &self,
        key: &str,
        payload: &Value,
    ) -> Result<(Tx<'_>, Option<Value>), LedgerError> {
        if key.is_empty() || key.len() > 128 || !key.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(LedgerError::Invalid(
                "idempotency key must be 1..128 printable ASCII bytes",
            ));
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL lock_timeout = '5s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(key)
            .execute(&mut *tx)
            .await?;
        if let Some(row) = sqlx::query("SELECT request, response FROM commands WHERE key = $1")
            .bind(key)
            .fetch_optional(&mut *tx)
            .await?
        {
            let stored: Value = row.try_get("request")?;
            if stored != *payload {
                return Err(LedgerError::IdempotencyConflict);
            }
            return Ok((tx, Some(row.try_get("response")?)));
        }
        // Preserve replay of operations created before migration 0002.
        if let Some(row) = sqlx::query(
            "SELECT id, kind, request FROM journal_transactions WHERE idempotency_key = $1",
        )
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?
        {
            let stored: Value = row.try_get("request")?;
            let kind: String = row.try_get("kind")?;
            if kind != "transfer" || stored != *payload {
                return Err(LedgerError::IdempotencyConflict);
            }
            let id: Uuid = row.try_get("id")?;
            return Ok((
                tx,
                Some(serde_json::json!({"transaction_id": id, "transfer": stored["transfer"]})),
            ));
        }
        Ok((tx, None))
    }
}

pub(crate) fn decode<T: DeserializeOwned>(value: Value) -> Result<T, LedgerError> {
    serde_json::from_value(value).map_err(|_| LedgerError::InvalidStoredResponse)
}

pub(crate) async fn finish<T: Serialize>(
    mut tx: Tx<'_>,
    key: &str,
    payload: &Value,
    result: T,
) -> Result<T, LedgerError> {
    let response = serde_json::to_value(&result).map_err(|_| LedgerError::InvalidStoredResponse)?;
    sqlx::query("INSERT INTO commands(key, request, response) VALUES ($1, $2, $3)")
        .bind(key)
        .bind(payload)
        .bind(response)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result)
}

pub(crate) async fn lock_accounts(
    tx: &mut Tx<'_>,
    ids: &[Uuid],
) -> Result<BTreeMap<Uuid, String>, LedgerError> {
    let rows =
        sqlx::query("SELECT id, kind FROM accounts WHERE id = ANY($1) ORDER BY id FOR UPDATE")
            .bind(ids)
            .fetch_all(&mut **tx)
            .await?;
    let mut accounts = BTreeMap::new();
    for row in rows {
        accounts.insert(row.try_get("id")?, row.try_get("kind")?);
    }
    if ids.iter().any(|id| !accounts.contains_key(id)) {
        return Err(LedgerError::NotFound);
    }
    Ok(accounts)
}

pub(crate) async fn available(tx: &mut Tx<'_>, id: Uuid) -> Result<i64, LedgerError> {
    let value: String =
        sqlx::query_scalar("SELECT (balance-reserved)::text FROM account_totals WHERE id = $1")
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;
    value.parse().map_err(|_| LedgerError::Overflow)
}

pub(crate) async fn post(
    tx: &mut Tx<'_>,
    id: Uuid,
    key: &str,
    kind: &str,
    payload: &Value,
    request: &Transfer,
) -> Result<(), LedgerError> {
    let accounts = lock_accounts(tx, &[request.from, request.to]).await?;
    if accounts.len() != 2 || accounts.values().any(|k| k == "issuer") {
        return Err(LedgerError::Invalid(
            "distinct non-issuer accounts are required",
        ));
    }
    if available(tx, request.from).await? < request.amount.units() {
        return Err(LedgerError::InsufficientFunds);
    }
    let balance: String =
        sqlx::query_scalar("SELECT balance::text FROM account_totals WHERE id = $1")
            .bind(request.to)
            .fetch_one(&mut **tx)
            .await?;
    if balance
        .parse::<i64>()
        .map_err(|_| LedgerError::Overflow)?
        .checked_add(request.amount.units())
        .is_none()
    {
        return Err(LedgerError::Overflow);
    }
    sqlx::query(
        "INSERT INTO journal_transactions(id, idempotency_key, kind, request) VALUES ($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(key)
    .bind(kind)
    .bind(payload)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO postings(transaction_id, account_id, delta) VALUES ($1,$2,$4), ($1,$3,-$4)",
    )
    .bind(id)
    .bind(request.to)
    .bind(request.from)
    .bind(request.amount.units())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
