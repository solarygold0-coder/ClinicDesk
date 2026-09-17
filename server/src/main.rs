mod auth;
mod config;
mod routes;
mod state;

use std::sync::Arc;

use axum_server::tls_rustls::RustlsConfig;
use chrono::Utc;
use config::Config;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "clinicdesk_server=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env().map_err(std::io::Error::other)?;
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState {
        pool,
        server_token: Arc::new(config.server_token.clone()),
        started_at: Utc::now(),
    };
    let app = routes::router(state);
    let tls = RustlsConfig::from_pem_file(&config.tls_cert, &config.tls_key).await?;

    tracing::info!(bind = %config.bind, "ClinicDesk Server listening with TLS");
    axum_server::bind_rustls(config.bind, tls)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
