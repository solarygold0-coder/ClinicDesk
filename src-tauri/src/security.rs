use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecurityState {
    pub auth_enabled: bool,
    pub user_count: i64,
    pub setup_required: bool,
}

pub fn state(c: &Connection) -> Result<SecurityState, String> {
    let enabled: i64 = c
        .query_row(
            "SELECT auth_enabled FROM security_settings WHERE id=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let user_count: i64 = c
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(SecurityState {
        auth_enabled: enabled == 1,
        user_count,
        setup_required: enabled == 0 && user_count == 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        c.execute_batch(include_str!("../migrations/008_optional_auth.sql"))
            .unwrap();
        c
    }

    #[test]
    fn fresh_install_keeps_auth_disabled_and_has_no_users() {
        let c = db();
        let s = state(&c).unwrap();
        assert!(!s.auth_enabled);
        assert_eq!(s.user_count, 0);
        assert!(s.setup_required);
    }

    #[test]
    fn schema_does_not_create_a_default_admin() {
        let c = db();
        let admins: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM users WHERE is_system_admin=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(admins, 0);
    }
}
