use qazna::{
    api,
    ledger::{Ledger, MIGRATOR},
};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "qazna=info".into()),
        )
        .init();
    let command = std::env::args().nth(1).unwrap_or_else(|| "serve".into());
    if !matches!(
        command.as_str(),
        "serve" | "migrate" | "coordinate" | "coordinate-once" | "enforce-evidence"
    ) {
        return Err(
            "usage: qazna [serve|migrate|coordinate|coordinate-once|enforce-evidence]".into(),
        );
    }
    let url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await?;
    if command == "migrate" {
        MIGRATOR.run(&pool).await?;
        tracing::info!("migrations complete");
        pool.close().await;
        return Ok(());
    }
    let ledger = Ledger::new(pool.clone());
    ledger.ready().await?;
    if command == "enforce-evidence" {
        let basis = std::env::var("QAZNA_EVIDENCE_BASIS")
            .map_err(|_| "QAZNA_EVIDENCE_BASIS is required")?;
        ledger.enable_evidence(&basis).await?;
        tracing::info!("evidence enforcement enabled; independent attestations now required");
        pool.close().await;
        return Ok(());
    }
    if command == "coordinate-once" {
        println!(
            "{}",
            serde_json::to_string(&ledger.coordinate_once().await?)?
        );
        pool.close().await;
        return Ok(());
    }
    if command == "coordinate" {
        tracing::info!("coordination worker started; operator approval remains required");
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                result = ledger.coordinate_once() => {
                    match result {
                        Ok(Some(r)) => tracing::info!(job=%r.job, draft=%r.draft, changed=r.changed, "calculation completed"),
                        Ok(None) => {},
                        Err(error) => tracing::warn!(%error, "calculation did not complete"),
                    }
                }
            }
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                _ = tokio::time::sleep(Duration::from_millis(500)) => {},
            }
        }
        pool.close().await;
        return Ok(());
    }
    let token =
        std::env::var("QAZNA_OPERATOR_TOKEN").map_err(|_| "QAZNA_OPERATOR_TOKEN is required")?;
    if token.len() < 32 || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("operator token must contain at least 32 printable ASCII bytes".into());
    }
    let tokens = api::AccessTokens::new(
        token,
        std::env::var("QAZNA_OBSERVER_TOKEN").ok(),
        std::env::var("QAZNA_REVIEW_TOKEN").ok(),
    )?;
    let attestors: Vec<api::AttestorCredential> = serde_json::from_str(
        &std::env::var("QAZNA_ATTESTORS_JSON").unwrap_or_else(|_| "[]".into()),
    )
    .map_err(|_| "invalid QAZNA_ATTESTORS_JSON")?;
    let tokens = tokens.with_attestors(attestors)?;
    let address: SocketAddr = std::env::var("QAZNA_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    if !address.ip().is_loopback() {
        return Err(
            "this milestone supports loopback only; public serving requires end-user authorization"
                .into(),
        );
    }
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "Qazna listening");
    axum::serve(listener, api::router_with_access(ledger, tokens))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    pool.close().await;
    Ok(())
}
