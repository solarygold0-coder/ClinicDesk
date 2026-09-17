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
    pub actor_user_id: Option<i64>,
    pub actor_display_name: Option<String>,
    pub actor_employee_code: Option<String>,
    pub actor_session_id: Option<String>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Default)]
pub struct AuditActor<'a> {
    pub user_id: Option<i64>,
    pub display_name: Option<&'a str>,
    pub employee_code: Option<&'a str>,
    pub session_id: Option<&'a str>,
}

#[derive(Debug, Default)]
pub struct AuditChange<'a> {
    pub before_json: Option<&'a str>,
    pub after_json: Option<&'a str>,
    pub reason: Option<&'a str>,
}

pub fn record(
    conn: &Connection,
    event_type: &str,
    entity_type: &str,
    entity_id: Option<i64>,
    details_json: Option<&str>,
) -> Result<(), String> {
    record_as(
        conn,
        event_type,
        entity_type,
        entity_id,
        details_json,
        &AuditActor::default(),
        &AuditChange::default(),
    )
}

pub fn record_as(
    conn: &Connection,
    event_type: &str,
    entity_type: &str,
    entity_id: Option<i64>,
    details_json: Option<&str>,
    actor: &AuditActor<'_>,
    change: &AuditChange<'_>,
) -> Result<(), String> {
    if event_type.trim().is_empty() || entity_type.trim().is_empty() {
        return Err("نوع حدث التدقيق والكيان مطلوبان".into());
    }
    let result = conn.execute(
        "INSERT INTO audit_log(
            event_type,entity_type,entity_id,details_json,
            actor_user_id,actor_display_name,actor_employee_code,actor_session_id,
            before_json,after_json,reason
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            event_type,
            entity_type,
            entity_id,
            details_json,
            actor.user_id,
            actor.display_name,
            actor.employee_code,
            actor.session_id,
            change.before_json,
            change.after_json,
            change.reason,
        ],
    );
    match result {
        Ok(_) => Ok(()),
        Err(error) if error.to_string().contains("no column named actor_") || error.to_string().contains("no column named before_json") => conn
            .execute(
                "INSERT INTO audit_log(event_type,entity_type,entity_id,details_json) VALUES(?1,?2,?3,?4)",
                params![event_type, entity_type, entity_id, details_json],
            )
            .map(|_| ())
            .map_err(|e| e.to_string()),
        Err(error) => Err(error.to_string()),
    }
}

pub fn recent(conn: &Connection, limit: i64) -> Result<Vec<AuditEntry>, String> {
    let limit = limit.clamp(1, 500);
    let mut stmt = conn
        .prepare(
            "SELECT id,event_type,entity_type,entity_id,details_json,
                    actor_user_id,actor_display_name,actor_employee_code,actor_session_id,
                    before_json,after_json,reason,created_at
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
                actor_user_id: r.get(5)?,
                actor_display_name: r.get(6)?,
                actor_employee_code: r.get(7)?,
                actor_session_id: r.get(8)?,
                before_json: r.get(9)?,
                after_json: r.get(10)?,
                reason: r.get(11)?,
                created_at: r.get(12)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audit_db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE audit_log(
                id INTEGER PRIMARY KEY,
                event_type TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id INTEGER,
                details_json TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                actor_user_id INTEGER,
                actor_display_name TEXT,
                actor_employee_code TEXT,
                actor_session_id TEXT,
                before_json TEXT,
                after_json TEXT,
                reason TEXT
            );",
        )
        .unwrap();
        db
    }

    #[test]
    fn records_and_lists_audit_events() {
        let db = audit_db();
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
    fn records_actor_session_and_before_after_values() {
        let db = audit_db();
        let actor = AuditActor {
            user_id: Some(7),
            display_name: Some("موظف السجلات"),
            employee_code: Some("U000007"),
            session_id: Some("session-7"),
        };
        let change = AuditChange {
            before_json: Some("{\"status\":\"scheduled\"}"),
            after_json: Some("{\"status\":\"cancelled\"}"),
            reason: Some("تعذر المعالج"),
        };
        record_as(
            &db,
            "appointment_cancelled",
            "appointment",
            Some(44),
            None,
            &actor,
            &change,
        )
        .unwrap();

        let event = recent(&db, 1).unwrap().remove(0);
        assert_eq!(event.actor_user_id, Some(7));
        assert_eq!(event.actor_employee_code.as_deref(), Some("U000007"));
        assert_eq!(event.actor_session_id.as_deref(), Some("session-7"));
        assert_eq!(event.reason.as_deref(), Some("تعذر المعالج"));
        assert!(event.before_json.is_some());
        assert!(event.after_json.is_some());
    }

    #[test]
    fn rejects_blank_event_metadata() {
        let db = audit_db();
        assert!(record(&db, "", "database", None, None).is_err());
    }
}
