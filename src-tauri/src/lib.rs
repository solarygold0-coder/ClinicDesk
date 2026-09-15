pub mod domain;pub mod patients;pub mod directory;
use rusqlite::Connection;use std::{fs,path::PathBuf,sync::Mutex};use tauri::Manager;
pub struct Db(pub Mutex<Connection>);
fn init_db(path:&PathBuf)->Result<Connection,String>{if let Some(p)=path.parent(){fs::create_dir_all(p).map_err(|e|e.to_string())?;}let db=Connection::open(path).map_err(|e|e.to_string())?;db.execute_batch(include_str!("../migrations/001_init.sql")).map_err(|e|e.to_string())?;db.execute_batch(include_str!("../migrations/002_touch_triggers.sql")).map_err(|e|e.to_string())?;Ok(db)}
fn with_db<T>(db:&tauri::State<Db>,f:impl FnOnce(&Connection)->Result<T,String>)->Result<T,String>{let g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;f(&g)}
#[tauri::command]fn health()->&'static str{"ok"}
#[tauri::command]fn patient_list(db:tauri::State<Db>,query:Option<String>,limit:Option<i64>)->Result<Vec<patients::Patient>,String>{with_db(&db,|c|patients::list(c,query,limit.unwrap_or(50)))}
#[tauri::command]fn patient_create(db:tauri::State<Db>,input:patients::PatientInput)->Result<patients::Patient,String>{let mut g=db.0.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;patients::create(&mut g,input)}
#[tauri::command]fn patient_update(db:tauri::State<Db>,id:i64,input:patients::PatientInput)->Result<patients::Patient,String>{with_db(&db,|c|patients::update(c,id,input))}
#[tauri::command]fn patient_delete(db:tauri::State<Db>,id:i64)->Result<(),String>{with_db(&db,|c|patients::soft_delete(c,id))}
#[tauri::command]fn clinic_list(db:tauri::State<Db>)->Result<Vec<directory::Clinic>,String>{with_db(&db,directory::list_clinics)}
#[tauri::command]fn clinic_create(db:tauri::State<Db>,input:directory::ClinicInput)->Result<directory::Clinic,String>{with_db(&db,|c|directory::create_clinic(c,input))}
#[tauri::command]fn doctor_list(db:tauri::State<Db>)->Result<Vec<directory::Doctor>,String>{with_db(&db,directory::list_doctors)}
#[tauri::command]fn doctor_create(db:tauri::State<Db>,input:directory::DoctorInput)->Result<directory::Doctor,String>{with_db(&db,|c|directory::create_doctor(c,input))}
#[cfg_attr(mobile,tauri::mobile_entry_point)]pub fn run(){tauri::Builder::default().setup(|app|{let path=app.path().app_data_dir()?.join("clinicdesk.sqlite3");let db=init_db(&path).map_err(std::io::Error::other)?;app.manage(Db(Mutex::new(db)));Ok(())}).invoke_handler(tauri::generate_handler![health,patient_list,patient_create,patient_update,patient_delete,clinic_list,clinic_create,doctor_list,doctor_create]).run(tauri::generate_context!()).expect("error while running ClinicDesk");}
