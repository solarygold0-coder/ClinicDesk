ALTER TABLE patients ADD COLUMN chronic_diseases TEXT;
ALTER TABLE patients ADD COLUMN allergies TEXT;
ALTER TABLE patients ADD COLUMN notes TEXT;

UPDATE app_meta SET value='4' WHERE key='schema_version';
