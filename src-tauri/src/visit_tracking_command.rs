// Runtime command bridge for reading existing visit tracking without resetting it.
use crate::{visit_tracking, Db};

#[tauri::command]
pub fn visit_tracking_get(
    db: tauri::State<Db>,
    id: i64,
) -> Result<visit_tracking::VisitTrackingInput, String> {
    let guard = db
        .0
        .lock()
        .map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    visit_tracking::get(&guard, id)
}
