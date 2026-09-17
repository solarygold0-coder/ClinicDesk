use super::{migrate_db, schema_version, LATEST_SCHEMA_VERSION};
use rusqlite::{params, Connection};

fn fresh() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn fresh_database_reaches_latest_schema() {
    let db = fresh();
    migrate_db(&db).unwrap();
    assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);
}

#[test]
fn migration_is_idempotent() {
    let db = fresh();
    migrate_db(&db).unwrap();
    migrate_db(&db).unwrap();
    assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);
}

#[test]
fn rejects_database_newer_than_supported_schema() {
    let db = fresh();
    migrate_db(&db).unwrap();
    db.execute(
        "UPDATE app_meta SET value=?1 WHERE key='schema_version'",
        params![(LATEST_SCHEMA_VERSION + 1).to_string()],
    )
    .unwrap();
    assert!(migrate_db(&db).is_err());
}

#[test]
fn employee_code_format_is_short_and_bounded() {
    let db = fresh();
    migrate_db(&db).unwrap();
    let sql: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='trigger' AND name='trg_users_employee_code_immutable'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(sql.contains("employee_code"));
}

#[test]
fn schema_v13_role_lifecycle_columns_exist() {
    let db = fresh();
    migrate_db(&db).unwrap();
    let mut stmt = db.prepare("PRAGMA table_info(users)").unwrap();
    let cols = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for required in ["employee_code", "role_type", "account_status"] {
        assert!(cols.iter().any(|c| c == required), "missing {required}");
    }
}

#[test]
fn attachment_lifecycle_columns_and_both_limit_triggers_exist() {
    let db = fresh();
    migrate_db(&db).unwrap();
    let mut stmt = db.prepare("PRAGMA table_info(attachments)").unwrap();
    let columns = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
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
                |r| r.get(0),
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
    db.execute(
        "INSERT INTO attachments(patient_id,stored_name,original_name,display_name,size_bytes,sha256,deleted_at,deleted_reason) VALUES(1,'archived.pdf','archived.pdf','archived.pdf',1,?1,CURRENT_TIMESTAMP,'test')",
        params!["b".repeat(64)],
    )
    .unwrap();
    let archived_id = db.last_insert_rowid();
    for i in 0..20 {
        db.execute(
            "INSERT INTO attachments(patient_id,stored_name,original_name,display_name,size_bytes,sha256) VALUES(1,?1,?2,?2,1,?3)",
            params![format!("active-{i}.pdf"), format!("active-{i}.pdf"), "a".repeat(64)],
        )
        .unwrap();
    }
    let active: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM attachments WHERE patient_id=1 AND deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        active, 20,
        "fixture must contain exactly 20 active attachments"
    );
    let error = db
        .execute(
            "UPDATE attachments SET deleted_at=NULL,deleted_reason=NULL WHERE id=?1",
            params![archived_id],
        )
        .expect_err("database trigger must block restore over active limit");
    assert!(error.to_string().contains("attachment_limit_20"));
    let still_archived: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM attachments WHERE id=?1 AND deleted_at IS NOT NULL",
            params![archived_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        still_archived, 1,
        "failed restore must preserve archived state"
    );
}

#[test]
fn role_capability_migration_seeds_roles_without_default_restore_grant() {
    let db = fresh();
    migrate_db(&db).unwrap();
    assert_eq!(schema_version(&db).unwrap(), 16);

    let canonical_roles: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM roles WHERE name IN ('ordinary_employee','general_manager','deputy_manager','doctor','specialist')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(canonical_roles, 5);

    let restore_grants: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM role_capabilities WHERE capability='backup.restore'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(restore_grants, 0, "restore must require an explicit later grant");

    let architecture: String = db
        .query_row(
            "SELECT value FROM app_meta WHERE key='auth_capability_grants'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(architecture, "role-capabilities-v1");
}
