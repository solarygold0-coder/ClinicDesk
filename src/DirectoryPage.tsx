import { FormEvent, useEffect, useState } from 'react';
import { Building2, Pencil, Plus, Stethoscope, Trash2, X } from 'lucide-react';
import { api, Clinic, Doctor } from './api';

export function DirectoryPage() {
  const [clinics, setClinics] = useState<Clinic[]>([]);
  const [doctors, setDoctors] = useState<Doctor[]>([]);
  const [clinicName, setClinicName] = useState('');
  const [doctorName, setDoctorName] = useState('');
  const [clinicId, setClinicId] = useState('');
  const [editingClinic, setEditingClinic] = useState<Clinic | null>(null);
  const [editingDoctor, setEditingDoctor] = useState<Doctor | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);

  async function load() {
    const [c, d] = await Promise.all([api.clinics(), api.doctors()]);
    setClinics(c);
    setDoctors(d);
  }
  useEffect(() => {
    void load().catch((e) => setError(String(e)));
  }, []);

  async function addClinic(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError('');
    try {
      if (editingClinic) await api.updateClinic(editingClinic.id, { name: clinicName });
      else await api.createClinic({ name: clinicName });
      setClinicName('');
      setEditingClinic(null);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function addDoctor(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError('');
    try {
      const input = { name: doctorName, clinicId: clinicId ? Number(clinicId) : null };
      if (editingDoctor) await api.updateDoctor(editingDoctor.id, input);
      else await api.createDoctor(input);
      setDoctorName('');
      setClinicId('');
      setEditingDoctor(null);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  function editClinic(c: Clinic) {
    if (busy) return;
    setEditingClinic(c);
    setClinicName(c.name);
  }
  function editDoctor(d: Doctor) {
    if (busy) return;
    setEditingDoctor(d);
    setDoctorName(d.name);
    setClinicId(d.clinicId ? String(d.clinicId) : '');
  }
  async function removeClinic(c: Clinic) {
    if (busy || !window.confirm(`تعطيل العيادة «${c.name}»؟ سيبقى سجلها التاريخي محفوظًا.`)) return;
    setBusy(true);
    setError('');
    try {
      await api.deleteClinic(c.id);
      if (editingClinic?.id === c.id) {
        setEditingClinic(null);
        setClinicName('');
      }
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function removeDoctor(d: Doctor) {
    if (busy || !window.confirm(`تعطيل الطبيب «${d.name}»؟ سيبقى سجله التاريخي محفوظًا.`)) return;
    setBusy(true);
    setError('');
    try {
      await api.deleteDoctor(d.id);
      if (editingDoctor?.id === d.id) {
        setEditingDoctor(null);
        setDoctorName('');
        setClinicId('');
      }
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="page" aria-busy={busy}>
      <div className="pageTitle"><div><h1>العيادات والأطباء</h1><p>ابدأ فارغًا وأضف بيانات منشأتك فقط. يمكنك التعديل أو التعطيل دون حذف السجل التاريخي.</p></div></div>
      {error && <div className="error" role="alert">{error}</div>}
      <div className="directoryGrid">
        <div className="tableCard">
          <div className="cardHead"><Building2 aria-hidden="true" /><div><h2>العيادات</h2><p>{clinics.length} عيادة</p></div></div>
          <form className="inlineAdd" onSubmit={addClinic} aria-label={editingClinic ? 'تعديل العيادة' : 'إضافة عيادة'}>
            <input disabled={busy} aria-label="اسم العيادة" value={clinicName} onChange={(e) => setClinicName(e.target.value)} placeholder="اسم العيادة" required />
            <button className="primary" disabled={busy}><Plus aria-hidden="true" />{busy ? 'جارٍ الحفظ…' : editingClinic ? 'حفظ التعديل' : 'إضافة'}</button>
            {editingClinic && <button type="button" disabled={busy} onClick={() => { setEditingClinic(null); setClinicName(''); }}><X aria-hidden="true" />إلغاء</button>}
          </form>
          {clinics.length === 0 ? <div className="miniEmpty">لا توجد عيادات حتى الآن. أضف أول عيادة من الحقل أعلاه.</div> : <div className="simpleList">{clinics.map((c) => <div key={c.id}><Building2 aria-hidden="true" /><b>{c.name}</b><button type="button" disabled={busy} onClick={() => editClinic(c)} title="تعديل" aria-label={`تعديل العيادة ${c.name}`}><Pencil aria-hidden="true" /></button><button type="button" disabled={busy} onClick={() => void removeClinic(c)} title="تعطيل" aria-label={`تعطيل العيادة ${c.name}`}><Trash2 aria-hidden="true" /></button></div>)}</div>}
        </div>
        <div className="tableCard">
          <div className="cardHead"><Stethoscope aria-hidden="true" /><div><h2>الأطباء</h2><p>{doctors.length} طبيب</p></div></div>
          <form className="inlineAdd stack" onSubmit={addDoctor} aria-label={editingDoctor ? 'تعديل الطبيب' : 'إضافة طبيب'}>
            <input disabled={busy} aria-label="اسم الطبيب" value={doctorName} onChange={(e) => setDoctorName(e.target.value)} placeholder="اسم الطبيب" required />
            <select disabled={busy} aria-label="عيادة الطبيب" value={clinicId} onChange={(e) => setClinicId(e.target.value)}><option value="">بدون عيادة محددة</option>{clinics.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select>
            <button className="primary" disabled={busy}><Plus aria-hidden="true" />{busy ? 'جارٍ الحفظ…' : editingDoctor ? 'حفظ التعديل' : 'إضافة طبيب'}</button>
            {editingDoctor && <button type="button" disabled={busy} onClick={() => { setEditingDoctor(null); setDoctorName(''); setClinicId(''); }}><X aria-hidden="true" />إلغاء</button>}
          </form>
          {doctors.length === 0 ? <div className="miniEmpty">لا يوجد أطباء حتى الآن. أضف أول طبيب من الحقول أعلاه.</div> : <div className="simpleList">{doctors.map((d) => <div key={d.id}><Stethoscope aria-hidden="true" /><b>{d.name}</b><small>{clinics.find((c) => c.id === d.clinicId)?.name || 'بدون عيادة'}</small><button type="button" disabled={busy} onClick={() => editDoctor(d)} title="تعديل" aria-label={`تعديل الطبيب ${d.name}`}><Pencil aria-hidden="true" /></button><button type="button" disabled={busy} onClick={() => void removeDoctor(d)} title="تعطيل" aria-label={`تعطيل الطبيب ${d.name}`}><Trash2 aria-hidden="true" /></button></div>)}</div>}
        </div>
      </div>
    </section>
  );
}
