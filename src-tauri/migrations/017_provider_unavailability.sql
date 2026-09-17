-- Provider-unavailability is an operational exception, not a patient no-show or ordinary cancellation.
-- Keep the original appointment and record any transfer/reschedule/cancellation as a classified resolution.
ALTER TABLE appointments ADD COLUMN disruption_kind TEXT;
ALTER TABLE appointments ADD COLUMN disruption_reason_code TEXT;
ALTER TABLE appointments ADD COLUMN disruption_reason_note TEXT;
ALTER TABLE appointments ADD COLUMN disruption_resolution TEXT;
ALTER TABLE appointments ADD COLUMN provider_unavailability_event_id INTEGER REFERENCES provider_unavailability_events(id);
ALTER TABLE appointments ADD COLUMN original_appointment_id INTEGER REFERENCES appointments(id);
ALTER TABLE appointments ADD COLUMN replacement_appointment_id INTEGER REFERENCES appointments(id);

CREATE TABLE IF NOT EXISTS provider_unavailability_events(
  id INTEGER PRIMARY KEY,
  doctor_id INTEGER NOT NULL REFERENCES doctors(id),
  unavailable_from TEXT NOT NULL,
  unavailable_to TEXT NOT NULL,
  reason_code TEXT NOT NULL CHECK(reason_code IN('leave','sudden_absence','assignment_meeting','emergency','other')),
  reason_note TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  resolved_at TEXT,
  CHECK(unavailable_to > unavailable_from),
  CHECK(reason_code <> 'other' OR (reason_note IS NOT NULL AND length(trim(reason_note)) > 0))
);

CREATE INDEX IF NOT EXISTS ix_provider_unavailability_doctor_window
  ON provider_unavailability_events(doctor_id, unavailable_from, unavailable_to);

CREATE TABLE IF NOT EXISTS provider_unavailability_actions(
  id INTEGER PRIMARY KEY,
  event_id INTEGER NOT NULL REFERENCES provider_unavailability_events(id) ON DELETE CASCADE,
  appointment_id INTEGER NOT NULL REFERENCES appointments(id),
  action_type TEXT NOT NULL CHECK(action_type IN('transfer','reschedule','cancel')),
  original_doctor_id INTEGER REFERENCES doctors(id),
  replacement_doctor_id INTEGER REFERENCES doctors(id),
  original_starts_at TEXT NOT NULL,
  new_starts_at TEXT,
  replacement_appointment_id INTEGER REFERENCES appointments(id),
  reason_code TEXT NOT NULL,
  reason_note TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS ix_provider_unavailability_actions_event
  ON provider_unavailability_actions(event_id, appointment_id);

INSERT OR REPLACE INTO app_meta(key,value) VALUES('provider_unavailability_schema','v1');
UPDATE app_meta SET value='17' WHERE key='schema_version';
