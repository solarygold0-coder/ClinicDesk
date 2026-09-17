pub mod appointment_status;
pub mod appointments;
pub mod attachments;
pub mod audit;
pub mod auth;
pub mod auth_commands;
pub mod authorization;
pub mod backup;
pub mod directory;
pub mod domain;
pub mod patients;
pub mod provider_unavailability;
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
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;
pub struct Db(pub Mutex<Connection>);
const LATEST_SCHEMA_VERSION: i64 = 18;
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
        return Err(format!("إصدار قاعدة البيانات {current} أحدث من الإصدار الذي يدعمه البرنامج {LATEST_SCHEMA_VERSION}"));
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
        (
            14,
            include_str!("../migrations/014_attachment_lifecycle.sql"),
        ),
        (
            15,
            include_str!("../migrations/015_attachment_restore_limit.sql"),
        ),
        (
            16,
            include_str!("../migrations/016_role_capability_grants.sql"),
        ),
        (
            17,
            include_str!("../migrations/017_provider_unavailability.sql"),
        ),
        (
            18,
            include_str!("../migrations/018_exact_four_password_policy.sql"),
        ),
    ] {
        if schema_version(db)? < version {
            db.execute_batch(sql).map_err(|e| e.to_string())?;
            if schema_version(db)? != version {
                return Err(format!("فشل ترحيل قاعدة البيانات إلى الإصدار {version}"));
            }
        }
    }
    if schema_version(db)? != LATEST_SCHEMA_VERSION {
        return Err("فشل الوصول إلى أحدث إصدار لقاعدة البيانات".into());
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
fn authorize_command(
    db: &tauri::State<Db>,
    actor_token: Option<&str>,
    capability: &str,
) -> Result<(), String> {
    with_db(db, |conn| {
        authorization::authorize(conn, actor_token, capability)?;
        Ok(())
    })
}
#[tauri::command]
fn health() -> &'static str {
    "ok"
}
#[tauri::command]
fn patient_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    query: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<patients::Patient>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::PATIENT_READ)?;
    with_db(&db, |c| patients::list(c, query, limit.unwrap_or(50)))
}
#[tauri::command]
fn patient_count(db: tauri::State<Db>, actor_token: Option<String>) -> Result<i64, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::PATIENT_READ)?;
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
    actor_token: Option<String>,
    file_no: i64,
) -> Result<Option<patients::Patient>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::PATIENT_READ)?;
    with_db(&db, |c| patients::get_by_file_no(c, file_no))
}
#[tauri::command]
fn patient_inactive(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    years: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<patients::Patient>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::PATIENT_READ)?;
    with_db(&db, |c| {
        patients::inactive_for_years(c, years.unwrap_or(10), limit.unwrap_or(100))
    })
}
#[tauri::command]
fn patient_create(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    input: patients::PatientInput,
) -> Result<patients::Patient, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::PATIENT_WRITE)?;
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    patients::create(&mut g, input)
}
#[tauri::command]
fn patient_update(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    input: patients::PatientInput,
) -> Result<patients::Patient, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::PATIENT_WRITE)?;
    with_db(&db, |c| patients::update(c, id, input))
}
#[tauri::command]
fn patient_delete(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::PATIENT_WRITE)?;
    with_db(&db, |c| patients::soft_delete(c, id))
}
#[tauri::command]
fn patient_future_appointment_count(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
) -> Result<i64, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::PATIENT_READ)?;
    with_db(&db, |c| patients::future_appointment_count(c, id))
}
#[tauri::command]
fn clinic_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<Vec<directory::Clinic>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::DIRECTORY_READ)?;
    with_db(&db, directory::list_clinics)
}
#[tauri::command]
fn clinic_create(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    input: directory::ClinicInput,
) -> Result<directory::Clinic, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::DIRECTORY_WRITE)?;
    with_db(&db, |c| directory::create_clinic(c, input))
}
#[tauri::command]
fn clinic_update(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    input: directory::ClinicInput,
) -> Result<directory::Clinic, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::DIRECTORY_WRITE)?;
    with_db(&db, |c| directory::update_clinic(c, id, input))
}
#[tauri::command]
fn clinic_delete(db: tauri::State<Db>, actor_token: Option<String>, id: i64) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::DIRECTORY_WRITE)?;
    with_db(&db, |c| directory::deactivate_clinic(c, id))
}
#[tauri::command]
fn doctor_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<Vec<directory::Doctor>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::DIRECTORY_READ)?;
    with_db(&db, directory::list_doctors)
}
#[tauri::command]
fn doctor_create(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    input: directory::DoctorInput,
) -> Result<directory::Doctor, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::DIRECTORY_WRITE)?;
    with_db(&db, |c| directory::create_doctor(c, input))
}
#[tauri::command]
fn doctor_update(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    input: directory::DoctorInput,
) -> Result<directory::Doctor, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::DIRECTORY_WRITE)?;
    with_db(&db, |c| directory::update_doctor(c, id, input))
}
#[tauri::command]
fn doctor_delete(db: tauri::State<Db>, actor_token: Option<String>, id: i64) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::DIRECTORY_WRITE)?;
    with_db(&db, |c| directory::deactivate_doctor(c, id))
}
#[tauri::command]
fn appointment_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    from: String,
    to: String,
) -> Result<Vec<appointments::Appointment>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::APPOINTMENT_READ)?;
    with_db(&db, |c| appointments::list(c, &from, &to))
}
#[tauri::command]
fn appointment_missed_history(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    to: String,
) -> Result<Vec<appointments::Appointment>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::APPOINTMENT_READ)?;
    with_db(&db, |c| {
        Ok(appointments::list(c, "1900-01-01T00:00:00", &to)?
            .into_iter()
            .filter(|a| a.status == "no_show")
            .collect())
    })
}
#[tauri::command]
fn appointment_overdue_history(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    to: String,
) -> Result<Vec<appointments::Appointment>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::APPOINTMENT_READ)?;
    with_db(&db, |c| appointments::list_overdue(c, &to, 500))
}
#[tauri::command]
fn appointment_upcoming_all(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    from: String,
) -> Result<Vec<appointments::Appointment>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::APPOINTMENT_READ)?;
    with_db(&db, |c| appointments::list(c, &from, "9999-12-31T23:59:59"))
}
#[tauri::command]
fn patient_appointments(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    patient_id: i64,
    limit: Option<i64>,
) -> Result<Vec<appointments::Appointment>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::APPOINTMENT_READ)?;
    with_db(&db, |c| {
        appointments::list_for_patient(c, patient_id, limit.unwrap_or(30))
    })
}
#[tauri::command]
fn appointment_create(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    input: appointments::AppointmentInput,
) -> Result<appointments::Appointment, String> {
    authorize_command(
        &db,
        actor_token.as_deref(),
        authorization::APPOINTMENT_WRITE,
    )?;
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    appointments::create(&mut g, input)
}
#[tauri::command]
fn appointment_update(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    input: appointments::AppointmentInput,
) -> Result<appointments::Appointment, String> {
    authorize_command(
        &db,
        actor_token.as_deref(),
        authorization::APPOINTMENT_WRITE,
    )?;
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    appointments::update(&mut g, id, input)
}
#[tauri::command]
fn appointment_status(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    status: String,
) -> Result<(), String> {
    authorize_command(
        &db,
        actor_token.as_deref(),
        authorization::APPOINTMENT_WRITE,
    )?;
    let mut g =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    appointment_status::set_status(&mut g, id, &status)
}
#[tauri::command]
fn provider_unavailability_list_open(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<Vec<provider_unavailability::ProviderUnavailabilityEvent>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::APPOINTMENT_READ)?;
    with_db(&db, provider_unavailability::list_open_events)
}
#[tauri::command]
fn provider_unavailability_create(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    input: provider_unavailability::ProviderUnavailabilityInput,
) -> Result<provider_unavailability::ProviderUnavailabilityEvent, String> {
    authorize_command(
        &db,
        actor_token.as_deref(),
        authorization::APPOINTMENT_WRITE,
    )?;
    with_db(&db, |c| provider_unavailability::create_event(c, input))
}
#[tauri::command]
fn provider_unavailability_affected(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    event_id: i64,
) -> Result<Vec<provider_unavailability::AffectedAppointment>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::APPOINTMENT_READ)?;
    with_db(&db, |c| {
        provider_unavailability::affected_appointments(c, event_id)
    })
}
#[tauri::command]
fn provider_unavailability_resolve_many(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    event_id: i64,
    resolutions: Vec<provider_unavailability::ResolutionInput>,
) -> Result<(), String> {
    authorize_command(
        &db,
        actor_token.as_deref(),
        authorization::APPOINTMENT_WRITE,
    )?;
    with_db(&db, |c| {
        provider_unavailability::resolve_many(c, event_id, resolutions)
    })
}
#[tauri::command]
fn visit_tracking_update(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    input: visit_tracking::VisitTrackingInput,
) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::VISIT_WRITE)?;
    with_db(&db, |c| visit_tracking::update(c, id, input))
}
#[tauri::command]
fn visit_follow_ups(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    from: String,
    to: String,
) -> Result<Vec<visit_tracking::FollowUpVisit>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::VISIT_READ)?;
    with_db(&db, |c| visit_tracking::follow_ups(c, &from, &to))
}
#[tauri::command]
fn attachment_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    patient_id: i64,
) -> Result<Vec<attachments::Attachment>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::ATTACHMENT_READ)?;
    with_db(&db, |c| attachments::list(c, patient_id))
}
#[tauri::command]
fn attachment_archived_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    patient_id: i64,
) -> Result<Vec<attachments::Attachment>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::ATTACHMENT_READ)?;
    with_db(&db, |c| attachments::list_archived(c, patient_id))
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
    actor_token: Option<String>,
    patient_id: i64,
    source_path: String,
    display_name: Option<String>,
    category: Option<String>,
) -> Result<i64, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::ATTACHMENT_WRITE)?;
    let root = attachment_root(&app)?;
    with_db(&db, |c| {
        attachments::import_file_named(
            c,
            patient_id,
            Path::new(&source_path),
            &root,
            display_name.as_deref(),
            category.as_deref(),
        )
    })
}
#[tauri::command]
fn attachment_open(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::ATTACHMENT_READ)?;
    let root = attachment_root(&app)?;
    let path = with_db(&db, |c| {
        let (stored_name, size_bytes, expected_sha): (String, i64, String) = c
            .query_row(
                "SELECT stored_name,size_bytes,sha256 FROM attachments WHERE id=?1 AND deleted_at IS NULL",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| "المرفق غير موجود أو مؤرشف".to_string())?;
        if stored_name.is_empty()
            || Path::new(&stored_name)
                .file_name()
                .and_then(|value| value.to_str())
                != Some(stored_name.as_str())
        {
            return Err("اسم المرفق المخزن غير صالح".into());
        }
        let path = root.join(&stored_name);
        let metadata =
            fs::metadata(&path).map_err(|_| "ملف المرفق مفقود من مجلد البرنامج".to_string())?;
        if !metadata.is_file() || metadata.len() != size_bytes as u64 {
            return Err("حجم ملف المرفق لا يطابق السجل؛ لن يتم فتحه".into());
        }
        if backup::sha256_file(&path)? != expected_sha.to_ascii_lowercase() {
            return Err("فشل التحقق من بصمة المرفق؛ لن يتم فتحه".into());
        }
        Ok(path)
    })?;
    let path_string = path
        .to_str()
        .ok_or_else(|| "مسار المرفق غير صالح".to_string())?
        .to_owned();
    app.opener()
        .open_path(path_string, None::<&str>)
        .map_err(|e| format!("تعذر فتح المرفق بواسطة Windows: {e}"))
}

#[tauri::command]
fn attachment_archive(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    reason: String,
) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::ATTACHMENT_WRITE)?;
    with_db(&db, |c| attachments::archive(c, id, &reason))
}
#[tauri::command]
fn attachment_restore(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    reason: String,
) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::ATTACHMENT_WRITE)?;
    with_db(&db, |c| attachments::restore(c, id, &reason))
}
#[tauri::command]
fn scheduling_settings_get(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<scheduling::SchedulingSettings, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::SCHEDULE_READ)?;
    with_db(&db, scheduling::get)
}
#[tauri::command]
fn scheduling_settings_update(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    input: scheduling::SchedulingSettings,
) -> Result<scheduling::SchedulingSettings, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::SCHEDULE_WRITE)?;
    with_db(&db, |c| scheduling::update(c, input))
}
#[tauri::command]
fn closure_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<Vec<scheduling::ClosureDate>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::SCHEDULE_READ)?;
    with_db(&db, scheduling::closures)
}
#[tauri::command]
fn closure_create(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    input: scheduling::ClosureInput,
) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::SCHEDULE_WRITE)?;
    with_db(&db, |c| scheduling::add_closure(c, input))
}
#[tauri::command]
fn closure_delete(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
) -> Result<(), String> {
    authorize_command(&db, actor_token.as_deref(), authorization::SCHEDULE_WRITE)?;
    with_db(&db, |c| scheduling::delete_closure(c, id))
}
fn security_log_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("security").join("events.jsonl"))
        .map_err(|e| e.to_string())
}
fn security_failure(
    log: &Path,
    event_type: &str,
    reference: &str,
    stage: &str,
    error: &str,
    sha: Option<&str>,
) {
    let _ = security_log::append_operation(
        log,
        event_type,
        "failure",
        reference,
        stage,
        security_log::failure_code(error),
        sha,
    );
}
fn record_security_log_warning(db: &Connection, reference: &str) {
    let warning = serde_json::json!({
        "operationRef": reference,
        "code": "security_log_write_failed"
    })
    .to_string();
    let _ = audit::record(
        db,
        "security_log_write_failed",
        "database",
        None,
        Some(&warning),
    );
}
#[tauri::command]
fn backup_create(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    actor_token: Option<String>,
    destination_path: String,
) -> Result<String, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::BACKUP_CREATE)?;
    let reference = format!("BKP-{}", Uuid::new_v4());
    let log = security_log_path(&app)?;
    security_log::verify(&log)?;
    security_log::append_operation(
        &log,
        "backup",
        "started",
        &reference,
        "preflight",
        "started",
        None,
    )?;
    let g = db.0.lock().map_err(|_| {
        let err = "تعذر الوصول إلى قاعدة البيانات".to_string();
        security_failure(&log, "backup", &reference, "database_lock", &err, None);
        err
    })?;
    let hash = match backup::create_database_backup(
        &g,
        Path::new(&destination_path),
        LATEST_SCHEMA_VERSION,
    ) {
        Ok(hash) => hash,
        Err(error) => {
            security_failure(&log, "backup", &reference, "backup_create", &error, None);
            return Err(error);
        }
    };
    let details = serde_json::json!({"sha256":hash,"backupRef":reference}).to_string();
    if let Err(error) = audit::record(&g, "backup_created", "database", None, Some(&details)) {
        security_failure(
            &log,
            "backup",
            &reference,
            "audit_write",
            &error,
            Some(&hash),
        );
    }
    if security_log::append_operation(
        &log,
        "backup",
        "success",
        &reference,
        "completed",
        "ok",
        Some(&hash),
    )
    .is_err()
    {
        record_security_log_warning(&g, &reference);
    }
    Ok(hash)
}
#[tauri::command]
fn backup_restore(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    actor_token: Option<String>,
    source_path: String,
) -> Result<String, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::BACKUP_RESTORE)?;
    let reference = format!("RST-{}", Uuid::new_v4());
    let log = security_log_path(&app)?;
    security_log::verify(&log)?;
    security_log::append_operation(
        &log,
        "restore",
        "started",
        &reference,
        "preflight",
        "started",
        None,
    )?;
    let source = Path::new(&source_path);
    let source_hash = match backup::sha256_file(source) {
        Ok(hash) => hash,
        Err(error) => {
            security_failure(&log, "restore", &reference, "source_read", &error, None);
            return Err(error);
        }
    };
    if let Err(error) = backup::verify_database(source, LATEST_SCHEMA_VERSION) {
        security_failure(
            &log,
            "restore",
            &reference,
            "source_database_verify",
            &error,
            Some(&source_hash),
        );
        return Err(error);
    }
    if let Err(error) = backup::verify_attachment_backup(source) {
        security_failure(
            &log,
            "restore",
            &reference,
            "source_attachment_verify",
            &error,
            Some(&source_hash),
        );
        return Err(error);
    }
    let mut g = match db.0.lock() {
        Ok(guard) => guard,
        Err(_) => {
            let error = "تعذر الوصول إلى قاعدة البيانات".to_string();
            security_failure(
                &log,
                "restore",
                &reference,
                "database_lock",
                &error,
                Some(&source_hash),
            );
            return Err(error);
        }
    };
    let details = serde_json::json!({"sha256":source_hash,"restoreRef":reference}).to_string();
    if let Err(error) = audit::record(&g, "restore_started", "database", None, Some(&details)) {
        security_failure(
            &log,
            "restore",
            &reference,
            "audit_start",
            &error,
            Some(&source_hash),
        );
        return Err(error);
    }
    let live_path = match app.path().app_data_dir() {
        Ok(path) => path.join("clinicdesk.sqlite3"),
        Err(error) => {
            let error = error.to_string();
            security_failure(
                &log,
                "restore",
                &reference,
                "live_path",
                &error,
                Some(&source_hash),
            );
            return Err(error);
        }
    };
    match backup::restore_database(&mut g, source, &live_path, LATEST_SCHEMA_VERSION) {
        Ok(hash) => {
            if let Err(error) =
                audit::record(&g, "restore_completed", "database", None, Some(&details))
            {
                security_failure(
                    &log,
                    "restore",
                    &reference,
                    "audit_complete",
                    &error,
                    Some(&hash),
                );
            }
            if security_log::append_operation(
                &log,
                "restore",
                "success",
                &reference,
                "completed",
                "ok",
                Some(&hash),
            )
            .is_err()
            {
                record_security_log_warning(&g, &reference);
            }
            Ok(hash)
        }
        Err(error) => {
            security_failure(
                &log,
                "restore",
                &reference,
                "restore_apply",
                &error,
                Some(&source_hash),
            );
            Err(error)
        }
    }
}
#[tauri::command]
fn audit_recent(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<audit::AuditEntry>, String> {
    authorize_command(&db, actor_token.as_deref(), authorization::AUDIT_READ)?;
    with_db(&db, |c| audit::recent(c, limit.unwrap_or(100)))
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let db = init_db(&dir.join("clinicdesk.sqlite3"))?;
            app.manage(Db(Mutex::new(db)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth_commands::auth_login,
            auth_commands::auth_logout,
            auth_commands::auth_validate_session,
            auth_commands::auth_change_password,
            auth_commands::user_list,
            auth_commands::user_create,
            auth_commands::user_update,
            auth_commands::user_set_status,
            auth_commands::user_reset_password,
            auth_commands::deputy_restore_permission_get,
            auth_commands::deputy_restore_permission_set,
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
            appointment_overdue_history,
            appointment_upcoming_all,
            patient_appointments,
            appointment_create,
            appointment_update,
            appointment_status,
            provider_unavailability_list_open,
            provider_unavailability_create,
            provider_unavailability_affected,
            provider_unavailability_resolve_many,
            visit_tracking_update,
            visit_tracking_command::visit_tracking_get,
            visit_follow_ups,
            attachment_list,
            attachment_archived_list,
            attachment_import,
            attachment_open,
            attachment_archive,
            attachment_restore,
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
