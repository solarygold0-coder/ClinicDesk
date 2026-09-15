use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_ATTACHMENT_BYTES: u64 = 25 * 1024 * 1024;
const ALLOWED_EXTENSIONS: &[&str] = &["pdf", "png", "jpg", "jpeg", "webp", "doc", "docx"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: i64,
    pub patient_id: i64,
    pub stored_name: String,
    pub original_name: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub sha256: String,
    pub created_at: String,
}

pub fn list(conn: &Connection, patient_id: i64) -> Result<Vec<Attachment>, String> {
    let mut stmt = conn.prepare("SELECT id,patient_id,stored_name,original_name,mime_type,size_bytes,sha256,created_at FROM attachments WHERE patient_id=?1 ORDER BY created_at DESC,id DESC").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![patient_id], |r| {
            Ok(Attachment {
                id: r.get(0)?,
                patient_id: r.get(1)?,
                stored_name: r.get(2)?,
                original_name: r.get(3)?,
                mime_type: r.get(4)?,
                size_bytes: r.get(5)?,
                sha256: r.get(6)?,
                created_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn ensure_patient(conn: &Connection, patient_id: i64) -> Result<(), String> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM patients WHERE id=?1 AND deleted_at IS NULL",
            params![patient_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        Err("ملف المريض غير موجود".into())
    } else {
        Ok(())
    }
}

fn extension(path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err("نوع الملف غير مسموح".into());
    }
    Ok(ext)
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "application/octet-stream",
    }
}

fn safe_stored_path(root: &Path, stored_name: &str) -> Result<PathBuf, String> {
    if stored_name.is_empty()
        || Path::new(stored_name).file_name().and_then(|v| v.to_str()) != Some(stored_name)
    {
        return Err("اسم المرفق المخزن غير صالح".into());
    }
    Ok(root.join(stored_name))
}

pub fn import_file(
    conn: &Connection,
    patient_id: i64,
    source: &Path,
    root: &Path,
) -> Result<i64, String> {
    ensure_patient(conn, patient_id)?;
    let metadata = fs::metadata(source).map_err(|_| "تعذر قراءة الملف المحدد".to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_ATTACHMENT_BYTES {
        return Err("حجم المرفق يجب أن يكون بين 1 بايت و25 ميجابايت".into());
    }
    let ext = extension(source)?;
    let original_name = source
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| "اسم الملف غير صالح".to_string())?
        .to_string();
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let stored_name = format!("{}.{}", Uuid::new_v4(), ext);
    let destination = safe_stored_path(root, &stored_name)?;
    let bytes = fs::read(source).map_err(|_| "تعذر قراءة الملف المحدد".to_string())?;
    if bytes.len() as u64 != metadata.len() {
        return Err("تغير الملف أثناء الاستيراد، أعد المحاولة".into());
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&destination, &bytes).map_err(|e| e.to_string())?;
    match add(
        conn,
        patient_id,
        &stored_name,
        &original_name,
        Some(mime_for(&ext)),
        bytes.len() as i64,
        &sha256,
    ) {
        Ok(id) => Ok(id),
        Err(e) => {
            let _ = fs::remove_file(&destination);
            Err(e)
        }
    }
}

pub fn remove_file(conn: &Connection, id: i64, root: &Path) -> Result<(), String> {
    let stored_name: String = conn
        .query_row(
            "SELECT stored_name FROM attachments WHERE id=?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|_| "المرفق غير موجود".to_string())?;
    let path = safe_stored_path(root, &stored_name)?;
    let quarantine = root.join(format!(".delete-{}", Uuid::new_v4()));
    let moved = path.exists();
    if moved {
        fs::rename(&path, &quarantine).map_err(|e| e.to_string())?;
    }
    if let Err(e) = remove(conn, id) {
        if moved {
            fs::rename(&quarantine, &path)
                .map_err(|restore| format!("{e}; تعذر استعادة المرفق بعد فشل الحذف: {restore}"))?;
        }
        return Err(e);
    }
    if moved {
        fs::remove_file(&quarantine)
            .map_err(|e| format!("تم حذف سجل المرفق لكن تعذر تنظيف الملف المؤقت: {e}"))?;
    }
    Ok(())
}

pub fn add(
    conn: &Connection,
    patient_id: i64,
    stored_name: &str,
    original_name: &str,
    mime_type: Option<&str>,
    size_bytes: i64,
    sha256: &str,
) -> Result<i64, String> {
    if original_name.trim().is_empty()
        || safe_stored_path(Path::new("."), stored_name).is_err()
        || sha256.len() != 64
        || !sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("بيانات المرفق غير صالحة".into());
    }
    if size_bytes <= 0 || size_bytes > MAX_ATTACHMENT_BYTES as i64 {
        return Err("حجم المرفق يجب أن يكون بين 1 بايت و25 ميجابايت".into());
    }
    ensure_patient(conn, patient_id)?;
    conn.execute("INSERT INTO attachments(patient_id,stored_name,original_name,mime_type,size_bytes,sha256) VALUES(?1,?2,?3,?4,?5,?6)", params![patient_id,stored_name,original_name,mime_type,size_bytes,sha256]).map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.execute("INSERT INTO audit_log(event_type,entity_type,entity_id,details_json) VALUES('attachment_added','patient',?1,?2)", params![patient_id,format!(r#"{{"attachmentId":{id}}}"#)]).map_err(|e| e.to_string())?;
    Ok(id)
}

pub fn remove(conn: &Connection, id: i64) -> Result<String, String> {
    let (patient_id, stored_name): (i64, String) = conn
        .query_row(
            "SELECT patient_id,stored_name FROM attachments WHERE id=?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "المرفق غير موجود".to_string())?;
    conn.execute("DELETE FROM attachments WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO audit_log(event_type,entity_type,entity_id,details_json) VALUES('attachment_removed','patient',?1,?2)", params![patient_id,format!(r#"{{"attachmentId":{id}}}"#)]).map_err(|e| e.to_string())?;
    Ok(stored_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        conn.execute(
            "INSERT INTO patients(file_no,full_name) VALUES(1,'مريض')",
            [],
        )
        .unwrap();
        conn
    }
    #[test]
    fn attachment_metadata_round_trip() {
        let conn = db();
        let id = add(
            &conn,
            1,
            "abc.pdf",
            "report.pdf",
            Some("application/pdf"),
            1024,
            &"a".repeat(64),
        )
        .unwrap();
        assert_eq!(list(&conn, 1).unwrap().len(), 1);
        assert_eq!(remove(&conn, id).unwrap(), "abc.pdf");
        assert!(list(&conn, 1).unwrap().is_empty());
    }
    #[test]
    fn rejects_oversized_attachment() {
        let conn = db();
        assert!(add(
            &conn,
            1,
            "x.pdf",
            "x.pdf",
            None,
            26 * 1024 * 1024,
            &"a".repeat(64)
        )
        .is_err());
    }
    #[test]
    fn rejects_path_traversal() {
        let conn = db();
        assert!(add(&conn, 1, "../x.pdf", "x.pdf", None, 1, &"a".repeat(64)).is_err());
    }
}
