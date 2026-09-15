use crate::domain::normalize_digits;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Patient {
    pub id: i64,
    pub file_no: i64,
    pub national_id: Option<String>,
    pub full_name: String,
    pub phone: Option<String>,
    pub birth_date: Option<String>,
    pub sex: Option<String>,
    pub medical_summary: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientInput {
    pub national_id: Option<String>,
    pub full_name: String,
    pub phone: Option<String>,
    pub birth_date: Option<String>,
    pub sex: Option<String>,
    pub medical_summary: Option<String>,
}
fn clean(v: Option<String>) -> Option<String> {
    v.map(|x| x.trim().to_string()).filter(|x| !x.is_empty())
}
fn validate(input: &PatientInput) -> Result<(String, Option<String>, Option<String>), String> {
    let name = input.full_name.trim().to_string();
    if name.len() < 2 {
        return Err("اسم المريض مطلوب".into());
    }
    let nid = clean(input.national_id.clone()).map(|x| normalize_digits(&x));
    if let Some(v) = &nid {
        if !v.chars().all(|c| c.is_ascii_digit()) || v.len() != 10 {
            return Err("رقم الهوية يجب أن يتكون من 10 أرقام".into());
        }
    }
    let phone = clean(input.phone.clone()).map(|x| normalize_digits(&x));
    if let Some(v) = &phone {
        if !v.chars().all(|c| c.is_ascii_digit() || c == '+') {
            return Err("رقم الجوال غير صالح".into());
        }
    }
    Ok((name, nid, phone))
}
fn allocate_file_no(tx: &Transaction) -> Result<i64, String> {
    let n: String = tx
        .query_row(
            "SELECT value FROM app_meta WHERE key='next_patient_file_no'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let n: i64 = n
        .parse()
        .map_err(|_| "عداد أرقام الملفات تالف".to_string())?;
    tx.execute(
        "UPDATE app_meta SET value=?1 WHERE key='next_patient_file_no'",
        [(n + 1).to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(n)
}
fn map_patient(r: &rusqlite::Row) -> rusqlite::Result<Patient> {
    Ok(Patient {
        id: r.get(0)?,
        file_no: r.get(1)?,
        national_id: r.get(2)?,
        full_name: r.get(3)?,
        phone: r.get(4)?,
        birth_date: r.get(5)?,
        sex: r.get(6)?,
        medical_summary: r.get(7)?,
    })
}
const PATIENT_SELECT: &str =
    "SELECT id,file_no,national_id,full_name,phone,birth_date,sex,medical_summary FROM patients";
pub fn create(conn: &mut Connection, input: PatientInput) -> Result<Patient, String> {
    let (name, nid, phone) = validate(&input)?;
    if let Some(v) = &nid {
        let exists: Option<i64> = conn
            .query_row(
                "SELECT id FROM patients WHERE national_id=?1 AND deleted_at IS NULL",
                [v],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if exists.is_some() {
            return Err("يوجد مريض مسجل بنفس رقم الهوية".into());
        }
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let file_no = allocate_file_no(&tx)?;
    tx.execute("INSERT INTO patients(file_no,national_id,full_name,phone,birth_date,sex,medical_summary) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![file_no,nid,name,phone,clean(input.birth_date),clean(input.sex),clean(input.medical_summary)]).map_err(|e|e.to_string())?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO audit_log(event_type,entity_type,entity_id) VALUES('create','patient',?1)",
        [id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    get(conn, id)?.ok_or_else(|| "تعذر قراءة المريض بعد الحفظ".into())
}
pub fn get(conn: &Connection, id: i64) -> Result<Option<Patient>, String> {
    conn.query_row(
        &format!("{PATIENT_SELECT} WHERE id=?1 AND deleted_at IS NULL"),
        [id],
        map_patient,
    )
    .optional()
    .map_err(|e| e.to_string())
}
pub fn get_by_file_no(conn: &Connection, file_no: i64) -> Result<Option<Patient>, String> {
    conn.query_row(
        &format!("{PATIENT_SELECT} WHERE file_no=?1 AND deleted_at IS NULL"),
        [file_no],
        map_patient,
    )
    .optional()
    .map_err(|e| e.to_string())
}
pub fn list(conn: &Connection, q: Option<String>, limit: i64) -> Result<Vec<Patient>, String> {
    let q = normalize_digits(&clean(q).unwrap_or_default());
    let pattern = format!("%{}%", q);
    let mut st=conn.prepare("SELECT id,file_no,national_id,full_name,phone,birth_date,sex,medical_summary FROM patients WHERE deleted_at IS NULL AND (?1='' OR full_name LIKE ?2 OR CAST(file_no AS TEXT)=?1 OR national_id LIKE ?2 OR phone LIKE ?2) ORDER BY id DESC LIMIT ?3").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![q, pattern, limit.clamp(1, 100)], map_patient)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
pub fn update(conn: &Connection, id: i64, input: PatientInput) -> Result<Patient, String> {
    let (name, nid, phone) = validate(&input)?;
    let changed=conn.execute("UPDATE patients SET national_id=?1,full_name=?2,phone=?3,birth_date=?4,sex=?5,medical_summary=?6 WHERE id=?7 AND deleted_at IS NULL",params![nid,name,phone,clean(input.birth_date),clean(input.sex),clean(input.medical_summary),id]).map_err(|e|if e.to_string().contains("UNIQUE constraint failed"){"يوجد مريض مسجل بنفس رقم الهوية".into()}else{e.to_string()})?;
    if changed == 0 {
        return Err("المريض غير موجود".into());
    }
    conn.execute(
        "INSERT INTO audit_log(event_type,entity_type,entity_id) VALUES('update','patient',?1)",
        [id],
    )
    .map_err(|e| e.to_string())?;
    get(conn, id)?.ok_or_else(|| "المريض غير موجود".into())
}
pub fn soft_delete(conn: &Connection, id: i64) -> Result<(), String> {
    let n = conn
        .execute(
            "UPDATE patients SET deleted_at=CURRENT_TIMESTAMP WHERE id=?1 AND deleted_at IS NULL",
            [id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("المريض غير موجود".into());
    }
    conn.execute(
        "INSERT INTO audit_log(event_type,entity_type,entity_id) VALUES('delete','patient',?1)",
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        c
    }
    fn input(name: &str, nid: &str) -> PatientInput {
        PatientInput {
            national_id: Some(nid.into()),
            full_name: name.into(),
            phone: None,
            birth_date: None,
            sex: None,
            medical_summary: None,
        }
    }
    #[test]
    fn numbering_is_not_reused() {
        let mut c = db();
        let a = create(&mut c, input("أحمد محمد", "١٢٣٤٥٦٧٨٩٠")).unwrap();
        soft_delete(&c, a.id).unwrap();
        let b = create(&mut c, input("سالم علي", "1234567891")).unwrap();
        assert_eq!(a.file_no, 1);
        assert_eq!(b.file_no, 2)
    }
    #[test]
    fn duplicate_nid_rejected() {
        let mut c = db();
        create(&mut c, input("أحمد محمد", "1234567890")).unwrap();
        assert!(create(&mut c, input("أحمد آخر", "١٢٣٤٥٦٧٨٩٠")).is_err())
    }
    #[test]
    fn lookup_by_file_no_works() {
        let mut c = db();
        let p = create(&mut c, input("مريض تجريبي", "1234567890")).unwrap();
        let found = get_by_file_no(&c, p.file_no).unwrap().unwrap();
        assert_eq!(found.id, p.id);
        assert_eq!(found.full_name, "مريض تجريبي")
    }
}
