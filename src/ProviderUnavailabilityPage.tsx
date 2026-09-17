import { FormEvent, useEffect, useMemo, useState } from 'react';
import { AlertTriangle, ArrowRightLeft, CalendarClock, UserRoundCheck } from 'lucide-react';
import {
  api,
  AffectedAppointment,
  Doctor,
  ProviderResolutionAction,
  ProviderResolutionInput,
  ProviderUnavailabilityEvent,
  ProviderUnavailabilityReason,
} from './api';

type ResolutionDraft = {
  selected: boolean;
  actionType: ProviderResolutionAction;
  replacementDoctorId: string;
  newStartsAt: string;
};

const reasons: Array<{ value: ProviderUnavailabilityReason; label: string }> = [
  { value: 'leave', label: 'إجازة' },
  { value: 'sudden_absence', label: 'غياب مفاجئ' },
  { value: 'assignment_meeting', label: 'تكليف أو اجتماع' },
  { value: 'emergency', label: 'ظرف طارئ' },
  { value: 'other', label: 'سبب آخر' },
];

const actionLabels: Record<ProviderResolutionAction, string> = {
  transfer: 'تحويل لمعالج بديل',
  reschedule: 'إعادة جدولة',
  cancel: 'إلغاء بسبب تعذّر المعالج',
};

const localDateTime = (value: string) => value.slice(0, 16);
const displayDateTime = (value: string) =>
  new Date(value).toLocaleString('ar-SA-u-ca-gregory', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });

export function ProviderUnavailabilityPage() {
  const [doctors, setDoctors] = useState<Doctor[]>([]);
  const [doctorId, setDoctorId] = useState('');
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [reasonCode, setReasonCode] = useState<ProviderUnavailabilityReason>('sudden_absence');
  const [reasonNote, setReasonNote] = useState('');
  const [event, setEvent] = useState<ProviderUnavailabilityEvent | null>(null);
  const [affected, setAffected] = useState<AffectedAppointment[]>([]);
  const [drafts, setDrafts] = useState<Record<number, ResolutionDraft>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');

  useEffect(() => {
    api.doctors().then(setDoctors).catch((e) => setError(String(e)));
  }, []);

  const originalDoctor = useMemo(
    () => doctors.find((doctor) => doctor.id === Number(doctorId)),
    [doctors, doctorId],
  );

  const alternatives = useMemo(
    () => doctors.filter((doctor) => doctor.id !== event?.doctorId),
    [doctors, event?.doctorId],
  );

  function initializeDrafts(rows: AffectedAppointment[]) {
    setDrafts(
      Object.fromEntries(
        rows.map((row) => [
          row.appointmentId,
          {
            selected: true,
            actionType: 'transfer' as ProviderResolutionAction,
            replacementDoctorId: '',
            newStartsAt: localDateTime(row.startsAt),
          },
        ]),
      ),
    );
  }

  async function refreshAffected(eventId: number) {
    const rows = await api.providerUnavailabilityAffected(eventId);
    setAffected(rows);
    initializeDrafts(rows);
    if (rows.length === 0) setMessage('لا توجد مواعيد متبقية تحتاج معالجة ضمن فترة التعذّر.');
  }

  async function registerEvent(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError('');
    setMessage('');
    try {
      if (!doctorId) throw new Error('اختر المعالج المتعذّر');
      if (!from || !to) throw new Error('حدد بداية ونهاية فترة التعذّر');
      if (reasonCode === 'other' && !reasonNote.trim()) throw new Error('اكتب سبب التعذّر عند اختيار «سبب آخر»');
      const created = await api.createProviderUnavailability({
        doctorId: Number(doctorId),
        unavailableFrom: from,
        unavailableTo: to,
        reasonCode,
        reasonNote: reasonNote.trim() || undefined,
      });
      setEvent(created);
      await refreshAffected(created.id);
      setMessage('تم تسجيل تعذّر المعالج واستخراج المواعيد المتأثرة.');
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  function patchDraft(id: number, patch: Partial<ResolutionDraft>) {
    setDrafts((current) => ({
      ...current,
      [id]: { ...current[id], ...patch },
    }));
  }

  function toggleAll(selected: boolean) {
    setDrafts((current) =>
      Object.fromEntries(Object.entries(current).map(([id, draft]) => [id, { ...draft, selected }])),
    );
  }

  async function resolveSelected() {
    if (!event || busy) return;
    setBusy(true);
    setError('');
    setMessage('');
    try {
      const resolutions: ProviderResolutionInput[] = affected
        .filter((row) => drafts[row.appointmentId]?.selected)
        .map((row) => {
          const draft = drafts[row.appointmentId];
          if (draft.actionType === 'transfer' && !draft.replacementDoctorId) {
            throw new Error(`اختر المعالج البديل للملف رقم ${row.fileNo}`);
          }
          if (draft.actionType === 'reschedule' && !draft.newStartsAt) {
            throw new Error(`حدد الموعد الجديد للملف رقم ${row.fileNo}`);
          }
          return {
            appointmentId: row.appointmentId,
            actionType: draft.actionType,
            replacementDoctorId: draft.replacementDoctorId ? Number(draft.replacementDoctorId) : undefined,
            newStartsAt: draft.actionType === 'reschedule' ? draft.newStartsAt : undefined,
          };
        });
      if (resolutions.length === 0) throw new Error('اختر موعدًا واحدًا على الأقل للمعالجة');
      await api.resolveProviderUnavailability(event.id, resolutions);
      await refreshAffected(event.id);
      setMessage(`تمت معالجة ${resolutions.length} موعد/مواعيد دفعة واحدة مع حفظ دورة الحياة.`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  function resetEvent() {
    setEvent(null);
    setAffected([]);
    setDrafts({});
    setMessage('');
    setError('');
    setDoctorId('');
    setFrom('');
    setTo('');
    setReasonCode('sudden_absence');
    setReasonNote('');
  }

  return (
    <section className="page providerUnavailabilityPage" dir="rtl" aria-busy={busy}>
      <div className="pageTitle">
        <div>
          <h1><AlertTriangle aria-hidden="true" /> تعذّر المعالج</h1>
          <p>مسار مستقل لمعالجة المواعيد المتأثرة دون احتسابها كعدم حضور للمريض أو إلغاء عادي.</p>
        </div>
        {event && <button type="button" onClick={resetEvent} disabled={busy}>تسجيل حالة تعذّر جديدة</button>}
      </div>

      {error && <div className="notice errorText" role="alert">{error}</div>}
      {message && <div className="notice" role="status">{message}</div>}

      {!event ? (
        <form className="settingsGrid" onSubmit={registerEvent}>
          <label>
            المعالج المتعذّر<span>*</span>
            <select required value={doctorId} onChange={(e) => setDoctorId(e.target.value)}>
              <option value="">اختر المعالج</option>
              {doctors.map((doctor) => <option key={doctor.id} value={doctor.id}>{doctor.name}{doctor.specialty ? ` — ${doctor.specialty}` : ''}</option>)}
            </select>
          </label>
          <label>
            بداية التعذّر<span>*</span>
            <input required type="datetime-local" min="1950-01-01T00:00" max="2050-12-31T23:59" value={from} onChange={(e) => setFrom(e.target.value)} />
          </label>
          <label>
            نهاية التعذّر<span>*</span>
            <input required type="datetime-local" min="1950-01-01T00:00" max="2050-12-31T23:59" value={to} onChange={(e) => setTo(e.target.value)} />
          </label>
          <label>
            سبب التعذّر<span>*</span>
            <select required value={reasonCode} onChange={(e) => setReasonCode(e.target.value as ProviderUnavailabilityReason)}>
              {reasons.map((reason) => <option key={reason.value} value={reason.value}>{reason.label}</option>)}
            </select>
          </label>
          <label className="span2">
            ملاحظة السبب {reasonCode === 'other' && <span>*</span>}
            <input required={reasonCode === 'other'} maxLength={250} value={reasonNote} onChange={(e) => setReasonNote(e.target.value)} placeholder="تفاصيل مختصرة عند الحاجة" />
          </label>
          <div className="span2">
            <button className="primary" type="submit" disabled={busy}>{busy ? 'جارٍ التحقق…' : 'تسجيل التعذّر واستخراج المواعيد المتأثرة'}</button>
          </div>
          {originalDoctor && <div className="notice span2">سيتم فحص جميع مواعيد <strong>{originalDoctor.name}</strong> داخل الفترة المحددة.</div>}
        </form>
      ) : (
        <>
          <div className="notice">
            <strong>{doctors.find((doctor) => doctor.id === event.doctorId)?.name || `المعالج #${event.doctorId}`}</strong>
            {' — '}{reasons.find((reason) => reason.value === event.reasonCode)?.label || event.reasonCode}
            {' — '}من {displayDateTime(event.unavailableFrom)} إلى {displayDateTime(event.unavailableTo)}
          </div>

          {affected.length > 0 && <div className="tableCard">
            <div className="appointmentToolbar">
              <label><input type="checkbox" checked={affected.every((row) => drafts[row.appointmentId]?.selected)} onChange={(e) => toggleAll(e.target.checked)} /> تحديد الكل</label>
              <div className="dayCount"><CalendarClock aria-hidden="true" /><span><strong>{affected.length}</strong> موعد متأثر</span></div>
            </div>
            <table>
              <thead><tr><th>اختيار</th><th>المريض</th><th>الموعد الحالي</th><th>الإجراء</th><th>المعالج البديل</th><th>الموعد الجديد</th></tr></thead>
              <tbody>
                {affected.map((row) => {
                  const draft = drafts[row.appointmentId];
                  if (!draft) return null;
                  return <tr key={row.appointmentId}>
                    <td><input aria-label={`اختيار الموعد ${row.appointmentId}`} type="checkbox" checked={draft.selected} onChange={(e) => patchDraft(row.appointmentId, { selected: e.target.checked })} /></td>
                    <td><strong>{row.patientName}</strong><br /><small>ملف <bdi>{row.fileNo}</bdi></small></td>
                    <td>{displayDateTime(row.startsAt)}</td>
                    <td><select value={draft.actionType} onChange={(e) => patchDraft(row.appointmentId, { actionType: e.target.value as ProviderResolutionAction })}>{Object.entries(actionLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></td>
                    <td>{draft.actionType !== 'cancel' ? <select value={draft.replacementDoctorId} onChange={(e) => patchDraft(row.appointmentId, { replacementDoctorId: e.target.value })}><option value="">{draft.actionType === 'reschedule' ? 'نفس المعالج أو اختر بديلًا' : 'اختر المعالج البديل'}</option>{alternatives.map((doctor) => <option key={doctor.id} value={doctor.id}>{doctor.name}{doctor.specialty ? ` — ${doctor.specialty}` : ''}</option>)}</select> : '—'}</td>
                    <td>{draft.actionType === 'reschedule' ? <input type="datetime-local" min="1950-01-01T00:00" max="2050-12-31T23:59" value={draft.newStartsAt} onChange={(e) => patchDraft(row.appointmentId, { newStartsAt: e.target.value })} /> : '—'}</td>
                  </tr>;
                })}
              </tbody>
            </table>
          </div>}

          {affected.length > 0 && <div className="providerBatchActions">
            <button className="primary" type="button" onClick={() => void resolveSelected()} disabled={busy}><ArrowRightLeft aria-hidden="true" />{busy ? 'جارٍ تنفيذ المعالجة…' : 'معالجة المواعيد المحددة دفعة واحدة'}</button>
            <span className="notice"><UserRoundCheck aria-hidden="true" /> يتم التحقق من دوام وسعة المعالج البديل والتعارضات قبل اعتماد أي تغيير، والمعالجة الجماعية ذرّية.</span>
          </div>}
        </>
      )}
    </section>
  );
}
