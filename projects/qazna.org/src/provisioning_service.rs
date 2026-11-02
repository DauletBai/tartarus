use crate::{
    domain::{Amount, Transfer},
    ledger::{Ledger, LedgerError, Tx, available, decode, finish, lock_accounts, post},
    provisioning::*,
};
use sqlx::Row;
use uuid::Uuid;

impl Ledger {
    pub async fn allocate(
        &self,
        key: &str,
        request: AllocationRequest,
    ) -> Result<AllocationReceipt, LedgerError> {
        if !valid_period(&request.period) || !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid("canonical date and basis required"));
        }
        let payload = serde_json::json!({"version":1,"allocation":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let accounts = lock_accounts(&mut tx, &[request.treasury, request.resident]).await?;
        if accounts[&request.treasury] != "treasury" || accounts[&request.resident] != "resident" {
            return Err(LedgerError::Invalid(
                "allocation requires treasury and resident",
            ));
        }
        let id = Uuid::new_v4();
        let transaction_id = Uuid::new_v4();
        let inserted = sqlx::query("INSERT INTO allocations(id, resident_id, period, category, amount, basis, transaction_id) VALUES ($1,$2,$3::text::date,$4,$5,$6,$7) ON CONFLICT (resident_id,period,category) DO NOTHING")
            .bind(id).bind(request.resident).bind(&request.period).bind(request.category.as_str())
            .bind(request.amount.units()).bind(&request.basis).bind(transaction_id).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::AllocationConflict);
        }
        let transfer = Transfer {
            from: request.treasury,
            to: request.resident,
            amount: request.amount,
        };
        post(
            &mut tx,
            transaction_id,
            key,
            "allocation",
            &payload,
            &transfer,
        )
        .await?;
        finish(
            tx,
            key,
            &payload,
            AllocationReceipt {
                id,
                transaction_id,
                allocation: request,
            },
        )
        .await
    }

    pub async fn create_resource(
        &self,
        key: &str,
        request: ResourceRequest,
    ) -> Result<Resource, LedgerError> {
        if !valid_text(&request.unit, 40) || !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid("unit and basis required"));
        }
        let payload = serde_json::json!({"version":1,"resource":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let accounts = lock_accounts(&mut tx, &[request.producer]).await?;
        if accounts[&request.producer] != "producer" {
            return Err(LedgerError::Invalid("resource owner must be producer"));
        }
        let inserted = sqlx::query("INSERT INTO resources(id,producer_id,category,unit,initial_quantity,unit_measure,basis) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO NOTHING")
            .bind(request.id).bind(request.producer).bind(request.category.as_str()).bind(&request.unit)
            .bind(request.quantity.units()).bind(request.unit_measure.units()).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ResourceConflict);
        }
        let result = resource_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, result).await
    }

    pub async fn resource(&self, id: Uuid) -> Result<Resource, LedgerError> {
        let mut tx = self.pool.begin().await?;
        resource_in(&mut tx, id).await
    }

    pub async fn reservation(&self, id: Uuid) -> Result<Reservation, LedgerError> {
        let mut tx = self.pool.begin().await?;
        reservation_in(&mut tx, id).await
    }

    pub async fn reserve(
        &self,
        key: &str,
        request: ReservationRequest,
    ) -> Result<Reservation, LedgerError> {
        if !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid("basis required"));
        }
        let payload = serde_json::json!({"version":1,"reservation":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let result = reserve_in(&mut tx, request).await?;
        finish(tx, key, &payload, result).await
    }

    pub async fn settle(
        &self,
        key: &str,
        id: Uuid,
        request: ResolutionRequest,
    ) -> Result<Reservation, LedgerError> {
        self.resolve(key, id, request, true).await
    }

    pub async fn cancel(
        &self,
        key: &str,
        id: Uuid,
        request: ResolutionRequest,
    ) -> Result<Reservation, LedgerError> {
        self.resolve(key, id, request, false).await
    }

    async fn resolve(
        &self,
        key: &str,
        id: Uuid,
        request: ResolutionRequest,
        settle: bool,
    ) -> Result<Reservation, LedgerError> {
        if !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid("basis required"));
        }
        let state = if settle { "settled" } else { "cancelled" };
        let payload = serde_json::json!({"version":1,"resolution":{"id":id,"state":state,"basis":request.basis}});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        let found = sqlx::query("SELECT id FROM reservations WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        if found.is_none() {
            return Err(LedgerError::ReservationNotFound);
        }
        let reservation = reservation_in(&mut tx, id).await?;
        if reservation.state != "reserved" {
            return Err(LedgerError::ReservationClosed);
        }
        let resource = resource_in(&mut tx, reservation.resource).await?;
        lock_accounts(&mut tx, &[reservation.resident, resource.producer]).await?;
        sqlx::query("SELECT id FROM resources WHERE id = $1 FOR UPDATE")
            .bind(resource.id)
            .execute(&mut *tx)
            .await?;
        let transaction_id = if settle { Some(Uuid::new_v4()) } else { None };
        // Release the hold inside the same transaction as payment. Neither change
        // becomes visible alone; the deferred DB constraint matches both postings.
        sqlx::query("INSERT INTO reservation_outcomes(reservation_id,state,transaction_id,basis) VALUES ($1,$2,$3,$4)")
            .bind(id).bind(state).bind(transaction_id).bind(&request.basis).execute(&mut *tx).await?;
        if let Some(transaction_id) = transaction_id {
            let transfer = Transfer {
                from: reservation.resident,
                to: resource.producer,
                amount: Amount::parse(&reservation.amount).map_err(|_| LedgerError::Overflow)?,
            };
            post(
                &mut tx,
                transaction_id,
                key,
                "settlement",
                &payload,
                &transfer,
            )
            .await?;
        }
        let result = reservation_in(&mut tx, id).await?;
        finish(tx, key, &payload, result).await
    }
}

pub(crate) async fn resource_in(tx: &mut Tx<'_>, id: Uuid) -> Result<Resource, LedgerError> {
    let row = sqlx::query("SELECT producer_id,category,unit,unit_measure::text,(initial_quantity-consumed)::text AS on_hand,reserved::text,(initial_quantity-consumed-reserved)::text AS available FROM resource_totals WHERE id = $1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ResourceNotFound)?;
    Ok(Resource {
        id,
        producer: row.try_get("producer_id")?,
        category: row.try_get("category")?,
        unit: row.try_get("unit")?,
        unit_measure: row.try_get("unit_measure")?,
        on_hand: row.try_get("on_hand")?,
        reserved: row.try_get("reserved")?,
        available: row.try_get("available")?,
    })
}

async fn reservation_in(tx: &mut Tx<'_>, id: Uuid) -> Result<Reservation, LedgerError> {
    let row = sqlx::query("SELECT r.resident_id,r.resource_id,r.quantity::text,r.amount::text,COALESCE(o.state,'reserved') AS state,o.transaction_id FROM reservations r LEFT JOIN reservation_outcomes o ON o.reservation_id = r.id WHERE r.id = $1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::ReservationNotFound)?;
    Ok(Reservation {
        id,
        resident: row.try_get("resident_id")?,
        resource: row.try_get("resource_id")?,
        quantity: row.try_get("quantity")?,
        amount: row.try_get("amount")?,
        state: row.try_get("state")?,
        transaction_id: row.try_get("transaction_id")?,
    })
}

pub(crate) async fn reserve_in(
    tx: &mut Tx<'_>,
    request: ReservationRequest,
) -> Result<Reservation, LedgerError> {
    let resource = resource_in(tx, request.resource).await?;
    let accounts = lock_accounts(tx, &[request.resident, resource.producer]).await?;
    if accounts[&request.resident] != "resident" || accounts[&resource.producer] != "producer" {
        return Err(LedgerError::Invalid(
            "reservation requires resident and producer",
        ));
    }
    sqlx::query("SELECT id FROM resources WHERE id = $1 FOR UPDATE")
        .bind(request.resource)
        .execute(&mut **tx)
        .await?;
    let resource = resource_in(tx, request.resource).await?;
    let measure = resource
        .unit_measure
        .parse::<i64>()
        .map_err(|_| LedgerError::Overflow)?;
    let amount = measure
        .checked_mul(request.quantity.units())
        .ok_or(LedgerError::Overflow)?;
    if resource
        .available
        .parse::<i64>()
        .map_err(|_| LedgerError::Overflow)?
        < request.quantity.units()
    {
        return Err(LedgerError::InsufficientStock);
    }
    if available(tx, request.resident).await? < amount {
        return Err(LedgerError::InsufficientFunds);
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO reservations(id,resident_id,resource_id,quantity,amount,basis) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(id).bind(request.resident).bind(request.resource).bind(request.quantity.units()).bind(amount).bind(&request.basis).execute(&mut **tx).await?;
    reservation_in(tx, id).await
}
