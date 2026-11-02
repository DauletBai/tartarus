use crate::{
    ledger::{Ledger, LedgerError, Tx, available, decode, finish, lock_accounts},
    needs::*,
    provisioning::{ReservationRequest, valid_period, valid_text},
    provisioning_service::{reserve_in, resource_in},
};
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

impl Ledger {
    pub async fn create_unit(&self, key: &str, request: UnitRequest) -> Result<Unit, LedgerError> {
        if !valid_text(&request.name, 120)
            || !valid_text(&request.function, 500)
            || !valid_text(&request.basis, 1000)
            || request.parent == Some(request.id)
        {
            return Err(LedgerError::Invalid(
                "unit name, function, basis and distinct parent required",
            ));
        }
        let payload = serde_json::json!({"version":1,"unit":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        if let Some(parent) = request.parent {
            unit_in(&mut tx, parent).await?;
            let depth: i32=sqlx::query_scalar("WITH RECURSIVE ancestors AS (SELECT id,parent_id,1 AS d FROM provisioning_units WHERE id=$1 UNION ALL SELECT u.id,u.parent_id,a.d+1 FROM provisioning_units u JOIN ancestors a ON u.id=a.parent_id) SELECT max(d) FROM ancestors")
                .bind(parent).fetch_one(&mut *tx).await?;
            if depth >= 64 {
                return Err(LedgerError::Invalid("hierarchy exceeds 64 levels"));
            }
        }
        let inserted=sqlx::query("INSERT INTO provisioning_units(id,parent_id,name,function,basis) VALUES ($1,$2,$3,$4,$5) ON CONFLICT(id) DO NOTHING")
            .bind(request.id).bind(request.parent).bind(&request.name).bind(&request.function).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::UnitConflict);
        }
        let result = unit_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, result).await
    }

    pub async fn unit(&self, id: Uuid) -> Result<Unit, LedgerError> {
        let mut tx = self.pool.begin().await?;
        unit_in(&mut tx, id).await
    }

    pub async fn link_provider(
        &self,
        key: &str,
        id: Uuid,
        request: ProviderRequest,
    ) -> Result<ProviderLink, LedgerError> {
        if !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid("basis required"));
        }
        let payload =
            serde_json::json!({"version":1,"provider_link":{"unit":id,"request":request}});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        unit_in(&mut tx, id).await?;
        let accounts = lock_accounts(&mut tx, &[request.producer]).await?;
        if accounts[&request.producer] != "producer" {
            return Err(LedgerError::Invalid("provider must have producer account"));
        }
        let inserted=sqlx::query("INSERT INTO unit_providers(producer_id,unit_id,basis) VALUES ($1,$2,$3) ON CONFLICT(producer_id) DO NOTHING")
            .bind(request.producer).bind(id).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::ProviderConflict);
        }
        finish(
            tx,
            key,
            &payload,
            ProviderLink {
                unit: id,
                producer: request.producer,
            },
        )
        .await
    }

    pub async fn create_need(&self, key: &str, request: NeedRequest) -> Result<Need, LedgerError> {
        if !valid_period(&request.period)
            || !valid_text(&request.unit, 40)
            || !valid_text(&request.basis, 1000)
        {
            return Err(LedgerError::Invalid(
                "canonical period, unit and basis required",
            ));
        }
        let payload = serde_json::json!({"version":1,"need":request});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        unit_in(&mut tx, request.assigned_unit).await?;
        let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM accounts WHERE id=$1")
            .bind(request.resident)
            .fetch_optional(&mut *tx)
            .await?;
        if kind.ok_or(LedgerError::NotFound)? != "resident" {
            return Err(LedgerError::Invalid("need must belong to resident"));
        }
        // A need is registered without consulting balances or available stock.
        let inserted=sqlx::query("INSERT INTO needs(id,resident_id,unit_id,period,category,unit,quantity,basis) VALUES ($1,$2,$3,$4::text::date,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
            .bind(request.id).bind(request.resident).bind(request.assigned_unit).bind(&request.period)
            .bind(request.category.as_str()).bind(&request.unit).bind(request.quantity.units()).bind(&request.basis).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            return Err(LedgerError::NeedConflict);
        }
        let result = need_in(&mut tx, request.id).await?;
        finish(tx, key, &payload, result).await
    }

    pub async fn need(&self, id: Uuid) -> Result<Need, LedgerError> {
        let mut tx = self.pool.begin().await?;
        need_in(&mut tx, id).await
    }

    pub async fn need_attempts(&self, id: Uuid) -> Result<Vec<NeedAttempt>, LedgerError> {
        let mut tx = self.pool.begin().await?;
        need_in(&mut tx, id).await?;
        let rows=sqlx::query("SELECT id,need_id,resource_id,quantity::text,reservation_id,reasons,uncovered_before::text,resource_available::text,accounting_available::text,required_measure::text FROM need_attempts WHERE need_id=$1 ORDER BY created_at DESC,id DESC LIMIT 50")
            .bind(id).fetch_all(&mut *tx).await?;
        rows.iter().map(attempt_from).collect()
    }

    pub async fn reserve_need(
        &self,
        key: &str,
        id: Uuid,
        request: NeedReservationRequest,
    ) -> Result<NeedAttempt, LedgerError> {
        if !valid_text(&request.basis, 1000) {
            return Err(LedgerError::Invalid("basis required"));
        }
        let payload =
            serde_json::json!({"version":1,"need_reservation":{"need":id,"request":request}});
        let (mut tx, replay) = self.start(key, &payload).await?;
        if let Some(value) = replay {
            return decode(value);
        }
        // Coverage is serialized per need before entering the existing account/resource lock order.
        let exists = sqlx::query("SELECT id FROM needs WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Err(LedgerError::NeedNotFound);
        }
        let need = need_in(&mut tx, id).await?;
        let resource = resource_in(&mut tx, request.resource).await?;
        if resource.category != need.category || resource.unit != need.unit {
            return Err(LedgerError::Invalid(
                "resource category and physical unit must match need exactly",
            ));
        }
        lock_accounts(&mut tx, &[need.resident, resource.producer]).await?;
        let in_scope:bool=sqlx::query_scalar("WITH RECURSIVE scope AS (SELECT id FROM provisioning_units WHERE id=$1 UNION ALL SELECT u.id FROM provisioning_units u JOIN scope p ON u.parent_id=p.id) SELECT EXISTS (SELECT 1 FROM unit_providers p JOIN scope s ON p.unit_id=s.id WHERE p.producer_id=$2)")
            .bind(need.assigned_unit).bind(resource.producer).fetch_one(&mut *tx).await?;
        if !in_scope {
            return Err(LedgerError::ProviderOutOfScope);
        }
        sqlx::query("SELECT id FROM resources WHERE id=$1 FOR UPDATE")
            .bind(resource.id)
            .execute(&mut *tx)
            .await?;
        // Refresh after account/resource waits: a concurrent cancellation may have freed coverage.
        let need = need_in(&mut tx, id).await?;
        let resource = resource_in(&mut tx, request.resource).await?;
        let uncovered = integer(&need.uncovered)?;
        let physical = integer(&resource.available)?;
        let accounting = available(&mut tx, need.resident).await?;
        let required = integer(&resource.unit_measure)?.checked_mul(request.quantity.units());
        let mut reasons = Vec::<String>::new();
        if request.quantity.units() > uncovered {
            reasons.push("coverage_limit".into());
        }
        if request.quantity.units() > physical {
            reasons.push("physical_shortage".into());
        }
        match required {
            Some(value) if value > accounting => reasons.push("accounting_shortage".into()),
            None => reasons.push("quantity_limit".into()),
            _ => {}
        }
        let reservation_id = if reasons.is_empty() {
            Some(
                reserve_in(
                    &mut tx,
                    ReservationRequest {
                        resident: need.resident,
                        resource: request.resource,
                        quantity: request.quantity,
                        basis: request.basis.clone(),
                    },
                )
                .await?
                .id,
            )
        } else {
            None
        };
        let attempt_id = Uuid::new_v4();
        sqlx::query("INSERT INTO need_attempts(id,need_id,resource_id,quantity,reservation_id,reasons,uncovered_before,resource_available,accounting_available,required_measure,basis) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(attempt_id).bind(id).bind(request.resource).bind(request.quantity.units()).bind(reservation_id)
            .bind(&reasons).bind(uncovered).bind(physical).bind(accounting).bind(required).bind(&request.basis).execute(&mut *tx).await?;
        let result = NeedAttempt {
            id: attempt_id,
            need: id,
            resource: request.resource,
            quantity: request.quantity.units().to_string(),
            status: if reservation_id.is_some() {
                "reserved"
            } else {
                "blocked"
            }
            .into(),
            reservation_id,
            reasons,
            snapshot: AttemptSnapshot {
                uncovered_before: uncovered.to_string(),
                resource_available: physical.to_string(),
                accounting_available: accounting.to_string(),
                required_measure: required.map(|n| n.to_string()),
            },
        };
        // A blocked outcome is durable feedback, not a discarded technical error.
        finish(tx, key, &payload, result).await
    }

    pub async fn unit_coverage(&self, id: Uuid, period: &str) -> Result<Coverage, LedgerError> {
        if !valid_period(period) {
            return Err(LedgerError::Invalid("canonical period required"));
        }
        let mut tx = self.pool.begin().await?;
        unit_in(&mut tx, id).await?;
        let rows=sqlx::query("WITH RECURSIVE scope AS (SELECT id FROM provisioning_units WHERE id=$1 UNION ALL SELECT u.id FROM provisioning_units u JOIN scope p ON u.parent_id=p.id) SELECT n.category,n.unit,count(*) AS needs,sum(n.quantity)::text AS requested,sum(n.received)::text AS received,sum(n.reserved)::text AS reserved,sum(n.uncovered)::text AS uncovered,sum(n.unmet)::text AS unmet FROM need_totals n JOIN scope s ON n.unit_id=s.id WHERE n.period=$2::text::date GROUP BY n.category,n.unit ORDER BY n.category,n.unit")
            .bind(id).bind(period).fetch_all(&mut *tx).await?;
        let lines = rows
            .iter()
            .map(|row| {
                Ok(CoverageLine {
                    category: row.try_get("category")?,
                    unit: row.try_get("unit")?,
                    needs: row.try_get("needs")?,
                    requested: row.try_get("requested")?,
                    received: row.try_get("received")?,
                    reserved: row.try_get("reserved")?,
                    uncovered: row.try_get("uncovered")?,
                    unmet: row.try_get("unmet")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?;
        Ok(Coverage {
            unit: id,
            period: period.into(),
            lines,
        })
    }
}

fn integer(value: &str) -> Result<i64, LedgerError> {
    value.parse().map_err(|_| LedgerError::Overflow)
}

async fn unit_in(tx: &mut Tx<'_>, id: Uuid) -> Result<Unit, LedgerError> {
    let row=sqlx::query("SELECT u.parent_id,u.name,u.function,ARRAY(SELECT c.id FROM provisioning_units c WHERE c.parent_id=u.id ORDER BY c.id) AS children,ARRAY(SELECT p.producer_id FROM unit_providers p WHERE p.unit_id=u.id ORDER BY p.producer_id) AS providers FROM provisioning_units u WHERE u.id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::UnitNotFound)?;
    Ok(Unit {
        id,
        parent: row.try_get("parent_id")?,
        name: row.try_get("name")?,
        function: row.try_get("function")?,
        children: row.try_get("children")?,
        providers: row.try_get("providers")?,
    })
}

async fn need_in(tx: &mut Tx<'_>, id: Uuid) -> Result<Need, LedgerError> {
    let row=sqlx::query("SELECT resident_id,unit_id,to_char(period,'YYYY-MM-DD') AS period,category,unit,quantity::text,reserved::text,received::text,uncovered::text,unmet::text,last_attempt_id,COALESCE(last_attempt_reasons,ARRAY[]::text[]) AS reasons FROM need_totals WHERE id=$1")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(LedgerError::NeedNotFound)?;
    Ok(Need {
        id,
        resident: row.try_get("resident_id")?,
        assigned_unit: row.try_get("unit_id")?,
        period: row.try_get("period")?,
        category: row.try_get("category")?,
        unit: row.try_get("unit")?,
        requested: row.try_get("quantity")?,
        reserved: row.try_get("reserved")?,
        received: row.try_get("received")?,
        uncovered: row.try_get("uncovered")?,
        unmet: row.try_get("unmet")?,
        last_attempt_id: row.try_get("last_attempt_id")?,
        last_attempt_reasons: row.try_get("reasons")?,
    })
}

fn attempt_from(row: &PgRow) -> Result<NeedAttempt, LedgerError> {
    let reservation_id: Option<Uuid> = row.try_get("reservation_id")?;
    Ok(NeedAttempt {
        id: row.try_get("id")?,
        need: row.try_get("need_id")?,
        resource: row.try_get("resource_id")?,
        quantity: row.try_get("quantity")?,
        status: if reservation_id.is_some() {
            "reserved"
        } else {
            "blocked"
        }
        .into(),
        reservation_id,
        reasons: row.try_get("reasons")?,
        snapshot: AttemptSnapshot {
            uncovered_before: row.try_get("uncovered_before")?,
            resource_available: row.try_get("resource_available")?,
            accounting_available: row.try_get("accounting_available")?,
            required_measure: row.try_get("required_measure")?,
        },
    })
}
