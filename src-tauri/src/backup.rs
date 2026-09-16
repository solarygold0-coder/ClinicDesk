use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use uuid::Uuid;

fn sqlite_quote(path: &Path) -> Result<String, String> {
    let value = path
        .to_str()
        .ok_or_else(|| "مسار النسخة الاحتياطية غير صالح".to_string())?;
    Ok(format!("'{}'", value.replace('\'', "''")))
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn attachment_sidecar(database_path: &Path) -> std::path::PathBuf {
    let name = database_path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("ClinicDesk-backup.sqlite3");
    database_path.with_file_name(format!("{name}.attachments"))
}

fn safe_attachment_name(stored_name: &str) -> Result<(), String> {
    if stored_name.is_empty()
        || Path::new(stored_name).file_name().and_then(|v| v.to_str()) != Some(stored_name)
    {
        return Err("اسم مرفق غير صالح داخل النسخة الاحتياطية".into());
    }
    Ok(())
}

pub fn verify_database(path: &Path, expected_schema: i64) -> Result<(), String> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "تعذر فتح ملف النسخة الاحتياطية".to_string())?;
    let integrity: String = db
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err("فشل فحص سلامة قاعدة البيانات".into());
    }
    let schema: i64 = db
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "النسخة لا تحتوي إصدار قاعدة بيانات صالح".to_string())?;
    if schema != expected_schema {
        return Err(format!(
            "إصدار النسخة الاحتياطية {schema} غير مطابق للإصدار المدعوم {expected_schema}"
        ));
    }
    Ok(())
}

pub fn verify_attachment_backup(database_path: &Path) -> Result<(), String> {
    let db = Connection::open_with_flags(database_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "تعذر فتح ملف النسخة الاحتياطية".to_string())?;
    let sidecar = attachment_sidecar(database_path);
    let mut stmt = db
        .prepare("SELECT stored_name,size_bytes,sha256 FROM attachments ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (stored_name, size_bytes, expected_hash) = row.map_err(|e| e.to_string())?;
        safe_attachment_name(&stored_name)?;
        let path = sidecar.join(&stored_name);
        let metadata = fs::metadata(&path)
            .map_err(|_| format!("المرفق {stored_name} مفقود من النسخة الاحتياطية"))?;
        if !metadata.is_file() || metadata.len() != size_bytes as u64 {
            return Err(format!("حجم المرفق {stored_name} لا يطابق قاعدة البيانات"));
        }
        if sha256_file(&path)? != expected_hash.to_ascii_lowercase() {
            return Err(format!("فشل التحقق من بصمة المرفق {stored_name}"));
        }
    }
    Ok(())
}

pub fn create_attachment_backup(
    conn: &Connection,
    attachment_root: &Path,
    database_path: &Path,
) -> Result<(), String> {
    let sidecar = attachment_sidecar(database_path);
    if sidecar.exists() {
        return Err("مجلد مرفقات النسخة الاحتياطية موجود مسبقاً".into());
    }
    fs::create_dir_all(&sidecar).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut stmt = conn
            .prepare("SELECT stored_name,size_bytes,sha256 FROM attachments ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (stored_name, size_bytes, expected_hash) = row.map_err(|e| e.to_string())?;
            safe_attachment_name(&stored_name)?;
            let source = attachment_root.join(&stored_name);
            let metadata = fs::metadata(&source)
                .map_err(|_| format!("المرفق {stored_name} مفقود من مجلد البرنامج"))?;
            if !metadata.is_file() || metadata.len() != size_bytes as u64 {
                return Err(format!("حجم المرفق {stored_name} لا يطابق السجل"));
            }
            if sha256_file(&source)? != expected_hash.to_ascii_lowercase() {
                return Err(format!("فشل التحقق من بصمة المرفق {stored_name}"));
            }
            fs::copy(&source, sidecar.join(&stored_name)).map_err(|e| e.to_string())?;
        }
        verify_attachment_backup(database_path)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&sidecar);
    }
    result
}

pub fn create_database_backup(
    conn: &Connection,
    destination: &Path,
    expected_schema: i64,
) -> Result<String, String> {
    if destination.exists() {
        return Err("ملف النسخة الاحتياطية موجود مسبقاً".into());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let quoted = sqlite_quote(destination)?;
    conn.execute_batch(&format!("VACUUM INTO {quoted}"))
        .map_err(|e| e.to_string())?;
    if let Err(e) = verify_database(destination, expected_schema) {
        let _ = fs::remove_file(destination);
        return Err(e);
    }
    sha256_file(destination)
}

pub fn restore_database(
    current: &mut Connection,
    source: &Path,
    live_path: &Path,
    expected_schema: i64,
) -> Result<String, String> {
    verify_database(source, expected_schema)?;
    let source_hash = sha256_file(source)?;
    let parent = live_path
        .parent()
        .ok_or_else(|| "مسار قاعدة البيانات غير صالح".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let staged = parent.join(format!("clinicdesk-restore-{}.sqlite3", Uuid::new_v4()));
    fs::copy(source, &staged).map_err(|e| e.to_string())?;
    if let Err(e) = verify_database(&staged, expected_schema) {
        let _ = fs::remove_file(&staged);
        return Err(e);
    }
    let staged_hash = sha256_file(&staged)?;
    if staged_hash != source_hash {
        let _ = fs::remove_file(&staged);
        return Err("فشل التحقق من تطابق النسخة أثناء الاستعادة".into());
    }

    let rollback = parent.join(format!(
        "clinicdesk-before-restore-{}.sqlite3",
        Uuid::new_v4()
    ));
    create_database_backup(current, &rollback, expected_schema)?;
    let replacement = Connection::open(&staged).map_err(|e| e.to_string())?;
    if let Err(e) = super::migrate_db(&replacement) {
        let _ = fs::remove_file(&staged);
        let _ = fs::remove_file(&rollback);
        return Err(e);
    }
    drop(replacement);

    let placeholder = Connection::open_in_memory().map_err(|e| e.to_string())?;
    let old = std::mem::replace(current, placeholder);
    drop(old);

    if let Err(e) = fs::remove_file(live_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            *current = Connection::open(live_path).map_err(|open| open.to_string())?;
            let _ = fs::remove_file(&staged);
            let _ = fs::remove_file(&rollback);
            return Err(format!("تعذر تجهيز قاعدة البيانات للاستعادة: {e}"));
        }
    }
    if let Err(e) = fs::rename(&staged, live_path) {
        fs::copy(&rollback, live_path)
            .map_err(|r| format!("تعذرت الاستعادة ({e}) وتعذر التراجع عنها ({r})"))?;
        *current = Connection::open(live_path).map_err(|open| open.to_string())?;
        let _ = fs::remove_file(&staged);
        let _ = fs::remove_file(&rollback);
        return Err(format!("تعذرت استعادة النسخة: {e}"));
    }
    let reopened = Connection::open(live_path).map_err(|e| e.to_string())?;
    if let Err(e) = super::migrate_db(&reopened) {
        drop(reopened);
        let _ = fs::remove_file(live_path);
        fs::copy(&rollback, live_path)
            .map_err(|r| format!("فشل التحقق بعد الاستعادة ({e}) وتعذر التراجع عنها ({r})"))?;
        *current = Connection::open(live_path).map_err(|open| open.to_string())?;
        let _ = fs::remove_file(&rollback);
        return Err(e);
    }
    *current = reopened;
    let _ = fs::remove_file(&rollback);
    Ok(source_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "clinicdesk-{name}-{}-{}.sqlite3",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn backup_is_integral_and_hashable() {
        let source = temp("source");
        let backup = temp("backup");
        let conn = Connection::open(&source).unwrap();
        super::super::migrate_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO patients(file_no,full_name) VALUES(1,'مريض')",
            [],
        )
        .unwrap();
        let hash =
            create_database_backup(&conn, &backup, super::super::LATEST_SCHEMA_VERSION).unwrap();
        assert_eq!(hash.len(), 64);
        verify_database(&backup, super::super::LATEST_SCHEMA_VERSION).unwrap();
        let copied = Connection::open(&backup).unwrap();
        let count: i64 = copied
            .query_row("SELECT COUNT(*) FROM patients", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        drop(copied);
        drop(conn);
        let _ = fs::remove_file(source);
        let _ = fs::remove_file(backup);
    }

    #[test]
    fn attachment_backup_detects_tampering() {
        let source = temp("attachment-source");
        let backup = temp("attachment-backup");
        let root = attachment_sidecar(&source);
        fs::create_dir_all(&root).unwrap();
        let bytes = b"verified attachment";
        let stored_name = "abc.pdf";
        fs::write(root.join(stored_name), bytes).unwrap();
        let conn = Connection::open(&source).unwrap();
        super::super::migrate_db(&conn).unwrap();
        conn.execute(
            "INSERT INTO patients(file_no,full_name) VALUES(1,'مريض')",
            [],
        )
        .unwrap();
        let patient_id = conn.last_insert_rowid();
        let hash = format!("{:x}", Sha256::digest(bytes));
        conn.execute(
            "INSERT INTO attachments(patient_id,stored_name,original_name,size_bytes,sha256) VALUES(?1,?2,'report.pdf',?3,?4)",
            rusqlite::params![patient_id, stored_name, bytes.len() as i64, hash],
        )
        .unwrap();
        create_database_backup(&conn, &backup, super::super::LATEST_SCHEMA_VERSION).unwrap();
        create_attachment_backup(&conn, &root, &backup).unwrap();
        verify_attachment_backup(&backup).unwrap();
        fs::write(attachment_sidecar(&backup).join(stored_name), b"tampered").unwrap();
        assert!(verify_attachment_backup(&backup).is_err());
        drop(conn);
        let _ = fs::remove_file(source);
        let _ = fs::remove_file(backup);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(attachment_sidecar(&backup));
    }

    #[test]
    fn restore_replaces_existing_live_database() {
        let live = temp("live");
        let source = temp("restore-source");
        let mut current = Connection::open(&live).unwrap();
        super::super::migrate_db(&current).unwrap();
        current
            .execute(
                "INSERT INTO patients(file_no,full_name) VALUES(1,'قديم')",
                [],
            )
            .unwrap();

        let source_conn = Connection::open(&source).unwrap();
        super::super::migrate_db(&source_conn).unwrap();
        source_conn
            .execute(
                "INSERT INTO patients(file_no,full_name) VALUES(2,'مستعاد')",
                [],
            )
            .unwrap();
        drop(source_conn);

        restore_database(
            &mut current,
            &source,
            &live,
            super::super::LATEST_SCHEMA_VERSION,
        )
        .unwrap();
        let name: String = current
            .query_row("SELECT full_name FROM patients WHERE file_no=2", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "مستعاد");
        let old_count: i64 = current
            .query_row("SELECT COUNT(*) FROM patients WHERE file_no=1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(old_count, 0);
        drop(current);
        let _ = fs::remove_file(live);
        let _ = fs::remove_file(source);
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let path = temp("wrong-schema");
        let conn = Connection::open(&path).unwrap();
        super::super::migrate_db(&conn).unwrap();
        drop(conn);
        assert!(verify_database(&path, super::super::LATEST_SCHEMA_VERSION + 1).is_err());
        let _ = fs::remove_file(path);
    }
}
