use crate::audit;
use chrono::{Datelike, NaiveDateTime, NaiveTime, Weekday};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const ACTIVE_STATUSES: &str = "('scheduled','arrived')";
const REASON_CODES: &[&str] = &[
    "leave",
    "sudden_absence",
    "assignment_meeting",
    "emergency",
    "other",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUnavailabilityEvent {
    pub id: i64,
    pub doctor_id: i64,
    pub unavailable_from: String,
    pub unavailable_to: String,
    pub reason_code: String,
    pub reason_note: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUnavailabilityInput {
    pub doctor_id: i64,
    pub unavailable_from: String,
    pub unavailable_to: String,
    pub reason_code: String,
    pub reason_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedAppointment {
    pub appointment_id: i64,
    pub patient_id: i64,
    pub file_no: i64,
    pub patient_name: String,
    pub clinic_id: Option<i64>,
    pub doctor_id: Option<i64>,
    pub starts_at: String,
    pub ends_at: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionInput {
    pub appointment_id: i64,
    pub action_type: String,
    pub replacement_doctor_id: Option<i64>,
    pub new_starts_at: Option<String>,
}

#[derive(Debug, Clone)]
struct AppointmentSnapshot {
    id: i64,
    patient_id: i64,
    clinic_id: Option<i64>,
    doctor_id: Option<i64>,
    starts_at: String,
    ends_at: String,
    status: String,
    notes: Option<String>,
}

fn parse_datetime(value: &str) -> Result<NaiveDateTime, String> {
    let parsed = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M"))
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| "تاريخ أو وقت تعذر المعالج غير صالح".to_string())?;
    if !(1950..=2050).contains(&parsed.year()) {
        return Err("التاريخ يجب أن يكون بين 1950 و2050".into());
    }
    Ok(parsed)
}

fn normalized_datetime(value: &str) -> Result<String, String> {
    Ok(parse_datetime(value)?
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string())
}

fn clean_note(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn validate_reason(code: &str, note: Option<String>) -> Result<(String, Option<String>), String> {
    let code = code.trim().to_string();
    if !REASON_CODES.contains(&code.as_str()) {
        return Err("سبب تعذر المعالج غير صالح".into());
    }
    let note = clean_note(note);
    if code == "other" && note.is_none() {
        return Err("الملاحظة إلزامية عند اختيار سبب آخر".into());
    }
    if note.as_ref().is_some_and(|v| v.chars().count() > 250) {
        return Err("ملاحظة تعذر المعالج طويلة جدًا".into());
    }
    Ok((code, note))
}

fn event_by_id(c: &Connection, id: i64) -> Result<ProviderUnavailabilityEvent, String> {
    c.query_row(
        "SELECT id,doctor_id,unavailable_from,unavailable_to,reason_code,reason_note,created_at,resolved_at FROM provider_unavailability_events WHERE id=?1",
        [id],
        |r| {
            Ok(ProviderUnavailabilityEvent {
                id: r.get(0)?,
                doctor_id: r.get(1)?,
                unavailable_from: r.get(2)?,
                unavailable_to: r.get(3)?,
                reason_code: r.get(4)?,
                reason_note: r.get(5)?,
                created_at: r.get(6)?,
                resolved_at: r.get(7)?,
            })
        },
    )
    .map_err(|_| "حدث تعذر المعالج غير موجود".to_string())
}

fn appointment_snapshot(c: &Connection, id: i64) -> Result<AppointmentSnapshot, String> {
    c.query_row(
        "SELECT id,patient_id,clinic_id,doctor_id,starts_at,ends_at,status,notes FROM appointments WHERE id=?1",
        [id],
        |r| {
            Ok(AppointmentSnapshot {
                id: r.get(0)?,
                patient_id: r.get(1)?,
                clinic_id: r.get(2)?,
                doctor_id: r.get(3)?,
                starts_at: r.get(4)?,
                ends_at: r.get(5)?,
                status: r.get(6)?,
                notes: r.get(7)?,
            })
        },
    )
    .map_err(|_| "الموعد غير موجود".to_string())
}

fn snapshot_json(value: &AppointmentSnapshot) -> String {
    serde_json::json!({
        "appointmentId": value.id,
        "patientId": value.patient_id,
        "clinicId": value.clinic_id,
        "doctorId": value.doctor_id,
        "startsAt": value.starts_at,
        "endsAt": value.ends_at,
        "status": value.status,
        "notes": value.notes,
    })
    .to_string()
}

fn ensure_doctor(c: &Connection, id: i64) -> Result<Option<i64>, String> {
    c.query_row(
        "SELECT clinic_id FROM doctors WHERE id=?1 AND is_active=1",
        [id],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "المعالج البديل غير موجود أو غير نشط".to_string())
}

fn parse_time(value: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| "إعداد وقت الدوام غير صالح".into())
}

fn validate_work_window(c: &Connection, start: NaiveDateTime, end: NaiveDateTime) -> Result<(), String> {
    if matches!(start.weekday(), Weekday::Fri | Weekday::Sat) {
        return Err("لا يمكن جدولة الموعد يوم الجمعة أو السبت".into());
    }
    let closed: Option<i64> = c
        .query_row(
            "SELECT id FROM closure_dates WHERE closure_date=?1",
            [start.date().format("%Y-%m-%d").to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if closed.is_some() {
        return Err("اليوم المحدد إجازة أو يوم إغلاق ولا يقبل مواعيد".into());
    }
    let (work_start, work_end, break_start, break_end): (String, String, Option<String>, Option<String>) = c
        .query_row(
            "SELECT work_start,work_end,break_start,break_end FROM scheduling_settings WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| e.to_string())?;
    let ws = parse_time(&work_start)?;
    let we = parse_time(&work_end)?;
    if start.time() < ws || end.time() > we {
        return Err(format!("الموعد خارج ساعات الدوام من {work_start} إلى {work_end}"));
    }
    if let (Some(bs), Some(be)) = (break_start, break_end) {
        let bs = parse_time(&bs)?;
        let be = parse_time(&be)?;
        if start.time() < be && end.time() > bs {
            return Err("الموعد يتعارض مع فترة الاستراحة".into());
        }
    }
    Ok(())
}

fn ensure_capacity(
    c: &Connection,
    doctor_id: i64,
    starts_at: &str,
    ends_at: &str,
    exclude_appointment_id: Option<i64>,
) -> Result<(), String> {
    let conflict: Option<i64> = c
        .query_row(
            "SELECT id FROM appointments
             WHERE doctor_id=?1
               AND status IN ('scheduled','arrived','in_progress')
               AND starts_at<?2 AND ends_at>?3
               AND (?4 IS NULL OR id<>?4)
             LIMIT 1",
            params![doctor_id, ends_at, starts_at, exclude_appointment_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if conflict.is_some() {
        Err("المعالج البديل لديه موعد متعارض في نفس الفترة".into())
    } else {
        Ok(())
    }
}

fn ensure_event_appointment(event: &ProviderUnavailabilityEvent, appointment: &AppointmentSnapshot) -> Result<(), String> {
    if appointment.doctor_id != Some(event.doctor_id)
        || appointment.starts_at >= event.unavailable_to
        || appointment.ends_at <= event.unavailable_from
    {
        return Err("الموعد لا يقع ضمن فترة تعذر هذا المعالج".into());
    }
    if !matches!(appointment.status.as_str(), "scheduled" | "arrived") {
        return Err("لا يمكن معالجة موعد منتهٍ أو جارٍ تنفيذه ضمن تعذر المعالج".into());
    }
    Ok(())
}

pub fn create_event(c: &Connection, input: ProviderUnavailabilityInput) -> Result<ProviderUnavailabilityEvent, String> {
    let doctor_exists: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM doctors WHERE id=?1 AND is_active=1",
            [input.doctor_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if doctor_exists == 0 {
        return Err("المعالج غير موجود أو غير نشط".into());
    }
    let from = normalized_datetime(&input.unavailable_from)?;
    let to = normalized_datetime(&input.unavailable_to)?;
    if to <= from {
        return Err("نهاية فترة تعذر المعالج يجب أن تكون بعد بدايتها".into());
    }
    let (reason_code, reason_note) = validate_reason(&input.reason_code, input.reason_note)?;
    let overlap: Option<i64> = c
        .query_row(
            "SELECT id FROM provider_unavailability_events
             WHERE doctor_id=?1 AND resolved_at IS NULL AND unavailable_from<?2 AND unavailable_to>?3 LIMIT 1",
            params![input.doctor_id, to, from],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if overlap.is_some() {
        return Err("توجد فترة تعذر مفتوحة ومتداخلة لهذا المعالج".into());
    }
    c.execute(
        "INSERT INTO provider_unavailability_events(doctor_id,unavailable_from,unavailable_to,reason_code,reason_note) VALUES(?1,?2,?3,?4,?5)",
        params![input.doctor_id, from, to, reason_code, reason_note],
    )
    .map_err(|e| e.to_string())?;
    let event = event_by_id(c, c.last_insert_rowid())?;
    let after = serde_json::to_string(&event).map_err(|e| e.to_string())?;
    let reason = event.reason_note.as_deref().unwrap_or(&event.reason_code);
    audit::record_as(
        c,
        "provider_unavailability_created",
        "provider_unavailability",
        Some(event.id),
        None,
        &audit::AuditActor::default(),
        &audit::AuditChange {
            before_json: None,
            after_json: Some(&after),
            reason: Some(reason),
        },
    )?;
    Ok(event)
}

pub fn affected_appointments(c: &Connection, event_id: i64) -> Result<Vec<AffectedAppointment>, String> {
    let event = event_by_id(c, event_id)?;
    let sql = format!(
        "SELECT a.id,a.patient_id,p.file_no,p.full_name,a.clinic_id,a.doctor_id,a.starts_at,a.ends_at,a.status
         FROM appointments a JOIN patients p ON p.id=a.patient_id
         WHERE a.doctor_id=?1 AND a.status IN {ACTIVE_STATUSES}
           AND a.starts_at<?2 AND a.ends_at>?3
         ORDER BY a.starts_at"
    );
    let mut stmt = c.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![event.doctor_id, event.unavailable_to, event.unavailable_from], |r| {
            Ok(AffectedAppointment {
                appointment_id: r.get(0)?,
                patient_id: r.get(1)?,
                file_no: r.get(2)?,
                patient_name: r.get(3)?,
                clinic_id: r.get(4)?,
                doctor_id: r.get(5)?,
                starts_at: r.get(6)?,
                ends_at: r.get(7)?,
                status: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn record_action(
    c: &Connection,
    event: &ProviderUnavailabilityEvent,
    before: &AppointmentSnapshot,
    after: &AppointmentSnapshot,
    action_type: &str,
    replacement_doctor_id: Option<i64>,
    new_starts_at: Option<&str>,
    replacement_appointment_id: Option<i64>,
) -> Result<(), String> {
    c.execute(
        "INSERT INTO provider_unavailability_actions(
            event_id,appointment_id,action_type,original_doctor_id,replacement_doctor_id,
            original_starts_at,new_starts_at,replacement_appointment_id,reason_code,reason_note
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            event.id,
            before.id,
            action_type,
            before.doctor_id,
            replacement_doctor_id,
            before.starts_at,
            new_starts_at,
            replacement_appointment_id,
            event.reason_code,
            event.reason_note,
        ],
    )
    .map_err(|e| e.to_string())?;
    let before_json = snapshot_json(before);
    let after_json = snapshot_json(after);
    let details = serde_json::json!({
        "eventId": event.id,
        "action": action_type,
        "originalDoctorId": before.doctor_id,
        "replacementDoctorId": replacement_doctor_id,
        "oldStartsAt": before.starts_at,
        "newStartsAt": new_starts_at,
        "replacementAppointmentId": replacement_appointment_id,
    })
    .to_string();
    let reason = event.reason_note.as_deref().unwrap_or(&event.reason_code);
    audit::record_as(
        c,
        &format!("provider_unavailability_{action_type}"),
        "appointment",
        Some(before.id),
        Some(&details),
        &audit::AuditActor::default(),
        &audit::AuditChange {
            before_json: Some(&before_json),
            after_json: Some(&after_json),
            reason: Some(reason),
        },
    )
}

fn transfer(
    c: &Connection,
    event: &ProviderUnavailabilityEvent,
    appointment_id: i64,
    replacement_doctor_id: i64,
) -> Result<(), String> {
    let before = appointment_snapshot(c, appointment_id)?;
    ensure_event_appointment(event, &before)?;
    if replacement_doctor_id == event.doctor_id {
        return Err("يجب اختيار معالج بديل مختلف".into());
    }
    let replacement_clinic = ensure_doctor(c, replacement_doctor_id)?;
    let start = parse_datetime(&before.starts_at)?;
    let end = parse_datetime(&before.ends_at)?;
    validate_work_window(c, start, end)?;
    ensure_capacity(c, replacement_doctor_id, &before.starts_at, &before.ends_at, Some(appointment_id))?;
    let clinic_id = replacement_clinic.or(before.clinic_id);
    c.execute(
        "UPDATE appointments SET doctor_id=?2,clinic_id=?3,
             disruption_kind='provider_unavailable',disruption_reason_code=?4,disruption_reason_note=?5,
             disruption_resolution='transfer',provider_unavailability_event_id=?6,updated_at=CURRENT_TIMESTAMP
         WHERE id=?1",
        params![appointment_id, replacement_doctor_id, clinic_id, event.reason_code, event.reason_note, event.id],
    )
    .map_err(|e| e.to_string())?;
    let after = appointment_snapshot(c, appointment_id)?;
    record_action(c, event, &before, &after, "transfer", Some(replacement_doctor_id), Some(&after.starts_at), None)
}

fn cancel(c: &Connection, event: &ProviderUnavailabilityEvent, appointment_id: i64) -> Result<(), String> {
    let before = appointment_snapshot(c, appointment_id)?;
    ensure_event_appointment(event, &before)?;
    c.execute(
        "UPDATE appointments SET status='cancelled',
             disruption_kind='provider_unavailable',disruption_reason_code=?2,disruption_reason_note=?3,
             disruption_resolution='cancel',provider_unavailability_event_id=?4,updated_at=CURRENT_TIMESTAMP
         WHERE id=?1",
        params![appointment_id, event.reason_code, event.reason_note, event.id],
    )
    .map_err(|e| e.to_string())?;
    let after = appointment_snapshot(c, appointment_id)?;
    record_action(c, event, &before, &after, "cancel", None, None, None)
}

fn reschedule(
    c: &Connection,
    event: &ProviderUnavailabilityEvent,
    appointment_id: i64,
    replacement_doctor_id: Option<i64>,
    new_starts_at: &str,
) -> Result<(), String> {
    let before = appointment_snapshot(c, appointment_id)?;
    ensure_event_appointment(event, &before)?;
    let old_start = parse_datetime(&before.starts_at)?;
    let old_end = parse_datetime(&before.ends_at)?;
    let duration = old_end - old_start;
    let new_start = parse_datetime(new_starts_at)?;
    let new_end = new_start + duration;
    validate_work_window(c, new_start, new_end)?;

    let target_doctor = replacement_doctor_id.or(before.doctor_id).ok_or_else(|| "المعالج غير محدد".to_string())?;
    let replacement_clinic = ensure_doctor(c, target_doctor)?;
    if target_doctor == event.doctor_id {
        let event_from = parse_datetime(&event.unavailable_from)?;
        let event_to = parse_datetime(&event.unavailable_to)?;
        if new_start < event_to && new_end > event_from {
            return Err("الموعد الجديد ما زال داخل فترة تعذر المعالج".into());
        }
    }
    let new_start_text = new_start.format("%Y-%m-%dT%H:%M:%S").to_string();
    let new_end_text = new_end.format("%Y-%m-%dT%H:%M:%S").to_string();
    ensure_capacity(c, target_doctor, &new_start_text, &new_end_text, None)?;
    let clinic_id = replacement_clinic.or(before.clinic_id);

    c.execute(
        "INSERT INTO appointments(
            patient_id,clinic_id,doctor_id,starts_at,ends_at,status,notes,
            disruption_kind,disruption_reason_code,disruption_reason_note,disruption_resolution,
            provider_unavailability_event_id,original_appointment_id
         ) VALUES(?1,?2,?3,?4,?5,'scheduled',?6,'provider_unavailable',?7,?8,'reschedule',?9,?10)",
        params![
            before.patient_id,
            clinic_id,
            target_doctor,
            new_start_text,
            new_end_text,
            before.notes,
            event.reason_code,
            event.reason_note,
            event.id,
            before.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    let replacement_id = c.last_insert_rowid();
    c.execute(
        "UPDATE appointments SET status='cancelled',
             disruption_kind='provider_unavailable',disruption_reason_code=?2,disruption_reason_note=?3,
             disruption_resolution='reschedule',provider_unavailability_event_id=?4,
             replacement_appointment_id=?5,updated_at=CURRENT_TIMESTAMP
         WHERE id=?1",
        params![before.id, event.reason_code, event.reason_note, event.id, replacement_id],
    )
    .map_err(|e| e.to_string())?;
    let after = appointment_snapshot(c, before.id)?;
    record_action(
        c,
        event,
        &before,
        &after,
        "reschedule",
        Some(target_doctor),
        Some(&new_start_text),
        Some(replacement_id),
    )?;
    let replacement = appointment_snapshot(c, replacement_id)?;
    let replacement_after = snapshot_json(&replacement);
    let reason = event.reason_note.as_deref().unwrap_or(&event.reason_code);
    audit::record_as(
        c,
        "provider_unavailability_replacement_created",
        "appointment",
        Some(replacement_id),
        Some(&serde_json::json!({"eventId":event.id,"originalAppointmentId":before.id}).to_string()),
        &audit::AuditActor::default(),
        &audit::AuditChange {
            before_json: None,
            after_json: Some(&replacement_after),
            reason: Some(reason),
        },
    )
}

fn maybe_resolve_event(c: &Connection, event: &ProviderUnavailabilityEvent) -> Result<(), String> {
    let remaining: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM appointments
             WHERE doctor_id=?1 AND status IN ('scheduled','arrived')
               AND starts_at<?2 AND ends_at>?3",
            params![event.doctor_id, event.unavailable_to, event.unavailable_from],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if remaining == 0 {
        c.execute(
            "UPDATE provider_unavailability_events SET resolved_at=COALESCE(resolved_at,CURRENT_TIMESTAMP) WHERE id=?1",
            [event.id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn resolve_many(c: &Connection, event_id: i64, resolutions: Vec<ResolutionInput>) -> Result<(), String> {
    if resolutions.is_empty() {
        return Err("اختر موعدًا واحدًا على الأقل لمعالجة تعذر المعالج".into());
    }
    let event = event_by_id(c, event_id)?;
    if event.resolved_at.is_some() {
        return Err("تم إغلاق حدث تعذر المعالج مسبقًا".into());
    }
    let tx = c.unchecked_transaction().map_err(|e| e.to_string())?;
    for resolution in resolutions {
        match resolution.action_type.as_str() {
            "transfer" => {
                let replacement = resolution
                    .replacement_doctor_id
                    .ok_or_else(|| "المعالج البديل مطلوب للتحويل".to_string())?;
                transfer(&tx, &event, resolution.appointment_id, replacement)?;
            }
            "reschedule" => {
                let new_starts_at = resolution
                    .new_starts_at
                    .as_deref()
                    .ok_or_else(|| "موعد إعادة الجدولة مطلوب".to_string())?;
                reschedule(
                    &tx,
                    &event,
                    resolution.appointment_id,
                    resolution.replacement_doctor_id,
                    new_starts_at,
                )?;
            }
            "cancel" => cancel(&tx, &event, resolution.appointment_id)?,
            _ => return Err("إجراء معالجة تعذر المعالج غير صالح".into()),
        }
    }
    maybe_resolve_event(&tx, &event)?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        c.execute_batch(include_str!("../migrations/003_scheduling_rules.sql")).unwrap();
        c.execute_batch(include_str!("../migrations/017_provider_unavailability.sql")).unwrap();
        c.execute_batch(
            "ALTER TABLE audit_log ADD COLUMN actor_user_id INTEGER;
             ALTER TABLE audit_log ADD COLUMN actor_display_name TEXT;
             ALTER TABLE audit_log ADD COLUMN actor_employee_code TEXT;
             ALTER TABLE audit_log ADD COLUMN actor_session_id TEXT;
             ALTER TABLE audit_log ADD COLUMN before_json TEXT;
             ALTER TABLE audit_log ADD COLUMN after_json TEXT;
             ALTER TABLE audit_log ADD COLUMN reason TEXT;
             INSERT INTO patients(file_no,full_name) VALUES(1,'مريض اختبار');
             INSERT INTO clinics(id,name) VALUES(1,'عيادة أولى'),(2,'عيادة ثانية');
             INSERT INTO doctors(id,clinic_id,name) VALUES(1,1,'المعالج الأصلي'),(2,2,'المعالج البديل');
             INSERT INTO appointments(patient_id,clinic_id,doctor_id,starts_at,ends_at,status)
             VALUES(1,1,1,'2026-09-20T10:00:00','2026-09-20T10:30:00','scheduled');",
        )
        .unwrap();
        c
    }

    fn event(c: &Connection) -> ProviderUnavailabilityEvent {
        create_event(
            c,
            ProviderUnavailabilityInput {
                doctor_id: 1,
                unavailable_from: "2026-09-20T09:00".into(),
                unavailable_to: "2026-09-20T12:00".into(),
                reason_code: "sudden_absence".into(),
                reason_note: Some("غياب مفاجئ".into()),
            },
        )
        .unwrap()
    }

    #[test]
    fn other_reason_requires_note() {
        let c = db();
        assert!(create_event(
            &c,
            ProviderUnavailabilityInput {
                doctor_id: 1,
                unavailable_from: "2026-09-20T09:00".into(),
                unavailable_to: "2026-09-20T12:00".into(),
                reason_code: "other".into(),
                reason_note: None,
            }
        )
        .is_err());
    }

    #[test]
    fn affected_appointments_are_detected() {
        let c = db();
        let e = event(&c);
        let affected = affected_appointments(&c, e.id).unwrap();
        assert_eq!(affected.len(), 1);
        assert_eq!(affected[0].appointment_id, 1);
    }

    #[test]
    fn transfer_checks_replacement_capacity_and_preserves_patient_status() {
        let c = db();
        let e = event(&c);
        resolve_many(
            &c,
            e.id,
            vec![ResolutionInput {
                appointment_id: 1,
                action_type: "transfer".into(),
                replacement_doctor_id: Some(2),
                new_starts_at: None,
            }],
        )
        .unwrap();
        let (doctor_id, status, kind): (i64, String, String) = c
            .query_row(
                "SELECT doctor_id,status,disruption_kind FROM appointments WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(doctor_id, 2);
        assert_eq!(status, "scheduled");
        assert_eq!(kind, "provider_unavailable");
    }

    #[test]
    fn cancellation_is_classified_not_no_show() {
        let c = db();
        let e = event(&c);
        resolve_many(
            &c,
            e.id,
            vec![ResolutionInput {
                appointment_id: 1,
                action_type: "cancel".into(),
                replacement_doctor_id: None,
                new_starts_at: None,
            }],
        )
        .unwrap();
        let (status, kind, resolution): (String, String, String) = c
            .query_row(
                "SELECT status,disruption_kind,disruption_resolution FROM appointments WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, "cancelled");
        assert_eq!(kind, "provider_unavailable");
        assert_eq!(resolution, "cancel");
    }

    #[test]
    fn reschedule_keeps_original_and_links_replacement() {
        let c = db();
        let e = event(&c);
        resolve_many(
            &c,
            e.id,
            vec![ResolutionInput {
                appointment_id: 1,
                action_type: "reschedule".into(),
                replacement_doctor_id: None,
                new_starts_at: Some("2026-09-21T10:00".into()),
            }],
        )
        .unwrap();
        let (status, replacement_id): (String, i64) = c
            .query_row(
                "SELECT status,replacement_appointment_id FROM appointments WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "cancelled");
        let (original_id, new_status): (i64, String) = c
            .query_row(
                "SELECT original_appointment_id,status FROM appointments WHERE id=?1",
                [replacement_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(original_id, 1);
        assert_eq!(new_status, "scheduled");
    }

    #[test]
    fn batch_resolution_is_atomic() {
        let c = db();
        c.execute(
            "INSERT INTO appointments(patient_id,clinic_id,doctor_id,starts_at,ends_at,status)
             VALUES(1,1,1,'2026-09-20T11:00:00','2026-09-20T11:30:00','scheduled')",
            [],
        )
        .unwrap();
        let e = event(&c);
        let result = resolve_many(
            &c,
            e.id,
            vec![
                ResolutionInput {
                    appointment_id: 1,
                    action_type: "cancel".into(),
                    replacement_doctor_id: None,
                    new_starts_at: None,
                },
                ResolutionInput {
                    appointment_id: 2,
                    action_type: "transfer".into(),
                    replacement_doctor_id: Some(999),
                    new_starts_at: None,
                },
            ],
        );
        assert!(result.is_err());
        let status: String = c.query_row("SELECT status FROM appointments WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(status, "scheduled");
    }
}
