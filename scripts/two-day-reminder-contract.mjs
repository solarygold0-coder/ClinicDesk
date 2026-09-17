import { readFileSync } from 'node:fs';

const dashboard = readFileSync('src/Dashboard.tsx', 'utf8');
const dateDisplay = readFileSync('src/dateDisplay.ts', 'utf8');
const tight = dashboard.replace(/\s+/g, '');

const checks = [
  ['dashboard queries appointments through the next two days', dashboard.includes('until.setDate(until.getDate() + 2)') && tight.includes('api.appointments(`${today}T00:00:00`,`${day(until)}T23:59:59`)')],
  ['only future scheduled appointments enter the two-day reminder', dashboard.includes("x.status === 'scheduled'") && dashboard.includes('new Date(x.startsAt).getTime() >= now.getTime()')],
  ['silent reminder is automatically shown when appointments exist', dashboard.includes('setShowUpcomingReminder(true)') && dashboard.includes('showUpcomingReminder && alerts.length > 0')],
  ['reminder is limited to once per work session and day', dashboard.includes('sessionStorage.getItem(reminderKey)') && dashboard.includes("sessionStorage.setItem(reminderKey, 'shown')") && dashboard.includes('clinicdesk.two-day-reminder.')],
  ['reminder is explicitly in-app and silent', dashboard.includes('تنبيه صامت للمواعيد القادمة') && dashboard.includes('يظهر هذا التنبيه داخل التطبيق فقط وبدون صوت')],
  ['reminder exposes appointment patient and full Gregorian date/time', dashboard.includes("key={`two-day-${a.id}`}") && ((dashboard.includes("toLocaleString('ar-SA-u-ca-gregory'") && dashboard.includes("weekday: 'long'")) || (dashboard.includes('fullGregorianDateTime(a.startsAt)') && dateDisplay.includes("ar-SA-u-ca-gregory") && dateDisplay.includes("weekday: 'long'") && dateDisplay.includes("month: 'long'")))],
  ['reminder can be dismissed without changing appointment state', dashboard.includes('setShowUpcomingReminder(false)') && !dashboard.includes('Notification(')],
];

const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
if (failed.length) {
  console.error(`Two-day silent reminder contract failed: ${failed.length} check(s)`);
  process.exit(1);
}
console.log(`Two-day silent reminder contract passed: ${checks.length}/${checks.length}`);
