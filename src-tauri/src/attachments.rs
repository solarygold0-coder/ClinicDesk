#![allow(clippy::too_many_arguments)]
use crate::audit;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_ATTACHMENT_BYTES: u64 = 25 * 1024 * 1024;
const MAX_ACTIVE_ATTACHMENTS: i64 = 20;
const BLOCKED_EXTENSIONS: &[&str] = &[
    "exe", "com", "bat", "cmd", "msi", "msp", "scr", "ps1", "psm1", "vbs", "vbe", "js", "jse",
    "wsf", "wsh", "hta", "lnk", "url", "reg", "dll", "sys", "cpl", "jar", "html", "htm", "xhtml",
    "svg", "chm",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: i64,
    pub patient_id: i64,
    pub stored_name: String,
    pub original_name: String,
    pub display_name: String,
    pub category: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub sha256: String,
    pub created_at: String,
    pub deleted_at: Option<String>,
    pub deleted_reason: Option<String>,
}

fn map_attachment(r: &rusqlite::Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: r.get(0)?,
        patient_id: r.get(1)?,
        stored_name: r.get(2)?,
        original_name: r.get(3)?,
        display_name: r.get(4)?,
        category: r.get(5)?,
        mime_type: r.get(6)?,
        size_bytes: r.get(7)?,
        sha256: r.get(8)?,
        created_at: r.get(9)?,
        deleted_at: r.get(10)?,
        deleted_reason: r.get(11)?,
    })
}

const ATTACHMENT_SELECT: &str = "SELECT id,patient_id,stored_name,original_name,COALESCE(display_name,original_name),category,mime_type,size_bytes,sha256,created_at,deleted_at,deleted_reason FROM attachments";

fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Attachment>, String> {
    conn.query_row(
        &format!("{ATTACHMENT_SELECT} WHERE id=?1"),
        [id],
        map_attachment,
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn query_list(
    conn: &Connection,
    patient_id: i64,
    archived: bool,
) -> Result<Vec<Attachment>, String> {
    let predicate = if archived {
        "deleted_at IS NOT NULL"
    } else {
        "deleted_at IS NULL"
    };
    let sql = format!(
        "{ATTACHMENT_SELECT} WHERE patient_id=?1 AND {predicate} ORDER BY created_at DESC,id DESC"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![patient_id], map_attachment)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn list(conn: &Connection, patient_id: i64) -> Result<Vec<Attachment>, String> {
    query_list(conn, patient_id, false)
}

pub fn list_archived(conn: &Connection, patient_id: i64) -> Result<Vec<Attachment>, String> {
    query_list(conn, patient_id, true)
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
    if ext.is_empty() || BLOCKED_EXTENSIONS.contains(&ext.as_str()) {
        return Err("نوع الملف غير مسموح لأسباب أمنية".into());
    }
    Ok(ext)
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "heic" | "heif" => "image/heic",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "rtf" => "application/rtf",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "dcm" => "application/dicom",
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

fn validate_label(value: &str, field: &str) -> Result<String, String> {
    let v = value.trim();
    if v.is_empty() || v.chars().count() > 120 || v.chars().any(|c| c.is_control()) {
        return Err(format!("{field} غير صالح"));
    }
    Ok(v.to_string())
}

fn active_count(conn: &Connection, patient_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM attachments WHERE patient_id=?1 AND deleted_at IS NULL",
        params![patient_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

fn audit_attachment(
    conn: &Connection,
    event_type: &str,
    attachment: &Attachment,
    before_json: Option<&str>,
    after_json: Option<&str>,
    reason: Option<&str>,
) -> Result<(), String> {
    let details = serde_json::json!({
        "patientId": attachment.patient_id,
        "displayName": &attachment.display_name,
        "category": &attachment.category,
    })
    .to_string();
    audit::record_as(
        conn,
        event_type,
        "attachment",
        Some(attachment.id),
        Some(&details),
        &audit::AuditActor::default(),
        &audit::AuditChange {
            before_json,
            after_json,
            reason,
        },
    )
}

pub fn import_file(
    conn: &Connection,
    patient_id: i64,
    source: &Path,
    root: &Path,
) -> Result<i64, String> {
    import_file_named(conn, patient_id, source, root, None, None)
}

pub fn import_file_named(
    conn: &Connection,
    patient_id: i64,
    source: &Path,
    root: &Path,
    display_name: Option<&str>,
    category: Option<&str>,
) -> Result<i64, String> {
    ensure_patient(conn, patient_id)?;
    if active_count(conn, patient_id)? >= MAX_ACTIVE_ATTACHMENTS {
        return Err("الحد الأقصى للمرفقات النشطة للمريض هو 20 مرفقاً".into());
    }
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
    let label = validate_label(display_name.unwrap_or(&original_name), "اسم المرفق")?;
    let category = category
        .map(|v| validate_label(v, "تصنيف المرفق"))
        .transpose()?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let stored_name = format!("{}.{}", Uuid::new_v4(), ext);
    let destination = safe_stored_path(root, &stored_name)?;
    let bytes = fs::read(source).map_err(|_| "تعذر قراءة الملف المحدد".to_string())?;
    if bytes.len() as u64 != metadata.len() {
        return Err("تغير الملف أثناء الاستيراد، أعد المحاولة".into());
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&destination, &bytes).map_err(|e| e.to_string())?;
    match add_named(
        conn,
        patient_id,
        &stored_name,
        &original_name,
        &label,
        category.as_deref(),
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

pub fn add(
    conn: &Connection,
    patient_id: i64,
    stored_name: &str,
    original_name: &str,
    mime_type: Option<&str>,
    size_bytes: i64,
    sha256: &str,
) -> Result<i64, String> {
    add_named(
        conn,
        patient_id,
        stored_name,
        original_name,
        original_name,
        None,
        mime_type,
        size_bytes,
        sha256,
    )
}

pub fn add_named(
    conn: &Connection,
    patient_id: i64,
    stored_name: &str,
    original_name: &str,
    display_name: &str,
    category: Option<&str>,
    mime_type: Option<&str>,
    size_bytes: i64,
    sha256: &str,
) -> Result<i64, String> {
    let label = validate_label(display_name, "اسم المرفق")?;
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
    if active_count(conn, patient_id)? >= MAX_ACTIVE_ATTACHMENTS {
        return Err("الحد الأقصى للمرفقات النشطة للمريض هو 20 مرفقاً".into());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO attachments(patient_id,stored_name,original_name,display_name,category,mime_type,size_bytes,sha256) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![patient_id,stored_name,original_name,label,category,mime_type,size_bytes,sha256]).map_err(|e|if e.to_string().contains("attachment_limit_20"){"الحد الأقصى للمرفقات النشطة للمريض هو 20 مرفقاً".to_string()}else{e.to_string()})?;
    let id = tx.last_insert_rowid();
    let attachment =
        get_by_id(&tx, id)?.ok_or_else(|| "تعذر قراءة المرفق بعد الحفظ".to_string())?;
    let after_json = serde_json::to_string(&attachment).map_err(|e| e.to_string())?;
    audit_attachment(
        &tx,
        "attachment_added",
        &attachment,
        None,
        Some(&after_json),
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

pub fn archive(conn: &Connection, id: i64, reason: &str) -> Result<(), String> {
    let reason = validate_label(reason, "سبب الأرشفة")?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let before = get_by_id(&tx, id)?
        .filter(|attachment| attachment.deleted_at.is_none())
        .ok_or_else(|| "المرفق غير موجود أو مؤرشف مسبقاً".to_string())?;
    let before_json = serde_json::to_string(&before).map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE attachments SET deleted_at=CURRENT_TIMESTAMP,deleted_reason=?2 WHERE id=?1",
        params![id, &reason],
    )
    .map_err(|e| e.to_string())?;
    let after = get_by_id(&tx, id)?.ok_or_else(|| "تعذر قراءة المرفق بعد الأرشفة".to_string())?;
    let after_json = serde_json::to_string(&after).map_err(|e| e.to_string())?;
    audit_attachment(
        &tx,
        "attachment_archived",
        &after,
        Some(&before_json),
        Some(&after_json),
        Some(&reason),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore(conn: &Connection, id: i64, reason: &str) -> Result<(), String> {
    let reason = validate_label(reason, "سبب الاستعادة")?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let before = get_by_id(&tx, id)?
        .filter(|attachment| attachment.deleted_at.is_some())
        .ok_or_else(|| "المرفق غير موجود أو غير مؤرشف".to_string())?;
    let before_json = serde_json::to_string(&before).map_err(|e| e.to_string())?;
    let count = active_count(&tx, before.patient_id)?;
    if count >= MAX_ACTIVE_ATTACHMENTS {
        return Err("لا يمكن الاستعادة: المريض لديه 20 مرفقاً نشطاً".into());
    }
    tx.execute(
        "UPDATE attachments SET deleted_at=NULL,deleted_reason=NULL WHERE id=?1",
        params![id],
    )
    .map_err(|e| {
        if e.to_string().contains("attachment_limit_20") {
            "لا يمكن الاستعادة: المريض لديه 20 مرفقاً نشطاً".to_string()
        } else {
            e.to_string()
        }
    })?;
    let after = get_by_id(&tx, id)?.ok_or_else(|| "تعذر قراءة المرفق بعد الاستعادة".to_string())?;
    let after_json = serde_json::to_string(&after).map_err(|e| e.to_string())?;
    audit_attachment(
        &tx,
        "attachment_restored",
        &after,
        Some(&before_json),
        Some(&after_json),
        Some(&reason),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        conn.execute_batch(include_str!("../migrations/014_attachment_lifecycle.sql"))
            .unwrap();
        conn.execute_batch(include_str!(
            "../migrations/015_attachment_restore_limit.sql"
        ))
        .unwrap();
        conn.execute_batch(
            "ALTER TABLE audit_log ADD COLUMN actor_user_id INTEGER;
             ALTER TABLE audit_log ADD COLUMN actor_display_name TEXT;
             ALTER TABLE audit_log ADD COLUMN actor_employee_code TEXT;
             ALTER TABLE audit_log ADD COLUMN actor_session_id TEXT;
             ALTER TABLE audit_log ADD COLUMN before_json TEXT;
             ALTER TABLE audit_log ADD COLUMN after_json TEXT;
             ALTER TABLE audit_log ADD COLUMN reason TEXT;",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO patients(file_no,full_name) VALUES(1,'مريض')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn lifecycle_preserves_metadata_and_archive_is_discoverable() {
        let conn = db();
        let id = add_named(
            &conn,
            1,
            "a.pdf",
            "a.pdf",
            "تحليل",
            Some("مختبر"),
            Some("application/pdf"),
            10,
            &"a".repeat(64),
        )
        .unwrap();
        assert_eq!(list(&conn, 1).unwrap()[0].display_name, "تحليل");
        archive(&conn, id, "انتهت الحاجة").unwrap();
        assert!(list(&conn, 1).unwrap().is_empty());
        let archived = list_archived(&conn, 1).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].deleted_reason.as_deref(), Some("انتهت الحاجة"));
        restore(&conn, id, "إعادة للمراجعة").unwrap();
        assert_eq!(list(&conn, 1).unwrap().len(), 1);
        assert!(list_archived(&conn, 1).unwrap().is_empty());
    }

    #[test]
    fn lifecycle_audit_uses_attachment_entity_and_before_after() {
        let conn = db();
        let id = add_named(
            &conn,
            1,
            "audit.pdf",
            "audit.pdf",
            "تقرير مراجعة",
            Some("تقارير"),
            Some("application/pdf"),
            10,
            &"b".repeat(64),
        )
        .unwrap();
        let (entity_type, entity_id, after_added): (String, i64, String) = conn
            .query_row(
                "SELECT entity_type,entity_id,after_json FROM audit_log WHERE event_type='attachment_added' ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(entity_type, "attachment");
        assert_eq!(entity_id, id);
        assert!(after_added.contains("تقرير مراجعة"));

        archive(&conn, id, "مراجعة منتهية").unwrap();
        let (before_archive, after_archive, archive_reason): (String, String, String) = conn
            .query_row(
                "SELECT before_json,after_json,reason FROM audit_log WHERE event_type='attachment_archived' ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert!(before_archive.contains("\"deletedAt\":null"));
        assert!(after_archive.contains("مراجعة منتهية"));
        assert_eq!(archive_reason, "مراجعة منتهية");

        restore(&conn, id, "إعادة فتح المرفق").unwrap();
        let (before_restore, after_restore, restore_reason): (String, String, String) = conn
            .query_row(
                "SELECT before_json,after_json,reason FROM audit_log WHERE event_type='attachment_restored' ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert!(before_restore.contains("مراجعة منتهية"));
        assert!(after_restore.contains("\"deletedAt\":null"));
        assert_eq!(restore_reason, "إعادة فتح المرفق");
    }

    #[test]
    fn enforces_twenty_active_attachments() {
        let conn = db();
        for i in 0..20 {
            add(
                &conn,
                1,
                &format!("{i}.pdf"),
                "x.pdf",
                None,
                1,
                &"a".repeat(64),
            )
            .unwrap();
        }
        assert!(add(&conn, 1, "20.pdf", "x.pdf", None, 1, &"a".repeat(64)).is_err());
    }

    #[test]
    fn archive_frees_slot_but_restore_respects_limit() {
        let conn = db();
        let first = add(&conn, 1, "first.pdf", "x.pdf", None, 1, &"a".repeat(64)).unwrap();
        for i in 1..20 {
            add(
                &conn,
                1,
                &format!("{i}.pdf"),
                "x.pdf",
                None,
                1,
                &"a".repeat(64),
            )
            .unwrap();
        }
        archive(&conn, first, "أرشفة").unwrap();
        add(
            &conn,
            1,
            "replacement.pdf",
            "x.pdf",
            None,
            1,
            &"a".repeat(64),
        )
        .unwrap();
        assert!(restore(&conn, first, "محاولة استعادة").is_err());
    }

    #[test]
    fn blocks_executable_and_active_content_extensions() {
        for name in [
            "evil.exe",
            "script.ps1",
            "active.html",
            "vector.svg",
            "help.chm",
        ] {
            assert!(
                extension(Path::new(name)).is_err(),
                "{name} must be blocked"
            );
        }
        for name in [
            "report.pdf",
            "scan.jpg",
            "note.docx",
            "sheet.xlsx",
            "slides.pptx",
            "image.tiff",
            "scan.dcm",
        ] {
            assert!(extension(Path::new(name)).is_ok(), "{name} must be allowed");
        }
    }

    #[test]
    fn common_formats_get_specific_mime_types() {
        assert_eq!(mime_for("pdf"), "application/pdf");
        assert_eq!(
            mime_for("pptx"),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        );
        assert_eq!(mime_for("dcm"), "application/dicom");
        assert_eq!(mime_for("unknownsafe"), "application/octet-stream");
    }

    #[test]
    fn rejects_path_traversal() {
        let conn = db();
        assert!(add(&conn, 1, "../x.pdf", "x.pdf", None, 1, &"a".repeat(64)).is_err());
    }
}
