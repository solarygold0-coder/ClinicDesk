use clinicdesk_lib::backup;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "clinicdesk-release-{name}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn migrate(db: &Connection) {
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
    ] {
        db.execute_batch(sql).unwrap();
    }
}

fn sidecar(database: &Path) -> PathBuf {
    let name = database.file_name().unwrap().to_string_lossy();
    database.with_file_name(format!("{name}.attachments"))
}

#[test]
fn ten_thousand_patients_survive_verified_backup_restore_with_attachment() {
    let source_dir = temp_dir("source");
    let live_dir = temp_dir("live");
    let backup_dir = temp_dir("backup");
    let source_db = source_dir.join("clinicdesk.sqlite3");
    let live_db = live_dir.join("clinicdesk.sqlite3");
    let backup_db = backup_dir.join("ClinicDesk-release.sqlite3");
    let source_attachments = source_dir.join("attachments");
    fs::create_dir_all(&source_attachments).unwrap();

    let source = Connection::open(&source_db).unwrap();
    migrate(&source);
    let tx = source.unchecked_transaction().unwrap();
    {
        let mut insert = tx
            .prepare("INSERT INTO patients(file_no, full_name) VALUES(?1, ?2)")
            .unwrap();
        for file_no in 1_i64..=10_000 {
            insert
                .execute(rusqlite::params![file_no, format!("مريض {file_no}")])
                .unwrap();
        }
    }
    tx.commit().unwrap();

    let patient_id: i64 = source
        .query_row("SELECT id FROM patients WHERE file_no=10000", [], |r| r.get(0))
        .unwrap();
    let attachment_bytes = b"ClinicDesk release regression attachment";
    let stored_name = "release-proof.pdf";
    fs::write(source_attachments.join(stored_name), attachment_bytes).unwrap();
    let hash = format!("{:x}", Sha256::digest(attachment_bytes));
    source
        .execute(
            "INSERT INTO attachments(patient_id,stored_name,original_name,size_bytes,sha256) VALUES(?1,?2,'release-proof.pdf',?3,?4)",
            rusqlite::params![patient_id, stored_name, attachment_bytes.len() as i64, hash],
        )
        .unwrap();

    let backup_hash = backup::create_database_backup(&source, &backup_db, 9).unwrap();
    assert_eq!(backup_hash.len(), 64);
    backup::verify_database(&backup_db, 9).unwrap();
    backup::verify_attachment_backup(&backup_db).unwrap();
    drop(source);

    let mut live = Connection::open(&live_db).unwrap();
    migrate(&live);
    live.execute("INSERT INTO patients(file_no,full_name) VALUES(1,'سيتم استبداله')", [])
        .unwrap();

    let restored_hash = backup::restore_database(&mut live, &backup_db, &live_db, 9).unwrap();
    assert_eq!(restored_hash, backup_hash);
    let patient_count: i64 = live
        .query_row("SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL", [], |r| r.get(0))
        .unwrap();
    assert_eq!(patient_count, 10_000);
    let last_name: String = live
        .query_row("SELECT full_name FROM patients WHERE file_no=10000", [], |r| r.get(0))
        .unwrap();
    assert_eq!(last_name, "مريض 10000");
    assert_eq!(
        fs::read(live_dir.join("attachments").join(stored_name)).unwrap(),
        attachment_bytes
    );

    drop(live);
    let _ = fs::remove_dir_all(source_dir);
    let _ = fs::remove_dir_all(live_dir);
    let _ = fs::remove_dir_all(backup_dir);
    let _ = fs::remove_dir_all(sidecar(&backup_db));
}
