use crate::{auth, Db};

fn with_db<T>(
    db: &tauri::State<Db>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let guard =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    f(&guard)
}

#[tauri::command]
pub fn user_list(db: tauri::State<Db>) -> Result<Vec<auth::UserSummary>, String> {
    with_db(&db, auth::list_users)
}

#[tauri::command]
pub fn user_create(
    db: tauri::State<Db>,
    username: String,
    display_name: String,
    password: String,
    role_type: String,
) -> Result<auth::UserSummary, String> {
    let mut guard =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    auth::create_user(&mut guard, username, display_name, password, role_type)
}

#[tauri::command]
pub fn user_update(
    db: tauri::State<Db>,
    id: i64,
    username: String,
    display_name: String,
    role_type: String,
) -> Result<auth::UserSummary, String> {
    with_db(&db, |conn| {
        auth::update_user(conn, id, username, display_name, role_type)
    })
}

#[tauri::command]
pub fn user_set_status(
    db: tauri::State<Db>,
    id: i64,
    status: String,
    reason: Option<String>,
) -> Result<(), String> {
    with_db(&db, |conn| auth::set_status(conn, id, status, reason))
}

#[tauri::command]
pub fn user_reset_password(
    db: tauri::State<Db>,
    id: i64,
    temporary_password: String,
) -> Result<(), String> {
    with_db(&db, |conn| {
        auth::reset_password(conn, id, temporary_password)
    })
}

#[tauri::command]
pub fn auth_login(
    db: tauri::State<Db>,
    username: String,
    password: String,
) -> Result<auth::AuthSession, String> {
    with_db(&db, |conn| auth::authenticate(conn, username, password))
}

#[tauri::command]
pub fn auth_validate_session(
    db: tauri::State<Db>,
    token: String,
) -> Result<auth::UserSummary, String> {
    with_db(&db, |conn| auth::validate_session(conn, token))
}

#[tauri::command]
pub fn auth_logout(db: tauri::State<Db>, token: String) -> Result<(), String> {
    with_db(&db, |conn| auth::logout(conn, token))
}
