use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub server_token: Arc<String>,
    pub started_at: DateTime<Utc>,
}
