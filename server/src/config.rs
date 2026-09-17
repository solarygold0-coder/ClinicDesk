use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: String,
    pub server_token: String,
    pub tls_cert: PathBuf,
    pub tls_key: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let bind = env::var("CLINICDESK_SERVER_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8443".to_string())
            .parse()
            .map_err(|_| "CLINICDESK_SERVER_BIND غير صالح".to_string())?;
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL مطلوب لتشغيل ClinicDesk Server".to_string())?;
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err("ClinicDesk Server يتطلب PostgreSQL ولا يقبل SQLite".into());
        }
        let server_token = env::var("CLINICDESK_SERVER_TOKEN")
            .map_err(|_| "CLINICDESK_SERVER_TOKEN مطلوب".to_string())?;
        if server_token.len() < 32 {
            return Err("CLINICDESK_SERVER_TOKEN يجب ألا يقل عن 32 محرفاً".into());
        }
        let tls_cert = env::var("CLINICDESK_TLS_CERT")
            .map(PathBuf::from)
            .map_err(|_| "CLINICDESK_TLS_CERT مطلوب".to_string())?;
        let tls_key = env::var("CLINICDESK_TLS_KEY")
            .map(PathBuf::from)
            .map_err(|_| "CLINICDESK_TLS_KEY مطلوب".to_string())?;
        Ok(Self { bind, database_url, server_token, tls_cert, tls_key })
    }
}
