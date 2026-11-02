use crate::{
    coordination::{MonitorRequest, ReviewRequest},
    domain::{NewAccount, Transfer},
    drafts::DraftRequest,
    ledger::{Ledger, LedgerError},
    needs::{NeedRequest, NeedReservationRequest, PeriodQuery, ProviderRequest, UnitRequest},
    planning::{NeedNorm, NormNeedRequest, PlanningRequest},
    production::{CapacityRequest, CompletionRequest, PlanRequest, RecipeRequest},
    provisioning::{AllocationRequest, ReservationRequest, ResolutionRequest, ResourceRequest},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// A local operator credential, not end-user authorization. Keep behind loopback
/// until OIDC, role and ownership checks are implemented.
pub fn router(ledger: Ledger, token: String) -> Router {
    let private = Router::new()
        .route("/v1/accounts", post(create_account))
        .route("/v1/accounts/{id}", get(account))
        .route("/v1/transfers", post(transfer))
        .route("/v1/allocations", post(allocate))
        .route("/v1/resources", post(create_resource))
        .route("/v1/resources/{id}", get(resource))
        .route("/v1/reservations", post(reserve))
        .route("/v1/reservations/{id}", get(reservation))
        .route("/v1/reservations/{id}/settle", post(settle))
        .route("/v1/reservations/{id}/cancel", post(cancel))
        .route("/v1/units", post(create_unit))
        .route("/v1/units/{id}", get(unit))
        .route("/v1/units/{id}/providers", post(link_provider))
        .route("/v1/units/{id}/coverage", get(unit_coverage))
        .route("/v1/needs", post(create_need))
        .route("/v1/needs/{id}", get(need))
        .route("/v1/needs/{id}/reserve", post(reserve_need))
        .route("/v1/needs/{id}/attempts", get(need_attempts))
        .route("/v1/production/capacities", post(create_capacity))
        .route("/v1/production/capacities/{id}", get(capacity))
        .route("/v1/production/recipes", post(create_recipe))
        .route("/v1/production/recipes/{id}", get(recipe))
        .route("/v1/production/assess", post(assess_production))
        .route("/v1/production/plans", post(plan_production))
        .route("/v1/production/plans/{id}", get(production_plan))
        .route(
            "/v1/production/plans/{id}/complete",
            post(complete_production),
        )
        .route(
            "/v1/production/plans/{id}/completion",
            get(production_completion),
        )
        .route("/v1/resources/{id}/provenance", get(resource_provenance))
        .route("/v1/production/plans/{id}/cancel", post(cancel_production))
        .route("/v1/need-norms", post(create_need_norm))
        .route("/v1/need-norms/{id}", get(need_norm))
        .route("/v1/needs/from-norm", post(create_need_from_norm))
        .route("/v1/needs/{id}/norm", get(need_norm_source))
        .route("/v1/planning/assess", post(propose_provisioning))
        .route("/v1/planning/drafts", post(create_planning_draft))
        .route("/v1/planning/drafts/{id}", get(planning_draft))
        .route("/v1/planning/drafts/{id}/apply", post(apply_planning_draft))
        .route(
            "/v1/planning/drafts/{id}/discard",
            post(discard_planning_draft),
        )
        .route("/v1/coordination/monitors", post(create_monitor))
        .route("/v1/coordination/monitors/{id}", get(monitor))
        .route("/v1/coordination/monitors/{id}/stop", post(stop_monitor))
        .route("/v1/coordination/results/{id}", get(coordination_result))
        .route(
            "/v1/coordination/results/{id}/reviews",
            get(ai_reviews).post(record_ai_review),
        )
        .route_layer(middleware::from_fn_with_state(token, authorize));
    Router::new()
        .route(
            "/health/live",
            get(|| async { Json(json!({"status": "live"})) }),
        )
        .route("/health/ready", get(ready))
        .merge(private)
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(ledger)
}

async fn authorize(State(token): State<String>, request: Request, next: Next) -> Response {
    let authorized = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|v| constant_time_equal(v.as_bytes(), token.as_bytes()));
    if !authorized {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "unauthorized"})),
        )
            .into_response();
    }
    next.run(request).await
}

fn constant_time_equal(a: &[u8], b: &[u8]) -> bool {
    bool::from(a.ct_eq(b))
}

async fn ready(State(ledger): State<Ledger>) -> Result<Json<serde_json::Value>, LedgerError> {
    ledger.ready().await?;
    Ok(Json(json!({"status": "ready"})))
}

async fn create_account(
    State(ledger): State<Ledger>,
    Json(request): Json<NewAccount>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok((
        StatusCode::CREATED,
        Json(ledger.create_account(request).await?),
    ))
}

async fn account(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.account(id).await?))
}

async fn transfer(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<Transfer>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger.transfer(command_key(&headers)?, request).await?,
    ))
}

fn command_key(headers: &HeaderMap) -> Result<&str, LedgerError> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(LedgerError::Invalid("Idempotency-Key header is required"))
}

async fn allocate(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<AllocationRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger.allocate(command_key(&headers)?, request).await?,
    ))
}
async fn create_resource(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<ResourceRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .create_resource(command_key(&headers)?, request)
            .await?,
    ))
}
async fn resource(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.resource(id).await?))
}
async fn reserve(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<ReservationRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.reserve(command_key(&headers)?, request).await?))
}
async fn reservation(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.reservation(id).await?))
}
async fn settle(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ResolutionRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger.settle(command_key(&headers)?, id, request).await?,
    ))
}
async fn cancel(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ResolutionRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger.cancel(command_key(&headers)?, id, request).await?,
    ))
}

async fn create_unit(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<UnitRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger.create_unit(command_key(&headers)?, request).await?,
    ))
}
async fn unit(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.unit(id).await?))
}
async fn link_provider(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ProviderRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .link_provider(command_key(&headers)?, id, request)
            .await?,
    ))
}
async fn unit_coverage(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    Query(query): Query<PeriodQuery>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.unit_coverage(id, &query.period).await?))
}
async fn create_need(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<NeedRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger.create_need(command_key(&headers)?, request).await?,
    ))
}
async fn need(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.need(id).await?))
}
async fn reserve_need(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<NeedReservationRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .reserve_need(command_key(&headers)?, id, request)
            .await?,
    ))
}
async fn need_attempts(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.need_attempts(id).await?))
}

impl IntoResponse for LedgerError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Self::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::NotFound => (StatusCode::NOT_FOUND, "account_not_found"),
            Self::AccountConflict => (StatusCode::CONFLICT, "account_exists"),
            Self::IdempotencyConflict => (StatusCode::CONFLICT, "idempotency_conflict"),
            Self::InsufficientFunds => (StatusCode::CONFLICT, "insufficient_funds"),
            Self::Overflow => (StatusCode::CONFLICT, "balance_limit"),
            Self::AllocationConflict => (StatusCode::CONFLICT, "allocation_exists"),
            Self::ResourceConflict => (StatusCode::CONFLICT, "resource_exists"),
            Self::ResourceNotFound => (StatusCode::NOT_FOUND, "resource_not_found"),
            Self::ReservationNotFound => (StatusCode::NOT_FOUND, "reservation_not_found"),
            Self::InsufficientStock => (StatusCode::CONFLICT, "insufficient_stock"),
            Self::ReservationClosed => (StatusCode::CONFLICT, "reservation_closed"),
            Self::UnitNotFound => (StatusCode::NOT_FOUND, "unit_not_found"),
            Self::UnitConflict => (StatusCode::CONFLICT, "unit_exists"),
            Self::ProviderConflict => (StatusCode::CONFLICT, "provider_already_assigned"),
            Self::ProviderOutOfScope => (StatusCode::CONFLICT, "provider_outside_scope"),
            Self::NeedNotFound => (StatusCode::NOT_FOUND, "need_not_found"),
            Self::NeedConflict => (StatusCode::CONFLICT, "need_exists"),
            Self::ProductionNotFound => (StatusCode::NOT_FOUND, "production_not_found"),
            Self::ProductionConflict => (StatusCode::CONFLICT, "production_exists"),
            Self::ProductionClosed => (StatusCode::CONFLICT, "production_closed"),
            Self::NormNotFound => (StatusCode::NOT_FOUND, "norm_not_found"),
            Self::NormConflict => (StatusCode::CONFLICT, "norm_revision_conflict"),
            Self::DraftNotFound => (StatusCode::NOT_FOUND, "draft_not_found"),
            Self::DraftConflict => (StatusCode::CONFLICT, "draft_exists"),
            Self::DraftClosed => (StatusCode::CONFLICT, "draft_closed"),
            Self::DraftStale => (StatusCode::CONFLICT, "draft_stale"),
            Self::DraftEmpty => (StatusCode::CONFLICT, "draft_empty"),
            Self::MonitorNotFound => (StatusCode::NOT_FOUND, "monitor_not_found"),
            Self::MonitorConflict => (StatusCode::CONFLICT, "monitor_exists"),
            Self::MonitorStopped => (StatusCode::CONFLICT, "monitor_stopped"),
            Self::CalculationNotFound => (StatusCode::NOT_FOUND, "calculation_not_found"),
            Self::CalculationFailed => (StatusCode::SERVICE_UNAVAILABLE, "calculation_failed"),
            Self::CalculationLeaseLost => (StatusCode::CONFLICT, "calculation_lease_lost"),
            Self::ReviewConflict => (StatusCode::CONFLICT, "review_exists"),
            Self::InvalidStoredResponse => {
                (StatusCode::INTERNAL_SERVER_ERROR, "invalid_stored_response")
            }
            Self::Database(error) => {
                tracing::error!(error = %error, "database request failed");
                (StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable")
            }
        };
        (status, Json(json!({"error": code}))).into_response()
    }
}

async fn create_capacity(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<CapacityRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .create_capacity(command_key(&headers)?, request)
            .await?,
    ))
}

async fn create_recipe(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<RecipeRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .create_recipe(command_key(&headers)?, request)
            .await?,
    ))
}

async fn plan_production(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<PlanRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .plan_production(command_key(&headers)?, request)
            .await?,
    ))
}

async fn capacity(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.capacity(id).await?))
}

async fn recipe(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.recipe(id).await?))
}

async fn production_plan(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.production_plan(id).await?))
}

async fn assess_production(
    State(ledger): State<Ledger>,
    Json(request): Json<PlanRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.assess_production(request).await?))
}
async fn cancel_production(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ResolutionRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .cancel_production(command_key(&headers)?, id, request)
            .await?,
    ))
}

async fn complete_production(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CompletionRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .complete_production(command_key(&headers)?, id, request)
            .await?,
    ))
}
async fn production_completion(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.production_completion(id).await?))
}
async fn resource_provenance(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.resource_provenance(id).await?))
}

async fn create_need_norm(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<NeedNorm>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .create_need_norm(command_key(&headers)?, request)
            .await?,
    ))
}

async fn create_need_from_norm(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<NormNeedRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .create_need_from_norm(command_key(&headers)?, request)
            .await?,
    ))
}

async fn need_norm(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.need_norm(id).await?))
}

async fn need_norm_source(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.need_norm_source(id).await?))
}

async fn propose_provisioning(
    State(ledger): State<Ledger>,
    Json(request): Json<PlanningRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.propose_provisioning(request).await?))
}

async fn create_planning_draft(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<DraftRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .create_planning_draft(command_key(&headers)?, request)
            .await?,
    ))
}
async fn planning_draft(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.planning_draft(id).await?))
}
async fn apply_planning_draft(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ResolutionRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .apply_planning_draft(command_key(&headers)?, id, request)
            .await?,
    ))
}
async fn discard_planning_draft(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ResolutionRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .discard_planning_draft(command_key(&headers)?, id, request)
            .await?,
    ))
}

async fn create_monitor(
    State(ledger): State<Ledger>,
    headers: HeaderMap,
    Json(request): Json<MonitorRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .create_monitor(command_key(&headers)?, request)
            .await?,
    ))
}
async fn monitor(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.monitor(id).await?))
}
async fn stop_monitor(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ResolutionRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .stop_monitor(command_key(&headers)?, id, request)
            .await?,
    ))
}
async fn coordination_result(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.coordination_result(id).await?))
}
async fn record_ai_review(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ReviewRequest>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(
        ledger
            .record_ai_review(command_key(&headers)?, id, request)
            .await?,
    ))
}
async fn ai_reviews(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, LedgerError> {
    Ok(Json(ledger.ai_reviews(id).await?))
}
