import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const files = {
  app: read('src/App.tsx'),
  main: read('src/main.tsx'),
  dashboard: read('src/Dashboard.tsx'),
  patients: read('src/PatientsPage.tsx'),
  appointments: read('src/AppointmentsPage.tsx'),
  scheduling: read('src/SchedulingPage.tsx'),
  print: read('src/print.css'),
  ui: read('src/ui-fixes.css'),
};

const checks = [
  ['root app remains RTL', files.app.includes('className="app" dir="rtl"')],
  ['document language is Arabic', files.main.includes("document.documentElement.lang = 'ar'")],
  ['document direction is RTL', files.main.includes("document.documentElement.dir = 'rtl'")],
  ['RTL stylesheet is loaded', files.main.includes("'./rtl.css'")],
  ['UI hardening stylesheet is loaded', files.main.includes("'./ui-fixes.css'")],
  ['print stylesheet is loaded', files.main.includes("'./print.css'")],
  ['F2 patient search shortcut exists', files.app.includes("event.key==='F2'")],
  ['one-click patient action is wired', files.dashboard.includes('onClick={onPatientAdd}')],
  ['one-click appointment action is wired', files.dashboard.includes('onClick={onAppointmentAdd}')],
  ['patient quick action is consumed', files.patients.includes("action.kind==='add'")],
  ['appointment quick action is consumed', files.appointments.includes("action?.kind==='add'")],
  ['patient dates are bounded to 1950-2050', files.patients.includes('min="1950-01-01"') && files.patients.includes('max="2050-12-31"')],
  ['appointment dates are bounded to 1950-2050', files.appointments.includes('min="1950-01-01"') && files.appointments.includes('max="2050-12-31"')],
  ['closure dates are bounded to 1950-2050', files.scheduling.includes('min="1950-01-01"') && files.scheduling.includes('max="2050-12-31"')],
  ['patient display explicitly uses Gregorian calendar', files.patients.includes('ar-SA-u-ca-gregory')],
  ['appointment display explicitly uses Gregorian calendar', files.appointments.includes('ar-SA-u-ca-gregory')],
  ['dashboard display explicitly uses Gregorian calendar', files.dashboard.includes('ar-SA-u-ca-gregory')],
  ['settings display explicitly uses Gregorian calendar', files.scheduling.includes('ar-SA-u-ca-gregory')],
  ['print media gate exists', files.print.includes('@media print')],
  ['print output targets A4 portrait', files.print.includes('@page{size:A4 portrait')],
  ['print scope hides non-target UI', files.print.includes('body *{visibility:hidden!important}')],
  ['four dashboard statistics have a four-column desktop grid', files.ui.includes('.stats{grid-template-columns:repeat(4,minmax(0,1fr))}')],
  ['appointment date/time spinner layout is styled', files.ui.includes('.dateTimeSpinners{')],
  ['settings grid is styled', files.ui.includes('.settingsGrid{')],
  ['patient record cards match article markup', files.ui.includes('.recordGrid>article{')],
  ['keyboard focus is visibly styled', files.ui.includes(':focus-visible{')],
];

const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
if (failed.length) {
  console.error(`RTL/UI contract failed: ${failed.length} check(s)`);
  process.exit(1);
}
console.log(`RTL/UI contract passed: ${checks.length}/${checks.length}`);
