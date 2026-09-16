-- Permanent employee identity hardening.
-- Employee codes identify accounts in audit/printing and are never credentials.
-- Existing codes from migration 011 are preserved. Future codes must be assigned by application code.

CREATE TRIGGER IF NOT EXISTS trg_users_employee_code_required_insert
BEFORE INSERT ON users
WHEN NEW.employee_code IS NULL OR trim(NEW.employee_code) = ''
BEGIN
  SELECT RAISE(ABORT, 'employee_code_required');
END;

CREATE TRIGGER IF NOT EXISTS trg_users_employee_code_immutable
BEFORE UPDATE OF employee_code ON users
WHEN NEW.employee_code IS NOT OLD.employee_code
BEGIN
  SELECT RAISE(ABORT, 'employee_code_immutable');
END;

-- Account deletion would destroy the canonical user-to-code mapping and weaken accountability.
-- Disable accounts with is_active=0 instead; audit history remains intact.
CREATE TRIGGER IF NOT EXISTS trg_users_preserve_identity
BEFORE DELETE ON users
BEGIN
  SELECT RAISE(ABORT, 'user_identity_must_be_preserved');
END;

CREATE INDEX IF NOT EXISTS ix_users_active_employee_code
ON users(is_active, employee_code);

INSERT OR REPLACE INTO app_meta(key,value)
VALUES('employee_identity_policy','immutable-preserved-v1');
UPDATE app_meta SET value='12' WHERE key='schema_version';
