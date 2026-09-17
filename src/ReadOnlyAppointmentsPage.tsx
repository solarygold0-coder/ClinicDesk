import { useEffect, useState } from 'react';
import { CalendarDays, Printer } from 'lucide-react';
import { api, Appointment } from './api';
import { fullGregorianDate, fullGregorianDateTime } from './dateDisplay';
import { appointmentDisplayClass, appointmentDisplayLabel } from './appointmentPresentation';
import { safePrint } from './safePrint';

const pad=(n:number)=>String(n).padStart(2,'0');
const day=(d=new Date())=>`${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}`;

export function ReadOnlyAppointmentsPage(){
  const[date,setDate]=useState(day());
  const[rows,setRows]=useState<Appointment[]>([]);
  const[error,setError]=useState('');
  const[busy,setBusy]=useState(false);
  useEffect(()=>{
    const load=async()=>{
      setBusy(true);setError('');
      try{setRows(await api.appointments(`${date}T00:00:00`,`${date}T23:59:59`));}
      catch(e){setError(String(e));}
      finally{setBusy(false);}
    };
    void load();
  },[date]);
  return <section className="page appointmentsPage print-scope" dir="rtl" aria-busy={busy}>
    <div className="printOnly printHeader"><h1>ClinicDesk — مواعيد اليوم</h1><p>التاريخ الميلادي: {fullGregorianDate(date)} • عدد المواعيد: {rows.length}</p></div>
    <div className="pageTitle noPrint"><div><h1>المواعيد</h1><p>عرض المواعيد المصرح بها للقراءة فقط.</p></div><button type="button" onClick={()=>void safePrint()}><Printer aria-hidden="true"/>طباعة اليوم</button></div>
    <div className="appointmentToolbar noPrint"><label>يوم العمل<input type="date" min="1950-01-01" max="2050-12-31" value={date} onChange={(e)=>setDate(e.target.value)}/></label><div className="dayCount"><CalendarDays aria-hidden="true"/><span><strong>{rows.length}</strong> موعد في هذا اليوم</span></div></div>
    {error&&<div className="error noPrint" role="alert">{error}</div>}
    <div className="notice noPrint" role="status">هذا الحساب للعرض فقط؛ لا يمكنه إضافة أو تعديل أو إلغاء المواعيد.</div>
    <div className="tableCard printTableCard">{rows.length===0?<div className="empty"><CalendarDays aria-hidden="true"/><h3>لا توجد مواعيد في هذا اليوم</h3></div>:<table><thead><tr><th>الوقت</th><th>رقم الملف</th><th>المريض</th><th>العيادة</th><th>الطبيب</th><th>الحالة</th></tr></thead><tbody>{rows.map((a)=><tr key={a.id}><td dir="ltr">{fullGregorianDateTime(a.startsAt)}</td><td><bdi>{a.fileNo}</bdi></td><td>{a.patientName}</td><td>{a.clinicName||'—'}</td><td>{a.doctorName||'—'}</td><td><span className={appointmentDisplayClass(a)}>{appointmentDisplayLabel(a)}</span></td></tr>)}</tbody></table>}</div>
  </section>;
}
