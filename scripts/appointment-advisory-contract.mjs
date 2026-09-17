import { readFileSync } from 'node:fs';

const page = readFileSync('src/AppointmentsPage.tsx', 'utf8');
const advisory = readFileSync('src/saudiScheduleAdvisory.ts', 'utf8');
const tight = (value) => value.replace(/\s+/g, '');
const p = tight(page);
const a = tight(advisory);

const checks = [
  ['appointment form loads scheduling settings and closure dates', p.includes('api.schedulingSettings()') && p.includes('api.closures()') && p.includes('setSchedule(s)') && p.includes('setClosures(closedDays)')],
  ['appointment form computes live scheduling advisories', p.includes('getSaudiScheduleAdvisories(form,schedule,closures)') && page.includes('scheduleAdvisories')],
  ['blocking advisories prevent submit and disable save', p.includes("hasBlockingAdvisory") && page.includes('لا يمكن حفظ الموعد قبل معالجة تنبيهات الجدولة المانعة') && p.includes('disabled={busy||!patient||hasBlockingAdvisory||')],
  ['weekly holiday is classified as blocking', a.includes("code:'weekend'") && advisory.includes('الجمعة أو السبت') && a.includes("severity:'block'" )],
  ['configured closure date is classified as blocking', a.includes("code:'closure'") && advisory.includes('اليوم محدد كيوم إغلاق')],
  ['outside working hours and break overlap are blocking', a.includes("code:'outside-hours'") && a.includes("code:'break-time'") && advisory.includes('خارج وقت الدوام المعتمد') && advisory.includes('فترة الاستراحة')],
  ['national occasions are surfaced as operational warnings', a.includes("code:'founding-day'") && a.includes("code:'national-day'") && advisory.includes('يوم التأسيس') && advisory.includes('اليوم الوطني')],
  ['Ramadan and Eid/Hajj periods are surfaced as operational warnings', a.includes("code:'ramadan'") && a.includes("code:'eid-fitr'") && a.includes("code:'hajj-eid-adha'") && advisory.includes('رمضان') && advisory.includes('عيد الفطر') && advisory.includes('الحج/عيد الأضحى')],
  ['religious-period calculation uses device calendar internally without changing displayed Gregorian calendar', advisory.includes("'islamic-umalqura'") && advisory.includes("'islamic'") && !page.includes('هجري')],
  ['advisories distinguish blocking from informational warnings in the UI', page.includes('تنبيه مانع للحجز') && page.includes('تنبيه تشغيلي') && a.includes("severity:'warning'")],
];

const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
if (failed.length) {
  console.error(`Appointment scheduling advisory contract failed: ${failed.length} check(s)`);
  process.exit(1);
}
console.log(`Appointment scheduling advisory contract passed: ${checks.length}/${checks.length}`);
