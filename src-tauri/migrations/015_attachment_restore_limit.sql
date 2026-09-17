CREATE TRIGGER IF NOT EXISTS trg_attachments_active_limit_restore
BEFORE UPDATE OF deleted_at ON attachments
WHEN OLD.deleted_at IS NOT NULL
 AND NEW.deleted_at IS NULL
 AND (SELECT COUNT(*) FROM attachments WHERE patient_id = NEW.patient_id AND deleted_at IS NULL) >= 20
BEGIN
  SELECT RAISE(ABORT, 'attachment_limit_20');
END;

UPDATE app_meta SET value = '15' WHERE key = 'schema_version';
