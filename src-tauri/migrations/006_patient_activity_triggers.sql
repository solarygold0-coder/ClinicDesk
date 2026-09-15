UPDATE patients
SET last_activity_at = MAX(
    COALESCE(updated_at, created_at),
    COALESCE(
        (
            SELECT MAX(a.starts_at)
            FROM appointments a
            WHERE a.patient_id = patients.id
              AND a.starts_at <= CURRENT_TIMESTAMP
              AND a.status IN ('arrived','in_progress','completed','no_show')
        ),
        created_at
    )
)
WHERE deleted_at IS NULL;

CREATE TRIGGER IF NOT EXISTS trg_appointments_patient_activity_insert
AFTER INSERT ON appointments
BEGIN
    UPDATE patients
    SET last_activity_at = CURRENT_TIMESTAMP
    WHERE id = NEW.patient_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_appointments_patient_activity_update
AFTER UPDATE ON appointments
BEGIN
    UPDATE patients
    SET last_activity_at = CURRENT_TIMESTAMP
    WHERE id = NEW.patient_id;

    UPDATE patients
    SET last_activity_at = CURRENT_TIMESTAMP
    WHERE id = OLD.patient_id AND OLD.patient_id <> NEW.patient_id;
END;

UPDATE app_meta SET value='6' WHERE key='schema_version';
