pub mod appointment_status;
pub mod appointments;
pub mod attachments;
pub mod audit;
pub mod backup;
pub mod directory;
pub mod domain;
pub mod patients;
pub mod scheduling;
pub mod security_log;
pub mod users;
pub mod visit_tracking;
pub mod visit_tracking_command;

#[cfg(test)]
mod migration_tests;

use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::Manager;
use uuid::Uuid;

pub struct Db(pub Mutex<Connection>);
const LATEST_SCHEMA_VERSION: i64 = 13;

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
        (
            12,
            include_str!("../migrations/012_immutable_employee_identity.sql"),
        ),
        (
            13,
            include_str!("../migrations/013_user_roles_lifecycle.sql"),
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
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
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

fn security_log_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("security").join("events.jsonl"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn backup_create(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    destination_path: String,
) -> Result<String, String> {
    let g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    let hash =
        backup::create_database_backup(&g, Path::new(&destination_path), LATEST_SCHEMA_VERSION)?;
    let details = serde_json::json!({"sha256": hash}).to_string();
    audit::record(&g, "backup_created", "database", None, Some(&details))?;
    security_log::append(
        &security_log_path(&app)?,
        "backup",
        "success",
        None,
        Some(&details),
    )?;
    Ok(hash)
}

#[tauri::command]
fn backup_restore(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    source_path: String,
) -> Result<String, String> {
    let security_path = security_log_path(&app)?;
    let restore_ref = format!("RST-{}", Uuid::new_v4());
    let source = Path::new(&source_path);
    let source_hash = backup::sha256_file(source)?;
    let start_details = serde_json::json!({"sha256": source_hash}).to_string();
    security_log::append(
        &security_path,
        "restore",
        "started",
        Some(&restore_ref),
        Some(&start_details),
    )?;

    let restore_result = (|| -> Result<String, String> {
        backup::verify_database(source, LATEST_SCHEMA_VERSION)?;
        backup::verify_attachment_backup(source)?;
        let live = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("clinicdesk.sqlite3");
        let mut guard =
            db.0.lock()
                .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
        audit::record(
            &guard,
            "restore_started",
            "database",
            None,
            Some(
                &serde_json::json!({"sha256": source_hash, "restore_ref": restore_ref}).to_string(),
            ),
        )?;
        let hash = backup::restore_database(&mut guard, source, &live, LATEST_SCHEMA_VERSION)?;
        audit::record(
            &guard,
            "restore_completed",
            "database",
            None,
            Some(&serde_json::json!({"sha256": hash, "restore_ref": restore_ref}).to_string()),
        )?;
        Ok(hash)
    })();

    match restore_result {
        Ok(hash) => {
            let details = serde_json::json!({"sha256": hash}).to_string();
            security_log::append(
                &security_path,
                "restore",
                "success",
                Some(&restore_ref),
                Some(&details),
            )?;
            Ok(hash)
        }
        Err(error) => {
            let details = serde_json::json!({"error": error}).to_string();
            security_log::append(
                &security_path,
                "restore",
                "failure",
                Some(&restore_ref),
                Some(&details),
            )?;
            Err(error)
        }
    }
}

#[tauri::command]
fn audit_recent(
    db: tauri::State<Db>,
    limit: Option<i64>,
) -> Result<Vec<audit::AuditEntry>, String> {
    with_db(&db, |c| audit::recent(c, limit.unwrap_or(100)))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let db_path = app
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("clinicdesk.sqlite3");
            let db = init_db(&db_path)?;
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
            visit_tracking_update,
            visit_tracking_command::visit_tracking_get,
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
