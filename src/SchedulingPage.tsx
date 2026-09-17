import { useEffect, useState } from 'react';
import { CalendarOff, Clock3, DatabaseBackup, FolderUp, Plus, Save, Trash2 } from 'lucide-react';
import { open, save as saveDialog } from '@tauri-apps/plugin-dialog';
import { api, ClosureDate, SchedulingSettings } from './api';
import { fullGregorianDate } from './dateDisplay';

const initial: SchedulingSettings = {
  workStart: '08:00',
  workEnd: '17:00',
  breakStart: '12:00',
  breakEnd: '12:55',
  slotMinutes: 30,
};

export function SchedulingPage() {
  const [form, setForm] = useState(initial);
  const [closures, setClosures] = useState<ClosureDate[]>([]);
  const [date, setDate] = useState('');
  const [reason, setReason] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [ok, setOk] = useState('');

  const load = async () => {
    const [s, c] = await Promise.all([api.schedulingSettings(), api.closures()]);
    setForm(s);
    setClosures(c);
  };
  useEffect(() => {
    void load().catch((e) => setError(String(e)));
  }, []);

  const save = async () => {
    if (busy) return;
    setBusy(true);
    setError('');
    setOk('');
    try {
      setForm(await api.updateSchedulingSettings(form));
      setOk('تم حفظ إعدادات المواعيد');
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const add = async () => {
    if (busy || !date) return;
    setBusy(true);
    setError('');
    setOk('');
    try {
      await api.createClosure({ closureDate: date, reason: reason.trim() || undefined });
      setDate('');
      setReason('');
      await load();
      setOk('تمت إضافة يوم الإغلاق');
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const remove = async (id: number) => {
    if (busy || !window.confirm('حذف يوم الإغلاق المحدد؟')) return;
    setBusy(true);
    setError('');
    setOk('');
    try {
      await api.deleteClosure(id);
      await load();
      setOk('تم حذف يوم الإغلاق');
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const backup = async () => {
    if (busy) return;
    const path = await saveDialog({
      title: 'حفظ نسخة احتياطية',
      defaultPath: `ClinicDesk-backup-${new Date().toISOString().slice(0, 10)}.sqlite3`,
      filters: [{ name: 'نسخة ClinicDesk', extensions: ['sqlite3'] }],
    });
    if (!path) return;
    setBusy(true);
    setError('');
    setOk('');
    try {
      const hash = await api.createBackup(path);
      setOk(`تم إنشاء النسخة الاحتياطية والتحقق منها • ${hash.slice(0, 12)}…`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const restore = async () => {
    if (busy) return;
    const path = await open({
      title: 'اختيار نسخة احتياطية',
      multiple: false,
      directory: false,
      filters: [{ name: 'نسخة ClinicDesk', extensions: ['sqlite3'] }],
    });
    if (!path) return;
    if (!window.confirm('سيتم استبدال قاعدة البيانات الحالية بعد فحص النسخة، مع إنشاء نقطة تراجع تلقائية. هل تريد المتابعة؟')) return;
    setBusy(true);
    setError('');
    setOk('');
    try {
      const hash = await api.restoreBackup(path);
      setOk(`تمت الاستعادة والتحقق بنجاح • ${hash.slice(0, 12)}…`);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="page schedulingPage" dir="rtl" aria-busy={busy}>
      <div className="pageTitle"><div><h1>الإعدادات</h1><p>إعدادات المواعيد والنسخ الاحتياطي المحلي.</p></div></div>
      {error && <div className="notice errorText" role="alert">{error}</div>}
      {ok && <div className="notice successText" role="status" aria-live="polite">{ok}</div>}
      <div className="settingsGrid">
        <div className="card">
          <div className="cardHeading"><Clock3 aria-hidden="true" /><div><h2>ساعات العمل</h2><p>تُطبّق هذه القواعد على إنشاء وتعديل المواعيد.</p></div></div>
          <div className="formGrid">
            <label>بداية الدوام<input disabled={busy} type="time" value={form.workStart} onChange={(e) => setForm({ ...form, workStart: e.target.value })} /></label>
            <label>نهاية الدوام<input disabled={busy} type="time" value={form.workEnd} onChange={(e) => setForm({ ...form, workEnd: e.target.value })} /></label>
            <label>بداية الاستراحة<input disabled={busy} type="time" value={form.breakStart ?? ''} onChange={(e) => setForm({ ...form, breakStart: e.target.value || undefined })} /></label>
            <label>نهاية الاستراحة<input disabled={busy} type="time" value={form.breakEnd ?? ''} onChange={(e) => setForm({ ...form, breakEnd: e.target.value || undefined })} /></label>
            <label>مدة الموعد<select disabled={busy} value={form.slotMinutes} onChange={(e) => setForm({ ...form, slotMinutes: Number(e.target.value) })}>{[10, 15, 20, 30, 60].map((x) => <option key={x} value={x}>{x} دقيقة</option>)}</select></label>
          </div>
          <button type="button" className="primary" disabled={busy} onClick={save}><Save aria-hidden="true" />{busy ? 'جارٍ التنفيذ…' : 'حفظ الإعدادات'}</button>
        </div>
        <div className="card">
          <div className="cardHeading"><CalendarOff aria-hidden="true" /><div><h2>أيام الإغلاق</h2><p>لن يسمح النظام بحجز موعد في هذه الأيام.</p></div></div>
          <div className="closureAdd">
            <label>التاريخ<input disabled={busy} type="date" min="1950-01-01" max="2050-12-31" value={date} onChange={(e) => setDate(e.target.value)} /></label>
            <label>السبب<input disabled={busy} value={reason} onChange={(e) => setReason(e.target.value)} placeholder="مثال: إجازة رسمية" /></label>
            <button type="button" className="primary" disabled={busy || !date} onClick={add}><Plus aria-hidden="true" />إضافة</button>
          </div>
          {closures.length === 0 ? <div className="emptyState"><CalendarOff aria-hidden="true" /><b>لا توجد أيام إغلاق مسجلة</b><span>أضف يومًا فقط عند الحاجة.</span></div> : <div className="closureList">{closures.map((c) => <div className="closureRow" key={c.id}><div><b>{fullGregorianDate(c.closureDate)}</b><span>{c.reason || 'بدون سبب'}</span></div><button type="button" className="iconButton danger" disabled={busy} onClick={() => void remove(c.id)} title="حذف" aria-label={`حذف يوم الإغلاق ${c.closureDate}`}><Trash2 aria-hidden="true" /></button></div>)}</div>}
        </div>
        <div className="card">
          <div className="cardHeading"><DatabaseBackup aria-hidden="true" /><div><h2>النسخ الاحتياطي والاستعادة</h2><p>ينشئ النظام نسخة SQLite متسقة ويتحقق من سلامتها وإصدارها قبل قبولها.</p></div></div>
          <div className="actions"><button type="button" className="primary" disabled={busy} onClick={backup}><DatabaseBackup aria-hidden="true" />إنشاء نسخة احتياطية</button><button type="button" disabled={busy} onClick={restore}><FolderUp aria-hidden="true" />استعادة نسخة</button></div>
          <p className="muted">الاستعادة تفحص النسخة أولًا وتنشئ نقطة تراجع تلقائية قبل استبدال قاعدة البيانات الحالية.</p>
        </div>
      </div>
    </section>
  );
}
