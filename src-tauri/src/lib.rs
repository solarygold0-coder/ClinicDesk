pub mod appointments;
pub mod attachments;
pub mod directory;
pub mod domain;
pub mod patients;
pub mod scheduling;
pub mod visit_tracking;
use rusqlite::Connection;
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::Manager;
/// Shared local SQLite connection used by Tauri commands.
pub struct Db(pub Mutex<Connection>);
fn init_db(path: &PathBuf) -> Result<Connection, String> {
    if let Some(p) = path.parent() { fs::create_dir_all(p).map_err(|e| e.to_string())?; }
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    db.execute_batch(include_str!("../migrations/001_init.sql")).map_err(|e| e.to_string())?;
    db.execute_batch(include_str!("../migrations/002_touch_triggers.sql")).map_err(|e| e.to_string())?;
    db.execute_batch(include_str!("../migrations/003_scheduling_rules.sql")).map_err(|e| e.to_string())?;
    let schema_version:i64=db.query_row("SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    if schema_version<4 { db.execute_batch(include_str!("../migrations/004_patient_medical_details.sql")).map_err(|e|e.to_string())?; }
    let schema_version:i64=db.query_row("SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    if schema_version<5 { db.execute_batch(include_str!("../migrations/005_patient_last_activity.sql")).map_err(|e|e.to_string())?; }
    let schema_version:i64=db.query_row("SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    if schema_version<6 { db.execute_batch(include_str!("../migrations/006_patient_activity_triggers.sql")).map_err(|e|e.to_string())?; }
    let schema_version:i64=db.query_row("SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    if schema_version<7 { db.execute_batch(include_str!("../migrations/007_appointment_visit_tracking.sql")).map_err(|e|e.to_string())?; }
    Ok(db)
}
fn with_db<T>(db:&tauri::State<Db>,f:impl FnOnce(&Connection)->Result<T,String>)->Result<T,String>{let g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;f(&g)}
#[tauri::command] fn health()->&'static str{"ok"}
#[tauri::command] fn patient_list(db:tauri::State<Db>,query:Option<String>,limit:Option<i64>)->Result<Vec<patients::Patient>,String>{with_db(&db,|c|patients::list(c,query,limit.unwrap_or(50)))}
#[tauri::command] fn patient_count(db:tauri::State<Db>)->Result<i64,String>{with_db(&db,|c|c.query_row("SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL",[],|r|r.get(0)).map_err(|e|e.to_string()))}
#[tauri::command] fn patient_by_file_no(db:tauri::State<Db>,file_no:i64)->Result<Option<patients::Patient>,String>{with_db(&db,|c|patients::get_by_file_no(c,file_no))}
#[tauri::command] fn patient_inactive(db:tauri::State<Db>,years:Option<i64>,limit:Option<i64>)->Result<Vec<patients::Patient>,String>{with_db(&db,|c|patients::inactive_for_years(c,years.unwrap_or(10),limit.unwrap_or(100)))}
#[tauri::command] fn patient_create(db:tauri::State<Db>,input:patients::PatientInput)->Result<patients::Patient,String>{let mut g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;patients::create(&mut g,input)}
#[tauri::command] fn patient_update(db:tauri::State<Db>,id:i64,input:patients::PatientInput)->Result<patients::Patient,String>{with_db(&db,|c|patients::update(c,id,input))}
#[tauri::command] fn patient_delete(db:tauri::State<Db>,id:i64)->Result<(),String>{with_db(&db,|c|patients::soft_delete(c,id))}
#[tauri::command] fn patient_future_appointment_count(db:tauri::State<Db>,id:i64)->Result<i64,String>{with_db(&db,|c|patients::future_appointment_count(c,id))}
#[tauri::command] fn clinic_list(db:tauri::State<Db>)->Result<Vec<directory::Clinic>,String>{with_db(&db,directory::list_clinics)}
#[tauri::command] fn clinic_create(db:tauri::State<Db>,input:directory::ClinicInput)->Result<directory::Clinic,String>{with_db(&db,|c|directory::create_clinic(c,input))}
#[tauri::command] fn doctor_list(db:tauri::State<Db>)->Result<Vec<directory::Doctor>,String>{with_db(&db,directory::list_doctors)}
#[tauri::command] fn doctor_create(db:tauri::State<Db>,input:directory::DoctorInput)->Result<directory::Doctor,String>{with_db(&db,|c|directory::create_doctor(c,input))}
#[tauri::command] fn appointment_list(db:tauri::State<Db>,from:String,to:String)->Result<Vec<appointments::Appointment>,String>{with_db(&db,|c|appointments::list(c,&from,&to))}
#[tauri::command] fn appointment_missed_history(db:tauri::State<Db>,to:String)->Result<Vec<appointments::Appointment>,String>{with_db(&db,|c|{let rows=appointments::list(c,"1900-01-01T00:00:00",&to)?;Ok(rows.into_iter().filter(|a|a.status=="no_show").collect())})}
#[tauri::command] fn appointment_upcoming_all(db:tauri::State<Db>,from:String)->Result<Vec<appointments::Appointment>,String>{with_db(&db,|c|appointments::list(c,&from,"9999-12-31T23:59:59"))}
#[tauri::command] fn patient_appointments(db:tauri::State<Db>,patient_id:i64,limit:Option<i64>)->Result<Vec<appointments::Appointment>,String>{with_db(&db,|c|appointments::list_for_patient(c,patient_id,limit.unwrap_or(30)))}
#[tauri::command] fn appointment_create(db:tauri::State<Db>,input:appointments::AppointmentInput)->Result<appointments::Appointment,String>{let mut g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;appointments::create(&mut g,input)}
#[tauri::command] fn appointment_update(db:tauri::State<Db>,id:i64,input:appointments::AppointmentInput)->Result<appointments::Appointment,String>{let mut g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;appointments::update(&mut g,id,input)}
#[tauri::command] fn appointment_status(db:tauri::State<Db>,id:i64,status:String)->Result<(),String>{with_db(&db,|c|appointments::set_status(c,id,&status))}
#[tauri::command] fn visit_tracking_update(db:tauri::State<Db>,id:i64,input:visit_tracking::VisitTrackingInput)->Result<(),String>{with_db(&db,|c|visit_tracking::update(c,id,input))}
#[tauri::command] fn visit_follow_ups(db:tauri::State<Db>,from:String,to:String)->Result<Vec<visit_tracking::FollowUpVisit>,String>{with_db(&db,|c|visit_tracking::follow_ups(c,&from,&to))}
#[tauri::command] fn attachment_list(db:tauri::State<Db>,patient_id:i64)->Result<Vec<attachments::Attachment>,String>{with_db(&db,|c|attachments::list(c,patient_id))}
#[tauri::command] fn attachment_add(db:tauri::State<Db>,patient_id:i64,stored_name:String,original_name:String,mime_type:Option<String>,size_bytes:i64,sha256:String)->Result<i64,String>{with_db(&db,|c|attachments::add(c,patient_id,&stored_name,&original_name,mime_type.as_deref(),size_bytes,&sha256))}
#[tauri::command] fn attachment_remove(db:tauri::State<Db>,id:i64)->Result<String,String>{with_db(&db,|c|attachments::remove(c,id))}
#[tauri::command] fn scheduling_settings_get(db:tauri::State<Db>)->Result<scheduling::SchedulingSettings,String>{with_db(&db,scheduling::get)}
#[tauri::command] fn scheduling_settings_update(db:tauri::State<Db>,input:scheduling::SchedulingSettings)->Result<scheduling::SchedulingSettings,String>{with_db(&db,|c|scheduling::update(c,input))}
#[tauri::command] fn closure_list(db:tauri::State<Db>)->Result<Vec<scheduling::ClosureDate>,String>{with_db(&db,scheduling::closures)}
#[tauri::command] fn closure_create(db:tauri::State<Db>,input:scheduling::ClosureInput)->Result<(),String>{with_db(&db,|c|scheduling::add_closure(c,input))}
#[tauri::command] fn closure_delete(db:tauri::State<Db>,id:i64)->Result<(),String>{with_db(&db,|c|scheduling::delete_closure(c,id))}
#[cfg_attr(mobile,tauri::mobile_entry_point)]
pub fn run(){tauri::Builder::default().setup(|app|{let path=app.path().app_data_dir()?.join("clinicdesk.sqlite3");let db=init_db(&path).map_err(std::io::Error::other)?;app.manage(Db(Mutex::new(db)));Ok(())}).invoke_handler(tauri::generate_handler![health,patient_list,patient_count,patient_by_file_no,patient_inactive,patient_create,patient_update,patient_delete,patient_future_appointment_count,clinic_list,clinic_create,doctor_list,doctor_create,appointment_list,appointment_missed_history,appointment_upcoming_all,patient_appointments,appointment_create,appointment_update,appointment_status,visit_tracking_update,visit_follow_ups,attachment_list,attachment_add,attachment_remove,scheduling_settings_get,scheduling_settings_update,closure_list,closure_create,closure_delete]).run(tauri::generate_context!()).expect("error while running ClinicDesk");}
