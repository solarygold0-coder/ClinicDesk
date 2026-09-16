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

    let rollback = parent.join(format!("clinicdesk-before-restore-{}.sqlite3", Uuid::new_v4()));
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
    if let Err(e) = fs::rename(&staged, live_path) {
        let _ = fs::remove_file(live_path);
        fs::rename(&rollback, live_path)
            .map_err(|r| format!("تعذرت الاستعادة ({e}) وتعذر التراجع عنها ({r})"))?;
        *current = Connection::open(live_path).map_err(|open| open.to_string())?;
        return Err(format!("تعذرت استعادة النسخة: {e}"));
    }
    let reopened = Connection::open(live_path).map_err(|e| e.to_string())?;
    if let Err(e) = super::migrate_db(&reopened) {
        drop(reopened);
        let _ = fs::remove_file(live_path);
        fs::rename(&rollback, live_path).map_err(|r| {
            format!("فشل التحقق بعد الاستعادة ({e}) وتعذر التراجع عنها ({r})")
        })?;
        *current = Connection::open(live_path).map_err(|open| open.to_string())?;
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
        conn.execute("INSERT INTO patients(file_no,full_name) VALUES(1,'مريض')", [])
            .unwrap();
        let hash = create_database_backup(&conn, &backup, super::super::LATEST_SCHEMA_VERSION).unwrap();
        assert_eq!(hash.len(), 64);
        verify_database(&backup, super::super::LATEST_SCHEMA_VERSION).unwrap();
        let copied = Connection::open(&backup).unwrap();
        let count: i64 = copied.query_row("SELECT COUNT(*) FROM patients", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
        drop(copied);
        drop(conn);
        let _ = fs::remove_file(source);
        let _ = fs::remove_file(backup);
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
