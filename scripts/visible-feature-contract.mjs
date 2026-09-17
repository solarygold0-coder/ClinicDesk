import { readFileSync } from 'node:fs';
const read=(p)=>readFileSync(p,'utf8');
const files={welcome:read('src/WelcomeScreen.tsx'),auth:read('src/AuthGate.tsx'),users:read('src/UsersPage.tsx'),app:read('src/App.tsx'),settings:read('src/SettingsHub.tsx'),patients:read('src/PatientsPage.tsx'),appointments:read('src/AppointmentsPage.tsx'),dashboard:read('src/Dashboard.tsx'),readonly:read('src/ReadOnlyAppointmentsPage.tsx'),presentation:read('src/appointmentPresentation.ts'),print:read('src/safePrint.ts'),api:read('src/api.ts'),rust:read('src-tauri/src/appointments.rs'),lib:read('src-tauri/src/lib.rs')};
const checks=[
['welcome is visible before authentication',files.app.includes('<WelcomeScreen onContinue={()=>setWelcomed(true)}/>')&&files.welcome.includes('مرحبًا بك في ClinicDesk')],
['welcome shows free edition and 5.1.0',files.welcome.includes('النسخة المجانية')&&files.welcome.includes('الإصدار 5.1.0')],
['password UI enforces exactly four newly chosen characters',files.auth.includes('length !== 4')&&files.auth.includes('maxLength={4}')&&files.users.includes('length !== 4')&&files.users.includes('maxLength={4}')],
['settings exposes users',files.settings.includes('المستخدمون والصلاحيات')&&files.settings.includes('<UsersPage')],
['settings exposes audit',files.settings.includes('سجل العمليات')&&files.settings.includes('<AuditPage')],
['free edition visible',files.app.includes('النسخة المجانية')&&files.settings.includes('النسخة المجانية')],
['patient file contains linked appointments',files.patients.includes('المواعيد المرتبطة بهذا الملف')&&files.patients.includes('api.patientAppointments(p.id, 100)')],
['overdue label is distinct',files.presentation.includes("overdue: 'فائت'")&&files.dashboard.includes("overdue: 'فائت'")],
['overdue backend remains scheduled',files.rust.includes("a.status='scheduled'")&&files.rust.includes('list_overdue')],
['overdue alert visible',files.dashboard.includes('موعد فائت')&&files.api.includes('appointment_overdue_history')],
['shared status presentation used in all appointment surfaces',files.patients.includes('appointmentDisplayLabel(a)')&&files.appointments.includes('appointmentDisplayLabel(a)')&&files.readonly.includes('appointmentDisplayLabel(a)')&&files.dashboard.includes('appointmentDisplayLabel(a)')],
['printing guards main window close',files.print.includes('onCloseRequested')&&files.print.includes('event.preventDefault()')],
['patient printing uses guard',files.patients.includes('void safePrint()')],
['appointment printing uses guard',files.appointments.includes('void safePrint()')&&files.readonly.includes('void safePrint()')],
['settings page is the management hub',files.app.includes('<SettingsHub currentUser={session.user} canManage={canManage}/>')],
];
let failed=0; for(const [name,ok] of checks){console.log(`${ok?'PASS':'FAIL'} ${name}`);if(!ok)failed++;} if(failed){console.error(`Visible feature contract failed: ${failed}`);process.exit(1);} console.log(`Visible feature contract passed: ${checks.length}/${checks.length}`);
