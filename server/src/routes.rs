use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use chrono::Utc;
use serde::Serialize;

use crate::{auth::require_server_token, state::AppState};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    database: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerInfo {
    service: &'static str,
    version: &'static str,
    database: &'static str,
    started_at: String,
    server_time: String,
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(HealthResponse {
                status: "ok",
                service: "clinicdesk-server",
                version: env!("CARGO_PKG_VERSION"),
                database: "postgresql",
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "degraded",
                service: "clinicdesk-server",
                version: env!("CARGO_PKG_VERSION"),
                database: "postgresql",
            }),
        )
            .into_response(),
    }
}

async fn server_info(State(state): State<AppState>) -> Json<ServerInfo> {
    Json(ServerInfo {
        service: "clinicdesk-server",
        version: env!("CARGO_PKG_VERSION"),
        database: "postgresql",
        started_at: state.started_at.to_rfc3339(),
        server_time: Utc::now().to_rfc3339(),
    })
}

pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/server/info", get(server_info))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_server_token,
        ));

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1", protected)
        .with_state(state)
}
