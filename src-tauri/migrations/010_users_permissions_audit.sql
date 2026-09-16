PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS users(
  id INTEGER PRIMARY KEY,
  username TEXT NOT NULL UNIQUE COLLATE NOCASE,
  display_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  job_title TEXT,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN(0,1)),
  is_system_admin INTEGER NOT NULL DEFAULT 0 CHECK(is_system_admin IN(0,1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS permissions(
  code TEXT PRIMARY KEY,
  label_ar TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_permissions(
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  permission_code TEXT NOT NULL REFERENCES permissions(code) ON DELETE CASCADE,
  allowed INTEGER NOT NULL DEFAULT 1 CHECK(allowed IN(0,1)),
  PRIMARY KEY(user_id, permission_code)
);

CREATE TABLE IF NOT EXISTS employee_lifecycle(
  id INTEGER PRIMARY KEY,
  actor_user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id INTEGER,
  entity_label TEXT,
  before_json TEXT,
  after_json TEXT,
  reason TEXT,
  occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS ix_employee_lifecycle_actor_time ON employee_lifecycle(actor_user_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS ix_employee_lifecycle_entity ON employee_lifecycle(entity_type, entity_id, occurred_at DESC);

INSERT OR IGNORE INTO permissions(code,label_ar) VALUES
('patients.view','عرض المرضى'),('patients.create','إضافة المرضى'),('patients.edit','تعديل المرضى'),('patients.delete','حذف المرضى'),
('attachments.view','عرض المرفقات'),('attachments.manage','إدارة المرفقات'),
('appointments.view','عرض المواعيد'),('appointments.create','حجز المواعيد'),('appointments.edit','تعديل المواعيد'),('appointments.cancel','إلغاء المواعيد'),('appointments.override_schedule','تجاوز قيود الدوام'),
('directory.manage','إدارة العيادات والأطباء'),('reports.print','الطباعة والتقارير'),
('backup.manage','النسخ الاحتياطي والاستعادة'),('audit.view','عرض سجل دورة حياة الموظف'),('users.manage','إدارة المستخدمين والصلاحيات');

UPDATE app_meta SET value='10' WHERE key='schema_version';
