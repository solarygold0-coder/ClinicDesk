-- Fixed user roles, permanent short employee codes and account lifecycle.
-- Additive migration: preserves every historical user identity and audit record.

DROP TRIGGER IF EXISTS trg_users_employee_code_immutable;

-- The legacy schema had no trustworthy fixed-role mapping. Refuse an ambiguous
-- conversion instead of silently placing more than the approved 20 identities
-- into the ordinary-employee role.
CREATE TEMP TRIGGER clinicdesk_v13_legacy_role_limit
BEFORE UPDATE OF employee_code ON users
WHEN (SELECT COUNT(*) FROM users) > 20
BEGIN
  SELECT RAISE(ABORT, 'legacy_user_role_assignment_required_over_20');
END;

UPDATE users
SET employee_code = 'U' || printf('%02d', (
  SELECT COUNT(*) FROM users AS prior WHERE prior.id <= users.id
));

DROP TRIGGER clinicdesk_v13_legacy_role_limit;

-- Keep historical audit actor codes aligned with the permanent identity after
-- the one-time U000001 -> U01 normalization. actor_user_id is authoritative.
UPDATE audit_log
SET actor_employee_code = (
  SELECT users.employee_code FROM users WHERE users.id = audit_log.actor_user_id
)
WHERE actor_user_id IS NOT NULL
  AND EXISTS(SELECT 1 FROM users WHERE users.id = audit_log.actor_user_id);

UPDATE identity_sequences
SET next_value = COALESCE((SELECT COUNT(*) + 1 FROM users), 1)
WHERE name='employee_code';

CREATE TRIGGER IF NOT EXISTS trg_users_employee_code_immutable
BEFORE UPDATE OF employee_code ON users
WHEN NEW.employee_code IS NOT OLD.employee_code
BEGIN
  SELECT RAISE(ABORT, 'employee_code_immutable');
END;

ALTER TABLE users ADD COLUMN role_type TEXT NOT NULL DEFAULT 'ordinary_employee'
  CHECK(role_type IN ('ordinary_employee','general_manager','deputy_manager','doctor','specialist'));
ALTER TABLE users ADD COLUMN account_status TEXT NOT NULL DEFAULT 'ACTIVE'
  CHECK(account_status IN ('ACTIVE','SUSPENDED','CLOSED_INACTIVITY','ENDED_SERVICE','RETIRED'));
ALTER TABLE users ADD COLUMN last_successful_login_at TEXT;
ALTER TABLE users ADD COLUMN closed_at TEXT;
ALTER TABLE users ADD COLUMN closed_reason TEXT;
ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0
  CHECK(must_change_password IN(0,1));

CREATE INDEX IF NOT EXISTS ix_users_role_status
ON users(role_type,account_status);
CREATE INDEX IF NOT EXISTS ix_users_last_login
ON users(last_successful_login_at);

CREATE TRIGGER IF NOT EXISTS trg_users_role_limit_insert
BEFORE INSERT ON users
BEGIN
  SELECT CASE
    WHEN NEW.role_type='ordinary_employee' AND (SELECT COUNT(*) FROM users WHERE role_type='ordinary_employee') >= 20 THEN RAISE(ABORT,'role_limit_ordinary_employee_20')
    WHEN NEW.role_type='general_manager' AND (SELECT COUNT(*) FROM users WHERE role_type='general_manager') >= 1 THEN RAISE(ABORT,'role_limit_general_manager_1')
    WHEN NEW.role_type='deputy_manager' AND (SELECT COUNT(*) FROM users WHERE role_type='deputy_manager') >= 1 THEN RAISE(ABORT,'role_limit_deputy_manager_1')
    WHEN NEW.role_type='doctor' AND (SELECT COUNT(*) FROM users WHERE role_type='doctor') >= 20 THEN RAISE(ABORT,'role_limit_doctor_20')
    WHEN NEW.role_type='specialist' AND (SELECT COUNT(*) FROM users WHERE role_type='specialist') >= 20 THEN RAISE(ABORT,'role_limit_specialist_20')
  END;
END;

CREATE TRIGGER IF NOT EXISTS trg_users_role_limit_update
BEFORE UPDATE OF role_type ON users
WHEN NEW.role_type <> OLD.role_type
BEGIN
  SELECT CASE
    WHEN NEW.role_type='ordinary_employee' AND (SELECT COUNT(*) FROM users WHERE role_type='ordinary_employee' AND id<>OLD.id) >= 20 THEN RAISE(ABORT,'role_limit_ordinary_employee_20')
    WHEN NEW.role_type='general_manager' AND (SELECT COUNT(*) FROM users WHERE role_type='general_manager' AND id<>OLD.id) >= 1 THEN RAISE(ABORT,'role_limit_general_manager_1')
    WHEN NEW.role_type='deputy_manager' AND (SELECT COUNT(*) FROM users WHERE role_type='deputy_manager' AND id<>OLD.id) >= 1 THEN RAISE(ABORT,'role_limit_deputy_manager_1')
    WHEN NEW.role_type='doctor' AND (SELECT COUNT(*) FROM users WHERE role_type='doctor' AND id<>OLD.id) >= 20 THEN RAISE(ABORT,'role_limit_doctor_20')
    WHEN NEW.role_type='specialist' AND (SELECT COUNT(*) FROM users WHERE role_type='specialist' AND id<>OLD.id) >= 20 THEN RAISE(ABORT,'role_limit_specialist_20')
  END;
END;

INSERT OR REPLACE INTO app_meta(key,value)
VALUES('user_role_policy','fixed-role-ceilings-v1');
INSERT OR REPLACE INTO app_meta(key,value)
VALUES('account_lifecycle_policy','status-90day-inactivity-v1');
UPDATE app_meta SET value='13' WHERE key='schema_version';
