CREATE TABLE IF NOT EXISTS scheduling_settings(
 id INTEGER PRIMARY KEY CHECK(id=1),
 work_start TEXT NOT NULL DEFAULT '08:00',
 work_end TEXT NOT NULL DEFAULT '17:00',
 break_start TEXT DEFAULT '12:00',
 break_end TEXT DEFAULT '12:55',
 slot_minutes INTEGER NOT NULL DEFAULT 30 CHECK(slot_minutes IN(10,15,20,30,60)),
 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT OR IGNORE INTO scheduling_settings(id) VALUES(1);
CREATE TABLE IF NOT EXISTS closure_dates(
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 closure_date TEXT NOT NULL UNIQUE,
 reason TEXT,
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_closure_date ON closure_dates(closure_date);
INSERT INTO app_meta(key,value) VALUES('schema_version','3') ON CONFLICT(key) DO UPDATE SET value='3';
