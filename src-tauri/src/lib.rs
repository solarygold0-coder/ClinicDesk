pub mod appointments;
pub mod directory;
pub mod domain;
pub mod patients;
pub mod scheduling;
use rusqlite::Connection;
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::Manager;
pub struct Db(pub Mutex<Connection>);
fn init_db(path: &PathBuf) -> Result<Connection, String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    db.execute_batch(include_str!("../migrations/001_init.sql"))
        .map_err(|e| e.to_string())?;
    db.execute_batch(include_str!("../migrations/002_touch_triggers.sql"))
        .map_err(|e| e.to_string())?;
    db.execute_batch(include_str!("../migrations/003_scheduling_rules.sql"))
        .map_err(|e| e.to_string())?;
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
fn patient_by_file_no(
    db: tauri::State<Db>,
    file_no: i64,
) -> Result<Option<patients::Patient>, String> {
    with_db(&db, |c| patients::get_by_file_no(c, file_no))
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
fn appointment_list(
    db: tauri::State<Db>,
    from: String,
    to: String,
) -> Result<Vec<appointments::Appointment>, String> {
    with_db(&db, |c| appointments::list(c, &from, &to))
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
    with_db(&db, |c| appointments::set_status(c, id, &status))
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
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("clinicdesk.sqlite3");
            let db = init_db(&path).map_err(std::io::Error::other)?;
            app.manage(Db(Mutex::new(db)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health,
            patient_list,
            patient_by_file_no,
            patient_create,
            patient_update,
            patient_delete,
            patient_future_appointment_count,
            clinic_list,
            clinic_create,
            doctor_list,
            doctor_create,
            appointment_list,
            patient_appointments,
            appointment_create,
            appointment_update,
            appointment_status,
            scheduling_settings_get,
            scheduling_settings_update,
            closure_list,
            closure_create,
            closure_delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running ClinicDesk");
}
