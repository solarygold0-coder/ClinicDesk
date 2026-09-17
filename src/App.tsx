import { useEffect, useRef, useState } from 'react';
import { Building2, CalendarDays, FileClock, LayoutDashboard, LogOut, Settings, UserCog, Users } from 'lucide-react';
import { Dashboard } from './Dashboard';
import { PatientsPage } from './PatientsPage';
import { DirectoryPage } from './DirectoryPage';
import { AppointmentsPage } from './AppointmentsPage';
import { SchedulingPage } from './SchedulingPage';
import { AuthGate } from './AuthGate';
import { UsersPage } from './UsersPage';
import { AuditPage } from './AuditPage';
import { api, AuthSession, setActorToken } from './api';

type Page='dashboard'|'patients'|'appointments'|'directory'|'settings'|'users'|'audit';
type PatientAction={kind:'add'|'search';token:number}|null;
type AppointmentAction={kind:'add';token:number}|null;

export function App(){
  const [session,setSession]=useState<AuthSession|null>(null);
  const [page,setPage]=useState<Page>('dashboard');
  const [patientFileNo,setPatientFileNo]=useState<number|null>(null);
  const [patientAction,setPatientAction]=useState<PatientAction>(null);
  const [appointmentAction,setAppointmentAction]=useState<AppointmentAction>(null);
  const [auditEmployee,setAuditEmployee]=useState('');
  const actionSeq=useRef(0);
  const canManage=session?.user.roleType==='general_manager'||session?.user.roleType==='deputy_manager';
  const restrictedClinical=session?.user.roleType==='doctor'||session?.user.roleType==='specialist';

  function nextToken(){actionSeq.current+=1;return actionSeq.current}
  function openPatients(fileNo?:number){if(restrictedClinical)return;setPatientAction(null);setPatientFileNo(fileNo??null);setPage('patients')}
  function openPatientAction(kind:'add'|'search'){if(restrictedClinical)return;setPatientFileNo(null);setPatientAction({kind,token:nextToken()});setPage('patients')}
  function openAppointments(){setAppointmentAction(null);setPage('appointments')}
  function addAppointment(){if(restrictedClinical)return;setAppointmentAction({kind:'add',token:nextToken()});setPage('appointments')}
  function navigate(next:Page){if(next!=='patients'){setPatientAction(null);setPatientFileNo(null)}if(next!=='appointments')setAppointmentAction(null);setPage(next)}
  function openLifecycle(employeeCode:string){setAuditEmployee(employeeCode);navigate('audit')}

  useEffect(()=>{
    if(!session)return;
    if(session.user.roleType==='doctor'||session.user.roleType==='specialist')setPage('appointments');
  },[session?.user.id]);

  useEffect(()=>{
    if(!session)return;
    const handler=(event:KeyboardEvent)=>{
      if(event.key==='F2'&&!restrictedClinical){event.preventDefault();openPatientAction('search');return}
      if(!event.altKey||event.ctrlKey||event.metaKey)return;
      if(event.key==='1'&&!restrictedClinical){event.preventDefault();navigate('dashboard')}
      else if(event.key==='2'&&!restrictedClinical){event.preventDefault();openPatients()}
      else if(event.key==='3'){event.preventDefault();openAppointments()}
    };
    window.addEventListener('keydown',handler);return()=>window.removeEventListener('keydown',handler)
  },[session?.user.id,restrictedClinical]);

  async function logout(){
    if(!session)return;
    try{await api.logout(session.token)}finally{sessionStorage.removeItem('clinicdesk.session');setActorToken(null);setSession(null);setPage('dashboard');setAuditEmployee('')}
  }

  if(!session)return <AuthGate onAuthenticated={setSession}/>;

  return <div className="app" dir="rtl">
    <aside aria-label="التنقل الرئيسي">
      <div className="brand"><div className="logo" aria-hidden="true">ط</div><div><b>ClinicDesk</b><small>إدارة سجلات المرضى</small></div></div>
      <nav>
        {!restrictedClinical&&<button aria-current={page==='dashboard'?'page':undefined} aria-keyshortcuts="Alt+1" className={page==='dashboard'?'active':''} onClick={()=>navigate('dashboard')}><LayoutDashboard aria-hidden="true"/>لوحة التحكم</button>}
        {!restrictedClinical&&<button aria-current={page==='patients'?'page':undefined} aria-keyshortcuts="Alt+2" className={page==='patients'?'active':''} onClick={()=>openPatients()}><Users aria-hidden="true"/>المرضى</button>}
        <button aria-current={page==='appointments'?'page':undefined} aria-keyshortcuts="Alt+3" className={page==='appointments'?'active':''} onClick={openAppointments}><CalendarDays aria-hidden="true"/>المواعيد</button>
        {!restrictedClinical&&<button aria-current={page==='directory'?'page':undefined} className={page==='directory'?'active':''} onClick={()=>navigate('directory')}><Building2 aria-hidden="true"/>العيادات والأطباء</button>}
        {canManage&&<button aria-current={page==='users'?'page':undefined} className={page==='users'?'active':''} onClick={()=>navigate('users')}><UserCog aria-hidden="true"/>المستخدمون والصلاحيات</button>}
        {canManage&&<button aria-current={page==='audit'?'page':undefined} className={page==='audit'?'active':''} onClick={()=>{setAuditEmployee('');navigate('audit')}}><FileClock aria-hidden="true"/>سجل العمليات</button>}
      </nav>
      <div className="asideBottom">
        {!restrictedClinical&&<button aria-current={page==='settings'?'page':undefined} className={page==='settings'?'active':''} onClick={()=>navigate('settings')}><Settings aria-hidden="true"/>الإعدادات</button>}
        <div className="signedUser"><b>{session.user.displayName}</b><small><bdi>{session.user.employeeCode}</bdi> • {session.user.roleType}</small></div>
        <button type="button" onClick={()=>void logout()}><LogOut aria-hidden="true"/>تسجيل الخروج</button>
        <small>قاعدة بيانات محلية • SQLite</small>
      </div>
    </aside>
    <main>
      {page==='dashboard'&&!restrictedClinical&&<Dashboard onPatients={()=>openPatients()} onPatient={openPatients} onPatientAdd={()=>openPatientAction('add')} onPatientSearch={()=>openPatientAction('search')} onDirectory={()=>navigate('directory')} onAppointments={openAppointments} onAppointmentAdd={addAppointment}/>} 
      {page==='patients'&&!restrictedClinical&&<PatientsPage initialFileNo={patientFileNo} action={patientAction}/>} 
      {page==='directory'&&!restrictedClinical&&<DirectoryPage/>}
      {page==='appointments'&&<AppointmentsPage onPatient={openPatients} action={appointmentAction}/>} 
      {page==='settings'&&!restrictedClinical&&<SchedulingPage/>}
      {page==='users'&&canManage&&<UsersPage currentUser={session.user} onLifecycle={openLifecycle}/>} 
      {page==='audit'&&canManage&&<AuditPage initialEmployeeCode={auditEmployee}/>} 
    </main>
  </div>
}
