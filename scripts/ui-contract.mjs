import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const tight = (text) => text.replace(/\s+/g, '');
const files = {
  app: read('src/App.tsx'),
  main: read('src/main.tsx'),
  dashboard: read('src/Dashboard.tsx'),
  patients: read('src/PatientsPage.tsx'),
  appointments: read('src/AppointmentsPage.tsx'),
  scheduling: read('src/SchedulingPage.tsx'),
  directory: read('src/DirectoryPage.tsx'),
  attachments: read('src/PatientAttachments.tsx'),
  api: read('src/api.ts'),
  rtl: read('src/rtl.css'),
  print: read('src/print.css'),
  ui: read('src/ui-fixes.css'),
};
const t = Object.fromEntries(Object.entries(files).map(([key, value]) => [key, tight(value)]));
const productText = [files.app, files.dashboard, files.patients, files.appointments, files.scheduling, files.directory, files.attachments].join('\n');

const checks = [
  ['root app remains RTL', t.app.includes('className="app"dir="rtl"')],
  ['document language is Arabic', t.main.includes("document.documentElement.lang='ar'")],
  ['document direction is RTL', t.main.includes("document.documentElement.dir='rtl'")],
  ['RTL stylesheet enforces page direction and logical sidebar/mobile boundaries', files.main.includes("'./rtl.css'") && t.rtl.includes('.page{direction:rtl;text-align:start}') && t.rtl.includes('.app>aside{border-inline-start:0;border-inline-end:1pxsolid#e6ebf1}') && t.rtl.includes('.alertsPopover{inset-inline:20px}') && !files.rtl.includes('left: 20px') && !files.rtl.includes('right: 20px')],
  ['UI hardening stylesheet is loaded', files.main.includes("'./ui-fixes.css'")],
  ['print stylesheet is loaded', files.main.includes("'./print.css'")],
  ['F2 patient search shortcut exists', t.app.includes("event.key==='F2'")],
  ['Alt+1 dashboard shortcut exists', t.app.includes("event.key==='1'") && t.app.includes("navigate('dashboard')")],
  ['Alt+2 patient shortcut exists', t.app.includes("event.key==='2'") && t.app.includes('openPatients()')],
  ['Alt+3 appointment shortcut exists', t.app.includes("event.key==='3'") && t.app.includes('openAppointments()')],
  ['primary navigation exposes current page', files.app.includes('aria-current=')],
  ['navigation has an accessible label', files.app.includes('aria-label="التنقل الرئيسي"')],
  ['one-click patient action is wired', t.dashboard.includes('onClick={onPatientAdd}')],
  ['one-click appointment action is wired', t.dashboard.includes('onClick={onAppointmentAdd}')],
  ['one-click patient search is wired', t.dashboard.includes('onClick={onPatientSearch}')],
  ['patient quick action is consumed', t.patients.includes("action.kind==='add'")],
  ['appointment quick action is consumed', t.appointments.includes("action?.kind==='add'")],
  ['patient search parameter matches backend contract', t.api.includes("patients:(query='')=>invoke<Patient[]>('patient_list',{query,limit:50})")],
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

if (checks.length !== 50) {
  console.error(`RTL/UI contract definition must contain exactly 50 checks, found ${checks.length}`);
  process.exit(1);
}
const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
if (failed.length) {
  console.error(`RTL/UI contract failed: ${failed.length} check(s)`);
  process.exit(1);
}
console.log(`RTL/UI contract passed: ${checks.length}/${checks.length}`);
