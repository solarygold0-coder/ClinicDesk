use crate::audit;
use rusqlite::{params, Connection, OptionalExtension};

/// Defines the only status transitions allowed by the scheduling workflow.
/// Keeping this policy separate makes it reusable by the local backend and
/// the future network API service.
pub fn transition_allowed(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    match from {
        "scheduled" => matches!(to, "arrived" | "cancelled" | "no_show"),
        "arrived" => matches!(to, "in_progress" | "cancelled"),
        "in_progress" => matches!(to, "completed" | "cancelled"),
        "completed" | "cancelled" | "no_show" => false,
        _ => false,
    }
}

/// Changes an appointment status only when the requested transition is valid.
/// The read, validation, update and audit entry happen in one transaction so
/// this rule can later be moved unchanged behind the network API boundary.
pub fn set_status(c: &mut Connection, id: i64, to: &str) -> Result<(), String> {
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let from: Option<String> = tx
        .query_row("SELECT status FROM appointments WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())?;
    let from = from.ok_or_else(|| "الموعد غير موجود".to_string())?;
    if !transition_allowed(&from, to) {
        return Err(format!("لا يمكن تغيير حالة الموعد من {from} إلى {to}"));
    }
    if from == to {
        return Ok(());
    }
    tx.execute(
        "UPDATE appointments SET status=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
        params![to, id],
    )
    .map_err(|e| e.to_string())?;

    let before_json = serde_json::json!({"status": from}).to_string();
    let after_json = serde_json::json!({"status": to}).to_string();
    let details = serde_json::json!({"from": from, "to": to}).to_string();
    audit::record_as(
        &tx,
        "appointment_status_changed",
        "appointment",
        Some(id),
        Some(&details),
        &audit::AuditActor::default(),
        &audit::AuditChange {
            before_json: Some(&before_json),
            after_json: Some(&after_json),
            reason: None,
        },
    )?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{set_status, transition_allowed};
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE appointments(id INTEGER PRIMARY KEY,status TEXT NOT NULL,updated_at TEXT);
             CREATE TABLE audit_log(
                id INTEGER PRIMARY KEY,
                event_type TEXT,
                entity_type TEXT,
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
             );
             INSERT INTO appointments(id,status) VALUES(1,'scheduled');",
        )
        .unwrap();
        c
    }

    #[test]
    fn normal_visit_flow_is_allowed() {
        assert!(transition_allowed("scheduled", "arrived"));
        assert!(transition_allowed("arrived", "in_progress"));
        assert!(transition_allowed("in_progress", "completed"));
    }

    #[test]
    fn invalid_shortcuts_are_blocked() {
        assert!(!transition_allowed("scheduled", "completed"));
        assert!(!transition_allowed("scheduled", "in_progress"));
        assert!(!transition_allowed("arrived", "completed"));
    }

    #[test]
    fn terminal_states_cannot_be_reopened() {
        for state in ["completed", "cancelled", "no_show"] {
            assert!(!transition_allowed(state, "scheduled"));
            assert!(!transition_allowed(state, "arrived"));
        }
    }

    #[test]
    fn cancellation_and_no_show_rules_are_explicit() {
        assert!(transition_allowed("scheduled", "cancelled"));
        assert!(transition_allowed("scheduled", "no_show"));
        assert!(transition_allowed("arrived", "cancelled"));
        assert!(transition_allowed("in_progress", "cancelled"));
        assert!(!transition_allowed("arrived", "no_show"));
    }

    #[test]
    fn database_enforces_full_visit_flow_and_audits_it() {
        let mut c = db();
        set_status(&mut c, 1, "arrived").unwrap();
        set_status(&mut c, 1, "in_progress").unwrap();
        set_status(&mut c, 1, "completed").unwrap();
        let status: String = c
            .query_row("SELECT status FROM appointments WHERE id=1", [], |r| {
                r.get(0)
            })
            .unwrap();
        let audit_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE entity_id=1 AND event_type='appointment_status_changed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "completed");
        assert_eq!(audit_count, 3);
    }

    #[test]
    fn status_audit_captures_before_and_after() {
        let mut c = db();
        set_status(&mut c, 1, "arrived").unwrap();
        let (before, after): (String, String) = c
            .query_row(
                "SELECT before_json,after_json FROM audit_log WHERE entity_id=1 ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(before.contains("scheduled"));
        assert!(after.contains("arrived"));
    }

    #[test]
    fn database_blocks_skipping_and_terminal_reopen() {
        let mut c = db();
        assert!(set_status(&mut c, 1, "completed").is_err());
        set_status(&mut c, 1, "arrived").unwrap();
        set_status(&mut c, 1, "in_progress").unwrap();
        set_status(&mut c, 1, "completed").unwrap();
        assert!(set_status(&mut c, 1, "scheduled").is_err());
        let status: String = c
            .query_row("SELECT status FROM appointments WHERE id=1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "completed");
    }
}
