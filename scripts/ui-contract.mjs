import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const tight = (text) => text.replace(/\s+/g, '');
// Provider-unavailability resume-after-restart is a permanent release regression gate.
const files = {
  app: read('src/App.tsx'),
  auth: read('src/AuthGate.tsx'),
  users: read('src/UsersPage.tsx'),
  audit: read('src/AuditPage.tsx'),
  readOnlyAppointments: read('src/ReadOnlyAppointmentsPage.tsx'),
  accountability: read('src/accountability.css'),
  main: read('src/main.tsx'),
  dashboard: read('src/Dashboard.tsx'),
  patients: read('src/PatientsPage.tsx'),
  appointments: read('src/AppointmentsPage.tsx'),
  scheduling: read('src/SchedulingPage.tsx'),
  directory: read('src/DirectoryPage.tsx'),
  attachments: read('src/PatientAttachments.tsx'),
  providerUnavailability: read('src/ProviderUnavailabilityPage.tsx'),
  providerCss: read('src/provider-unavailability.css'),
  providerRust: read('src-tauri/src/provider_unavailability.rs'),
  providerMigration: read('src-tauri/migrations/017_provider_unavailability.sql'),
  runtime: read('src-tauri/src/lib.rs'),
  api: read('src/api.ts'),
  rtl: read('src/rtl.css'),
  print: read('src/print.css'),
  ui: read('src/ui-fixes.css'),
};
const t = Object.fromEntries(Object.entries(files).map(([key, value]) => [key, tight(value)]));
const productText = [files.app, files.auth, files.users, files.audit, files.readOnlyAppointments, files.dashboard, files.patients, files.appointments, files.scheduling, files.directory, files.attachments, files.providerUnavailability].join('\n');

const checks = [
  ['root app remains RTL', t.app.includes('className="app"dir="rtl"')],
  ['document language is Arabic', t.main.includes("document.documentElement.lang='ar'")],
  ['document direction is RTL', t.main.includes("document.documentElement.dir='rtl'")],
  ['RTL stylesheet enforces page direction and logical sidebar/mobile boundaries', files.main.includes("'./rtl.css'") && t.rtl.includes('.page{direction:rtl;text-align:start;}') && t.rtl.includes('.app>aside{border-inline-start:0;border-inline-end:1pxsolid#e6ebf1;}') && t.rtl.includes('.alertsPopover{inset-inline:20px;}') && !files.rtl.includes('left: 20px') && !files.rtl.includes('right: 20px')],
  ['UI hardening stylesheet is loaded', files.main.includes("'./ui-fixes.css'")],
  ['accountability stylesheet is loaded', files.main.includes("'./accountability.css'") && t.accountability.includes('.authShell{') && t.accountability.includes('.usersPage') && t.accountability.includes('.auditPage')],
  ['print stylesheet is loaded', files.main.includes("'./print.css'")],
  ['F2 patient search shortcut exists', t.app.includes("event.key==='F2'")],
  ['Alt+1 dashboard shortcut exists', t.app.includes("event.key==='1'") && t.app.includes("navigate('dashboard')")],
  ['Alt+2 patient shortcut exists', t.app.includes("event.key==='2'") && t.app.includes('openPatients()')],
  ['Alt+3 appointment shortcut exists', t.app.includes("event.key==='3'") && t.app.includes('openAppointments()')],
  ['primary navigation exposes current page', files.app.includes('aria-current=')],
  ['navigation has an accessible label', files.app.includes('aria-label="التنقل الرئيسي"')],
  ['authentication gate blocks application until a session exists', t.app.includes('if(!session)return<AuthGate') && files.app.includes('onAuthenticated={setSession}')],
  ['first-run setup creates only a general manager then logs in', t.auth.includes("api.createUser(username.trim(),displayName.trim(),password,'general_manager')") && t.auth.includes('api.login(username.trim(),password)')],
  ['session token is restored validated and propagated', files.auth.includes("sessionStorage.getItem('clinicdesk.session')") && t.auth.includes('api.validateSession(saved)') && t.auth.includes('setActorToken(saved)')],
  ['temporary passwords must be replaced before application access', files.auth.includes('mustChangePassword') && files.auth.includes('تغيير كلمة المرور المؤقتة') && t.auth.includes('api.changePassword(pendingSession.token,password,newPassword)') && t.api.includes("changePassword:(token:string,currentPassword:string,newPassword:string)=>invoke<UserSummary>('auth_validate_session',{token,currentPassword,newPassword})")],
  ['logout revokes session and clears frontend actor context', t.app.includes('api.logout(session.token)') && files.app.includes("sessionStorage.removeItem('clinicdesk.session')") && t.app.includes('setActorToken(null)')],
  ['users and lifecycle navigation are role gated', files.app.includes('المستخدمون والصلاحيات') && files.app.includes('سجل العمليات') && t.app.includes("session?.user.roleType==='general_manager'||session?.user.roleType==='deputy_manager'")],
  ['users page supports role status password and lifecycle controls', files.users.includes('إضافة مستخدم') && files.users.includes('دورة الحياة') && t.users.includes('api.setUserStatus(') && t.users.includes('api.resetUserPassword(') && t.users.includes('api.updateUser(')],
  ['audit page exposes actor employee code and before-after details', files.audit.includes('دورة حياة الموظف وسجل العمليات') && files.audit.includes('actorEmployeeCode') && files.audit.includes('beforeJson') && files.audit.includes('afterJson')],
  ['clinical roles use a true read-only appointment surface', t.app.includes("restrictedClinical?<ReadOnlyAppointmentsPage/>:<AppointmentsPage") && files.readOnlyAppointments.includes('للقراءة فقط') && !files.readOnlyAppointments.includes('api.createAppointment') && !files.readOnlyAppointments.includes('api.updateAppointment') && !files.readOnlyAppointments.includes('api.appointmentStatus')],
  ['all operational API calls use authenticated wrapper', t.api.includes("constauthed=<T>(command:string,args:Record<string,unknown>={})=>invoke<T>(command,{...args,actorToken:actorToken??undefined})") && t.api.includes("patients:(query='')=>authed<Patient[]>('patient_list',{query,limit:50})") && t.api.includes("createBackup:(destinationPath:string)=>authed<string>('backup_create',{destinationPath})")],
  ['provider unavailability navigation is visible only outside restricted clinical mode', files.app.includes('تعذّر المعالج') && t.app.includes("navigate('provider-unavailability')") && t.app.includes("page==='provider-unavailability'&&!restrictedClinical&&<ProviderUnavailabilityPage/>")],
  ['provider unavailability stylesheet is loaded and responsive', files.main.includes("'./provider-unavailability.css'") && t.providerCss.includes('.providerUnavailabilityPage') && t.providerCss.includes('@media(max-width:900px)')],
  ['provider unavailability reasons are structured and other reason requires a note', ['leave','sudden_absence','assignment_meeting','emergency','other'].every((reason)=>files.providerUnavailability.includes(reason)) && files.providerUnavailability.includes("reasonCode === 'other'") && files.providerUnavailability.includes("required={reasonCode === 'other'}")],
  ['provider unavailability supports transfer reschedule and classified cancellation', files.providerUnavailability.includes('تحويل لمعالج بديل') && files.providerUnavailability.includes('إعادة جدولة') && files.providerUnavailability.includes('إلغاء بسبب تعذّر المعالج') && t.api.includes("ProviderResolutionAction='transfer'|'reschedule'|'cancel'")],
  ['provider unavailability batches selected appointments atomically through one API call', files.providerUnavailability.includes('دفعة واحدة') && t.providerUnavailability.includes('api.resolveProviderUnavailability(event.id,resolutions)') && t.providerRust.includes('unchecked_transaction()')],
  ['provider unavailability API commands use authenticated wrapper', t.api.includes("authed<ProviderUnavailabilityEvent>('provider_unavailability_create',{input})") && t.api.includes("authed<AffectedAppointment[]>('provider_unavailability_affected',{eventId})") && t.api.includes("authed<void>('provider_unavailability_resolve_many',{eventId,resolutions})")],
  ['provider unavailability open events can be resumed after restart', t.api.includes("providerUnavailabilityOpen:()=>authed<ProviderUnavailabilityEvent[]>('provider_unavailability_list_open')") && files.providerUnavailability.includes('حالات تعذّر مفتوحة تحتاج استكمال') && t.providerUnavailability.includes('resumeEvent(item)')],
  ['provider unavailability backend exposes authenticated open-event listing', t.providerRust.includes('pubfnlist_open_events') && files.runtime.includes('provider_unavailability_list_open')],
  ['provider unavailability backend checks working window replacement capacity and preserves classification', files.providerRust.includes('validate_work_window') && files.providerRust.includes('ensure_capacity') && files.providerRust.includes("disruption_kind='provider_unavailable'") && files.providerRust.includes("disruption_resolution='cancel'")],
  ['provider unavailability schema 17 and Tauri runtime are registered end to end', files.providerMigration.includes('provider_unavailability_events') && files.providerMigration.includes('provider_unavailability_actions') && files.runtime.includes('const LATEST_SCHEMA_VERSION: i64 = 17;') && files.runtime.includes('../migrations/017_provider_unavailability.sql') && files.runtime.includes('provider_unavailability_resolve_many')],
  ['one-click patient action is wired', t.dashboard.includes('onClick={onPatientAdd}')],
  ['one-click appointment action is wired', t.dashboard.includes('onClick={onAppointmentAdd}')],
  ['one-click patient search is wired', t.dashboard.includes('onClick={onPatientSearch}')],
  ['patient quick action is consumed', t.patients.includes("action.kind==='add'")],
  ['appointment quick action is consumed', t.appointments.includes("action?.kind==='add'")],
  ['patient search parameter matches authenticated backend contract', t.api.includes("patients:(query='')=>authed<Patient[]>('patient_list',{query,limit:50})")],
  ['patient birth dates start at 1950', files.patients.includes('min="1950-01-01"')],
  ['patient birth dates end at 2050', files.patients.includes('max="2050-12-31"')],
  ['appointment workday dates are bounded', files.appointments.includes('min="1950-01-01"') && files.appointments.includes('max="2050-12-31"')],
  ['appointment follow-up dates are bounded', files.appointments.includes('min="1950-01-01T00:00"') && files.appointments.includes('max="2050-12-31T23:59"')],
  ['closure dates are bounded', files.scheduling.includes('min="1950-01-01"') && files.scheduling.includes('max="2050-12-31"')],
  ['patient display explicitly uses Gregorian calendar', files.patients.includes('ar-SA-u-ca-gregory')],
  ['appointment display explicitly uses Gregorian calendar', files.appointments.includes('ar-SA-u-ca-gregory')],
  ['dashboard display explicitly uses Gregorian calendar', files.dashboard.includes('ar-SA-u-ca-gregory')],
  ['settings display explicitly uses Gregorian calendar', files.scheduling.includes('ar-SA-u-ca-gregory')],
  ['attachment display explicitly uses Gregorian calendar', files.attachments.includes('ar-SA-u-ca-gregory')],
  ['audit display explicitly uses Gregorian calendar', files.audit.includes('ar-SA-u-ca-gregory')],
  ['Hijri calendar markers are absent from product UI', !/(hijri|ummalqura|islamic|هجري)/i.test(productText)],
  ['print media gate exists', files.print.includes('@media print')],
  ['print output targets A4 portrait', files.print.includes('@page{size:A4 portrait')],
  ['print hides non-target page content', files.print.includes('body *{visibility:hidden!important}')],
  ['print scope restores target visibility', files.print.includes('.print-scope,.print-scope *{visibility:visible!important}')],
  ['print excludes interactive controls', files.print.includes('.noPrint,.print-scope button,.print-scope select,.print-scope input,.print-scope textarea')],
  ['patient record has print action', t.patients.includes('onClick={()=>window.print()}') && files.patients.includes('طباعة الملف')],
  ['appointment day has print action', t.appointments.includes('onClick={()=>window.print()}') && files.appointments.includes('طباعة اليوم')],
  ['four dashboard statistics have a four-column desktop grid', t.ui.includes('.stats{grid-template-columns:repeat(4,minmax(0,1fr))}')],
  ['dashboard statistics collapse to two columns', t.ui.includes('@media(max-width:1200px){.stats{grid-template-columns:repeat(2,minmax(0,1fr))}')],
  ['dashboard statistics collapse to one column', t.ui.includes('@media(max-width:900px){.stats{grid-template-columns:1fr}')],
  ['appointment date/time spinner layout is styled', t.ui.includes('.dateTimeSpinners{')],
  ['settings grid is styled', t.ui.includes('.settingsGrid{')],
  ['patient record cards match article markup', t.ui.includes('.recordGrid>article{')],
  ['keyboard focus and reduced motion are protected', t.ui.includes(':focus-visible{') && t.ui.includes('@media(prefers-reduced-motion:reduce)')],
  ['patient dialogs are accessible, escapable, and use archive semantics', files.patients.includes('role="dialog"') && files.patients.includes('aria-modal="true"') && files.patients.includes('aria-labelledby="patient-record-title"') && files.patients.includes("event.key !== 'Escape'") && files.patients.includes('أرشفة الملف') && !files.patients.includes('حذف ملف المريض')],
  ['appointment dialog is accessible and calendar-day aware', files.appointments.includes('role="dialog"') && files.appointments.includes('aria-modal="true"') && files.appointments.includes('aria-labelledby="appointment-form-title"') && files.appointments.includes("event.key !== 'Escape'") && files.appointments.includes('daysInMonth') && files.appointments.includes('max={maxDay}')],
  ['settings surface errors and blocks duplicate actions', files.scheduling.includes('role="alert"') && files.scheduling.includes('aria-busy={busy}') && t.scheduling.includes('if(busy)return;')],
  ['directory surfaces errors and blocks duplicate mutations', files.directory.includes('role="alert"') && files.directory.includes('aria-busy={busy}') && t.directory.includes('if(busy)return;')],
  ['attachments use Windows associations and block duplicate mutation UI', files.attachments.includes('openPath(') && files.attachments.includes('برنامج في Windows') && files.attachments.includes('aria-busy={busy}') && t.attachments.includes('if(busy')],
  ['dangerous attachment formats remain explicitly blocked', files.attachments.includes('الملفات التنفيذية والخطرة محظورة أمنيًا')],
  ['dashboard accessibility semantics are present', files.dashboard.includes('aria-expanded={showAlerts}') && files.dashboard.includes('aria-pressed={filter === key}') && files.dashboard.includes('role="alert"')],
];

if (checks.length !== 72) {
  console.error(`RTL/UI/accountability contract definition must contain exactly 72 checks, found ${checks.length}`);
  process.exit(1);
}
const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
if (failed.length) {
  console.error(`RTL/UI/accountability contract failed: ${failed.length} check(s)`);
  process.exit(1);
}
console.log(`RTL/UI/accountability contract passed: ${checks.length}/${checks.length}`);