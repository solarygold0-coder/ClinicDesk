import { readFileSync } from 'node:fs';

const audit = readFileSync('src/AuditPage.tsx', 'utf8');
const required = [
  ['appointment_status_changed', 'تغيير حالة موعد'],
  ['provider_unavailability_created', 'تسجيل تعذّر معالج'],
  ['provider_unavailability_transfer', 'تحويل موعد بسبب تعذّر المعالج'],
  ['provider_unavailability_reschedule', 'إعادة جدولة بسبب تعذّر المعالج'],
  ['provider_unavailability_cancel', 'إلغاء بسبب تعذّر المعالج'],
  ['provider_unavailability_replacement_created', 'إنشاء موعد بديل بسبب تعذّر المعالج'],
  ['provider_unavailability', 'تعذّر معالج'],
];

const missing = required.filter(([key, label]) => !audit.includes(key) || !audit.includes(label));
for (const [key, label] of required) {
  console.log(`${missing.some(([missingKey]) => missingKey === key) ? 'FAIL' : 'PASS'} ${key} -> ${label}`);
}
if (missing.length) {
  console.error(`Arabic lifecycle audit-label contract failed: ${missing.length} mapping(s) missing`);
  process.exit(1);
}
console.log(`Arabic lifecycle audit-label contract passed: ${required.length}/${required.length}`);
