use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub const MAX_USER_ACCOUNTS: i64 = 62;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub employee_code: String,
    pub is_active: bool,
    pub is_system_admin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUserAccount {
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub is_system_admin: Option<bool>,
}

fn clean_required(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} مطلوب"));
    }
    Ok(value.to_string())
}

fn row_to_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserAccount> {
    Ok(UserAccount {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        employee_code: row.get(3)?,
        is_active: row.get::<_, i64>(4)? != 0,
        is_system_admin: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

pub fn list(db: &Connection) -> Result<Vec<UserAccount>, String> {
    let mut stmt = db
        .prepare(
            "SELECT id,username,display_name,employee_code,is_active,is_system_admin,created_at,updated_at
             FROM users ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_user).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn create(db: &mut Connection, input: NewUserAccount) -> Result<UserAccount, String> {
    let username = clean_required(&input.username, "اسم المستخدم")?;
    let display_name = clean_required(&input.display_name, "اسم الموظف")?;
    let password_hash = clean_required(&input.password_hash, "بصمة كلمة المرور")?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let total: i64 = tx
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if total >= MAX_USER_ACCOUNTS {
        return Err("تم بلوغ الحد الأقصى لحسابات النظام (62 مستخدمًا)".into());
    }
    let next: i64 = tx
        .query_row(
            "SELECT next_value FROM identity_sequences WHERE name='employee_code'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if next > MAX_USER_ACCOUNTS {
        return Err("نفدت رموز المستخدمين الدائمة U01-U62؛ الرموز المعطلة لا يعاد استخدامها".into());
    }
    let employee_code = format!("U{next:02}");
    tx.execute(
        "INSERT INTO users(username,display_name,password_hash,is_system_admin,employee_code)
         VALUES(?1,?2,?3,?4,?5)",
        params![
            username,
            display_name,
            password_hash,
            input.is_system_admin.unwrap_or(false) as i64,
            employee_code
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("users.username") {
            "اسم المستخدم مستخدم مسبقًا".to_string()
        } else {
            e.to_string()
        }
    })?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "UPDATE identity_sequences SET next_value=?1 WHERE name='employee_code' AND next_value=?2",
        params![next + 1, next],
    )
    .map_err(|e| e.to_string())?;
    let user = tx
        .query_row(
            "SELECT id,username,display_name,employee_code,is_active,is_system_admin,created_at,updated_at
             FROM users WHERE id=?1",
            [id],
            row_to_user,
        )
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(user)
}

pub fn set_active(db: &Connection, id: i64, active: bool) -> Result<(), String> {
    let changed = db
        .execute(
            "UPDATE users SET is_active=?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            params![id, active as i64],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("المستخدم غير موجود".into());
    }
    if !active {
        db.execute(
            "UPDATE auth_sessions SET revoked_at=COALESCE(revoked_at,CURRENT_TIMESTAMP) WHERE user_id=?1 AND revoked_at IS NULL",
            [id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE users(id INTEGER PRIMARY KEY,username TEXT NOT NULL COLLATE NOCASE UNIQUE,display_name TEXT NOT NULL,password_hash TEXT NOT NULL,is_active INTEGER NOT NULL DEFAULT 1,is_system_admin INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,employee_code TEXT NOT NULL UNIQUE);
             CREATE TABLE identity_sequences(name TEXT PRIMARY KEY,next_value INTEGER NOT NULL);
             INSERT INTO identity_sequences VALUES('employee_code',1);
             CREATE TABLE auth_sessions(id TEXT PRIMARY KEY,user_id INTEGER NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,expires_at TEXT NOT NULL,revoked_at TEXT);",
        ).unwrap();
        db
    }

    fn input(n: i64) -> NewUserAccount {
        NewUserAccount {
            username: format!("user{n}"),
            display_name: format!("مستخدم {n}"),
            password_hash: "hash".into(),
            is_system_admin: None,
        }
    }

    #[test]
    fn allocates_short_permanent_codes_without_reuse() {
        let mut db = db();
        let first = create(&mut db, input(1)).unwrap();
        let second = create(&mut db, input(2)).unwrap();
        assert_eq!(first.employee_code, "U01");
        assert_eq!(second.employee_code, "U02");
        set_active(&db, first.id, false).unwrap();
        let third = create(&mut db, input(3)).unwrap();
        assert_eq!(third.employee_code, "U03");
    }

    #[test]
    fn refuses_more_than_sixty_two_permanent_accounts() {
        let mut db = db();
        for n in 1..=MAX_USER_ACCOUNTS {
            create(&mut db, input(n)).unwrap();
        }
        let err = create(&mut db, input(63)).unwrap_err();
        assert!(err.contains("62"));
    }
}
