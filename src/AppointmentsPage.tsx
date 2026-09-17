import { FormEvent, useEffect, useMemo, useState } from 'react';
import { CalendarPlus, Clock3, Pencil, Printer, UserRound, X } from 'lucide-react';
import { api, Appointment, AppointmentInput, Clinic, Doctor, Patient, VisitTrackingInput } from './api';

const pad = (n: number) => String(n).padStart(2, '0');
const day = (d = new Date()) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const blank = (durationMinutes = 30): AppointmentInput => ({
  patientFileNo: 0,
  startsAt: `${day()}T09:00`,
  durationMinutes,
  notes: '',
});
const blankVisit = (): VisitTrackingInput => ({ visitType: 'new', visitStage: 'scheduled' });
const labels: Record<string, string> = {
  scheduled: 'مجدول',
  arrived: 'وصل',
  in_progress: 'قيد الخدمة',
  completed: 'مكتمل',
  cancelled: 'ملغي',
  no_show: 'لم يحضر',
};
const minutes = (a: Appointment) => Math.round((new Date(a.endsAt).getTime() - new Date(a.startsAt).getTime()) / 60000);
const parts = (v: string) => {
  const m = v.match(/^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/);
  return m ? m.slice(1).map(Number) : [new Date().getFullYear(), 1, 1, 9, 0];
};
const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
const daysInMonth = (year: number, month: number) => new Date(year, month, 0).getDate();
type AppointmentAction = { kind: 'add'; token: number } | null;

export function AppointmentsPage({
  onPatient,
  action,
}: {
  onPatient: (fileNo: number) => void;
  action?: AppointmentAction;
}) {
  const [rows, setRows] = useState<Appointment[]>([]);
  const [clinics, setClinics] = useState<Clinic[]>([]);
  const [doctors, setDoctors] = useState<Doctor[]>([]);
  const [defaultDuration, setDefaultDuration] = useState(30);
  const [date, setDate] = useState(day());
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<number | null>(null);
  const [form, setForm] = useState<AppointmentInput>(blank());
  const [visit, setVisit] = useState<VisitTrackingInput>(blankVisit());
  const [visitReady, setVisitReady] = useState(true);
  const [patient, setPatient] = useState<Patient | null>(null);
  const [lookup, setLookup] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [statusBusyId, setStatusBusyId] = useState<number | null>(null);
  const filteredDoctors = useMemo(
    () => form.clinicId ? doctors.filter((d) => !d.clinicId || d.clinicId === form.clinicId) : doctors,
    [doctors, form.clinicId],
  );

  const setStartPart = (index: number, value: number) => {
    const p = parts(form.startsAt);
    if (Number.isFinite(value)) p[index] = value;
    const y = clamp(p[0], 1950, 2050);
    const m = clamp(p[1], 1, 12);
    const d = clamp(p[2], 1, daysInMonth(y, m));
    const h = clamp(p[3], 0, 23);
    const mi = clamp(p[4], 0, 59);
    setForm({ ...form, startsAt: `${y}-${pad(m)}-${pad(d)}T${pad(h)}:${pad(mi)}` });
  };
  async function load(d = date) {
    try {
      const [from, to] = [`${d}T00:00:00`, `${d}T23:59:59`];
      const [a, c, dr, s] = await Promise.all([
        api.appointments(from, to),
        api.clinics(),
        api.doctors(),
        api.schedulingSettings(),
      ]);
      setRows(a);
      setClinics(c.filter((x) => x.isActive !== false));
      setDoctors(dr.filter((x) => x.isActive !== false));
      setDefaultDuration(s.slotMinutes);
      setError('');
    } catch (e) {
      setError(String(e));
    }
  }
  useEffect(() => { void load(date); }, [date]);
  useEffect(() => { if (action?.kind === 'add') add(); }, [action?.token]);
  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || busy) return;
      event.preventDefault();
      setOpen(false);
      setEditing(null);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open, busy]);

  async function findPatient(fileNo: number) {
    setPatient(null);
    setLookup('');
    if (!fileNo) return;
    try {
      const p = await api.patientByFileNo(fileNo);
      setPatient(p);
      if (!p) setLookup('لا يوجد مريض نشط بهذا الرقم');
    } catch (e) {
      setLookup(String(e));
    }
  }
  function add() {
    setEditing(null);
    setForm(blank(defaultDuration));
    setVisit(blankVisit());
    setVisitReady(true);
    setPatient(null);
    setLookup('');
    setError('');
    setOpen(true);
  }
  async function edit(a: Appointment) {
    if (busy || statusBusyId !== null) return;
    setEditing(a.id);
    setForm({
      patientFileNo: a.fileNo,
      clinicId: a.clinicId,
      doctorId: a.doctorId,
      startsAt: a.startsAt.slice(0, 16),
      durationMinutes: minutes(a),
      notes: a.notes || '',
    });
    setVisitReady(false);
    setPatient(null);
    setLookup('');
    setError('');
    setOpen(true);
    try {
      const [currentVisit] = await Promise.all([api.visitTracking(a.id), findPatient(a.fileNo)]);
      setVisit(currentVisit);
      setVisitReady(true);
    } catch (e) {
      setError(`تعذر تحميل بيانات الزيارة الحالية: ${String(e)}`);
    }
  }
  async function submit(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    if (editing !== null && !visitReady) {
      setError('لا يمكن حفظ التعديل قبل تحميل بيانات الزيارة الحالية بنجاح');
      return;
    }
    if (!patient || patient.fileNo !== form.patientFileNo) {
      setError('تحقق من رقم ملف المريض قبل حفظ الموعد');
      return;
    }
    setBusy(true);
    setError('');
    try {
      let id = editing;
      if (editing !== null) await api.updateAppointment(editing, form);
      else id = (await api.createAppointment(form)).id;
      if (id !== null) await api.updateVisitTracking(id, visit);
      setOpen(false);
      setEditing(null);
      setForm(blank(defaultDuration));
      setVisit(blankVisit());
      setVisitReady(true);
      setPatient(null);
      await load(date);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function updateStatus(id: number, nextStatus: string) {
    if (statusBusyId !== null) return;
    setStatusBusyId(id);
    setError('');
    try {
      await api.appointmentStatus(id, nextStatus);
      await load(date);
    } catch (e) {
      setError(String(e));
    } finally {
      setStatusBusyId(null);
    }
  }

  const [y, mo, da, hr, mn] = parts(form.startsAt);
  const maxDay = daysInMonth(clamp(y, 1950, 2050), clamp(mo, 1, 12));
  return (
    <section className="page appointmentsPage print-scope" dir="rtl" aria-busy={busy || statusBusyId !== null}>
      <div className="printOnly printHeader"><h1>ClinicDesk — مواعيد اليوم</h1><p>التاريخ الميلادي: <bdi>{date}</bdi> • عدد المواعيد: {rows.length}</p></div>
      <div className="pageTitle noPrint">
        <div><h1>المواعيد</h1><p>جدولة المراجعين وإدارة حركة اليوم</p></div>
        <div className="headActions"><button type="button" onClick={() => window.print()}><Printer aria-hidden="true" />طباعة اليوم</button><button type="button" className="primary" disabled={busy} onClick={add}><CalendarPlus aria-hidden="true" />موعد جديد</button></div>
      </div>
      <div className="appointmentToolbar noPrint">
        <label>يوم العمل<input type="date" min="1950-01-01" max="2050-12-31" value={date} onChange={(e) => setDate(e.target.value)} /></label>
        <div className="dayCount" aria-live="polite"><Clock3 aria-hidden="true" /><span><strong>{rows.length}</strong> موعد في هذا اليوم</span></div>
      </div>
      {error && !open && <div className="error noPrint" role="alert">{error}</div>}
      <div className="tableCard printTableCard">
        {rows.length === 0 ? (
          <div className="empty"><CalendarPlus aria-hidden="true" /><h3>لا توجد مواعيد في هذا اليوم</h3><p>لا توجد بيانات للطباعة لهذا التاريخ.</p></div>
        ) : (
          <table>
            <thead><tr><th>الوقت</th><th>رقم الملف</th><th>المريض</th><th>العيادة</th><th>الطبيب</th><th>الحالة</th><th className="noPrint">إجراءات</th></tr></thead>
            <tbody>{rows.map((a) => (
              <tr key={a.id}>
                <td dir="ltr">{new Date(a.startsAt).toLocaleTimeString('ar-SA-u-ca-gregory', { hour: 'numeric', minute: '2-digit' })}</td>
                <td><button type="button" className="linkButton noPrint" onClick={() => onPatient(a.fileNo)} aria-label={`فتح ملف المريض رقم ${a.fileNo}`}><bdi>{a.fileNo}</bdi></button><span className="printOnly"><bdi>{a.fileNo}</bdi></span></td>
                <td><button type="button" className="linkButton noPrint" onClick={() => onPatient(a.fileNo)}>{a.patientName}</button><span className="printOnly">{a.patientName}</span></td>
                <td>{a.clinicName || '—'}</td><td>{a.doctorName || '—'}</td>
                <td><span className={`status status-${a.status}`}>{labels[a.status] || a.status}</span></td>
                <td className="noPrint"><div className="rowActions">
                  <button type="button" className="icon" title="تعديل الموعد" aria-label={`تعديل موعد ${a.patientName}`} disabled={statusBusyId !== null || ['completed', 'cancelled', 'no_show'].includes(a.status)} onClick={() => void edit(a)}><Pencil aria-hidden="true" /></button>
                  <select aria-label={`تغيير حالة موعد ${a.patientName}`} disabled={statusBusyId !== null} value={a.status} onChange={(e) => void updateStatus(a.id, e.target.value)}><option value="scheduled">مجدول</option><option value="arrived">وصل</option><option value="in_progress">قيد الخدمة</option><option value="completed">مكتمل</option><option value="cancelled">ملغي</option><option value="no_show">لم يحضر</option></select>
                </div></td>
              </tr>
            ))}</tbody>
          </table>
        )}
      </div>

      {open && (
        <div className="modalBackdrop noPrint" onMouseDown={(e) => e.target === e.currentTarget && !busy && setOpen(false)}>
          <form className="modal" dir="rtl" onSubmit={submit} role="dialog" aria-modal="true" aria-labelledby="appointment-form-title" aria-busy={busy}>
            <div className="modalHead">
              <div><h2 id="appointment-form-title">{editing !== null ? 'تعديل الموعد' : 'إضافة موعد'}</h2><p>{editing !== null ? (visitReady ? 'عدّل بيانات الموعد وسيعاد فحص التعارضات تلقائيًا.' : 'جارٍ تحميل بيانات الزيارة الحالية…') : 'أدخل رقم الملف للتحقق من المريض قبل الحفظ.'}</p></div>
              <button type="button" className="icon" disabled={busy} aria-label="إغلاق نموذج الموعد" onClick={() => setOpen(false)}><X aria-hidden="true" /></button>
            </div>
            <div className="formGrid">
              <label>رقم ملف المريض<span>*</span><input autoFocus required min="1" type="number" inputMode="numeric" dir="ltr" value={form.patientFileNo || ''} onChange={(e) => { const n = Number(e.target.value); setForm({ ...form, patientFileNo: n }); setPatient(null); setLookup(''); }} onBlur={() => void findPatient(form.patientFileNo)} /></label>
              <div className="patientPreview" aria-live="polite">{patient ? <><UserRound aria-hidden="true" /><div><strong>{patient.fullName}</strong><small>ملف رقم <bdi>{patient.fileNo}</bdi>{patient.phone ? <> • <bdi>{patient.phone}</bdi></> : ''}</small></div></> : lookup ? <span className="errorText">{lookup}</span> : <span>أدخل رقم الملف ثم انتقل للحقل التالي للتحقق.</span>}</div>
              <label className="full">التاريخ والوقت<span>*</span><div className="dateTimeSpinners" dir="ltr" role="group" aria-label="تاريخ ووقت الموعد"><input aria-label="السنة" title="السنة" type="number" min="1950" max="2050" value={y} onChange={(e) => setStartPart(0, Number(e.target.value))} /><input aria-label="الشهر" title="الشهر" type="number" min="1" max="12" value={mo} onChange={(e) => setStartPart(1, Number(e.target.value))} /><input aria-label="اليوم" title="اليوم" type="number" min="1" max={maxDay} value={da} onChange={(e) => setStartPart(2, Number(e.target.value))} /><span>—</span><input aria-label="الساعة" title="الساعة" type="number" min="0" max="23" value={hr} onChange={(e) => setStartPart(3, Number(e.target.value))} /><span>:</span><input aria-label="الدقيقة" title="الدقيقة" type="number" min="0" max="59" step="5" value={mn} onChange={(e) => setStartPart(4, Number(e.target.value))} /></div><small>السنة / الشهر / اليوم — الساعة : الدقيقة. عدد أيام الشهر يُضبط تلقائيًا.</small></label>
              <label>العيادة<select value={form.clinicId || ''} onChange={(e) => setForm({ ...form, clinicId: e.target.value ? Number(e.target.value) : undefined, doctorId: undefined })}><option value="">بدون تحديد</option>{clinics.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select></label>
              <label>الطبيب<select value={form.doctorId || ''} onChange={(e) => setForm({ ...form, doctorId: e.target.value ? Number(e.target.value) : undefined })}><option value="">بدون تحديد</option>{filteredDoctors.map((d) => <option key={d.id} value={d.id}>{d.name}{d.specialty ? ` — ${d.specialty}` : ''}</option>)}</select></label>
              <label>مدة الموعد<select value={form.durationMinutes} onChange={(e) => setForm({ ...form, durationMinutes: Number(e.target.value) })}>{[10, 15, 20, 30, 60].map((n) => <option key={n} value={n}>{n} دقيقة</option>)}</select></label>
              <label>نوع الزيارة<select disabled={editing !== null && !visitReady} value={visit.visitType} onChange={(e) => setVisit({ ...visit, visitType: e.target.value as VisitTrackingInput['visitType'] })}><option value="new">زيارة جديدة</option><option value="follow_up">متابعة</option><option value="renewal">تجديد</option></select></label>
              <label>مرحلة الزيارة<select disabled={editing !== null && !visitReady} value={visit.visitStage} onChange={(e) => setVisit({ ...visit, visitStage: e.target.value as VisitTrackingInput['visitStage'] })}><option value="scheduled">مجدولة</option><option value="reception">الاستقبال</option><option value="with_doctor">لدى الطبيب</option><option value="completed">مكتملة</option></select></label>
              <label>موعد المتابعة القادم<input disabled={editing !== null && !visitReady} type="datetime-local" min="1950-01-01T00:00" max="2050-12-31T23:59" value={visit.followUpAt || ''} onChange={(e) => setVisit({ ...visit, followUpAt: e.target.value || undefined })} /></label>
              <label className="full">ملاحظات<textarea value={form.notes || ''} onChange={(e) => setForm({ ...form, notes: e.target.value })} /></label>
            </div>
            {error && <div className="error modalError" role="alert">{error}</div>}
            <div className="modalActions"><button type="button" disabled={busy} onClick={() => setOpen(false)}>إلغاء</button><button className="primary" disabled={busy || !patient || (editing !== null && !visitReady)}>{busy ? 'جارٍ الحفظ…' : editing !== null && !visitReady ? 'بانتظار بيانات الزيارة…' : editing !== null ? 'حفظ التعديلات' : 'حفظ الموعد'}</button></div>
          </form>
        </div>
      )}
    </section>
  );
}
