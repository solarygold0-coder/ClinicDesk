-- Authentication is mandatory once at least one employee account exists.
-- Fresh installations remain in bootstrap mode until the first general manager is created.
UPDATE security_settings
SET auth_enabled = CASE WHEN EXISTS(SELECT 1 FROM users) THEN 1 ELSE 0 END,
    updated_at = CURRENT_TIMESTAMP
WHERE id = 1;

INSERT OR REPLACE INTO app_meta(key,value) VALUES('accountability_mode','required-after-bootstrap');
UPDATE app_meta SET value='17' WHERE key='schema_version';
