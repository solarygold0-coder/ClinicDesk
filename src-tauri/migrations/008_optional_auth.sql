CREATE TABLE IF NOT EXISTS security_settings(
  id INTEGER PRIMARY KEY CHECK(id=1),
  auth_enabled INTEGER NOT NULL DEFAULT 0 CHECK(auth_enabled IN(0,1)),
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT OR IGNORE INTO security_settings(id,auth_enabled) VALUES(1,0);

CREATE TABLE IF NOT EXISTS users(
  id INTEGER PRIMARY KEY,
  username TEXT NOT NULL COLLATE NOCASE UNIQUE,
  display_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN(0,1)),
  is_system_admin INTEGER NOT NULL DEFAULT 0 CHECK(is_system_admin IN(0,1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS roles(
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL COLLATE NOCASE UNIQUE,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS role_capabilities(
  role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  capability TEXT NOT NULL,
  PRIMARY KEY(role_id,capability)
);

CREATE TABLE IF NOT EXISTS user_roles(
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  PRIMARY KEY(user_id,role_id)
);

CREATE TABLE IF NOT EXISTS auth_sessions(
  id TEXT PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at TEXT NOT NULL,
  revoked_at TEXT
);
CREATE INDEX IF NOT EXISTS ix_auth_sessions_user ON auth_sessions(user_id,expires_at);

INSERT OR IGNORE INTO app_meta(key,value) VALUES('auth_architecture','optional-v1');
UPDATE app_meta SET value='8' WHERE key='schema_version';
