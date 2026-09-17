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

pub const LATEST_SCHEMA_VERSION: i64 = 15;

pub struct Db(pub Mutex<Connection>);

pub fn migrate_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|e| e.to_string())?;
    conn.execute_batch(include_str!("../migrations/001_init.sql"))
        .map_err(|e| e.to_string())?;
    let mut version: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if version > LATEST_SCHEMA_VERSION {
        return Err(format!("إصدار قاعدة البيانات {version} أحدث من الإصدار المدعوم {LATEST_SCHEMA_VERSION}"));
    }
    let migrations: &[(i64, &str)] = &[
        (2, include_str!("../migrations/002_indexes.sql")),
        (3, include_str!("../migrations/003_directory.sql")),
        (4, include_str!("../migrations/004_scheduling.sql")),
        (5, include_str!("../migrations/005_audit.sql")),
        (6, include_str!("../migrations/006_attachments.sql")),
        (7, include_str!("../migrations/007_visit_tracking.sql")),
        (8, include_str!("../migrations/008_appointment_status.sql")),
        (9, include_str!("../migrations/009_patient_search.sql")),
        (10, include_str!("../migrations/010_users_roles_audit_actor.sql")),
        (11, include_str!("../migrations/011_accountability_identity.sql")),
        (12, include_str!("../migrations/012_immutable_employee_identity.sql")),
        (13, include_str!("../migrations/013_user_roles_lifecycle.sql")),
        (14, include_str!("../migrations/014_attachment_lifecycle.sql")),
        (15, include_str!("../migrations/015_attachment_restore_limit.sql")),
    ];
    for (target, sql) in migrations {
        if version < *target {
            conn.execute_batch(sql).map_err(|e| e.to_string())?;
            version = *target;
        }
    }
    Ok(())
}

fn init_db(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    migrate_db(&conn)?;
    Ok(conn)
}

fn with_db<T>(db: &tauri::State<Db>, f: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
    let g = db.0.lock().map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    f(&g)
}

#[tauri::command]
fn health() -> &'static str { "ok" }
#[tauri::command]
fn patient_list(db: tauri::State<Db>, search: Option<String>, limit: Option<i64>) -> Result<Vec<patients::Patient>, String> { with_db(&db, |c| patients::list(c, search.as_deref(), limit.unwrap_or(100))) }
#[tauri::command]
fn patient_count(db: tauri::State<Db>) -> Result<i64, String> { with_db(&db, patients::count) }
#[tauri::command]
fn patient_by_file_no(db: tauri::State<Db>, file_no: i64) -> Result<Option<patients::Patient>, String> { with_db(&db, |c| patients::by_file_no(c, file_no)) }
#[tauri::command]
fn patient_inactive(db: tauri::State<Db>, years: Option<i64>, limit: Option<i64>) -> Result<Vec<patients::Patient>, String> { with_db(&db, |c| patients::inactive_for_years(c, years.unwrap_or(10), limit.unwrap_or(100))) }
#[tauri::command]
fn patient_create(db: tauri::State<Db>, input: patients::PatientInput) -> Result<patients::Patient, String> { let mut g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?; patients::create(&mut g,input) }
#[tauri::command]
fn patient_update(db: tauri::State<Db>, id:i64, input:patients::PatientInput)->Result<patients::Patient,String>{with_db(&db,|c|patients::update(c,id,input))}
#[tauri::command]
fn patient_delete(db:tauri::State<Db>,id:i64)->Result<(),String>{with_db(&db,|c|patients::soft_delete(c,id))}
#[tauri::command]
fn patient_future_appointment_count(db:tauri::State<Db>,id:i64)->Result<i64,String>{with_db(&db,|c|patients::future_appointment_count(c,id))}
#[tauri::command]
fn clinic_list(db:tauri::State<Db>)->Result<Vec<directory::Clinic>,String>{with_db(&db,directory::list_clinics)}
#[tauri::command]
fn clinic_create(db:tauri::State<Db>,input:directory::ClinicInput)->Result<directory::Clinic,String>{with_db(&db,|c|directory::create_clinic(c,input))}
#[tauri::command]
fn clinic_update(db:tauri::State<Db>,id:i64,input:directory::ClinicInput)->Result<directory::Clinic,String>{with_db(&db,|c|directory::update_clinic(c,id,input))}
#[tauri::command]
fn clinic_delete(db:tauri::State<Db>,id:i64)->Result<(),String>{with_db(&db,|c|directory::deactivate_clinic(c,id))}
#[tauri::command]
fn doctor_list(db:tauri::State<Db>)->Result<Vec<directory::Doctor>,String>{with_db(&db,directory::list_doctors)}
#[tauri::command]
fn doctor_create(db:tauri::State<Db>,input:directory::DoctorInput)->Result<directory::Doctor,String>{with_db(&db,|c|directory::create_doctor(c,input))}
#[tauri::command]
fn doctor_update(db:tauri::State<Db>,id:i64,input:directory::DoctorInput)->Result<directory::Doctor,String>{with_db(&db,|c|directory::update_doctor(c,id,input))}
#[tauri::command]
fn doctor_delete(db:tauri::State<Db>,id:i64)->Result<(),String>{with_db(&db,|c|directory::deactivate_doctor(c,id))}
#[tauri::command]
fn appointment_list(db:tauri::State<Db>,from:String,to:String)->Result<Vec<appointments::Appointment>,String>{with_db(&db,|c|appointments::list(c,&from,&to))}
#[tauri::command]
fn appointment_missed_history(db:tauri::State<Db>,to:String)->Result<Vec<appointments::Appointment>,String>{with_db(&db,|c|Ok(appointments::list(c,"1900-01-01T00:00:00",&to)?.into_iter().filter(|a|a.status=="no_show").collect()))}
#[tauri::command]
fn appointment_upcoming_all(db:tauri::State<Db>,from:String)->Result<Vec<appointments::Appointment>,String>{with_db(&db,|c|appointments::list(c,&from,"9999-12-31T23:59:59"))}
#[tauri::command]
fn patient_appointments(db:tauri::State<Db>,patient_id:i64,limit:Option<i64>)->Result<Vec<appointments::Appointment>,String>{with_db(&db,|c|appointments::list_for_patient(c,patient_id,limit.unwrap_or(30)))}
#[tauri::command]
fn appointment_create(db:tauri::State<Db>,input:appointments::AppointmentInput)->Result<appointments::Appointment,String>{let mut g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;appointments::create(&mut g,input)}
#[tauri::command]
fn appointment_update(db:tauri::State<Db>,id:i64,input:appointments::AppointmentInput)->Result<appointments::Appointment,String>{let mut g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;appointments::update(&mut g,id,input)}
#[tauri::command]
fn appointment_status(db:tauri::State<Db>,id:i64,status:String)->Result<(),String>{let mut g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;appointment_status::set_status(&mut g,id,&status)}
#[tauri::command]
fn visit_tracking_update(db:tauri::State<Db>,id:i64,input:visit_tracking::VisitTrackingInput)->Result<(),String>{with_db(&db,|c|visit_tracking::update(c,id,input))}
#[tauri::command]
fn visit_follow_ups(db:tauri::State<Db>,from:String,to:String)->Result<Vec<visit_tracking::FollowUpVisit>,String>{with_db(&db,|c|visit_tracking::follow_ups(c,&from,&to))}
#[tauri::command]
fn attachment_list(db:tauri::State<Db>,patient_id:i64)->Result<Vec<attachments::Attachment>,String>{with_db(&db,|c|attachments::list(c,patient_id))}
#[tauri::command]
fn attachment_archived_list(db:tauri::State<Db>,patient_id:i64)->Result<Vec<attachments::Attachment>,String>{with_db(&db,|c|attachments::list_archived(c,patient_id))}
fn attachment_root(app:&tauri::AppHandle)->Result<PathBuf,String>{app.path().app_data_dir().map(|p|p.join("attachments")).map_err(|e|e.to_string())}
#[tauri::command]
fn attachment_import(app:tauri::AppHandle,db:tauri::State<Db>,patient_id:i64,source_path:String,display_name:Option<String>,category:Option<String>)->Result<i64,String>{let root=attachment_root(&app)?;with_db(&db,|c|attachments::import_file_named(c,patient_id,Path::new(&source_path),&root,display_name.as_deref(),category.as_deref()))}
#[tauri::command]
fn attachment_archive(db:tauri::State<Db>,id:i64,reason:String)->Result<(),String>{with_db(&db,|c|attachments::archive(c,id,&reason))}
#[tauri::command]
fn attachment_restore(db:tauri::State<Db>,id:i64,reason:String)->Result<(),String>{with_db(&db,|c|attachments::restore(c,id,&reason))}
#[tauri::command]
fn scheduling_settings_get(db:tauri::State<Db>)->Result<scheduling::SchedulingSettings,String>{with_db(&db,scheduling::get)}
#[tauri::command]
fn scheduling_settings_update(db:tauri::State<Db>,input:scheduling::SchedulingSettings)->Result<scheduling::SchedulingSettings,String>{with_db(&db,|c|scheduling::update(c,input))}
#[tauri::command]
fn closure_list(db:tauri::State<Db>)->Result<Vec<scheduling::ClosureDate>,String>{with_db(&db,scheduling::closures)}
#[tauri::command]
fn closure_create(db:tauri::State<Db>,input:scheduling::ClosureInput)->Result<(),String>{with_db(&db,|c|scheduling::add_closure(c,input))}
#[tauri::command]
fn closure_delete(db:tauri::State<Db>,id:i64)->Result<(),String>{with_db(&db,|c|scheduling::delete_closure(c,id))}

fn security_log_path(app:&tauri::AppHandle)->Result<PathBuf,String>{app.path().app_data_dir().map(|p|p.join("security").join("events.jsonl")).map_err(|e|e.to_string())}
fn security_failure(log:&Path,event_type:&str,reference:&str,stage:&str,error:&str,sha:Option<&str>){let _=security_log::append_operation(log,event_type,"failure",reference,stage,security_log::failure_code(error),sha);}

#[tauri::command]
fn backup_create(app:tauri::AppHandle,db:tauri::State<Db>,destination_path:String)->Result<String,String>{
    let reference=format!("BKP-{}",Uuid::new_v4());
    let log=security_log_path(&app)?;
    security_log::verify(&log)?;
    security_log::append_operation(&log,"backup","started",&reference,"backup_create","started",None)?;
    let g=db.0.lock().map_err(|_|{let e="تعذر الوصول إلى قاعدة البيانات".to_string();security_failure(&log,"backup",&reference,"db_lock",&e,None);e})?;
    let hash=match backup::create_database_backup(&g,Path::new(&destination_path),LATEST_SCHEMA_VERSION){Ok(h)=>h,Err(e)=>{security_failure(&log,"backup",&reference,"backup_create",&e,None);return Err(e)}};
    let details=serde_json::json!({"sha256":hash,"backupRef":reference}).to_string();
    if let Err(e)=audit::record(&g,"backup_created","database",None,Some(&details)){security_failure(&log,"backup",&reference,"audit_write",&e,Some(&hash));return Err(e)}
    security_log::append_operation(&log,"backup","success",&reference,"backup_create","completed",Some(&hash))?;
    Ok(hash)
}

#[tauri::command]
fn backup_restore(app:tauri::AppHandle,db:tauri::State<Db>,source_path:String)->Result<String,String>{
    let reference=format!("RST-{}",Uuid::new_v4());
    let log=security_log_path(&app)?;
    security_log::verify(&log)?;
    security_log::append_operation(&log,"restore","started",&reference,"restore_request","started",None)?;
    let source=Path::new(&source_path);
    let source_hash=match backup::sha256_file(source){Ok(h)=>h,Err(e)=>{security_failure(&log,"restore",&reference,"source_read",&e,None);return Err(e)}};
    if let Err(e)=backup::verify_database(source,LATEST_SCHEMA_VERSION){security_failure(&log,"restore",&reference,"source_verify",&e,Some(&source_hash));return Err(e)}
    if let Err(e)=backup::verify_attachment_backup(source){security_failure(&log,"restore",&reference,"attachment_verify",&e,Some(&source_hash));return Err(e)}
    let mut g=match db.0.lock(){Ok(g)=>g,Err(_)=>{let e="تعذر الوصول إلى قاعدة البيانات".to_string();security_failure(&log,"restore",&reference,"db_lock",&e,Some(&source_hash));return Err(e)}};
    let details=serde_json::json!({"sha256":source_hash,"restoreRef":reference}).to_string();
    if let Err(e)=audit::record(&g,"restore_started","database",None,Some(&details)){security_failure(&log,"restore",&reference,"audit_start",&e,Some(&source_hash));return Err(e)}
    let live_path=app.path().app_data_dir().map_err(|e|e.to_string())?.join("clinicdesk.sqlite3");
    match backup::restore_database(&mut g,source,&live_path,LATEST_SCHEMA_VERSION){
        Ok(hash)=>{
            audit::record(&g,"restore_completed","database",None,Some(&details))?;
            if security_log::append_operation(&log,"restore","success",&reference,"restore_commit","completed",Some(&hash)).is_err(){
                let warning=serde_json::json!({"restoreRef":reference,"code":"security_log_write_failed"}).to_string();
                let _=audit::record(&g,"security_log_write_failed","database",None,Some(&warning));
            }
            Ok(hash)
        }
        Err(e)=>{security_failure(&log,"restore",&reference,"restore_commit",&e,Some(&source_hash));Err(e)}
    }
}

#[tauri::command]
fn audit_recent(db:tauri::State<Db>,limit:Option<i64>)->Result<Vec<audit::AuditEntry>,String>{with_db(&db,|c|audit::recent(c,limit.unwrap_or(100)))}

#[cfg_attr(mobile,tauri::mobile_entry_point)]
pub fn run(){
    tauri::Builder::default().plugin(tauri_plugin_opener::init()).setup(|app|{let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;fs::create_dir_all(&dir).map_err(|e|e.to_string())?;let db=init_db(&dir.join("clinicdesk.sqlite3"))?;app.manage(Db(Mutex::new(db)));Ok(())}).invoke_handler(tauri::generate_handler![
        health,patient_list,patient_count,patient_by_file_no,patient_inactive,patient_create,patient_update,patient_delete,patient_future_appointment_count,
        clinic_list,clinic_create,clinic_update,clinic_delete,doctor_list,doctor_create,doctor_update,doctor_delete,
        appointment_list,appointment_missed_history,appointment_upcoming_all,patient_appointments,appointment_create,appointment_update,appointment_status,
        visit_tracking_update,visit_tracking_command::visit_tracking_get,visit_follow_ups,
        attachment_list,attachment_archived_list,attachment_import,attachment_archive,attachment_restore,
        scheduling_settings_get,scheduling_settings_update,closure_list,closure_create,closure_delete,
        users::user_list,users::user_create,users::user_update,users::user_deactivate,
        backup_create,backup_restore,audit_recent
    ]).run(tauri::generate_context!()).expect("error while running ClinicDesk");
}
