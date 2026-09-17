pub mod appointment_status;
pub mod appointments;
pub mod attachments;
pub mod audit;
pub mod backup;
pub mod directory;
pub mod domain;
pub mod patients;
pub mod scheduling;
pub mod users;
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
const LATEST_SCHEMA_VERSION: i64 = 12;

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
        (4, include_str!("../migrations/004_patient_medical_details.sql")),
        (5, include_str!("../migrations/005_patient_last_activity.sql")),
        (6, include_str!("../migrations/006_patient_activity_triggers.sql")),
        (7, include_str!("../migrations/007_appointment_visit_tracking.sql")),
        (8, include_str!("../migrations/008_optional_auth.sql")),
        (9, include_str!("../migrations/009_remove_optional_auth.sql")),
        (10, include_str!("../migrations/010_restore_users_roles_audit_actor.sql")),
        (11, include_str!("../migrations/011_accountability_identity.sql")),
        (12, include_str!("../migrations/012_immutable_employee_identity.sql")),
    ] {
        if schema_version(db)? < version {
            db.execute_batch(sql).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn open_database(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
        .map_err(|e| e.to_string())?;
    migrate_db(&db)?;
    Ok(db)
}

fn database_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("clinicdesk.sqlite3"))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let path = database_path(&app.handle())?;
            let db = open_database(&path).map_err(std::io::Error::other)?;
            app.manage(Db(Mutex::new(db)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            patients::patient_list,
            patients::patient_get,
            patients::patient_create,
            patients::patient_update,
            patients::patient_delete,
            appointments::appointment_list,
            appointments::appointment_create,
            appointments::appointment_update,
            appointments::appointment_delete,
            directory::clinic_list,
            directory::clinic_create,
            directory::clinic_update,
            directory::clinic_set_active,
            directory::doctor_list,
            directory::doctor_create,
            directory::doctor_update,
            directory::doctor_set_active,
            visit_tracking_command::appointment_check_in,
            visit_tracking_command::appointment_mark_seen,
            visit_tracking_command::appointment_complete,
            visit_tracking_command::appointment_mark_no_show,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ClinicDesk");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_schema_is_twelve() {
        let db = Connection::open_in_memory().unwrap();
        migrate_db(&db).unwrap();
        assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);
    }
}
