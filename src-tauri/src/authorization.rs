use rusqlite::Connection;

use crate::auth::{self, UserSummary};

pub const APPOINTMENT_READ: &str = "appointment.read";
pub const APPOINTMENT_WRITE: &str = "appointment.write";
pub const PATIENT_READ: &str = "patient.read";
pub const PATIENT_WRITE: &str = "patient.write";
pub const DIRECTORY_READ: &str = "directory.read";
pub const DIRECTORY_WRITE: &str = "directory.write";
pub const VISIT_READ: &str = "visit.read";
pub const VISIT_WRITE: &str = "visit.write";
pub const ATTACHMENT_READ: &str = "attachment.read";
pub const ATTACHMENT_WRITE: &str = "attachment.write";
pub const SCHEDULE_READ: &str = "schedule.read";
pub const SCHEDULE_WRITE: &str = "schedule.write";
pub const AUDIT_READ: &str = "audit.read";
pub const USER_MANAGE: &str = "user.manage";
pub const BACKUP_CREATE: &str = "backup.create";
pub const BACKUP_RESTORE: &str = "backup.restore";

pub fn auth_enabled(db: &Connection) -> Result<bool, String> {
    db.query_row(
        "SELECT auth_enabled FROM security_settings WHERE id=1",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .map_err(|e| e.to_string())
}

fn role_allows(role: &str, capability: &str) -> bool {
    match role {
        "general_manager" => true,
        "deputy_manager" => capability != BACKUP_RESTORE,
        "ordinary_employee" => matches!(
            capability,
            APPOINTMENT_READ
                | APPOINTMENT_WRITE
                | PATIENT_READ
                | PATIENT_WRITE
                | DIRECTORY_READ
                | DIRECTORY_WRITE
                | VISIT_READ
                | VISIT_WRITE
                | ATTACHMENT_READ
                | ATTACHMENT_WRITE
                | SCHEDULE_READ
        ),
        "doctor" | "specialist" => capability == APPOINTMENT_READ,
        _ => false,
    }
}

pub fn authorize(
    db: &Connection,
    token: Option<&str>,
    capability: &str,
) -> Result<Option<UserSummary>, String> {
    if !auth_enabled(db)? {
        return Ok(None);
    }
    let token = token
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "تسجيل الدخول مطلوب لتنفيذ هذه العملية".to_string())?;
    let user = auth::validate_session(db, token.to_string())?;
    if !role_allows(&user.role_type, capability) {
        return Err("ليس لديك صلاحية لتنفيذ هذه العملية".to_string());
    }
    Ok(Some(user))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth;
    use rusqlite::Connection;

    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE security_settings(id INTEGER PRIMARY KEY CHECK(id=1),auth_enabled INTEGER NOT NULL DEFAULT 0);\
             INSERT INTO security_settings(id,auth_enabled) VALUES(1,0);\
             CREATE TABLE users(id INTEGER PRIMARY KEY,username TEXT NOT NULL COLLATE NOCASE UNIQUE,display_name TEXT NOT NULL,password_hash TEXT NOT NULL,is_active INTEGER NOT NULL DEFAULT 1,is_system_admin INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,employee_code TEXT NOT NULL UNIQUE,role_type TEXT NOT NULL DEFAULT 'ordinary_employee',account_status TEXT NOT NULL DEFAULT 'ACTIVE',last_successful_login_at TEXT,closed_at TEXT,closed_reason TEXT,must_change_password INTEGER NOT NULL DEFAULT 0);\
             CREATE TABLE identity_sequences(name TEXT PRIMARY KEY,next_value INTEGER NOT NULL);\
             INSERT INTO identity_sequences VALUES('employee_code',1);\
             CREATE TABLE auth_sessions(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,expires_at TEXT NOT NULL,revoked_at TEXT);",
        )
        .unwrap();
        db
    }

    fn session(db: &mut Connection, n: i64, role: &str) -> String {
        let username = format!("user{n}");
        let password = format!("Password-{n}-Strong");
        auth::create_user(
            db,
            username.clone(),
            format!("مستخدم {n}"),
            password.clone(),
            role.to_string(),
        )
        .unwrap();
        auth::authenticate(db, username, password).unwrap().token
    }

    #[test]
    fn local_mode_allows_operations_without_fake_user() {
        let db = db();
        assert!(authorize(&db, None, BACKUP_RESTORE).unwrap().is_none());
    }

    #[test]
    fn doctor_and_specialist_are_appointment_read_only() {
        for role in ["doctor", "specialist"] {
            let mut db = db();
            let token = session(&mut db, 1, role);
            db.execute("UPDATE security_settings SET auth_enabled=1 WHERE id=1", [])
                .unwrap();
            assert!(authorize(&db, Some(&token), APPOINTMENT_READ).is_ok());
            assert!(authorize(&db, Some(&token), APPOINTMENT_WRITE).is_err());
            assert!(authorize(&db, Some(&token), PATIENT_READ).is_err());
            assert!(authorize(&db, Some(&token), ATTACHMENT_READ).is_err());
        }
    }

    #[test]
    fn ordinary_employee_can_operate_but_cannot_administer_security() {
        let mut db = db();
        let token = session(&mut db, 1, "ordinary_employee");
        db.execute("UPDATE security_settings SET auth_enabled=1 WHERE id=1", [])
            .unwrap();
        assert!(authorize(&db, Some(&token), PATIENT_WRITE).is_ok());
        assert!(authorize(&db, Some(&token), APPOINTMENT_WRITE).is_ok());
        assert!(authorize(&db, Some(&token), USER_MANAGE).is_err());
        assert!(authorize(&db, Some(&token), BACKUP_CREATE).is_err());
    }

    #[test]
    fn manager_and_deputy_backup_policy_is_enforced() {
        let mut gm_db = db();
        let gm = session(&mut gm_db, 1, "general_manager");
        gm_db
            .execute("UPDATE security_settings SET auth_enabled=1 WHERE id=1", [])
            .unwrap();
        assert!(authorize(&gm_db, Some(&gm), USER_MANAGE).is_ok());
        assert!(authorize(&gm_db, Some(&gm), BACKUP_CREATE).is_ok());
        assert!(authorize(&gm_db, Some(&gm), BACKUP_RESTORE).is_ok());

        let mut deputy_db = db();
        let deputy = session(&mut deputy_db, 1, "deputy_manager");
        deputy_db
            .execute("UPDATE security_settings SET auth_enabled=1 WHERE id=1", [])
            .unwrap();
        assert!(authorize(&deputy_db, Some(&deputy), USER_MANAGE).is_ok());
        assert!(authorize(&deputy_db, Some(&deputy), BACKUP_CREATE).is_ok());
        assert!(authorize(&deputy_db, Some(&deputy), BACKUP_RESTORE).is_err());
    }

    #[test]
    fn enabled_auth_rejects_missing_or_revoked_session() {
        let mut db = db();
        let token = session(&mut db, 1, "general_manager");
        db.execute("UPDATE security_settings SET auth_enabled=1 WHERE id=1", [])
            .unwrap();
        assert!(authorize(&db, None, USER_MANAGE).is_err());
        auth::logout(&db, token.clone()).unwrap();
        assert!(authorize(&db, Some(&token), USER_MANAGE).is_err());
    }
}
