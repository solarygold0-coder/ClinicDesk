-- Accountability identity hardening.
-- Employee codes are permanent public identifiers, never credentials and never reused.
ALTER TABLE users ADD COLUMN employee_code TEXT;

-- Backfill any users that may already exist before enforcing uniqueness for future writes.
UPDATE users
SET employee_code = 'U' || printf('%06d', id)
WHERE employee_code IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ux_users_employee_code
ON users(employee_code)
WHERE employee_code IS NOT NULL;

CREATE TABLE IF NOT EXISTS identity_sequences(
  name TEXT PRIMARY KEY,
  next_value INTEGER NOT NULL CHECK(next_value >= 1)
);
INSERT OR IGNORE INTO identity_sequences(name,next_value)
VALUES('employee_code',1);
UPDATE identity_sequences
SET next_value = MAX(
  next_value,
  COALESCE((SELECT MAX(id) + 1 FROM users),1)
)
WHERE name='employee_code';

-- Preserve the actor snapshot even if the employee account is later renamed or disabled.
ALTER TABLE audit_log ADD COLUMN actor_employee_code TEXT;
ALTER TABLE audit_log ADD COLUMN actor_session_id TEXT;
ALTER TABLE audit_log ADD COLUMN before_json TEXT;
ALTER TABLE audit_log ADD COLUMN after_json TEXT;
ALTER TABLE audit_log ADD COLUMN reason TEXT;

CREATE INDEX IF NOT EXISTS ix_audit_log_employee_code
ON audit_log(actor_employee_code,created_at);
CREATE INDEX IF NOT EXISTS ix_audit_log_session
ON audit_log(actor_session_id,created_at);

INSERT OR REPLACE INTO app_meta(key,value)
VALUES('accountability_identity','employee-code-v1');
UPDATE app_meta SET value='11' WHERE key='schema_version';
