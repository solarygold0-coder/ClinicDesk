ALTER TABLE appointments ADD COLUMN visit_type TEXT NOT NULL DEFAULT 'new' CHECK(visit_type IN('new','follow_up','renewal'));
ALTER TABLE appointments ADD COLUMN visit_stage TEXT NOT NULL DEFAULT 'scheduled' CHECK(visit_stage IN('scheduled','reception','with_doctor','completed'));
ALTER TABLE appointments ADD COLUMN follow_up_at TEXT;

CREATE INDEX IF NOT EXISTS ix_appointments_visit_type ON appointments(visit_type, starts_at);
CREATE INDEX IF NOT EXISTS ix_appointments_follow_up ON appointments(follow_up_at) WHERE follow_up_at IS NOT NULL;

UPDATE app_meta SET value='7' WHERE key='schema_version';
