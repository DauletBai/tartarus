use crate::{
    domain::{NewAccount, Transfer},
    ledger::{Ledger, LedgerError},
    provisioning::{AllocationRequest, ReservationRequest, ResolutionRequest, ResourceRequest},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
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
