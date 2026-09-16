use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub id: i64,
    pub event_type: String,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub details_json: Option<String>,
    pub created_at: String,
}

pub fn record(
    conn: &Connection,
    event_type: &str,
    entity_type: &str,
    entity_id: Option<i64>,
    details_json: Option<&str>,
) -> Result<(), String> {
    if event_type.trim().is_empty() || entity_type.trim().is_empty() {
        return Err("نوع حدث التدقيق والكيان مطلوبان".into());
    }
    conn.execute(
        "INSERT INTO audit_log(event_type,entity_type,entity_id,details_json) VALUES(?1,?2,?3,?4)",
        params![event_type, entity_type, entity_id, details_json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn recent(conn: &Connection, limit: i64) -> Result<Vec<AuditEntry>, String> {
    let limit = limit.clamp(1, 500);
    let mut stmt = conn
        .prepare(
            "SELECT id,event_type,entity_type,entity_id,details_json,created_at
             FROM audit_log ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([limit], |r| {
            Ok(AuditEntry {
                id: r.get(0)?,
                event_type: r.get(1)?,
                entity_type: r.get(2)?,
                entity_id: r.get(3)?,
                details_json: r.get(4)?,
                created_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_lists_audit_events() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE audit_log(id INTEGER PRIMARY KEY,event_type TEXT NOT NULL,entity_type TEXT NOT NULL,entity_id INTEGER,details_json TEXT,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);",
        )
        .unwrap();
        record(
            &db,
            "backup_created",
            "database",
            None,
            Some("{\"sha256\":\"abc\"}"),
        )
        .unwrap();
        let events = recent(&db, 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "backup_created");
        assert_eq!(events[0].entity_type, "database");
    }

    #[test]
    fn rejects_blank_event_metadata() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE audit_log(id INTEGER PRIMARY KEY,event_type TEXT NOT NULL,entity_type TEXT NOT NULL,entity_id INTEGER,details_json TEXT,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);",
        )
        .unwrap();
        assert!(record(&db, "", "database", None, None).is_err());
    }
}
