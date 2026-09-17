ALTER TABLE attachments ADD COLUMN display_name TEXT;
ALTER TABLE attachments ADD COLUMN category TEXT;
ALTER TABLE attachments ADD COLUMN deleted_at TEXT;
ALTER TABLE attachments ADD COLUMN deleted_reason TEXT;

UPDATE attachments
SET display_name = original_name
WHERE display_name IS NULL OR trim(display_name) = '';

CREATE INDEX IF NOT EXISTS ix_attachments_patient_active
ON attachments(patient_id, created_at DESC)
WHERE deleted_at IS NULL;

CREATE TRIGGER IF NOT EXISTS trg_attachments_active_limit_insert
BEFORE INSERT ON attachments
WHEN (SELECT COUNT(*) FROM attachments WHERE patient_id=NEW.patient_id AND deleted_at IS NULL) >= 20
BEGIN
  SELECT RAISE(ABORT, 'attachment_limit_20');
END;

UPDATE app_meta SET value='14' WHERE key='schema_version';
