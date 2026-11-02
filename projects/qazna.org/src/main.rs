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
    if !matches!(command.as_str(), "serve" | "migrate") {
        return Err("usage: qazna [serve|migrate]".into());
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
    let token =
        std::env::var("QAZNA_OPERATOR_TOKEN").map_err(|_| "QAZNA_OPERATOR_TOKEN is required")?;
    if token.len() < 32 || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("operator token must contain at least 32 printable ASCII bytes".into());
    }
    let address: SocketAddr = std::env::var("QAZNA_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    if !address.ip().is_loopback() {
        return Err(
            "this milestone supports loopback only; public serving requires end-user authorization"
                .into(),
        );
    }
    let ledger = Ledger::new(pool.clone());
    ledger.ready().await?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "Qazna listening");
    axum::serve(listener, api::router(ledger, token))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    pool.close().await;
    Ok(())
}
