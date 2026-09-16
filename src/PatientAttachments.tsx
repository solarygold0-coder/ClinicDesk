import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { FilePlus2, Paperclip, Trash2 } from 'lucide-react';
import { api, Attachment } from './api';

function sizeLabel(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function PatientAttachments({ patientId }: { patientId: number }) {
  const [items, setItems] = useState<Attachment[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  async function load() {
    try {
      setItems(await api.attachments(patientId));
      setError('');
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    void load();
  }, [patientId]);

  async function add() {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'المرفقات', extensions: ['pdf', 'png', 'jpg', 'jpeg', 'webp', 'doc', 'docx'] }],
    });
    if (!selected) return;
    setBusy(true);
    setError('');
    try {
      await api.importAttachment(patientId, selected);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function remove(item: Attachment) {
    if (!confirm(`حذف المرفق ${item.originalName}؟`)) return;
    setBusy(true);
    setError('');
    try {
      await api.removeAttachment(item.id);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="recordSummary">
      <div className="attachmentHeader">
        <h3><Paperclip /> المرفقات</h3>
        <button type="button" className="primary" disabled={busy} onClick={() => void add()}>
          <FilePlus2 /> {busy ? 'جارٍ التنفيذ…' : 'إضافة مرفق'}
        </button>
      </div>
      {error && <div className="error">{error}</div>}
      {items.length === 0 ? <p>لا توجد مرفقات لهذا المريض.</p> : (
        <div className="attachmentList">
          {items.map(item => (
            <article key={item.id} className="attachmentItem">
              <Paperclip />
              <span>
                <strong>{item.originalName}</strong>
                <small>{sizeLabel(item.sizeBytes)} • {new Date(item.createdAt).toLocaleString('ar-SA')}</small>
              </span>
              <button type="button" className="dangerIcon" disabled={busy} aria-label={`حذف ${item.originalName}`} onClick={() => void remove(item)}>
                <Trash2 />
              </button>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
