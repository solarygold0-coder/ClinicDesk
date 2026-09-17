use super::{migrate_db, schema_version, LATEST_SCHEMA_VERSION};
use rusqlite::{params, Connection};

fn fresh() -> Connection {
    Connection::open_in_memory().expect("open in-memory database")
}

#[test]
fn fresh_database_reaches_latest_schema() {
    let db = fresh();
    migrate_db(&db).expect("fresh migration must succeed");
    assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);
    assert_eq!(LATEST_SCHEMA_VERSION, 15);
}

#[test]
fn migration_is_idempotent() {
    let db = fresh();
    migrate_db(&db).unwrap();
    let before = schema_version(&db).unwrap();
    migrate_db(&db).expect("re-running migrations must be safe");
    assert_eq!(schema_version(&db).unwrap(), before);
}

#[test]
fn rejects_database_newer_than_supported_schema() {
    let db = fresh();
    migrate_db(&db).unwrap();
    db.execute(
        "UPDATE app_meta SET value = '16' WHERE key = 'schema_version'",
        [],
    )
    .unwrap();
    let err = migrate_db(&db).expect_err("future schema must be rejected");
    assert!(err.contains("أحدث من الإصدار"));
}

#[test]
fn employee_code_format_is_short_and_bounded() {
    let db = fresh();
    migrate_db(&db).unwrap();
    let sql: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='trigger' AND name='trg_users_employee_code_immutable'",
            [],
            |row| row.get(0),
        )
        .expect("immutable employee identity trigger must exist");
    assert!(sql.contains("employee_code"));
}

#[test]
fn schema_v13_role_lifecycle_columns_exist() {
    let db = fresh();
    migrate_db(&db).unwrap();
    let mut stmt = db.prepare("PRAGMA table_info(users)").unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for required in [
        "employee_code",
        "role_type",
        "account_status",
        "last_successful_login_at",
        "closed_at",
        "closed_reason",
        "must_change_password",
    ] {
        assert!(columns.iter().any(|c| c == required), "missing {required}");
    }
}

#[test]
fn attachment_lifecycle_columns_and_both_limit_triggers_exist() {
    let db = fresh();
    migrate_db(&db).unwrap();
    let mut stmt = db.prepare("PRAGMA table_info(attachments)").unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for required in ["display_name", "category", "deleted_at", "deleted_reason"] {
        assert!(columns.iter().any(|c| c == required), "missing {required}");
    }
    for name in [
        "trg_attachments_active_limit_insert",
        "trg_attachments_active_limit_restore",
    ] {
        let sql: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
                params![name],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| panic!("{name} must exist"));
        assert!(sql.contains(">= 20"));
    }
}

#[test]
fn database_rejects_restore_that_would_exceed_twenty_active_attachments() {
    let db = fresh();
    migrate_db(&db).unwrap();
    db.execute(
        "INSERT INTO patients(file_no,full_name) VALUES(1,'مريض')",
        [],
    )
    .unwrap();
    for i in 0..20 {
        db.execute(
            "INSERT INTO attachments(patient_id,stored_name,original_name,display_name,size_bytes,sha256) VALUES(1,?1,?2,?2,1,?3)",
            params![format!("active-{i}.pdf"), format!("active-{i}.pdf"), "a".repeat(64)],
        )
        .unwrap();
    }
    db.execute(
        "INSERT INTO attachments(patient_id,stored_name,original_name,display_name,size_bytes,sha256,deleted_at,deleted_reason) VALUES(1,'archived.pdf','archived.pdf','archived.pdf',1,?1,CURRENT_TIMESTAMP,'test')",
        params!["b".repeat(64)],
    )
    .unwrap();
    let archived_id = db.last_insert_rowid();
    let error = db
        .execute(
            "UPDATE attachments SET deleted_at=NULL,deleted_reason=NULL WHERE id=?1",
            params![archived_id],
        )
        .expect_err("database trigger must block restore over active limit");
    assert!(error.to_string().contains("attachment_limit_20"));
}
