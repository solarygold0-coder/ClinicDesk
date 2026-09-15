use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitTrackingInput {
    pub visit_type: String,
    pub visit_stage: String,
    pub follow_up_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowUpVisit {
    pub appointment_id: i64,
    pub patient_id: i64,
    pub file_no: i64,
    pub patient_name: String,
    pub follow_up_at: String,
    pub visit_type: String,
}

fn validate(input: &VisitTrackingInput) -> Result<(), String> {
    if !["new", "follow_up", "renewal"].contains(&input.visit_type.as_str()) {
        return Err("نوع الزيارة غير صالح".into());
    }
    if !["scheduled", "reception", "with_doctor", "completed"]
        .contains(&input.visit_stage.as_str())
    {
        return Err("مرحلة الزيارة غير صالحة".into());
    }
    if input
        .follow_up_at
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err("تاريخ المتابعة غير صالح".into());
    }
    Ok(())
}

pub fn update(c: &Connection, id: i64, input: VisitTrackingInput) -> Result<(), String> {
    validate(&input)?;
    let follow_up_at = input
        .follow_up_at
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let changed = c
        .execute(
            "UPDATE appointments SET visit_type=?1,visit_stage=?2,follow_up_at=?3,updated_at=CURRENT_TIMESTAMP WHERE id=?4",
            params![input.visit_type, input.visit_stage, follow_up_at, id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("الموعد غير موجود".into());
    }
    c.execute(
        "INSERT INTO audit_log(event_type,entity_type,entity_id,details_json) VALUES('visit_tracking','appointment',?1,?2)",
        params![id, format!("{{\"visitType\":\"{}\",\"visitStage\":\"{}\"}}", input.visit_type, input.visit_stage)],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn follow_ups(c: &Connection, from: &str, to: &str) -> Result<Vec<FollowUpVisit>, String> {
    let mut statement = c
        .prepare(
            "SELECT a.id,a.patient_id,p.file_no,p.full_name,a.follow_up_at,a.visit_type FROM appointments a JOIN patients p ON p.id=a.patient_id WHERE p.deleted_at IS NULL AND a.follow_up_at IS NOT NULL AND a.follow_up_at>=?1 AND a.follow_up_at<?2 ORDER BY a.follow_up_at",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![from, to], |row| {
            Ok(FollowUpVisit {
                appointment_id: row.get(0)?,
                patient_id: row.get(1)?,
                file_no: row.get(2)?,
                patient_name: row.get(3)?,
                follow_up_at: row.get(4)?,
                visit_type: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        c.execute_batch(include_str!("../migrations/007_appointment_visit_tracking.sql"))
            .unwrap();
        c.execute("INSERT INTO patients(file_no,full_name) VALUES(1,'مريض')", [])
            .unwrap();
        c.execute("INSERT INTO appointments(patient_id,starts_at,ends_at) VALUES(1,'2026-09-20T10:00:00','2026-09-20T10:30:00')", [])
            .unwrap();
        c
    }

    #[test]
    fn visit_tracking_is_saved_and_follow_up_is_queryable() {
        let c = db();
        update(
            &c,
            1,
            VisitTrackingInput {
                visit_type: "follow_up".into(),
                visit_stage: "reception".into(),
                follow_up_at: Some("2026-10-01T10:00:00".into()),
            },
        )
        .unwrap();
        let rows = follow_ups(&c, "2026-10-01T00:00:00", "2026-10-02T00:00:00").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].file_no, 1);
    }

    #[test]
    fn invalid_visit_type_is_rejected() {
        let c = db();
        let result = update(
            &c,
            1,
            VisitTrackingInput {
                visit_type: "invalid".into(),
                visit_stage: "scheduled".into(),
                follow_up_at: None,
            },
        );
        assert!(result.is_err());
    }
}
