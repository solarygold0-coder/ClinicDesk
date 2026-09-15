use rusqlite::{params, Connection};
use serde::Serialize;

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
    let mut stmt = conn
        .prepare(
            "SELECT id,patient_id,stored_name,original_name,mime_type,size_bytes,sha256,created_at
             FROM attachments WHERE patient_id=?1 ORDER BY created_at DESC,id DESC",
        )
        .map_err(|e| e.to_string())?;
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

pub fn add(
    conn: &Connection,
    patient_id: i64,
    stored_name: &str,
    original_name: &str,
    mime_type: Option<&str>,
    size_bytes: i64,
    sha256: &str,
) -> Result<i64, String> {
    if original_name.trim().is_empty() || stored_name.trim().is_empty() || sha256.len() != 64 {
        return Err("بيانات المرفق غير صالحة".into());
    }
    if size_bytes <= 0 || size_bytes > 25 * 1024 * 1024 {
        return Err("حجم المرفق يجب أن يكون بين 1 بايت و25 ميجابايت".into());
    }
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM patients WHERE id=?1 AND deleted_at IS NULL",
            params![patient_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        return Err("ملف المريض غير موجود".into());
    }
    conn.execute(
        "INSERT INTO attachments(patient_id,stored_name,original_name,mime_type,size_bytes,sha256)
         VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            patient_id,
            stored_name,
            original_name,
            mime_type,
            size_bytes,
            sha256
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO audit_log(event_type,entity_type,entity_id,details_json)
         VALUES('attachment_added','patient',?1,?2)",
        params![patient_id, format!("{\"attachmentId\":{id}}")],
    )
    .map_err(|e| e.to_string())?;
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
    conn.execute(
        "INSERT INTO audit_log(event_type,entity_type,entity_id,details_json)
         VALUES('attachment_removed','patient',?1,?2)",
        params![patient_id, format!("{\"attachmentId\":{id}}")],
    )
    .map_err(|e| e.to_string())?;
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
        assert!(add(&conn, 1, "x", "x", None, 26 * 1024 * 1024, &"a".repeat(64)).is_err());
    }
}
