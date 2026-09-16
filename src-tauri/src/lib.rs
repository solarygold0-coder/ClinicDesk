pub mod appointment_status;
pub mod appointments;
pub mod attachments;
pub mod audit;
pub mod backup;
pub mod directory;
pub mod domain;
pub mod patients;
pub mod scheduling;
pub mod visit_tracking;
pub mod visit_tracking_command;

use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::Manager;

pub struct Db(pub Mutex<Connection>);
const LATEST_SCHEMA_VERSION: i64 = 11;

fn schema_version(db: &Connection) -> Result<i64, String> {
    db.query_row(
        "SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'",
        [],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

fn migrate_db(db: &Connection) -> Result<(), String> {
    db.execute_batch(include_str!("../migrations/001_init.sql"))
        .map_err(|e| e.to_string())?;
    let current = schema_version(db)?;
    if current > LATEST_SCHEMA_VERSION {
        return Err(format!(
            "إصدار قاعدة البيانات {current} أحدث من الإصدار الذي يدعمه البرنامج {LATEST_SCHEMA_VERSION}"
        ));
    }
    for (version, sql) in [
        (2, include_str!("../migrations/002_touch_triggers.sql")),
        (3, include_str!("../migrations/003_scheduling_rules.sql")),
        (
            4,
            include_str!("../migrations/004_patient_medical_details.sql"),
        ),
        (
            5,
            include_str!("../migrations/005_patient_last_activity.sql"),
        ),
        (
            6,
            include_str!("../migrations/006_patient_activity_triggers.sql"),
        ),
        (
            7,
            include_str!("../migrations/007_appointment_visit_tracking.sql"),
        ),
        (8, include_str!("../migrations/008_optional_auth.sql")),
        (
            9,
            include_str!("../migrations/009_remove_optional_auth.sql"),
        ),
        (
            10,
            include_str!("../migrations/010_restore_users_roles_audit_actor.sql"),
        ),
        (
            11,
            include_str!("../migrations/011_accountability_identity.sql"),
        ),
    ] {
        if schema_version(db)? < version {
            db.execute_batch(sql).map_err(|e| e.to_string())?;
            let applied = schema_version(db)?;
            if applied != version {
                return Err(format!(
                    "فشل ترحيل قاعدة البيانات إلى الإصدار {version}: الإصدار المسجل {applied}"
                ));
            }
        }
    }
    let final_version = schema_version(db)?;
    if final_version != LATEST_SCHEMA_VERSION {
        return Err(format!(
            "إصدار قاعدة البيانات بعد الترحيل {final_version} بدلاً من {LATEST_SCHEMA_VERSION}"
        ));
    }
    Ok(())
}

fn init_db(path: &PathBuf) -> Result<Connection, String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?
    }
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    migrate_db(&db)?;
    Ok(db)
}

fn with_db<T>(
    db: &tauri::State<Db>,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    f(&g)
}

#[tauri::command]
fn health() -> &'static str {
    "ok"
}

#[tauri::command]
fn patient_list(
    db: tauri::State<Db>,
    query: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<patients::Patient>, String> {
    with_db(&db, |c| patients::list(c, query, limit.unwrap_or(50)))
}

#[tauri::command]
fn patient_count(db: tauri::State<Db>) -> Result<i64, String> {
    with_db(&db, |c| {
        c.query_row(
            "SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn patient_by_file_no(
    db: tauri::State<Db>,
    file_no: i64,
) -> Result<Option<patients::Patient>, String> {
    with_db(&db, |c| patients::get_by_file_no(c, file_no))
}

#[tauri::command]
fn patient_inactive(
    db: tauri::State<Db>,
    years: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<patients::Patient>, String> {
    with_db(&db, |c| {
        patients::inactive_for_years(c, years.unwrap_or(10), limit.unwrap_or(100))
    })
}

#[tauri::command]
fn patient_create(
    db: tauri::State<Db>,
    input: patients::PatientInput,
) -> Result<patients::Patient, String> {
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    patients::create(&mut g, input)
}

#[tauri::command]
fn patient_update(
    db: tauri::State<Db>,
    id: i64,
    input: patients::PatientInput,
) -> Result<patients::Patient, String> {
    with_db(&db, |c| patients::update(c, id, input))
}

#[tauri::command]
fn patient_delete(db: tauri::State<Db>, id: i64) -> Result<(), String> {
    with_db(&db, |c| patients::soft_delete(c, id))
}

#[tauri::command]
fn patient_future_appointment_count(db: tauri::State<Db>, id: i64) -> Result<i64, String> {
    with_db(&db, |c| patients::future_appointment_count(c, id))
}

#[tauri::command]
fn clinic_list(db: tauri::State<Db>) -> Result<Vec<directory::Clinic>, String> {
    with_db(&db, directory::list_clinics)
}

#[tauri::command]
fn clinic_create(
    db: tauri::State<Db>,
    input: directory::ClinicInput,
) -> Result<directory::Clinic, String> {
    with_db(&db, |c| directory::create_clinic(c, input))
}

#[tauri::command]
fn clinic_update(
    db: tauri::State<Db>,
    id: i64,
    input: directory::ClinicInput,
) -> Result<directory::Clinic, String> {
    with_db(&db, |c| directory::update_clinic(c, id, input))
}

#[tauri::command]
fn clinic_delete(db: tauri::State<Db>, id: i64) -> Result<(), String> {
    with_db(&db, |c| directory::deactivate_clinic(c, id))
}

#[tauri::command]
fn doctor_list(db: tauri::State<Db>) -> Result<Vec<directory::Doctor>, String> {
    with_db(&db, directory::list_doctors)
}

#[tauri::command]
fn doctor_create(
    db: tauri::State<Db>,
    input: directory::DoctorInput,
) -> Result<directory::Doctor, String> {
    with_db(&db, |c| directory::create_doctor(c, input))
}

#[tauri::command]
fn doctor_update(
    db: tauri::State<Db>,
    id: i64,
    input: directory::DoctorInput,
) -> Result<directory::Doctor, String> {
    with_db(&db, |c| directory::update_doctor(c, id, input))
}

#[tauri::command]
fn doctor_delete(db: tauri::State<Db>, id: i64) -> Result<(), String> {
    with_db(&db, |c| directory::deactivate_doctor(c, id))
}

#[tauri::command]
fn appointment_list(
    db: tauri::State<Db>,
    from: String,
    to: String,
) -> Result<Vec<appointments::Appointment>, String> {
    with_db(&db, |c| appointments::list(c, &from, &to))
}

#[tauri::command]
fn appointment_missed_history(
    db: tauri::State<Db>,
    to: String,
) -> Result<Vec<appointments::Appointment>, String> {
    with_db(&db, |c| {
        Ok(appointments::list(c, "1900-01-01T00:00:00", &to)?
            .into_iter()
            .filter(|a| a.status == "no_show")
            .collect())
    })
}

#[tauri::command]
fn appointment_upcoming_all(
    db: tauri::State<Db>,
    from: String,
) -> Result<Vec<appointments::Appointment>, String> {
    with_db(&db, |c| appointments::list(c, &from, "9999-12-31T23:59:59"))
}

#[tauri::command]
fn patient_appointments(
    db: tauri::State<Db>,
    patient_id: i64,
    limit: Option<i64>,
) -> Result<Vec<appointments::Appointment>, String> {
    with_db(&db, |c| {
        appointments::list_for_patient(c, patient_id, limit.unwrap_or(30))
    })
}

#[tauri::command]
fn appointment_create(
    db: tauri::State<Db>,
    input: appointments::AppointmentInput,
) -> Result<appointments::Appointment, String> {
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    appointments::create(&mut g, input)
}

#[tauri::command]
fn appointment_update(
    db: tauri::State<Db>,
    id: i64,
    input: appointments::AppointmentInput,
) -> Result<appointments::Appointment, String> {
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    appointments::update(&mut g, id, input)
}

#[tauri::command]
fn appointment_status(db: tauri::State<Db>, id: i64, status: String) -> Result<(), String> {
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    appointment_status::set_status(&mut g, id, &status)
}

#[tauri::command]
fn visit_tracking_update(
    db: tauri::State<Db>,
    id: i64,
    input: visit_tracking::VisitTrackingInput,
) -> Result<(), String> {
    with_db(&db, |c| visit_tracking::update(c, id, input))
}

#[tauri::command]
fn visit_follow_ups(
    db: tauri::State<Db>,
    from: String,
    to: String,
) -> Result<Vec<visit_tracking::FollowUpVisit>, String> {
    with_db(&db, |c| visit_tracking::follow_ups(c, &from, &to))
}

#[tauri::command]
fn attachment_list(
    db: tauri::State<Db>,
    patient_id: i64,
) -> Result<Vec<attachments::Attachment>, String> {
    with_db(&db, |c| attachments::list(c, patient_id))
}

fn attachment_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("attachments"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn attachment_import(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    patient_id: i64,
    source_path: String,
) -> Result<i64, String> {
    let root = attachment_root(&app)?;
    with_db(&db, |c| {
        attachments::import_file(c, patient_id, Path::new(&source_path), &root)
    })
}

#[tauri::command]
fn attachment_remove(app: tauri::AppHandle, db: tauri::State<Db>, id: i64) -> Result<(), String> {
    let root = attachment_root(&app)?;
    with_db(&db, |c| attachments::remove_file(c, id, &root))
}

#[tauri::command]
fn scheduling_settings_get(db: tauri::State<Db>) -> Result<scheduling::SchedulingSettings, String> {
    with_db(&db, scheduling::get)
}

#[tauri::command]
fn scheduling_settings_update(
    db: tauri::State<Db>,
    input: scheduling::SchedulingSettings,
) -> Result<scheduling::SchedulingSettings, String> {
    with_db(&db, |c| scheduling::update(c, input))
}

#[tauri::command]
fn closure_list(db: tauri::State<Db>) -> Result<Vec<scheduling::ClosureDate>, String> {
    with_db(&db, scheduling::closures)
}

#[tauri::command]
fn closure_create(db: tauri::State<Db>, input: scheduling::ClosureInput) -> Result<(), String> {
    with_db(&db, |c| scheduling::add_closure(c, input))
}

#[tauri::command]
fn closure_delete(db: tauri::State<Db>, id: i64) -> Result<(), String> {
    with_db(&db, |c| scheduling::delete_closure(c, id))
}

#[tauri::command]
fn backup_create(db: tauri::State<Db>, destination_path: String) -> Result<String, String> {
    let g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    let hash =
        backup::create_database_backup(&g, Path::new(&destination_path), LATEST_SCHEMA_VERSION)?;
    let details = serde_json::json!({"sha256":hash}).to_string();
    audit::record(&g, "backup_created", "database", None, Some(&details))?;
    Ok(hash)
}

#[tauri::command]
fn backup_restore(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    source_path: String,
) -> Result<String, String> {
    let live = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("clinicdesk.sqlite3");
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    let hash = backup::restore_database(
        &mut g,
        Path::new(&source_path),
        &live,
        LATEST_SCHEMA_VERSION,
    )?;
    let details = serde_json::json!({"sha256":hash}).to_string();
    audit::record(&g, "backup_restored", "database", None, Some(&details))?;
    Ok(hash)
}

#[tauri::command]
fn audit_recent(
    db: tauri::State<Db>,
    limit: Option<i64>,
) -> Result<Vec<audit::AuditEntry>, String> {
    with_db(&db, |c| audit::recent(c, limit.unwrap_or(100)))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("clinicdesk.sqlite3");
            let db = init_db(&path).map_err(std::io::Error::other)?;
            app.manage(Db(Mutex::new(db)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health,
            patient_list,
            patient_count,
            patient_by_file_no,
            patient_inactive,
            patient_create,
            patient_update,
            patient_delete,
            patient_future_appointment_count,
            clinic_list,
            clinic_create,
            clinic_update,
            clinic_delete,
            doctor_list,
            doctor_create,
            doctor_update,
            doctor_delete,
            appointment_list,
            appointment_missed_history,
            appointment_upcoming_all,
            patient_appointments,
            appointment_create,
            appointment_update,
            appointment_status,
            visit_tracking_command::visit_tracking_get,
            visit_tracking_update,
            visit_follow_ups,
            attachment_list,
            attachment_import,
            attachment_remove,
            scheduling_settings_get,
            scheduling_settings_update,
            closure_list,
            closure_create,
            closure_delete,
            backup_create,
            backup_restore,
            audit_recent
        ])
        .run(tauri::generate_context!())
        .expect("error while running ClinicDesk");
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    fn table_exists(db: &Connection, name: &str) -> i64 {
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [name],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn column_exists(db: &Connection, table: &str, name: &str) -> i64 {
        let sql = format!(
            "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name=?1",
            table.replace('\'', "''")
        );
        db.query_row(&sql, [name], |r| r.get(0)).unwrap()
    }

    #[test]
    fn migrations_reach_latest_version_and_are_idempotent() {
        let db = Connection::open_in_memory().unwrap();
        migrate_db(&db).unwrap();
        assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);
        migrate_db(&db).unwrap();
        assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);
        assert_eq!(table_exists(&db, "users"), 1);
        assert_eq!(table_exists(&db, "roles"), 1);
        assert_eq!(column_exists(&db, "users", "employee_code"), 1);
        assert_eq!(column_exists(&db, "audit_log", "actor_employee_code"), 1);
        assert_eq!(column_exists(&db, "audit_log", "actor_session_id"), 1);
        assert_eq!(column_exists(&db, "audit_log", "before_json"), 1);
        assert_eq!(column_exists(&db, "audit_log", "after_json"), 1);
        assert_eq!(column_exists(&db, "audit_log", "reason"), 1);
    }

    #[test]
    fn migration_rejects_future_schema_version() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        db.execute(
            "UPDATE app_meta SET value='99' WHERE key='schema_version'",
            [],
        )
        .unwrap();
        assert!(migrate_db(&db).unwrap_err().contains("أحدث"));
    }

    #[test]
    fn migration_from_v9_restores_auth_and_preserves_data() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../migrations/001_init.sql"))
            .unwrap();
        for sql in [
            include_str!("../migrations/002_touch_triggers.sql"),
            include_str!("../migrations/003_scheduling_rules.sql"),
            include_str!("../migrations/004_patient_medical_details.sql"),
            include_str!("../migrations/005_patient_last_activity.sql"),
            include_str!("../migrations/006_patient_activity_triggers.sql"),
            include_str!("../migrations/007_appointment_visit_tracking.sql"),
            include_str!("../migrations/008_optional_auth.sql"),
            include_str!("../migrations/009_remove_optional_auth.sql"),
        ] {
            db.execute_batch(sql).unwrap();
        }
        db.execute(
            "INSERT INTO patients(file_no,full_name,national_id) VALUES(9,'مريض محفوظ من إصدار 9','9234567890')",
            [],
        )
        .unwrap();
        assert_eq!(schema_version(&db).unwrap(), 9);
        assert_eq!(table_exists(&db, "users"), 0);

        migrate_db(&db).unwrap();

        assert_eq!(schema_version(&db).unwrap(), 11);
        assert_eq!(table_exists(&db, "users"), 1);
        assert_eq!(table_exists(&db, "roles"), 1);
        let name: String = db
            .query_row(
                "SELECT full_name FROM patients WHERE file_no=9",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "مريض محفوظ من إصدار 9");
        let actor_cols: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('audit_log') WHERE name IN ('actor_user_id','actor_display_name')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(actor_cols, 2);
        assert_eq!(column_exists(&db, "users", "employee_code"), 1);
        assert_eq!(column_exists(&db, "audit_log", "actor_employee_code"), 1);
    }
}
