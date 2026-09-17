use super::{migrate_db, schema_version, LATEST_SCHEMA_VERSION};
use rusqlite::Connection;

fn fresh() -> Connection { Connection::open_in_memory().expect("open in-memory database") }

#[test]
fn fresh_database_reaches_latest_schema() {
    let db = fresh();
    migrate_db(&db).expect("fresh migration must succeed");
    assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);
    assert_eq!(LATEST_SCHEMA_VERSION, 14);
}

#[test]
fn migration_is_idempotent() {
    let db = fresh(); migrate_db(&db).unwrap(); let before = schema_version(&db).unwrap();
    migrate_db(&db).expect("re-running migrations must be safe"); assert_eq!(schema_version(&db).unwrap(), before);
}

#[test]
fn rejects_database_newer_than_supported_schema() {
    let db=fresh(); migrate_db(&db).unwrap();
    db.execute("UPDATE app_meta SET value = '15' WHERE key = 'schema_version'",[]).unwrap();
    let err=migrate_db(&db).expect_err("future schema must be rejected"); assert!(err.contains("أحدث من الإصدار"));
}

#[test]
fn employee_code_format_is_short_and_bounded() {
    let db=fresh(); migrate_db(&db).unwrap();
    let sql:String=db.query_row("SELECT sql FROM sqlite_master WHERE type='trigger' AND name='trg_users_employee_code_immutable'",[],|row|row.get(0)).expect("immutable employee identity trigger must exist");
    assert!(sql.contains("employee_code"));
}

#[test]
fn schema_v13_role_lifecycle_columns_exist() {
    let db=fresh(); migrate_db(&db).unwrap(); let mut stmt=db.prepare("PRAGMA table_info(users)").unwrap();
    let columns:Vec<String>=stmt.query_map([],|row|row.get::<_,String>(1)).unwrap().map(Result::unwrap).collect();
    for required in ["employee_code","role_type","account_status","last_successful_login_at","closed_at","closed_reason","must_change_password"] { assert!(columns.iter().any(|c|c==required),"missing {required}"); }
}

#[test]
fn schema_v14_attachment_lifecycle_and_limit_exist() {
    let db=fresh(); migrate_db(&db).unwrap();
    let mut stmt=db.prepare("PRAGMA table_info(attachments)").unwrap();
    let columns:Vec<String>=stmt.query_map([],|row|row.get::<_,String>(1)).unwrap().map(Result::unwrap).collect();
    for required in ["display_name","category","deleted_at","deleted_reason"] { assert!(columns.iter().any(|c|c==required),"missing {required}"); }
    let trigger:String=db.query_row("SELECT sql FROM sqlite_master WHERE type='trigger' AND name='trg_attachments_active_limit_insert'",[],|row|row.get(0)).expect("attachment limit trigger must exist");
    assert!(trigger.contains(">= 20"));
}
