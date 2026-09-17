use crate::audit;
use chrono::{Datelike, NaiveDateTime, NaiveTime, Weekday};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
const ACTIVE: &str = "('scheduled','arrived','in_progress')";
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Appointment {
    pub id: i64,
    pub patient_id: i64,
    pub file_no: i64,
    pub patient_name: String,
    pub clinic_id: Option<i64>,
    pub clinic_name: Option<String>,
    pub doctor_id: Option<i64>,
    pub doctor_name: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub status: String,
    pub notes: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppointmentInput {
    pub patient_file_no: i64,
    pub clinic_id: Option<i64>,
    pub doctor_id: Option<i64>,
    pub starts_at: String,
    pub duration_minutes: i64,
    pub notes: Option<String>,
}
fn parse(v: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| "تاريخ أو وقت الموعد غير صالح".into())
}
fn validate_day(dt: NaiveDateTime) -> Result<(), String> {
    if matches!(dt.weekday(), Weekday::Fri | Weekday::Sat) {
        Err("لا يمكن حجز موعد يوم الجمعة أو السبت".into())
    } else {
        Ok(())
    }
}
fn time(v: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(v, "%H:%M").map_err(|_| "إعداد وقت الدوام غير صالح".into())
}
fn validate_schedule(
    tx: &Transaction,
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Result<(), String> {
    let closed: Option<i64> = tx
        .query_row(
            "SELECT id FROM closure_dates WHERE closure_date=?1",
            [start.date().format("%Y-%m-%d").to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if closed.is_some() {
        return Err("هذا اليوم مسجل كيوم إغلاق ولا يقبل مواعيد".into());
    }
    let (w1, w2, b1, b2): (String, String, Option<String>, Option<String>) = tx
        .query_row(
            "SELECT work_start,work_end,break_start,break_end FROM scheduling_settings WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| e.to_string())?;
    let ws = time(&w1)?;
    let we = time(&w2)?;
    if start.time() < ws || end.time() > we {
        return Err(format!("الموعد خارج ساعات الدوام من {w1} إلى {w2}"));
    }
    if let (Some(bs), Some(be)) = (b1, b2) {
        let bs = time(&bs)?;
        let be = time(&be)?;
        if start.time() < be && end.time() > bs {
            return Err("الموعد يتعارض مع فترة الاستراحة".into());
        }
    }
    Ok(())
}
fn map(r: &rusqlite::Row) -> rusqlite::Result<Appointment> {
    Ok(Appointment {
        id: r.get(0)?,
        patient_id: r.get(1)?,
        file_no: r.get(2)?,
        patient_name: r.get(3)?,
        clinic_id: r.get(4)?,
        clinic_name: r.get(5)?,
        doctor_id: r.get(6)?,
        doctor_name: r.get(7)?,
        starts_at: r.get(8)?,
        ends_at: r.get(9)?,
        status: r.get(10)?,
        notes: r.get(11)?,
    })
}
const SELECT:&str="SELECT a.id,a.patient_id,p.file_no,p.full_name,a.clinic_id,c.name,a.doctor_id,d.name,a.starts_at,a.ends_at,a.status,a.notes FROM appointments a JOIN patients p ON p.id=a.patient_id LEFT JOIN clinics c ON c.id=a.clinic_id LEFT JOIN doctors d ON d.id=a.doctor_id";
pub fn list(c: &Connection, from: &str, to: &str) -> Result<Vec<Appointment>, String> {
    let sql = format!("{SELECT} WHERE a.starts_at>=?1 AND a.starts_at<?2 ORDER BY a.starts_at");
    let mut s = c.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = s
        .query_map(params![from, to], map)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
pub fn list_for_patient(
    c: &Connection,
    patient_id: i64,
    limit: i64,
) -> Result<Vec<Appointment>, String> {
    let sql = format!("{SELECT} WHERE a.patient_id=?1 ORDER BY a.starts_at DESC LIMIT ?2");
    let mut s = c.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = s
        .query_map(params![patient_id, limit.clamp(1, 100)], map)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
fn ensure_refs(tx: &Transaction, i: &AppointmentInput) -> Result<i64, String> {
    let pid: Option<i64> = tx
        .query_row(
            "SELECT id FROM patients WHERE file_no=?1 AND deleted_at IS NULL",
            [i.patient_file_no],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let pid = pid.ok_or_else(|| "رقم ملف المريض غير موجود".to_string())?;
    if let Some(cid) = i.clinic_id {
        if tx
            .query_row(
                "SELECT id FROM clinics WHERE id=?1 AND is_active=1",
                [cid],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Err("العيادة المحددة غير موجودة".into());
        }
    }
    if let Some(did) = i.doctor_id {
        let row: Option<Option<i64>> = tx
            .query_row(
                "SELECT clinic_id FROM doctors WHERE id=?1 AND is_active=1",
                [did],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let dc = row.ok_or_else(|| "الطبيب المحدد غير موجود".to_string())?;
        if i.clinic_id.is_some() && dc.is_some() && i.clinic_id != dc {
            return Err("الطبيب غير مرتبط بالعيادة المحددة".into());
        }
    }
    Ok(pid)
}
fn normalized(
    i: &AppointmentInput,
) -> Result<(NaiveDateTime, NaiveDateTime, String, String), String> {
    if ![10, 15, 20, 30, 60].contains(&i.duration_minutes) {
        return Err("مدة الموعد غير مدعومة".into());
    }
    let start = parse(&i.starts_at)?;
    if !(1950..=2050).contains(&start.year()) {
        return Err("تاريخ الموعد يجب أن يكون بين 1950 و2050".into());
    }
    validate_day(start)?;
    let end = start + chrono::Duration::minutes(i.duration_minutes);
    if !(1950..=2050).contains(&end.year()) {
        return Err("نهاية الموعد يجب أن تكون بين 1950 و2050".into());
    }
    Ok((
        start,
        end,
        start.format("%Y-%m-%dT%H:%M:%S").to_string(),
        end.format("%Y-%m-%dT%H:%M:%S").to_string(),
    ))
}
fn ensure_no_conflict(
    tx: &Transaction,
    i: &AppointmentInput,
    starts: &str,
    ends: &str,
    exclude_id: Option<i64>,
) -> Result<(), String> {
    let sql = format!(
        "SELECT id FROM appointments WHERE status IN {ACTIVE} AND starts_at<?1 AND ends_at>?2 AND ((?3 IS NOT NULL AND doctor_id=?3) OR (?3 IS NULL AND ?4 IS NOT NULL AND doctor_id IS NULL AND clinic_id=?4)) AND (?5 IS NULL OR id<>?5) LIMIT 1"
    );
    let conflict: Option<i64> = tx
        .query_row(
            &sql,
            params![ends, starts, i.doctor_id, i.clinic_id, exclude_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if conflict.is_some() {
        Err("يوجد موعد متعارض في نفس الفترة".into())
    } else {
        Ok(())
    }
}
pub fn create(c: &mut Connection, i: AppointmentInput) -> Result<Appointment, String> {
    let (start, end, starts, ends) = normalized(&i)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    validate_schedule(&tx, start, end)?;
    let pid = ensure_refs(&tx, &i)?;
    ensure_no_conflict(&tx, &i, &starts, &ends, None)?;
    tx.execute("INSERT INTO appointments(patient_id,clinic_id,doctor_id,starts_at,ends_at,status,notes)VALUES(?1,?2,?3,?4,?5,'scheduled',?6)",params![pid,i.clinic_id,i.doctor_id,starts,ends,i.notes.map(|x|x.trim().to_string()).filter(|x|!x.is_empty())]).map_err(|e|e.to_string())?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "UPDATE patients SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
        [pid],
    )
    .map_err(|e| e.to_string())?;
    let created = get(&tx, id)?.ok_or_else(|| "تعذر قراءة الموعد بعد الحفظ".to_string())?;
    let after_json = serde_json::to_string(&created).map_err(|e| e.to_string())?;
    audit::record_as(
        &tx,
        "create",
        "appointment",
        Some(id),
        None,
        &audit::AuditActor::default(),
        &audit::AuditChange { before_json: None, after_json: Some(&after_json), reason: None },
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(created)
}
pub fn update(c: &mut Connection, id: i64, i: AppointmentInput) -> Result<Appointment, String> {
    let (start, end, starts, ends) = normalized(&i)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let before = get(&tx, id)?.ok_or_else(|| "الموعد غير موجود".to_string())?;
    if matches!(before.status.as_str(), "completed" | "cancelled" | "no_show") {
        return Err("لا يمكن تعديل موعد منتهي أو ملغي".into());
    }
    let before_json = serde_json::to_string(&before).map_err(|e| e.to_string())?;
    validate_schedule(&tx, start, end)?;
    let pid = ensure_refs(&tx, &i)?;
    ensure_no_conflict(&tx, &i, &starts, &ends, Some(id))?;
    let notes = i
        .notes
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty());
    tx.execute("UPDATE appointments SET patient_id=?1,clinic_id=?2,doctor_id=?3,starts_at=?4,ends_at=?5,notes=?6,updated_at=CURRENT_TIMESTAMP WHERE id=?7",params![pid,i.clinic_id,i.doctor_id,starts,ends,notes,id]).map_err(|e|e.to_string())?;
    tx.execute(
        "UPDATE patients SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
        [pid],
    )
    .map_err(|e| e.to_string())?;
    let after = get(&tx, id)?.ok_or_else(|| "تعذر قراءة الموعد بعد التعديل".to_string())?;
    let after_json = serde_json::to_string(&after).map_err(|e| e.to_string())?;
    audit::record_as(
        &tx,
        "update",
        "appointment",
        Some(id),
        None,
        &audit::AuditActor::default(),
        &audit::AuditChange { before_json: Some(&before_json), after_json: Some(&after_json), reason: None },
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(after)
}
pub fn get(c: &Connection, id: i64) -> Result<Option<Appointment>, String> {
    let sql = format!("{SELECT} WHERE a.id=?1");
    c.query_row(&sql, [id], map)
        .optional()
        .map_err(|e| e.to_string())
}
pub fn set_status(c: &Connection, id: i64, status: &str) -> Result<(), String> {
    if ![
        "scheduled",
        "arrived",
        "in_progress",
        "completed",
        "cancelled",
        "no_show",
    ]
    .contains(&status)
    {
        return Err("حالة الموعد غير صالحة".into());
    }
    let before = get(c, id)?.ok_or_else(|| "الموعد غير موجود".to_string())?;
    let before_json = serde_json::to_string(&before).map_err(|e| e.to_string())?;
    let n = c
        .execute(
            "UPDATE appointments SET status=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            params![status, id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("الموعد غير موجود".into());
    }
    let after = get(c, id)?.ok_or_else(|| "الموعد غير موجود بعد تغيير الحالة".to_string())?;
    let after_json = serde_json::to_string(&after).map_err(|e| e.to_string())?;
    let details = serde_json::json!({"from":before.status,"to":status}).to_string();
    audit::record_as(
        c,
        "status",
        "appointment",
        Some(id),
        Some(&details),
        &audit::AuditActor::default(),
        &audit::AuditChange { before_json: Some(&before_json), after_json: Some(&after_json), reason: None },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        c.execute_batch(include_str!("../migrations/003_scheduling_rules.sql"))
            .unwrap();
        c.execute(
            "INSERT INTO patients(file_no,full_name)VALUES(1,'مريض اختبار')",
            [],
        )
        .unwrap();
        c.execute("INSERT INTO clinics(name)VALUES('عيادة')", [])
            .unwrap();
        c.execute("INSERT INTO doctors(clinic_id,name)VALUES(1,'طبيب')", [])
            .unwrap();
        c
    }
    fn i(t: &str) -> AppointmentInput {
        AppointmentInput {
            patient_file_no: 1,
            clinic_id: Some(1),
            doctor_id: Some(1),
            starts_at: t.into(),
            duration_minutes: 30,
            notes: None,
        }
    }
    #[test]
    fn friday_is_blocked() {
        let mut c = db();
        assert!(create(&mut c, i("2026-09-18T10:00"))
            .unwrap_err()
            .contains("الجمعة"))
    }
    #[test]
    fn saturday_is_blocked() {
        let mut c = db();
        assert!(create(&mut c, i("2026-09-19T10:00"))
            .unwrap_err()
            .contains("السبت"))
    }
    #[test]
    fn overlapping_slot_is_blocked() {
        let mut c = db();
        create(&mut c, i("2026-09-20T10:00")).unwrap();
        assert!(create(&mut c, i("2026-09-20T10:15")).is_err())
    }
    #[test]
    fn adjacent_slot_is_allowed() {
        let mut c = db();
        create(&mut c, i("2026-09-20T10:00")).unwrap();
        assert!(create(&mut c, i("2026-09-20T10:30")).is_ok())
    }
    #[test]
    fn different_doctors_in_same_clinic_can_overlap() {
        let mut c = db();
        c.execute(
            "INSERT INTO doctors(clinic_id,name)VALUES(1,'طبيب ثان')",
            [],
        )
        .unwrap();
        create(&mut c, i("2026-09-20T10:00")).unwrap();
        let mut other = i("2026-09-20T10:15");
        other.doctor_id = Some(2);
        assert!(create(&mut c, other).is_ok())
    }
    #[test]
    fn clinic_only_appointments_still_conflict_with_each_other() {
        let mut c = db();
        let mut first = i("2026-09-20T10:00");
        first.doctor_id = None;
        create(&mut c, first).unwrap();
        let mut second = i("2026-09-20T10:15");
        second.doctor_id = None;
        assert!(create(&mut c, second).is_err())
    }
    #[test]
    fn outside_hours_is_blocked() {
        let mut c = db();
        assert!(create(&mut c, i("2026-09-20T07:30"))
            .unwrap_err()
            .contains("خارج ساعات"))
    }
    #[test]
    fn break_is_blocked() {
        let mut c = db();
        assert!(create(&mut c, i("2026-09-20T12:15"))
            .unwrap_err()
            .contains("الاستراحة"))
    }
    #[test]
    fn closure_is_blocked() {
        let mut c = db();
        c.execute(
            "INSERT INTO closure_dates(closure_date,reason)VALUES('2026-09-20','إغلاق')",
            [],
        )
        .unwrap();
        assert!(create(&mut c, i("2026-09-20T10:00"))
            .unwrap_err()
            .contains("إغلاق"))
    }
    #[test]
    fn appointment_range_is_enforced() {
        let mut c = db();
        assert!(create(&mut c, i("2051-01-02T10:00")).is_err())
    }
    #[test]
    fn reschedule_excludes_current_appointment() {
        let mut c = db();
        let a = create(&mut c, i("2026-09-20T10:00")).unwrap();
        assert!(update(&mut c, a.id, i("2026-09-20T10:15")).is_ok())
    }
    #[test]
    fn reschedule_still_blocks_other_conflicts() {
        let mut c = db();
        let a = create(&mut c, i("2026-09-20T10:00")).unwrap();
        create(&mut c, i("2026-09-20T11:00")).unwrap();
        assert!(update(&mut c, a.id, i("2026-09-20T11:15")).is_err())
    }
    #[test]
    fn completed_appointment_cannot_be_rescheduled() {
        let mut c = db();
        let a = create(&mut c, i("2026-09-20T10:00")).unwrap();
        set_status(&c, a.id, "completed").unwrap();
        assert!(update(&mut c, a.id, i("2026-09-20T11:00")).is_err())
    }
}