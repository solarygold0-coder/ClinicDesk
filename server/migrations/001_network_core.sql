CREATE TABLE patients (
    id BIGSERIAL PRIMARY KEY,
    file_no BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL,
    national_id VARCHAR(10) UNIQUE NOT NULL CHECK (national_id ~ '^[0-9]{10}$'),
    full_name TEXT NOT NULL CHECK (length(btrim(full_name)) >= 2),
    phone TEXT,
    medical_summary TEXT,
    version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE appointments (
    id BIGSERIAL PRIMARY KEY,
    patient_id BIGINT NOT NULL REFERENCES patients(id),
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'scheduled' CHECK (status IN ('scheduled','arrived','in_progress','completed','cancelled','no_show')),
    version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (ends_at > starts_at)
);

CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    actor_id TEXT NOT NULL,
    workstation_id TEXT NOT NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id BIGINT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    details JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX idx_patients_active_file_no ON patients(file_no) WHERE deleted_at IS NULL;
CREATE INDEX idx_appointments_starts_at ON appointments(starts_at);
CREATE INDEX idx_audit_entity ON audit_log(entity_type, entity_id, occurred_at DESC);
