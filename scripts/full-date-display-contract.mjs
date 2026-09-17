import { readFileSync } from 'node:fs';
const read=(p)=>readFileSync(p,'utf8');
const helper=read('src/dateDisplay.ts');
const patients=read('src/PatientsPage.tsx');
const appointments=read('src/AppointmentsPage.tsx');
const readonly=read('src/ReadOnlyAppointmentsPage.tsx');
const dashboard=read('src/Dashboard.tsx');
const scheduling=read('src/SchedulingPage.tsx');
const audit=read('src/AuditPage.tsx');
const attachments=read('src/PatientAttachments.tsx');
const provider=read('src/ProviderUnavailabilityPage.tsx');
const checks=[
 ['canonical formatter is Gregorian with weekday Arabic month and month number',helper.includes("ar-SA-u-ca-gregory")&&helper.includes("weekday: 'long'")&&helper.includes("month: 'long'")&&helper.includes('الشهر ${String(date.getMonth() + 1)')],
 ['patient birth date uses canonical full date',patients.includes('fullGregorianDate(details.birthDate)')&&!patients.includes("details.birthDate || 'غير مسجل'")],
 ['patient appointment history uses canonical full date-time',patients.includes('fullGregorianDateTime(a.startsAt)')],
 ['editable appointment print header uses canonical full date',appointments.includes('التاريخ الميلادي: {fullGregorianDate(date)}')],
 ['editable appointment rows show canonical full date-time',appointments.includes('fullGregorianDateTime(a.startsAt)')],
 ['read-only appointment print header uses canonical full date',readonly.includes('التاريخ الميلادي: {fullGregorianDate(date)}')],
 ['read-only appointment rows show canonical full date-time',readonly.includes('fullGregorianDateTime(a.startsAt)')],
 ['dashboard appointment and reminder dates use canonical full date-time',dashboard.includes("import { fullGregorianDateTime } from './dateDisplay'")&&(dashboard.match(/fullGregorianDateTime\(/g)?.length??0)>=3],
 ['closure days use canonical full date',scheduling.includes('fullGregorianDate(c.closureDate)')],
 ['audit timestamps retain weekday month name and month number',audit.includes("weekday:'long'")&&audit.includes('الشهر ${String(date.getMonth() + 1)')],
 ['attachment timestamps retain weekday month name and month number',attachments.includes("weekday: 'long'")&&attachments.includes('الشهر ${String(date.getMonth() + 1)')],
 ['provider-unavailability timestamps retain weekday month name and month number',provider.includes("weekday: 'long'")&&provider.includes('الشهر ${String(date.getMonth() + 1)')],
];
const failed=checks.filter(([,ok])=>!ok);
for(const[name,ok]of checks)console.log(`${ok?'PASS':'FAIL'} ${name}`);
if(failed.length){console.error(`Full Gregorian date display contract failed: ${failed.length} check(s)`);process.exit(1);}
console.log(`Full Gregorian date display contract passed: ${checks.length}/${checks.length}`);
