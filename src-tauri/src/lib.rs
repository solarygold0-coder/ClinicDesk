pub mod domain;
use rusqlite::Connection;use std::{fs,path::PathBuf};use tauri::Manager;
fn init_db(path:&PathBuf)->Result<(),String>{if let Some(p)=path.parent(){fs::create_dir_all(p).map_err(|e|e.to_string())?;}let db=Connection::open(path).map_err(|e|e.to_string())?;db.execute_batch(include_str!("../migrations/001_init.sql")).map_err(|e|e.to_string())?;Ok(())}
#[tauri::command]fn health()->&'static str{"ok"}
#[cfg_attr(mobile,tauri::mobile_entry_point)]pub fn run(){tauri::Builder::default().setup(|app|{let path=app.path().app_data_dir()?.join("clinicdesk.sqlite3");init_db(&path).map_err(std::io::Error::other)?;Ok(())}).invoke_handler(tauri::generate_handler![health]).run(tauri::generate_context!()).expect("error while running ClinicDesk");}
