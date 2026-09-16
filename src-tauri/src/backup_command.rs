use crate::{backup, Db, LATEST_SCHEMA_VERSION};
use std::path::Path;
use tauri::Manager;

#[tauri::command]
pub fn backup_create(
    db: tauri::State<Db>,
    destination_path: String,
) -> Result<String, String> {
    let guard = db
        .0
        .lock()
        .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    backup::create_database_backup(
        &guard,
        Path::new(&destination_path),
        LATEST_SCHEMA_VERSION,
    )
}

#[tauri::command]
pub fn backup_restore(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    source_path: String,
) -> Result<String, String> {
    let live_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("clinicdesk.sqlite3");
    let mut guard = db
        .0
        .lock()
        .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    backup::restore_database(
        &mut guard,
        Path::new(&source_path),
        &live_path,
        LATEST_SCHEMA_VERSION,
    )
}
