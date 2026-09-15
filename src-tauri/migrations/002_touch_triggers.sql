CREATE TRIGGER IF NOT EXISTS patients_touch AFTER UPDATE ON patients FOR EACH ROW WHEN NEW.updated_at=OLD.updated_at BEGIN UPDATE patients SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id; END;
CREATE TRIGGER IF NOT EXISTS appointments_touch AFTER UPDATE ON appointments FOR EACH ROW WHEN NEW.updated_at=OLD.updated_at BEGIN UPDATE appointments SET updated_at=CURRENT_TIMESTAMP WHERE id=NEW.id; END;
