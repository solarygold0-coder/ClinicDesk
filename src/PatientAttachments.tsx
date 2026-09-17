import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { openPath } from '@tauri-apps/plugin-opener';
import { appDataDir, join } from '@tauri-apps/api/path';
import { Archive, ExternalLink, FilePlus2, Paperclip, Printer, RotateCcw } from 'lucide-react';
import { api, Attachment } from './api';

function sizeLabel(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
function extensionLabel(name: string) {
  const ext = name.split('.').pop()?.toUpperCase();
  return ext && ext !== name.toUpperCase() ? ext : 'ملف';
}
function fullGregorianDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const text = date.toLocaleString('ar-SA-u-ca-gregory', {
    weekday: 'long', year: 'numeric', month: 'long', day: 'numeric', hour: '2-digit', minute: '2-digit',
  });
  return `${text} • الشهر ${String(date.getMonth() + 1).padStart(2, '0')}`;
}

export function PatientAttachments({ patientId }: { patientId: number }) {
  const [items, setItems] = useState<Attachment[]>([]);
  const [archived, setArchived] = useState<Attachment[]>([]);
  const [showArchived, setShowArchived] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  async function load() {
    try {
      const [active, old] = await Promise.all([api.attachments(patientId), api.archivedAttachments(patientId)]);
      setItems(active);
      setArchived(old);
      setError('');
    } catch (e) {
      setError(String(e));
    }
  }
  useEffect(() => { void load(); }, [patientId]);

  async function add() {
    if (busy || items.length >= 20) return;
    const selected = await open({ multiple: false, directory: false });
    if (!selected) return;
    const defaultName = selected.split(/[\\/]/).pop() || 'مرفق';
    const displayName = window.prompt('اسم المرفق', defaultName)?.trim();
    if (!displayName) return;
    const category = window.prompt('تصنيف المرفق (اختياري)', '')?.trim() || undefined;
    setBusy(true);
    setError('');
    try {
      await api.importAttachment(patientId, selected, displayName, category);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function openAttachment(item: Attachment) {
    if (busy) return;
    setError('');
    try {
      const root = await appDataDir();
      await openPath(await join(root, 'attachments', item.storedName));
    } catch (e) {
      setError(
        `تعذر فتح المرفق. تأكد من وجود برنامج في Windows يدعم صيغة ${extensionLabel(item.originalName)}. ${String(e)}`,
      );
    }
  }
  async function archiveItem(item: Attachment) {
    if (busy) return;
    const reason = window.prompt(`سبب أرشفة ${item.displayName || item.originalName}`, '')?.trim();
    if (!reason) return;
    setBusy(true);
    setError('');
    try {
      await api.archiveAttachment(item.id, reason);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function restoreItem(item: Attachment) {
    if (busy || items.length >= 20) return;
    const reason = window.prompt(`سبب استعادة ${item.displayName || item.originalName}`, '')?.trim();
    if (!reason) return;
    setBusy(true);
    setError('');
    try {
      await api.restoreAttachment(item.id, reason);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const visible = showArchived ? archived : items;
  return (
    <section className="recordSummary" dir="rtl" aria-busy={busy} aria-label="مرفقات المريض">
      <div className="attachmentHeader">
        <h3><Paperclip aria-hidden="true" /> المرفقات <small>({items.length}/20)</small></h3>
        <div className="noPrint">
          <button type="button" disabled={busy} onClick={() => setShowArchived((v) => !v)} aria-pressed={showArchived}>{showArchived ? 'عرض النشطة' : `الأرشيف (${archived.length})`}</button>
          {!showArchived && <button type="button" className="primary" disabled={busy || items.length >= 20} onClick={() => void add()}><FilePlus2 aria-hidden="true" /> {busy ? 'جارٍ التنفيذ…' : 'إضافة مرفق'}</button>}
        </div>
      </div>
      <p className="attachmentHint noPrint">تُفتح المرفقات بواسطة البرنامج الافتراضي في Windows. يمكن فتح وطباعة PDF والصور وملفات Office والنصوص وغيرها إذا كان البرنامج المناسب مثبتًا؛ الملفات التنفيذية والخطرة محظورة أمنيًا.</p>
      {error && <div className="error" role="alert">{error}</div>}
      {items.length >= 20 && !showArchived && <p className="error" role="alert">وصل الملف إلى الحد الأقصى: 20 مرفقاً نشطاً. أرشف مرفقاً قبل إضافة آخر.</p>}
      {visible.length === 0 ? <p>{showArchived ? 'لا توجد مرفقات مؤرشفة.' : 'لا توجد مرفقات لهذا المريض.'}</p> : (
        <div className="attachmentList">
          {visible.map((item) => (
            <article key={item.id} className="attachmentItem">
              <Paperclip aria-hidden="true" />
              <button type="button" className="attachmentOpen" disabled={busy} onClick={() => void openAttachment(item)} aria-label={`فتح المرفق ${item.displayName || item.originalName}`} title={`فتح ${item.displayName || item.originalName}`}>
                <span><strong>{item.displayName || item.originalName}</strong><small><bdi>{extensionLabel(item.originalName)}</bdi> • {item.category ? `${item.category} • ` : ''}{sizeLabel(item.sizeBytes)} • {fullGregorianDateTime(item.createdAt)}{showArchived && item.deletedReason ? ` • سبب الأرشفة: ${item.deletedReason}` : ''}</small></span>
                <ExternalLink aria-hidden="true" />
              </button>
              <button type="button" className="noPrint" disabled={busy} aria-label={`فتح ${item.displayName || item.originalName} للطباعة`} title="فتح في البرنامج الافتراضي للطباعة" onClick={() => void openAttachment(item)}><Printer aria-hidden="true" /></button>
              {showArchived ? <button type="button" className="noPrint" disabled={busy || items.length >= 20} aria-label={`استعادة ${item.displayName || item.originalName}`} onClick={() => void restoreItem(item)}><RotateCcw aria-hidden="true" /></button> : <button type="button" className="dangerIcon noPrint" disabled={busy} aria-label={`أرشفة ${item.displayName || item.originalName}`} onClick={() => void archiveItem(item)}><Archive aria-hidden="true" /></button>}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
