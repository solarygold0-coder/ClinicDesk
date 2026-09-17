// Runtime command bridges for visit tracking.
use crate::{authorization, visit_tracking, Db};

#[tauri::command]
pub fn visit_tracking_get(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
) -> Result<visit_tracking::VisitTrackingInput, String> {
    let guard =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    authorization::authorize(&guard, actor_token.as_deref(), authorization::VISIT_READ)?;
    visit_tracking::get(&guard, id)
}

#[tauri::command]
pub fn visit_tracking_update_cmd(
    db: tauri::State<Db>,
    actor_token: Option<String>,
    id: i64,
    input: visit_tracking::VisitTrackingInput,
) -> Result<(), String> {
    let guard =
        db.0.lock()
            .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    authorization::authorize(&guard, actor_token.as_deref(), authorization::VISIT_WRITE)?;
    visit_tracking::update(&guard, id, input)
}
