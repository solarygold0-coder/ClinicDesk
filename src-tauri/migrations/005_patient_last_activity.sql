ALTER TABLE patients ADD COLUMN last_activity_at TEXT;

UPDATE patients
SET last_activity_at = COALESCE(
    (
        SELECT MAX(a.starts_at)
        FROM appointments a
        WHERE a.patient_id = patients.id
          AND a.status IN ('scheduled','arrived','in_progress','completed','no_show')
    ),
    updated_at,
    created_at
)
WHERE last_activity_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_patients_last_activity_active
ON patients(last_activity_at)
WHERE deleted_at IS NULL;

UPDATE app_meta SET value='5' WHERE key='schema_version';
