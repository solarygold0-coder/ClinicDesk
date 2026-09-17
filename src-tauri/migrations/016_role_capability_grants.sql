-- Seed canonical role rows so exceptional capabilities can be granted explicitly.
-- Default role behavior remains implemented in Rust; this table stores explicit additions only.
INSERT OR IGNORE INTO roles(name,description) VALUES
  ('ordinary_employee','Operational employee'),
  ('general_manager','General manager'),
  ('deputy_manager','Deputy manager'),
  ('doctor','Doctor read-only appointment access'),
  ('specialist','Specialist read-only appointment access');

INSERT OR REPLACE INTO app_meta(key,value) VALUES('auth_capability_grants','role-capabilities-v1');
UPDATE app_meta SET value='16' WHERE key='schema_version';
