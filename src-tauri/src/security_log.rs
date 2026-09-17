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

fn last_hash(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Ok("GENESIS".to_string());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let Some(line) = content.lines().rev().find(|line| !line.trim().is_empty()) else {
        return Ok("GENESIS".to_string());
    };
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|_| "سجل الأمان الخارجي تالف ولا يمكن متابعة سلسلة البصمات".to_string())?;
    value
        .get("event_hash")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "سجل الأمان الخارجي لا يحتوي بصمة حدث صالحة".to_string())
}

pub fn append(
    path: &Path,
    event_type: &str,
    status: &str,
    reference: Option<&str>,
    details: Option<&str>,
) -> Result<String, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let previous_hash = last_hash(path)?;
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
    let event_hash = hash_bytes(
        serde_json::to_string(&canonical)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    );
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
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(event.event_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_form_a_hash_chain_and_refuse_corrupt_tail() {
        let path =
            std::env::temp_dir().join(format!("clinicdesk-security-{}.jsonl", Uuid::new_v4()));
        append(&path, "restore", "started", Some("RST-1"), None).unwrap();
        append(&path, "restore", "success", Some("RST-1"), None).unwrap();
        let lines: Vec<serde_json::Value> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["previous_hash"], "GENESIS");
        assert_eq!(lines[1]["previous_hash"], lines[0]["event_hash"]);
        fs::write(&path, "corrupt\n").unwrap();
        assert!(append(&path, "restore", "failure", Some("RST-1"), None).is_err());
        let _ = fs::remove_file(path);
    }
}
