from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    if old not in text:
        raise SystemExit(f"missing expected pattern: {label}")
    return text.replace(old, new, 1)


p = Path("src/PatientsPage.tsx")
s = p.read_text()
s = replace_once(
    s,
    "import { PatientAttachments } from './PatientAttachments';",
    "import { PatientAttachments } from './PatientAttachments';\nimport { fullGregorianDate, fullGregorianDateTime } from './dateDisplay';",
    "patients date import",
)
s = replace_once(
    s,
    '<strong dir="ltr">{details.birthDate || \'غير مسجل\'}</strong>',
    '<strong>{fullGregorianDate(details.birthDate)}</strong>',
    "patient birth date",
)
s = re.sub(
    r"new Date\(a\.startsAt\)\.toLocaleString\('ar-SA-u-ca-gregory', \{ dateStyle: 'medium', timeStyle: 'short' \}\)",
    "fullGregorianDateTime(a.startsAt)",
    s,
)
p.write_text(s)

p = Path("src/AppointmentsPage.tsx")
s = p.read_text()
s = replace_once(
    s,
    "import { getSaudiScheduleAdvisories } from './saudiScheduleAdvisory';",
    "import { getSaudiScheduleAdvisories } from './saudiScheduleAdvisory';\nimport { fullGregorianDate, fullGregorianDateTime } from './dateDisplay';",
    "appointments date import",
)
s = replace_once(
    s,
    'التاريخ الميلادي: <bdi>{date}</bdi> • عدد المواعيد:',
    'التاريخ الميلادي: {fullGregorianDate(date)} • عدد المواعيد:',
    "appointment print date",
)
s = re.sub(
    r"new Date\(a\.startsAt\)\.toLocaleTimeString\('ar-SA-u-ca-gregory', \{ hour: 'numeric', minute: '2-digit' \}\)",
    "fullGregorianDateTime(a.startsAt)",
    s,
)
p.write_text(s)

p = Path("src/ReadOnlyAppointmentsPage.tsx")
s = p.read_text()
if "fullGregorianDateTime" not in s:
    s = s.replace(
        "import { fullGregorianDate } from './dateDisplay';",
        "import { fullGregorianDate, fullGregorianDateTime } from './dateDisplay';",
    )
s = re.sub(
    r"new Date\(a\.startsAt\)\.toLocaleTimeString\('ar-SA-u-ca-gregory',\{hour:'numeric',minute:'2-digit'\}\)",
    "fullGregorianDateTime(a.startsAt)",
    s,
)
p.write_text(s)

p = Path("src/Dashboard.tsx")
s = p.read_text()
s = replace_once(
    s,
    "import { api, Appointment, FollowUpVisit, Patient } from './api';",
    "import { api, Appointment, FollowUpVisit, Patient } from './api';\nimport { fullGregorianDateTime } from './dateDisplay';",
    "dashboard date import",
)
s = re.sub(
    r"new Date\(a\.startsAt\)\.toLocaleString\('ar-SA-u-ca-gregory', \{\s*weekday: 'short',\s*hour: 'numeric',\s*minute: '2-digit',?\s*\}\)",
    "fullGregorianDateTime(a.startsAt)",
    s,
)
s = re.sub(
    r"new Date\(a\.startsAt\)\.toLocaleString\('ar-SA-u-ca-gregory', \{ weekday: 'long', year: 'numeric', month: 'long', day: 'numeric', hour: 'numeric', minute: '2-digit' \}\)",
    "fullGregorianDateTime(a.startsAt)",
    s,
)
s = re.sub(
    r"new Date\(a\.followUpAt\)\.toLocaleString\('ar-SA-u-ca-gregory', \{ weekday: 'short', hour: 'numeric', minute: '2-digit' \}\)",
    "fullGregorianDateTime(a.followUpAt)",
    s,
)
p.write_text(s)

Path("scripts/full-date-display-contract.mjs").write_text(
    r'''import { readFileSync } from 'node:fs';
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
'''
)

p = Path("scripts/ui-contract.mjs")
s = p.read_text()
if "dateDisplay: read('src/dateDisplay.ts')" not in s:
    s = s.replace(
        "  api: read('src/api.ts'),\n",
        "  api: read('src/api.ts'),\n  dateDisplay: read('src/dateDisplay.ts'),\n",
    )
replacements = {
    "['patient display explicitly uses Gregorian calendar', files.patients.includes('ar-SA-u-ca-gregory')],": "['patient display explicitly uses Gregorian calendar', files.patients.includes('ar-SA-u-ca-gregory') || (files.patients.includes(\"from './dateDisplay'\") && files.dateDisplay.includes('ar-SA-u-ca-gregory'))],",
    "['appointment display explicitly uses Gregorian calendar', files.appointments.includes('ar-SA-u-ca-gregory')],": "['appointment display explicitly uses Gregorian calendar', files.appointments.includes('ar-SA-u-ca-gregory') || (files.appointments.includes(\"from './dateDisplay'\") && files.dateDisplay.includes('ar-SA-u-ca-gregory'))],",
    "['dashboard display explicitly uses Gregorian calendar', files.dashboard.includes('ar-SA-u-ca-gregory')],": "['dashboard display explicitly uses Gregorian calendar', files.dashboard.includes('ar-SA-u-ca-gregory') || (files.dashboard.includes(\"from './dateDisplay'\") && files.dateDisplay.includes('ar-SA-u-ca-gregory'))],",
    "['settings display explicitly uses Gregorian calendar', files.scheduling.includes('ar-SA-u-ca-gregory')],": "['settings display explicitly uses Gregorian calendar', files.scheduling.includes('ar-SA-u-ca-gregory') || (files.scheduling.includes(\"from './dateDisplay'\") && files.dateDisplay.includes('ar-SA-u-ca-gregory'))],",
}
for old, new in replacements.items():
    if old in s:
        s = s.replace(old, new)
    elif new not in s:
        raise SystemExit(f"missing RTL Gregorian contract pattern: {old}")
p.write_text(s)

p = Path(".github/workflows/ui-contract.yml")
s = p.read_text()
marker = "      - name: Verify silent two-day appointment reminder\n        run: node scripts/two-day-reminder-contract.mjs\n"
addition = marker + "      - name: Verify canonical full Gregorian date display\n        run: node scripts/full-date-display-contract.mjs\n"
if "full-date-display-contract.mjs" not in s:
    if marker not in s:
        raise SystemExit("missing UI workflow insertion marker")
    s = s.replace(marker, addition)
p.write_text(s)
