-- ClinicDesk 5.1 password policy: every newly chosen password is exactly four characters.
-- Existing hashes remain valid for one login so users are not locked out; active accounts
-- are forced through the existing must_change_password flow to adopt the new policy.
UPDATE users
SET must_change_password=1,
    updated_at=CURRENT_TIMESTAMP
WHERE account_status='ACTIVE';

INSERT OR REPLACE INTO app_meta(key,value)
VALUES('password_policy','exact-4-v1');
UPDATE app_meta SET value='18' WHERE key='schema_version';
