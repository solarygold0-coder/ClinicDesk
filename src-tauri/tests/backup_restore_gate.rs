use clinicdesk::{attachments, backup, security_log};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use uuid::Uuid;

const SCHEMA: i64 = 15;

fn temp_db(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clinicdesk-gate-{label}-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    dir.join("clinicdesk.sqlite3")
}

fn migrate_fixture(conn: &Connection) {
    for sql in [
        include_str!("../migrations/001_init.sql"),
        include_str!("../migrations/002_touch_triggers.sql"),
        include_str!("../migrations/003_scheduling_rules.sql"),
        include_str!("../migrations/004_patient_medical_details.sql"),
        include_str!("../migrations/005_patient_last_activity.sql"),
        include_str!("../migrations/006_patient_activity_triggers.sql"),
        include_str!("../migrations/007_appointment_visit_tracking.sql"),
        include_str!("../migrations/008_optional_auth.sql"),
        include_str!("../migrations/009_remove_optional_auth.sql"),
        include_str!("../migrations/010_restore_users_roles_audit_actor.sql"),
        include_str!("../migrations/011_accountability_identity.sql"),
        include_str!("../migrations/012_immutable_employee_identity.sql"),
        include_str!("../migrations/013_user_roles_lifecycle.sql"),
        include_str!("../migrations/014_attachment_lifecycle.sql"),
        include_str!("../migrations/015_attachment_restore_limit.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
}

fn add_archived_attachment(conn: &Connection, db_path: &PathBuf) -> (i64, String, Vec<u8>) {
    conn.execute(
        "INSERT INTO patients(file_no,full_name) VALUES(1,'مريض مؤرشف المرفق')",
        [],
    )
    .unwrap();
    let patient_id = conn.last_insert_rowid();
    let root = db_path.parent().unwrap().join("attachments");
    fs::create_dir_all(&root).unwrap();
    let stored_name = "archived-gate.pdf".to_string();
    let bytes = b"archived attachment survives backup restore".to_vec();
    fs::write(root.join(&stored_name), &bytes).unwrap();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    conn.execute(
        "INSERT INTO attachments(patient_id,stored_name,original_name,display_name,category,mime_type,size_bytes,sha256,deleted_at,deleted_reason) VALUES(?1,?2,'archive.pdf','مرفق مؤرشف','تقارير','application/pdf',?3,?4,'2026-09-17T10:00:00Z','اختبار الاستعادة')",
        rusqlite::params![patient_id, stored_name, bytes.len() as i64, hash],
    )
    .unwrap();
    (patient_id, stored_name, bytes)
}

#[test]
fn archived_attachment_survives_full_backup_and_restore() {
    let source = temp_db("archived-source");
    let backup_path = temp_db("archived-backup");
    let live = temp_db("archived-live");

    let source_conn = Connection::open(&source).unwrap();
    migrate_fixture(&source_conn);
    let (patient_id, stored_name, bytes) = add_archived_attachment(&source_conn, &source);

    let backup_hash = backup::create_database_backup(&source_conn, &backup_path, SCHEMA).unwrap();
    assert_eq!(backup_hash.len(), 64);
    backup::verify_database(&backup_path, SCHEMA).unwrap();
    backup::verify_attachment_backup(&backup_path).unwrap();

    let copied = Connection::open(&backup_path).unwrap();
    let archived = attachments::list_archived(&copied, patient_id).unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].display_name, "مرفق مؤرشف");
    assert_eq!(archived[0].deleted_reason.as_deref(), Some("اختبار الاستعادة"));
    assert_eq!(archived[0].sha256, format!("{:x}", Sha256::digest(&bytes)));
    drop(copied);

    let mut live_conn = Connection::open(&live).unwrap();
    migrate_fixture(&live_conn);
    live_conn
        .execute("INSERT INTO patients(file_no,full_name) VALUES(99,'بيانات قبل الاستعادة')", [])
        .unwrap();

    let restored_hash = backup::restore_database(&mut live_conn, &backup_path, &live, SCHEMA).unwrap();
    assert_eq!(restored_hash, backup_hash);
    let restored = attachments::list_archived(&live_conn, patient_id).unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].stored_name, stored_name);
    let restored_path = live.parent().unwrap().join("attachments").join(&stored_name);
    assert_eq!(fs::read(&restored_path).unwrap(), bytes);
    assert_eq!(backup::sha256_file(&restored_path).unwrap(), restored[0].sha256);

    drop(live_conn);
    drop(source_conn);
    let _ = fs::remove_dir_all(source.parent().unwrap());
    let _ = fs::remove_dir_all(backup_path.parent().unwrap());
    let _ = fs::remove_dir_all(live.parent().unwrap());
}

#[test]
fn tampered_archived_attachment_blocks_restore_before_live_change() {
    let source = temp_db("tamper-source");
    let backup_path = temp_db("tamper-backup");
    let live = temp_db("tamper-live");

    let source_conn = Connection::open(&source).unwrap();
    migrate_fixture(&source_conn);
    let (_patient_id, stored_name, _bytes) = add_archived_attachment(&source_conn, &source);
    backup::create_database_backup(&source_conn, &backup_path, SCHEMA).unwrap();

    let sidecar = backup_path.with_file_name(format!(
        "{}.attachments",
        backup_path.file_name().unwrap().to_string_lossy()
    ));
    fs::write(sidecar.join(&stored_name), b"tampered archived attachment").unwrap();
    assert!(backup::verify_attachment_backup(&backup_path).is_err());

    let mut live_conn = Connection::open(&live).unwrap();
    migrate_fixture(&live_conn);
    live_conn
        .execute("INSERT INTO patients(file_no,full_name) VALUES(77,'يجب أن يبقى')", [])
        .unwrap();

    assert!(backup::restore_database(&mut live_conn, &backup_path, &live, SCHEMA).is_err());
    let retained: String = live_conn
        .query_row("SELECT full_name FROM patients WHERE file_no=77", [], |r| r.get(0))
        .unwrap();
    assert_eq!(retained, "يجب أن يبقى");

    drop(live_conn);
    drop(source_conn);
    let _ = fs::remove_dir_all(source.parent().unwrap());
    let _ = fs::remove_dir_all(backup_path.parent().unwrap());
    let _ = fs::remove_dir_all(live.parent().unwrap());
}

#[test]
fn strict_external_operation_log_rejects_invalid_lifecycle_and_detects_tamper() {
    let path = std::env::temp_dir().join(format!("clinicdesk-gate-log-{}.jsonl", Uuid::new_v4()));
    let hash = "a".repeat(64);

    assert!(security_log::append_operation(
        &path,
        "backup",
        "started",
        "RST-wrong-kind",
        "preflight",
        "started",
        None,
    )
    .is_err());
    assert!(security_log::append_operation(
        &path,
        "backup",
        "failure",
        "BKP-gate",
        "source_read",
        "raw path C:/secret",
        None,
    )
    .is_err());

    security_log::append_operation(
        &path,
        "backup",
        "started",
        "BKP-gate",
        "preflight",
        "started",
        None,
    )
    .unwrap();
    security_log::append_operation(
        &path,
        "backup",
        "success",
        "BKP-gate",
        "completed",
        "ok",
        Some(&hash),
    )
    .unwrap();
    security_log::verify(&path).unwrap();

    let original = fs::read_to_string(&path).unwrap();
    let tampered = original.replacen("\"status\":\"success\"", "\"status\":\"failure\"", 1);
    assert_ne!(original, tampered);
    fs::write(&path, tampered).unwrap();
    assert!(security_log::verify(&path).is_err());
    let _ = fs::remove_file(path);
}
