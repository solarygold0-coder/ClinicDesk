use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};
use uuid::Uuid;

#[derive(Serialize)]
struct SecurityEvent<'a> {
    event_id: String,
    occurred_at: String,
    event_type: &'a str,
    status: &'a str,
    reference: Option<&'a str>,
    details: Option<&'a str>,
    previous_hash: String,
    event_hash: String,
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn canonical_hash(value: &serde_json::Value) -> Result<String, String> {
    let canonical = serde_json::json!({
        "event_id": value.get("event_id").and_then(|v| v.as_str()).ok_or_else(|| "سجل الأمان الخارجي يفتقد معرف الحدث".to_string())?,
        "occurred_at": value.get("occurred_at").and_then(|v| v.as_str()).ok_or_else(|| "سجل الأمان الخارجي يفتقد وقت الحدث".to_string())?,
        "event_type": value.get("event_type").and_then(|v| v.as_str()).ok_or_else(|| "سجل الأمان الخارجي يفتقد نوع الحدث".to_string())?,
        "status": value.get("status").and_then(|v| v.as_str()).ok_or_else(|| "سجل الأمان الخارجي يفتقد حالة الحدث".to_string())?,
        "reference": value.get("reference").cloned().unwrap_or(serde_json::Value::Null),
        "details": value.get("details").cloned().unwrap_or(serde_json::Value::Null),
        "previous_hash": value.get("previous_hash").and_then(|v| v.as_str()).ok_or_else(|| "سجل الأمان الخارجي يفتقد بصمة الحدث السابق".to_string())?,
    });
    Ok(hash_bytes(serde_json::to_string(&canonical).map_err(|e| e.to_string())?.as_bytes()))
}

pub fn verify(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Ok("GENESIS".to_string());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut expected_previous = "GENESIS".to_string();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|_| "سجل الأمان الخارجي تالف ولا يمكن التحقق من سلسلة البصمات".to_string())?;
        let previous = value.get("previous_hash").and_then(|v| v.as_str())
            .ok_or_else(|| "سجل الأمان الخارجي لا يحتوي بصمة سابقة صالحة".to_string())?;
        if previous != expected_previous {
            return Err("تم اكتشاف انقطاع أو تعديل في سلسلة سجل الأمان الخارجي".to_string());
        }
        let stored = value.get("event_hash").and_then(|v| v.as_str())
            .ok_or_else(|| "سجل الأمان الخارجي لا يحتوي بصمة حدث صالحة".to_string())?;
        let calculated = canonical_hash(&value)?;
        if stored != calculated {
            return Err("تم اكتشاف تعديل في محتوى سجل الأمان الخارجي".to_string());
        }
        expected_previous = stored.to_string();
    }
    Ok(expected_previous)
}

pub fn append(path: &Path, event_type: &str, status: &str, reference: Option<&str>, details: Option<&str>) -> Result<String, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let previous_hash = verify(path)?;
    let event_id = Uuid::new_v4().to_string();
    let occurred_at = chrono::Utc::now().to_rfc3339();
    let canonical = serde_json::json!({
        "event_id": event_id,
        "occurred_at": occurred_at,
        "event_type": event_type,
        "status": status,
        "reference": reference,
        "details": details,
        "previous_hash": previous_hash,
    });
    let event_hash = hash_bytes(serde_json::to_string(&canonical).map_err(|e| e.to_string())?.as_bytes());
    let event = SecurityEvent {
        event_id: canonical["event_id"].as_str().unwrap().to_string(),
        occurred_at: canonical["occurred_at"].as_str().unwrap().to_string(),
        event_type,
        status,
        reference,
        details,
        previous_hash: canonical["previous_hash"].as_str().unwrap().to_string(),
        event_hash,
    };
    let line = serde_json::to_string(&event).map_err(|e| e.to_string())?;
    let mut file = fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(event.event_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_form_a_hash_chain_and_detect_content_tampering() {
        let path = std::env::temp_dir().join(format!("clinicdesk-security-{}.jsonl", Uuid::new_v4()));
        append(&path, "restore", "started", Some("RST-1"), None).unwrap();
        append(&path, "restore", "success", Some("RST-1"), Some("ok")).unwrap();
        assert_ne!(verify(&path).unwrap(), "GENESIS");

        let original = fs::read_to_string(&path).unwrap();
        let tampered = original.replace("\"status\":\"success\"", "\"status\":\"failure\"");
        assert_ne!(original, tampered);
        fs::write(&path, tampered).unwrap();
        assert!(verify(&path).is_err());
        assert!(append(&path, "backup", "success", None, None).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn detects_deleted_or_reordered_chain_entries() {
        let path = std::env::temp_dir().join(format!("clinicdesk-security-{}.jsonl", Uuid::new_v4()));
        append(&path, "restore", "started", Some("RST-2"), None).unwrap();
        append(&path, "restore", "success", Some("RST-2"), None).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        fs::write(&path, format!("{}\n", lines[1])).unwrap();
        assert!(verify(&path).is_err());
        let _ = fs::remove_file(path);
    }
}
