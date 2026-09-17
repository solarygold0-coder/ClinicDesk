use crate::{audit, auth, authorization, Db};

fn with_db<T>(
    db: &tauri::State<Db>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let guard =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    f(&guard)
}

fn authorize_user_management(
    conn: &rusqlite::Connection,
    actor_token: Option<&str>,
) -> Result<Option<auth::UserSummary>, String> {
    authorization::authorize(conn, actor_token, authorization::USER_MANAGE)
}

fn audit_actor<'a>(user: &'a auth::UserSummary, token: Option<&'a str>) -> audit::AuditActor<'a> {
    audit::AuditActor {
        user_id: Some(user.id),
        display_name: Some(&user.display_name),
        employee_code: Some(&user.employee_code),
        session_id: token,
    }
}

fn user_by_id(conn: &rusqlite::Connection, id: i64) -> Result<Option<auth::UserSummary>, String> {
    Ok(auth::list_users(conn)?
        .into_iter()
        .find(|user| user.id == id))
}

#[tauri::command]
pub fn user_list(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<Vec<auth::UserSummary>, String> {
    with_db(&db, |conn| {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if count == 0 {
            return Ok(Vec::new());
        }
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
    let mut guard =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    let count: i64 = guard
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    if count == 0 {
        if role_type.trim() != "general_manager" {
            return Err("أول حساب في النظام يجب أن يكون المدير العام".into());
        }
        let user = auth::create_user(&mut guard, username, display_name, password, role_type)?;
        guard
            .execute(
                "UPDATE security_settings SET auth_enabled=1,updated_at=CURRENT_TIMESTAMP WHERE id=1",
                [],
            )
            .map_err(|e| e.to_string())?;
        let after = serde_json::to_string(&user).map_err(|e| e.to_string())?;
        audit::record_as(
            &guard,
            "system_bootstrap",
            "user",
            Some(user.id),
            None,
            &audit::AuditActor {
                user_id: Some(user.id),
                display_name: Some(&user.display_name),
                employee_code: Some(&user.employee_code),
                session_id: None,
            },
            &audit::AuditChange {
                before_json: None,
                after_json: Some(&after),
                reason: Some("first_run_setup"),
            },
        )?;
        return Ok(user);
    }

    let actor = authorize_user_management(&guard, actor_token.as_deref())?
        .ok_or_else(|| "تسجيل الدخول مطلوب لإدارة المستخدمين".to_string())?;
    let user = auth::create_user(&mut guard, username, display_name, password, role_type)?;
    let after = serde_json::to_string(&user).map_err(|e| e.to_string())?;
    audit::record_as(
        &guard,
        "user_created",
        "user",
        Some(user.id),
        None,
        &audit_actor(&actor, actor_token.as_deref()),
        &audit::AuditChange {
            before_json: None,
            after_json: Some(&after),
            reason: None,
        },
    )?;
    Ok(user)
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
        let actor = authorize_user_management(conn, actor_token.as_deref())?
            .ok_or_else(|| "تسجيل الدخول مطلوب لإدارة المستخدمين".to_string())?;
        let before = user_by_id(conn, id)?.ok_or_else(|| "المستخدم غير موجود".to_string())?;
        let updated = auth::update_user(conn, id, username, display_name, role_type)?;
        let before_json = serde_json::to_string(&before).map_err(|e| e.to_string())?;
        let after_json = serde_json::to_string(&updated).map_err(|e| e.to_string())?;
        audit::record_as(
            conn,
            "user_updated",
            "user",
            Some(id),
            None,
            &audit_actor(&actor, actor_token.as_deref()),
            &audit::AuditChange {
                before_json: Some(&before_json),
                after_json: Some(&after_json),
                reason: None,
            },
        )?;
        Ok(updated)
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
        let actor = authorize_user_management(conn, actor_token.as_deref())?
            .ok_or_else(|| "تسجيل الدخول مطلوب لإدارة المستخدمين".to_string())?;
        if actor.id == id && status.trim() != "ACTIVE" {
            return Err("لا يمكن للمستخدم تعطيل حسابه الحالي".into());
        }
        let before = user_by_id(conn, id)?.ok_or_else(|| "المستخدم غير موجود".to_string())?;
        auth::set_status(conn, id, status.clone(), reason.clone())?;
        let after =
            user_by_id(conn, id)?.ok_or_else(|| "المستخدم غير موجود بعد التحديث".to_string())?;
        let before_json = serde_json::to_string(&before).map_err(|e| e.to_string())?;
        let after_json = serde_json::to_string(&after).map_err(|e| e.to_string())?;
        let details = serde_json::json!({"status":status}).to_string();
        audit::record_as(
            conn,
            "user_status_changed",
            "user",
            Some(id),
            Some(&details),
            &audit_actor(&actor, actor_token.as_deref()),
            &audit::AuditChange {
                before_json: Some(&before_json),
                after_json: Some(&after_json),
                reason: reason.as_deref(),
            },
        )
    })
}

#[tauri::command]
pub fn user_reset_password(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    temporary_password: String,
) -> Result<(), String> {
    if temporary_password.chars().count() != 4 {
        return Err("كلمة المرور المؤقتة يجب أن تتكون من 4 خانات بالضبط".into());
    }
    with_db(&db, |conn| {
        let actor = authorize_user_management(conn, actor_token.as_deref())?
            .ok_or_else(|| "تسجيل الدخول مطلوب لإدارة المستخدمين".to_string())?;
        auth::reset_password(conn, id, temporary_password)?;
        audit::record_as(
            conn,
            "user_password_reset",
            "user",
            Some(id),
            None,
            &audit_actor(&actor, actor_token.as_deref()),
            &audit::AuditChange {
                before_json: None,
                after_json: None,
                reason: Some("temporary_password_issued"),
            },
        )
    })
}

fn change_password(
    conn: &rusqlite::Connection,
    actor_token: String,
    current_password: String,
    new_password: String,
) -> Result<auth::UserSummary, String> {
    if new_password.chars().count() != 4 {
        return Err("كلمة المرور الجديدة يجب أن تتكون من 4 خانات بالضبط".into());
    }
    if current_password == new_password {
        return Err("كلمة المرور الجديدة يجب أن تختلف عن الحالية".into());
    }
    let actor = auth::validate_session(conn, actor_token.clone())?;
    let verification = auth::authenticate(conn, actor.username.clone(), current_password)?;
    let hash = auth::hash_password(&new_password)?;
    conn.execute(
        "UPDATE users SET password_hash=?2,must_change_password=0,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
        rusqlite::params![actor.id, hash],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE auth_sessions SET revoked_at=COALESCE(revoked_at,CURRENT_TIMESTAMP) WHERE user_id=?1 AND id<>?2 AND revoked_at IS NULL",
        rusqlite::params![actor.id, actor_token],
    )
    .map_err(|e| e.to_string())?;
    let _ = auth::logout(conn, verification.token);
    let updated = auth::validate_session(conn, actor_token.clone())?;
    audit::record_as(
        conn,
        "user_password_changed",
        "user",
        Some(actor.id),
        None,
        &audit_actor(&updated, Some(&actor_token)),
        &audit::AuditChange {
            before_json: None,
            after_json: None,
            reason: Some("self_service_password_change"),
        },
    )?;
    Ok(updated)
}

#[tauri::command]
pub fn auth_change_password(
    db: tauri::State<Db>,
    actor_token: String,
    current_password: String,
    new_password: String,
) -> Result<auth::UserSummary, String> {
    with_db(&db, |conn| {
        change_password(conn, actor_token, current_password, new_password)
    })
}

#[tauri::command]
pub fn deputy_restore_permission_get(
    db: tauri::State<Db>,
    actor_token: Option<String>,
) -> Result<bool, String> {
    with_db(&db, |conn| {
        authorization::deputy_restore_granted(conn, actor_token.as_deref())
    })
}

#[tauri::command]
pub fn deputy_restore_permission_set(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    enabled: bool,
) -> Result<(), String> {
    with_db(&db, |conn| {
        let actor =
            authorization::authorize(conn, actor_token.as_deref(), authorization::SECURITY_MANAGE)?
                .ok_or_else(|| "تسجيل الدخول مطلوب لتعديل صلاحيات الأمان".to_string())?;
        authorization::set_deputy_restore_grant(conn, actor_token.as_deref(), enabled)?;
        let details = serde_json::json!({"enabled":enabled}).to_string();
        audit::record_as(
            conn,
            "deputy_restore_permission_changed",
            "security",
            None,
            Some(&details),
            &audit_actor(&actor, actor_token.as_deref()),
            &audit::AuditChange::default(),
        )
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
    current_password: Option<String>,
    new_password: Option<String>,
) -> Result<auth::UserSummary, String> {
    match (current_password, new_password) {
        (None, None) => with_db(&db, |conn| auth::validate_session(conn, token)),
        (Some(current), Some(new_value)) => {
            with_db(&db, |conn| change_password(conn, token, current, new_value))
        }
        _ => Err("يجب إرسال كلمة المرور الحالية والجديدة معًا".into()),
    }
}

#[tauri::command]
pub fn auth_logout(db: tauri::State<Db>, token: String) -> Result<(), String> {
    with_db(&db, |conn| auth::logout(conn, token))
}
