use crate::{auth, authorization, Db};

fn with_db<T>(
    db: &tauri::State<Db>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let guard = db
        .0
        .lock()
        .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    f(&guard)
}

fn authorize_user_management(
    conn: &rusqlite::Connection,
    actor_token: Option<&str>,
) -> Result<(), String> {
    authorization::authorize(conn, actor_token, authorization::USER_MANAGE)?;
    Ok(())
}

#[tauri::command]
pub fn user_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<Vec<auth::UserSummary>, String> {
    with_db(&db, |conn| {
        authorize_user_management(conn, actor_token.as_deref())?;
        auth::list_users(conn)
    })
}

#[tauri::command]
pub fn user_create(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    username: String,
    display_name: String,
    password: String,
    role_type: String,
) -> Result<auth::UserSummary, String> {
    let mut guard = db
        .0
        .lock()
        .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    authorize_user_management(&guard, actor_token.as_deref())?;
    auth::create_user(&mut guard, username, display_name, password, role_type)
}

#[tauri::command]
pub fn user_update(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    username: String,
    display_name: String,
    role_type: String,
) -> Result<auth::UserSummary, String> {
    with_db(&db, |conn| {
        authorize_user_management(conn, actor_token.as_deref())?;
        auth::update_user(conn, id, username, display_name, role_type)
    })
}

#[tauri::command]
pub fn user_set_status(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    status: String,
    reason: Option<String>,
) -> Result<(), String> {
    with_db(&db, |conn| {
        authorize_user_management(conn, actor_token.as_deref())?;
        auth::set_status(conn, id, status, reason)
    })
}

#[tauri::command]
pub fn user_reset_password(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    temporary_password: String,
) -> Result<(), String> {
    with_db(&db, |conn| {
        authorize_user_management(conn, actor_token.as_deref())?;
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
