use crate::audit;
use chrono::{NaiveDate, NaiveTime};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulingSettings {
    pub work_start: String,
    pub work_end: String,
    pub break_start: Option<String>,
    pub break_end: Option<String>,
    pub slot_minutes: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosureDate {
    pub id: i64,
    pub closure_date: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosureInput {
    pub closure_date: String,
    pub reason: Option<String>,
}

fn t(v: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(v, "%H:%M").map_err(|_| "صيغة الوقت يجب أن تكون HH:MM".into())
}

fn validate(s: &SchedulingSettings) -> Result<(), String> {
    let ws = t(&s.work_start)?;
    let we = t(&s.work_end)?;
    if ws >= we {
        return Err("بداية الدوام يجب أن تسبق نهاية الدوام".into());
    }
    if ![10, 15, 20, 30, 60].contains(&s.slot_minutes) {
        return Err("مدة الفترة الافتراضية غير مدعومة".into());
    }
    match (&s.break_start, &s.break_end) {
        (None, None) => {}
        (Some(bs), Some(be)) => {
            let bs = t(bs)?;
            let be = t(be)?;
            if bs >= be || bs < ws || be > we {
                return Err("فترة الاستراحة يجب أن تكون داخل ساعات الدوام".into());
            }
        }
        _ => return Err("حدد بداية ونهاية الاستراحة معًا أو اتركهما فارغين".into()),
    }
    Ok(())
}

pub fn get(c: &Connection) -> Result<SchedulingSettings, String> {
    c.query_row(
        "SELECT work_start,work_end,break_start,break_end,slot_minutes FROM scheduling_settings WHERE id=1",
        [],
        |r| {
            Ok(SchedulingSettings {
                work_start: r.get(0)?,
                work_end: r.get(1)?,
                break_start: r.get(2)?,
                break_end: r.get(3)?,
                slot_minutes: r.get(4)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn update(c: &Connection, s: SchedulingSettings) -> Result<SchedulingSettings, String> {
    validate(&s)?;
    let before = get(c)?;
    c.execute(
        "UPDATE scheduling_settings SET work_start=?1,work_end=?2,break_start=?3,break_end=?4,slot_minutes=?5,updated_at=CURRENT_TIMESTAMP WHERE id=1",
        params![s.work_start, s.work_end, s.break_start, s.break_end, s.slot_minutes],
    )
    .map_err(|e| e.to_string())?;
    let after = get(c)?;
    let before_json = serde_json::to_string(&before).map_err(|e| e.to_string())?;
    let after_json = serde_json::to_string(&after).map_err(|e| e.to_string())?;
    audit::record_as(
        c,
        "scheduling_updated",
        "scheduling",
        Some(1),
        None,
        &audit::AuditActor::default(),
        &audit::AuditChange {
            before_json: Some(&before_json),
            after_json: Some(&after_json),
            reason: None,
        },
    )?;
    Ok(after)
}

pub fn closures(c: &Connection) -> Result<Vec<ClosureDate>, String> {
    let mut q = c
        .prepare("SELECT id,closure_date,reason FROM closure_dates ORDER BY closure_date")
        .map_err(|e| e.to_string())?;
    let rows = q
        .query_map([], |r| {
            Ok(ClosureDate {
                id: r.get(0)?,
                closure_date: r.get(1)?,
                reason: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn add_closure(c: &Connection, i: ClosureInput) -> Result<(), String> {
    NaiveDate::parse_from_str(&i.closure_date, "%Y-%m-%d")
        .map_err(|_| "تاريخ الإغلاق غير صالح".to_string())?;
    let year = i.closure_date.get(0..4).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    if !(1950..=2050).contains(&year) {
        return Err("تاريخ الإغلاق يجب أن يكون بين 1950 و2050".into());
    }
    let closure_date = i.closure_date;
    let reason = i.reason.map(|x| x.trim().to_string()).filter(|x| !x.is_empty());
    c.execute(
        "INSERT INTO closure_dates(closure_date,reason)VALUES(?1,?2)",
        params![closure_date, reason],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "هذا التاريخ مضاف مسبقًا".into()
        } else {
            e.to_string()
        }
    })?;
    let id = c.last_insert_rowid();
    let after = serde_json::json!({"id":id,"closureDate":closure_date,"reason":reason}).to_string();
    audit::record_as(
        c,
        "closure_created",
        "closure",
        Some(id),
        None,
        &audit::AuditActor::default(),
        &audit::AuditChange { before_json: None, after_json: Some(&after), reason: reason.as_deref() },
    )
}

pub fn delete_closure(c: &Connection, id: i64) -> Result<(), String> {
    let before: (String, Option<String>) = c
        .query_row("SELECT closure_date,reason FROM closure_dates WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|_| "تاريخ الإغلاق غير موجود".to_string())?;
    let before_json = serde_json::json!({"id":id,"closureDate":before.0,"reason":before.1}).to_string();
    let n = c.execute("DELETE FROM closure_dates WHERE id=?1", [id]).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("تاريخ الإغلاق غير موجود".into());
    }
    audit::record_as(
        c,
        "closure_deleted",
        "closure",
        Some(id),
        None,
        &audit::AuditActor::default(),
        &audit::AuditChange { before_json: Some(&before_json), after_json: None, reason: before.1.as_deref() },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
        c.execute_batch(include_str!("../migrations/003_scheduling_rules.sql")).unwrap();
        c
    }
    #[test]
    fn valid_settings_save() {
        let c = db();
        assert!(update(&c,SchedulingSettings{work_start:"08:00".into(),work_end:"18:00".into(),break_start:Some("12:00".into()),break_end:Some("13:00".into()),slot_minutes:30}).is_ok());
    }
    #[test]
    fn invalid_hours_blocked() {
        let c = db();
        assert!(update(&c,SchedulingSettings{work_start:"18:00".into(),work_end:"08:00".into(),break_start:None,break_end:None,slot_minutes:30}).is_err());
    }
    #[test]
    fn partial_break_blocked() {
        let c = db();
        assert!(update(&c,SchedulingSettings{work_start:"08:00".into(),work_end:"18:00".into(),break_start:Some("12:00".into()),break_end:None,slot_minutes:30}).is_err());
    }
    #[test]
    fn closure_crud() {
        let c = db();
        add_closure(&c,ClosureInput{closure_date:"2026-09-23".into(),reason:Some("إجازة".into())}).unwrap();
        let x=closures(&c).unwrap();assert_eq!(x.len(),1);delete_closure(&c,x[0].id).unwrap();assert!(closures(&c).unwrap().is_empty());
    }
    #[test]
    fn closure_range_is_enforced_in_backend() {
        let c=db();
        assert!(add_closure(&c,ClosureInput{closure_date:"2051-01-01".into(),reason:None}).is_err());
    }
}
