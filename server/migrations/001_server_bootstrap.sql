CREATE SEQUENCE IF NOT EXISTS patient_file_no_seq START WITH 1;

CREATE TABLE IF NOT EXISTS server_meta(
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS users(
  id BIGSERIAL PRIMARY KEY,
  employee_code VARCHAR(3) NOT NULL UNIQUE CHECK(employee_code ~ '^U[0-9]{2}$'),
  username TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  role_type TEXT NOT NULL CHECK(role_type IN ('ordinary_employee','general_manager','deputy_manager','doctor','specialist')),
  account_status TEXT NOT NULL DEFAULT 'ACTIVE' CHECK(account_status IN ('ACTIVE','SUSPENDED','CLOSED_INACTIVITY','ENDED_SERVICE','RETIRED')),
  must_change_password BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS auth_sessions(
  id UUID PRIMARY KEY,
  user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL,
  revoked_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS patients(
  id BIGSERIAL PRIMARY KEY,
  file_no BIGINT NOT NULL UNIQUE DEFAULT nextval('patient_file_no_seq'),
  national_id VARCHAR(10) UNIQUE,
  full_name TEXT NOT NULL,
  phone TEXT,
  birth_date DATE,
  sex TEXT,
  medical_summary TEXT,
  chronic_diseases TEXT,
  allergies TEXT,
  notes TEXT,
  deleted_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS clinics(
  id BIGSERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  phone TEXT,
  address TEXT,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS doctors(
  id BIGSERIAL PRIMARY KEY,
  clinic_id BIGINT REFERENCES clinics(id),
  name TEXT NOT NULL,
  specialty TEXT,
  phone TEXT,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS appointments(
  id BIGSERIAL PRIMARY KEY,
  patient_id BIGINT NOT NULL REFERENCES patients(id),
  clinic_id BIGINT REFERENCES clinics(id),
  doctor_id BIGINT REFERENCES doctors(id),
  starts_at TIMESTAMPTZ NOT NULL,
  ends_at TIMESTAMPTZ NOT NULL,
  status TEXT NOT NULL DEFAULT 'scheduled' CHECK(status IN ('scheduled','arrived','in_progress','completed','cancelled','no_show')),
  notes TEXT,
  version BIGINT NOT NULL DEFAULT 1,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK(ends_at > starts_at)
);
CREATE INDEX IF NOT EXISTS ix_appointments_doctor_time ON appointments(doctor_id,starts_at,ends_at);
CREATE INDEX IF NOT EXISTS ix_appointments_patient ON appointments(patient_id,starts_at DESC);

CREATE TABLE IF NOT EXISTS attachments(
  id BIGSERIAL PRIMARY KEY,
  patient_id BIGINT NOT NULL REFERENCES patients(id),
  storage_key TEXT NOT NULL UNIQUE,
  original_name TEXT NOT NULL,
  display_name TEXT NOT NULL,
  category TEXT,
  mime_type TEXT,
  size_bytes BIGINT NOT NULL CHECK(size_bytes > 0),
  sha256 CHAR(64) NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  archived_at TIMESTAMPTZ,
  archived_reason TEXT
);

CREATE TABLE IF NOT EXISTS audit_log(
  id BIGSERIAL PRIMARY KEY,
  actor_user_id BIGINT REFERENCES users(id),
  actor_employee_code VARCHAR(3),
  session_id UUID,
  event_type TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id BIGINT,
  before_json JSONB,
  after_json JSONB,
  reason TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_audit_actor_time ON audit_log(actor_user_id,created_at DESC);

CREATE TABLE IF NOT EXISTS workstations(
  id UUID PRIMARY KEY,
  display_name TEXT NOT NULL,
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO server_meta(key,value) VALUES('schema_version','1')
ON CONFLICT(key) DO UPDATE SET value=EXCLUDED.value,updated_at=NOW();
