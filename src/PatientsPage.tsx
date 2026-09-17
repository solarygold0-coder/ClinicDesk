import { FormEvent, useEffect, useRef, useState } from 'react';
import {
  Archive,
  CalendarDays,
  Clock3,
  FileText,
  IdCard,
  Pencil,
  Phone,
  Printer,
  Search,
  UserPlus,
  X,
} from 'lucide-react';
import { api, Appointment, Patient, PatientInput } from './api';
import { PatientAttachments } from './PatientAttachments';
import { fullGregorianDate, fullGregorianDateTime } from './dateDisplay';

const blank: PatientInput = {
  fullName: '',
  nationalId: '',
  phone: '',
  birthDate: '',
  sex: '',
  medicalSummary: '',
  chronicDiseases: '',
  allergies: '',
  notes: '',
};
const status: Record<string, string> = {
  scheduled: 'مجدول',
  arrived: 'وصل',
  in_progress: 'قيد الخدمة',
  completed: 'مكتمل',
  cancelled: 'ملغي',
  no_show: 'لم يحضر',
};
type PatientAction = { kind: 'add' | 'search'; token: number } | null;

export function PatientsPage({
  initialFileNo,
  onInitialHandled,
  action,
}: {
  initialFileNo?: number | null;
  onInitialHandled?: () => void;
  action?: PatientAction;
}) {
  const [rows, setRows] = useState<Patient[]>([]);
  const [q, setQ] = useState('');
  const [open, setOpen] = useState(false);
  const [details, setDetails] = useState<Patient | null>(null);
  const [history, setHistory] = useState<Appointment[]>([]);
  const [historyBusy, setHistoryBusy] = useState(false);
  const [editing, setEditing] = useState<Patient | null>(null);
  const [form, setForm] = useState<PatientInput>(blank);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  async function load(query = q) {
    try {
      setRows(await api.patients(query));
      setError('');
    } catch (e) {
      setError(String(e));
    }
  }
  async function show(p: Patient) {
    setDetails(p);
    setHistory([]);
    setHistoryBusy(true);
    try {
      setHistory(await api.patientAppointments(p.id, 30));
    } catch (e) {
      setError(String(e));
    } finally {
      setHistoryBusy(false);
    }
  }
  useEffect(() => {
    if (initialFileNo != null) {
      const query = String(initialFileNo);
      setQ(query);
      void api
        .patientByFileNo(initialFileNo)
        .then((p) => {
          if (p) {
            setRows([p]);
            setError('');
            void show(p);
          } else {
            setRows([]);
            setError('ملف المريض غير موجود');
          }
        })
        .catch((e) => setError(String(e)))
        .finally(() => onInitialHandled?.());
      return;
    }
    void load('');
  }, [initialFileNo]);
  useEffect(() => {
    if (!action) return;
    if (action.kind === 'add') {
      add();
      return;
    }
    setDetails(null);
    setOpen(false);
    requestAnimationFrame(() => {
      searchRef.current?.focus();
      searchRef.current?.select();
    });
  }, [action?.token]);
  useEffect(() => {
    if (!open && !details) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || busy) return;
      event.preventDefault();
      if (open) {
        setOpen(false);
        setEditing(null);
      } else {
        setDetails(null);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open, details, busy]);

  function add() {
    setEditing(null);
    setDetails(null);
    setForm(blank);
    setError('');
    setOpen(true);
  }
  function edit(p: Patient) {
    setEditing(p);
    setForm({
      fullName: p.fullName,
      nationalId: p.nationalId || '',
      phone: p.phone || '',
      birthDate: p.birthDate || '',
      sex: p.sex || '',
      medicalSummary: p.medicalSummary || '',
      chronicDiseases: p.chronicDiseases || '',
      allergies: p.allergies || '',
      notes: p.notes || '',
    });
    setError('');
    setDetails(null);
    setOpen(true);
  }
  async function submit(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError('');
    try {
      if (editing) await api.updatePatient(editing.id, form);
      else await api.createPatient(form);
      setOpen(false);
      setEditing(null);
      setForm(blank);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function archivePatient(p: Patient) {
    if (busy) return;
    try {
      const future = await api.patientFutureAppointmentCount(p.id);
      if (future > 0) {
        setError(
          `لا يمكن أرشفة ملف ${p.fullName}: لديه ${future} موعد/مواعيد مستقبلية نشطة. ألغِ المواعيد أولاً.`,
        );
        return;
      }
      if (
        !window.confirm(
          `أرشفة ملف المريض ${p.fullName}؟ سيبقى السجل ورقم الملف محفوظين ولن يعاد استخدام الرقم.`,
        )
      )
        return;
      setBusy(true);
      setError('');
      await api.deletePatient(p.id);
      setDetails(null);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="page patientsPage" dir="rtl" aria-busy={busy}>
      <div className="pageTitle">
        <div><h1>المرضى</h1><p>إدارة ملفات المرضى والبحث السريع</p></div>
        <button type="button" className="primary" disabled={busy} onClick={add}><UserPlus aria-hidden="true" />إضافة مريض</button>
      </div>
      <div className="searchbar">
        <Search aria-hidden="true" />
        <input
          ref={searchRef}
          aria-label="بحث المرضى"
          aria-keyshortcuts="F2"
          value={q}
          onChange={(e) => { setQ(e.target.value); void load(e.target.value); }}
          placeholder="ابحث بالاسم، رقم الملف، الهوية أو الجوال"
        />
        <span aria-live="polite">{q ? `${rows.length} نتيجة` : `${rows.length} ملف`}</span>
      </div>
      {error && !open && <div className="error" role="alert">{error}</div>}
      <div className="tableCard">
        {rows.length === 0 ? (
          <div className="empty">
            <UserPlus aria-hidden="true" />
            <h3>{q ? 'لا توجد نتائج مطابقة' : 'لا توجد ملفات مرضى'}</h3>
            <p>{q ? 'جرّب الاسم أو رقم الملف أو الهوية أو الجوال.' : 'أضف أول مريض للبدء.'}</p>
            {!q && <button type="button" className="primary" disabled={busy} onClick={add}>إضافة مريض</button>}
          </div>
        ) : (
          <table>
            <thead><tr><th>رقم الملف</th><th>اسم المريض</th><th>رقم الهوية</th><th>الجوال</th><th>إجراءات</th></tr></thead>
            <tbody>
              {rows.map((p) => (
                <tr key={p.id} className="clickableRow" onDoubleClick={() => void show(p)}>
                  <td><bdi>{p.fileNo}</bdi></td>
                  <td><button type="button" className="patientNameLink" onClick={() => void show(p)}>{p.fullName}</button></td>
                  <td><bdi>{p.nationalId || '—'}</bdi></td>
                  <td><bdi>{p.phone || '—'}</bdi></td>
                  <td><div className="rowActions">
                    <button type="button" disabled={busy} aria-label="فتح الملف" title="فتح الملف" onClick={() => void show(p)}><FileText aria-hidden="true" /></button>
                    <button type="button" disabled={busy} aria-label="تعديل" title="تعديل" onClick={() => edit(p)}><Pencil aria-hidden="true" /></button>
                    <button type="button" disabled={busy} className="dangerIcon" aria-label={`أرشفة ملف ${p.fullName}`} title="أرشفة الملف" onClick={() => void archivePatient(p)}><Archive aria-hidden="true" /></button>
                  </div></td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {details && (
        <div className="modalBackdrop" onMouseDown={(e) => e.target === e.currentTarget && !busy && setDetails(null)}>
          <div className="modal patientRecord print-scope" dir="rtl" role="dialog" aria-modal="true" aria-labelledby="patient-record-title">
            <div className="printOnly printHeader"><h1>ClinicDesk — ملف مريض</h1><p>رقم الملف: <bdi>{details.fileNo}</bdi> • {details.fullName}</p></div>
            <div className="modalHead">
              <div><h2 id="patient-record-title">{details.fullName}</h2><p>ملف المريض رقم <bdi>{details.fileNo}</bdi></p></div>
              <button type="button" className="icon noPrint" disabled={busy} aria-label="إغلاق ملف المريض" onClick={() => setDetails(null)}><X aria-hidden="true" /></button>
            </div>
            <div className="recordGrid">
              <article><IdCard aria-hidden="true" /><span><small>رقم الهوية</small><strong dir="ltr">{details.nationalId || 'غير مسجل'}</strong></span></article>
              <article><Phone aria-hidden="true" /><span><small>رقم الجوال</small><strong dir="ltr">{details.phone || 'غير مسجل'}</strong></span></article>
              <article><CalendarDays aria-hidden="true" /><span><small>تاريخ الميلاد</small><strong>{fullGregorianDate(details.birthDate)}</strong></span></article>
              <article><UserPlus aria-hidden="true" /><span><small>الجنس</small><strong>{details.sex || 'غير محدد'}</strong></span></article>
            </div>
            <section className="recordSummary"><h3>المعلومات الطبية المختصرة</h3><p>{details.medicalSummary || 'لا توجد معلومات طبية مختصرة مسجلة.'}</p></section>
            <section className="recordSummary"><h3>الأمراض المزمنة</h3><p>{details.chronicDiseases || 'لا توجد أمراض مزمنة مسجلة.'}</p></section>
            <section className="recordSummary"><h3>الحساسية</h3><p>{details.allergies || 'لا توجد حساسية مسجلة.'}</p></section>
            <section className="recordSummary"><h3>ملاحظات</h3><p>{details.notes || 'لا توجد ملاحظات مسجلة.'}</p></section>
            <PatientAttachments patientId={details.id} />
            <section className="recordSummary">
              <h3><CalendarDays aria-hidden="true" /> سجل المواعيد</h3>
              {historyBusy ? <p>جارٍ تحميل المواعيد…</p> : history.length === 0 ? <p>لا توجد مواعيد مسجلة لهذا المريض.</p> : (
                <div className="patientHistory">{history.map((a) => (
                  <article key={a.id}><Clock3 aria-hidden="true" /><span><strong>{fullGregorianDateTime(a.startsAt)}</strong><small>{a.clinicName || 'بدون عيادة'}{a.doctorName ? ` • ${a.doctorName}` : ''}</small></span><em className={`status status-${a.status}`}>{status[a.status] || a.status}</em></article>
                ))}</div>
              )}
            </section>
            <div className="modalActions noPrint">
              <button type="button" disabled={busy} onClick={() => window.print()}><Printer aria-hidden="true" />طباعة الملف</button>
              <button type="button" className="danger" disabled={busy} onClick={() => void archivePatient(details)}><Archive aria-hidden="true" />أرشفة الملف</button>
              <button type="button" className="primary" disabled={busy} onClick={() => edit(details)}><Pencil aria-hidden="true" />تعديل البيانات</button>
            </div>
          </div>
        </div>
      )}

      {open && (
        <div className="modalBackdrop" onMouseDown={(e) => e.target === e.currentTarget && !busy && setOpen(false)}>
          <form className="modal" dir="rtl" onSubmit={submit} role="dialog" aria-modal="true" aria-labelledby="patient-form-title" aria-busy={busy}>
            <div className="modalHead">
              <div><h2 id="patient-form-title">{editing ? 'تعديل بيانات المريض' : 'إضافة مريض'}</h2><p>{editing ? `رقم الملف ${editing.fileNo} ثابت ولا يتغير.` : 'سيُنشأ رقم الملف تلقائيًا بالتسلسل.'}</p></div>
              <button type="button" className="icon" disabled={busy} aria-label="إغلاق نموذج المريض" onClick={() => setOpen(false)}><X aria-hidden="true" /></button>
            </div>
            <div className="formGrid">
              <label className="full">اسم المريض<span>*</span><input autoFocus value={form.fullName} onChange={(e) => setForm({ ...form, fullName: e.target.value })} required /></label>
              <label>رقم الهوية<input inputMode="numeric" dir="ltr" value={form.nationalId} onChange={(e) => setForm({ ...form, nationalId: e.target.value.replace(/[^0-9٠-٩]/g, '') })} maxLength={10} /></label>
              <label>رقم الجوال<input inputMode="tel" dir="ltr" value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} /></label>
              <label>تاريخ الميلاد<input type="date" min="1950-01-01" max="2050-12-31" value={form.birthDate} onChange={(e) => setForm({ ...form, birthDate: e.target.value })} /></label>
              <label>الجنس<select value={form.sex} onChange={(e) => setForm({ ...form, sex: e.target.value })}><option value="">غير محدد</option><option>ذكر</option><option>أنثى</option></select></label>
              <label className="full">معلومات طبية مختصرة<textarea value={form.medicalSummary} onChange={(e) => setForm({ ...form, medicalSummary: e.target.value })} /></label>
              <label className="full">الأمراض المزمنة<textarea value={form.chronicDiseases} onChange={(e) => setForm({ ...form, chronicDiseases: e.target.value })} placeholder="مثال: السكري، ضغط الدم" /></label>
              <label className="full">الحساسية<textarea value={form.allergies} onChange={(e) => setForm({ ...form, allergies: e.target.value })} placeholder="مثال: حساسية البنسلين" /></label>
              <label className="full">ملاحظات<textarea value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} /></label>
            </div>
            {error && <div className="error modalError" role="alert">{error}</div>}
            <div className="modalActions"><button type="button" disabled={busy} onClick={() => setOpen(false)}>إلغاء</button><button className="primary" disabled={busy}>{busy ? 'جارٍ الحفظ…' : editing ? 'حفظ التعديلات' : 'حفظ المريض'}</button></div>
          </form>
        </div>
      )}
    </section>
  );
}
