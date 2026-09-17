use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::users;

const SESSION_HOURS: i64 = 8;
const VALID_ROLES: &[&str] = &[
    "ordinary_employee",
    "general_manager",
    "deputy_manager",
    "doctor",
    "specialist",
];
const VALID_STATUSES: &[&str] = &[
    "ACTIVE",
    "SUSPENDED",
    "CLOSED_INACTIVITY",
    "ENDED_SERVICE",
    "RETIRED",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub employee_code: String,
    pub role_type: String,
    pub account_status: String,
    pub must_change_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub token: String,
    pub expires_at: String,
    pub user: UserSummary,
}

fn required(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} مطلوب"));
    }
    Ok(value.to_string())
}

fn valid_role(role: &str) -> Result<&str, String> {
    if VALID_ROLES.contains(&role) {
        Ok(role)
    } else {
        Err("نوع المستخدم غير صالح".into())
    }
}

fn valid_status(status: &str) -> Result<&str, String> {
    if VALID_STATUSES.contains(&status) {
        Ok(status)
    } else {
        Err("حالة المستخدم غير صالحة".into())
    }
}

fn is_admin_role(role: &str) -> bool {
    matches!(role, "general_manager" | "deputy_manager")
}

fn ensure_runtime_audit_trigger(db: &Connection) -> Result<(), String> {
    db.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS audit_fill_runtime_actor
         AFTER INSERT ON audit_log
         WHEN NEW.actor_user_id IS NULL
          AND EXISTS(SELECT 1 FROM app_meta WHERE key='runtime_actor_user_id')
         BEGIN
           UPDATE audit_log
              SET actor_user_id=CAST((SELECT value FROM app_meta WHERE key='runtime_actor_user_id') AS INTEGER),
                  actor_display_name=(SELECT value FROM app_meta WHERE key='runtime_actor_display_name'),
                  actor_employee_code=(SELECT value FROM app_meta WHERE key='runtime_actor_employee_code'),
                  actor_session_id=(SELECT value FROM app_meta WHERE key='runtime_actor_session_id')
            WHERE id=NEW.id;
         END;",
    )
    .map_err(|e| e.to_string())
}

fn set_runtime_actor(db: &Connection, user: &UserSummary, session_id: &str) -> Result<(), String> {
    ensure_runtime_audit_trigger(db)?;
    for (key, value) in [
        ("runtime_actor_user_id", user.id.to_string()),
        ("runtime_actor_display_name", user.display_name.clone()),
        ("runtime_actor_employee_code", user.employee_code.clone()),
        ("runtime_actor_session_id", session_id.to_string()),
    ] {
        db.execute(
            "INSERT OR REPLACE INTO app_meta(key,value) VALUES(?1,?2)",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn clear_runtime_actor(db: &Connection, session_id: &str) -> Result<(), String> {
    let current: Option<String> = db
        .query_row(
            "SELECT value FROM app_meta WHERE key='runtime_actor_session_id'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if current.as_deref() == Some(session_id) {
        db.execute("DELETE FROM app_meta WHERE key LIKE 'runtime_actor_%'", [])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let password = required(password, "كلمة المرور")?;
    if password.chars().count() != 4 {
        return Err("كلمة المرور يجب أن تتكون من 4 خانات بالضبط".into());
    }
    let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes())
        .map_err(|_| "تعذر إنشاء بصمة كلمة المرور".to_string())?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| "تعذر إنشاء بصمة كلمة المرور".to_string())
}

fn verify_password(password: &str, encoded: &str) -> Result<bool, String> {
    let parsed = PasswordHash::new(encoded)
        .map_err(|_| "صيغة كلمة المرور قديمة وتحتاج إعادة تعيين".to_string())?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

fn summary_by_id(db: &Connection, id: i64) -> Result<UserSummary, String> {
    db.query_row(
        "SELECT id,username,display_name,employee_code,role_type,account_status,must_change_password FROM users WHERE id=?1",
        [id],
        |r| Ok(UserSummary { id:r.get(0)?, username:r.get(1)?, display_name:r.get(2)?, employee_code:r.get(3)?, role_type:r.get(4)?, account_status:r.get(5)?, must_change_password:r.get::<_,i64>(6)? != 0 }),
    ).map_err(|e| e.to_string())
}

pub fn list_users(db: &Connection) -> Result<Vec<UserSummary>, String> {
    let mut stmt = db.prepare("SELECT id,username,display_name,employee_code,role_type,account_status,must_change_password FROM users ORDER BY id").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(UserSummary {
                id: r.get(0)?,
                username: r.get(1)?,
                display_name: r.get(2)?,
                employee_code: r.get(3)?,
                role_type: r.get(4)?,
                account_status: r.get(5)?,
                must_change_password: r.get::<_, i64>(6)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn create_user(
    db: &mut Connection,
    username: String,
    display_name: String,
    password: String,
    role_type: String,
) -> Result<UserSummary, String> {
    let role = valid_role(role_type.trim())?.to_string();
    let password_hash = hash_password(&password)?;
    let created = users::create_with_role(
        db,
        users::NewUserAccount {
            username,
            display_name,
            password_hash,
            is_system_admin: Some(is_admin_role(&role)),
        },
        &role,
    )?;
    summary_by_id(db, created.id)
}

pub fn update_user(
    db: &Connection,
    id: i64,
    username: String,
    display_name: String,
    role_type: String,
) -> Result<UserSummary, String> {
    let username = required(&username, "اسم المستخدم")?;
    let display_name = required(&display_name, "اسم الموظف")?;
    let role = valid_role(role_type.trim())?.to_string();
    let changed = db.execute("UPDATE users SET username=?2,display_name=?3,role_type=?4,is_system_admin=?5,updated_at=CURRENT_TIMESTAMP WHERE id=?1", params![id, username, display_name, role, is_admin_role(&role) as i64]).map_err(|e| {
        let text=e.to_string();
        if text.contains("users.username") { "اسم المستخدم مستخدم مسبقًا".to_string() } else if text.contains("role_limit_") { "تم بلوغ الحد الأقصى لهذا الدور الوظيفي".to_string() } else { text }
    })?;
    if changed == 0 {
        return Err("المستخدم غير موجود".into());
    }
    summary_by_id(db, id)
}

pub fn set_status(
    db: &Connection,
    id: i64,
    status: String,
    reason: Option<String>,
) -> Result<(), String> {
    let status = valid_status(status.trim())?.to_string();
    let active = status == "ACTIVE";
    let changed = if active {
        db.execute("UPDATE users SET account_status='ACTIVE',is_active=1,closed_at=NULL,closed_reason=NULL,updated_at=CURRENT_TIMESTAMP WHERE id=?1", [id])
    } else {
        db.execute("UPDATE users SET account_status=?2,is_active=0,closed_at=CURRENT_TIMESTAMP,closed_reason=?3,updated_at=CURRENT_TIMESTAMP WHERE id=?1", params![id, status, reason.as_deref().map(str::trim).filter(|v| !v.is_empty())])
    }.map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("المستخدم غير موجود".into());
    }
    if !active {
        revoke_sessions(db, id)?;
    }
    Ok(())
}

pub fn reset_password(db: &Connection, id: i64, temporary_password: String) -> Result<(), String> {
    let hash = hash_password(&temporary_password)?;
    let changed = db.execute("UPDATE users SET password_hash=?2,must_change_password=1,updated_at=CURRENT_TIMESTAMP WHERE id=?1", params![id, hash]).map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("المستخدم غير موجود".into());
    }
    revoke_sessions(db, id)
}

fn revoke_sessions(db: &Connection, user_id: i64) -> Result<(), String> {
    db.execute("UPDATE auth_sessions SET revoked_at=COALESCE(revoked_at,CURRENT_TIMESTAMP) WHERE user_id=?1 AND revoked_at IS NULL", [user_id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn authenticate(
    db: &Connection,
    username: String,
    password: String,
) -> Result<AuthSession, String> {
    let username = required(&username, "اسم المستخدم")?;
    let (id, hash, is_active, status): (i64,String,i64,String) = db.query_row("SELECT id,password_hash,is_active,account_status FROM users WHERE username=?1 COLLATE NOCASE", [username], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_| "بيانات الدخول غير صحيحة".to_string())?;
    if is_active == 0 || status != "ACTIVE" {
        return Err("الحساب غير نشط".into());
    }
    let inactive: i64 = db.query_row("SELECT CASE WHEN datetime(COALESCE(last_successful_login_at,created_at)) <= datetime('now','-90 days') THEN 1 ELSE 0 END FROM users WHERE id=?1", [id], |r| r.get(0)).map_err(|e| e.to_string())?;
    if inactive != 0 {
        set_status(
            db,
            id,
            "CLOSED_INACTIVITY".into(),
            Some("90_day_inactivity".into()),
        )?;
        return Err("الحساب مغلق بسبب عدم النشاط ويحتاج إعادة تفعيل".into());
    }
    if !verify_password(&password, &hash)? {
        return Err("بيانات الدخول غير صحيحة".into());
    }
    db.execute("UPDATE users SET last_successful_login_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE id=?1", [id]).map_err(|e| e.to_string())?;
    let token = Uuid::new_v4().to_string();
    let expires_at = (Utc::now() + Duration::hours(SESSION_HOURS))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    db.execute(
        "INSERT INTO auth_sessions(id,user_id,expires_at) VALUES(?1,?2,?3)",
        params![token, id, expires_at],
    )
    .map_err(|e| e.to_string())?;
    let user = summary_by_id(db, id)?;
    set_runtime_actor(db, &user, &token)?;
    Ok(AuthSession {
        token,
        expires_at,
        user,
    })
}

pub fn validate_session(db: &Connection, token: String) -> Result<UserSummary, String> {
    let token = required(&token, "رمز الجلسة")?;
    let id: i64 = db.query_row("SELECT s.user_id FROM auth_sessions s JOIN users u ON u.id=s.user_id WHERE s.id=?1 AND s.revoked_at IS NULL AND datetime(s.expires_at)>datetime('now') AND u.is_active=1 AND u.account_status='ACTIVE'", [&token], |r| r.get(0)).map_err(|_| "الجلسة غير صالحة أو منتهية".to_string())?;
    let user = summary_by_id(db, id)?;
    set_runtime_actor(db, &user, &token)?;
    Ok(user)
}

pub fn logout(db: &Connection, token: String) -> Result<(), String> {
    let token = required(&token, "رمز الجلسة")?;
    db.execute(
        "UPDATE auth_sessions SET revoked_at=COALESCE(revoked_at,CURRENT_TIMESTAMP) WHERE id=?1",
        [&token],
    )
    .map_err(|e| e.to_string())?;
    clear_runtime_actor(db, &token)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE app_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE audit_log(id INTEGER PRIMARY KEY,event_type TEXT NOT NULL,entity_type TEXT NOT NULL,entity_id INTEGER,details_json TEXT,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,actor_user_id INTEGER,actor_display_name TEXT,actor_employee_code TEXT,actor_session_id TEXT,before_json TEXT,after_json TEXT,reason TEXT); CREATE TABLE users(id INTEGER PRIMARY KEY,username TEXT NOT NULL COLLATE NOCASE UNIQUE,display_name TEXT NOT NULL,password_hash TEXT NOT NULL,is_active INTEGER NOT NULL DEFAULT 1,is_system_admin INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,employee_code TEXT NOT NULL UNIQUE,role_type TEXT NOT NULL DEFAULT 'ordinary_employee',account_status TEXT NOT NULL DEFAULT 'ACTIVE',last_successful_login_at TEXT,closed_at TEXT,closed_reason TEXT,must_change_password INTEGER NOT NULL DEFAULT 0); CREATE TABLE identity_sequences(name TEXT PRIMARY KEY,next_value INTEGER NOT NULL); INSERT INTO identity_sequences VALUES('employee_code',1); CREATE TABLE auth_sessions(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,expires_at TEXT NOT NULL,revoked_at TEXT);").unwrap();
        db
    }
    #[test]
    fn hashes_and_authenticates_without_plaintext() {
        let mut db = db();
        let u = create_user(
            &mut db,
            "gm".into(),
            "مدير".into(),
            "StrongPassword1".into(),
            "general_manager".into(),
        )
        .unwrap();
        let stored: String = db
            .query_row("SELECT password_hash FROM users WHERE id=?1", [u.id], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(stored.starts_with("$argon2"));
        assert!(!stored.contains("StrongPassword1"));
        let s = authenticate(&db, "GM".into(), "StrongPassword1".into()).unwrap();
        assert_eq!(validate_session(&db, s.token).unwrap().employee_code, "U01");
    }
    #[test]
    fn login_context_stamps_following_audit_events() {
        let mut db = db();
        let user = create_user(
            &mut db,
            "auditor".into(),
            "موظف تدقيق".into(),
            "StrongPassword1".into(),
            "general_manager".into(),
        )
        .unwrap();
        let session = authenticate(&db, "auditor".into(), "StrongPassword1".into()).unwrap();
        db.execute(
            "INSERT INTO audit_log(event_type,entity_type) VALUES('test','patient')",
            [],
        )
        .unwrap();
        let actor: (i64, String, String) = db
            .query_row(
                "SELECT actor_user_id,actor_employee_code,actor_session_id FROM audit_log ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(actor.0, user.id);
        assert_eq!(actor.1, user.employee_code);
        assert_eq!(actor.2, session.token);
    }
    #[test]
    fn wrong_password_is_rejected() {
        let mut db = db();
        create_user(
            &mut db,
            "user".into(),
            "موظف".into(),
            "StrongPassword1".into(),
            "ordinary_employee".into(),
        )
        .unwrap();
        assert!(authenticate(&db, "user".into(), "wrong".into()).is_err());
    }
    #[test]
    fn reset_revokes_sessions_and_forces_change() {
        let mut db = db();
        let u = create_user(
            &mut db,
            "user".into(),
            "موظف".into(),
            "StrongPassword1".into(),
            "ordinary_employee".into(),
        )
        .unwrap();
        let s = authenticate(&db, "user".into(), "StrongPassword1".into()).unwrap();
        reset_password(&db, u.id, "TemporaryPassword2".into()).unwrap();
        assert!(validate_session(&db, s.token).is_err());
        let s2 = authenticate(&db, "user".into(), "TemporaryPassword2".into()).unwrap();
        assert!(s2.user.must_change_password);
    }
    #[test]
    fn non_active_status_revokes_session() {
        let mut db = db();
        let u = create_user(
            &mut db,
            "user".into(),
            "موظف".into(),
            "StrongPassword1".into(),
            "ordinary_employee".into(),
        )
        .unwrap();
        let s = authenticate(&db, "user".into(), "StrongPassword1".into()).unwrap();
        set_status(&db, u.id, "RETIRED".into(), Some("service_end".into())).unwrap();
        assert!(validate_session(&db, s.token).is_err());
    }
}
