import { useEffect, useMemo, useState } from 'react';
import {
  Archive,
  Bell,
  Building2,
  CalendarDays,
  Clock3,
  Search,
  UserPlus,
  Users,
  X,
} from 'lucide-react';
import { api, Appointment, FollowUpVisit, Patient } from './api';
import { fullGregorianDateTime } from './dateDisplay';
import { appointmentDisplayClass, appointmentDisplayLabel, appointmentDisplayStatus } from './appointmentPresentation';

const pad = (n: number) => String(n).padStart(2, '0');
const day = (d = new Date()) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
type Filter = 'today' | 'week' | 'overdue' | 'missed' | 'renewal' | 'threeMonths' | 'upcoming';
const filterLabels: Record<Filter, string> = {
  today: 'اليوم',
  week: 'الأسبوع القادم',
  overdue: 'فائت',
  missed: 'لم يحضر',
  renewal: 'التجديد',
  threeMonths: 'متابعة بعد 3 أشهر',
  upcoming: 'كل القادمة',
};

export function Dashboard({
  onPatients: _onPatients,
  onPatient,
  onPatientAdd,
  onPatientSearch,
  onDirectory,
  onAppointments,
  onAppointmentAdd,
}: {
  onPatients: () => void;
  onPatient: (fileNo: number) => void;
  onPatientAdd: () => void;
  onPatientSearch: () => void;
  onDirectory: () => void;
  onAppointments: () => void;
  onAppointmentAdd: () => void;
}) {
  const [rows, setRows] = useState<Appointment[]>([]);
  const [rangeRows, setRangeRows] = useState<Appointment[]>([]);
  const [upcomingRows, setUpcomingRows] = useState<Appointment[]>([]);
  const [missedRows, setMissedRows] = useState<Appointment[]>([]);
  const [overdueRows, setOverdueRows] = useState<Appointment[]>([]);
  const [followUps, setFollowUps] = useState<FollowUpVisit[]>([]);
  const [alerts, setAlerts] = useState<Appointment[]>([]);
  const [showUpcomingReminder, setShowUpcomingReminder] = useState(false);
  const [inactive, setInactive] = useState<Patient[]>([]);
  const [patientCount, setPatientCount] = useState<number | null>(null);
  const [showAlerts, setShowAlerts] = useState(false);
  const [filter, setFilter] = useState<Filter>('today');
  const [error, setError] = useState('');

  useEffect(() => {
    const now = new Date();
    const today = day(now);
    const until = new Date(now);
    const rangeEnd = new Date(now);
    const followEnd = new Date(now);
    until.setDate(until.getDate() + 2);
    rangeEnd.setDate(rangeEnd.getDate() + 90);
    followEnd.setFullYear(followEnd.getFullYear() + 10);
    void Promise.all([
      api.appointments(`${today}T00:00:00`, `${today}T23:59:59`),
      api.appointments(`${today}T00:00:00`, `${day(until)}T23:59:59`),
      api.appointments(`${today}T00:00:00`, `${day(rangeEnd)}T23:59:59`),
      api.upcomingAppointments(`${today}T00:00:00`),
      api.missedAppointments(`${today}T00:00:00`),
      api.overdueAppointments(`${today}T00:00:00`),
      api.followUps(`${today}T00:00:00`, `${day(followEnd)}T23:59:59`),
      api.patientCount(),
      api.inactivePatients(10, 100),
    ])
      .then(([a, next, range, upcoming, missed, overdue, followUpRows, totalPatients, inactivePatients]) => {
        const nextAlerts = next.filter((x) => x.status === 'scheduled' && new Date(x.startsAt).getTime() >= now.getTime());
        setRows(a);
        setAlerts(nextAlerts);
        setRangeRows(range);
        setUpcomingRows(upcoming);
        setMissedRows(missed);
        setOverdueRows(overdue);
        setFollowUps(followUpRows);
        setPatientCount(totalPatients);
        setInactive(inactivePatients);
        if (nextAlerts.length > 0) {
          const reminderKey = `clinicdesk.two-day-reminder.${today}`;
          try {
            if (!sessionStorage.getItem(reminderKey)) {
              setShowUpcomingReminder(true);
              sessionStorage.setItem(reminderKey, 'shown');
            }
          } catch {
            setShowUpcomingReminder(true);
          }
        }
        setError('');
      })
      .catch((e) => setError(String(e)));
  }, []);

  const waiting = rows.filter((a) => ['upcoming', 'due_now', 'arrived'].includes(appointmentDisplayStatus(a))).length;
  const current = useMemo(
    () =>
      rows.find((a) => a.status === 'in_progress') ||
      rows.find((a) => {
        const now = Date.now();
        const start = new Date(a.startsAt).getTime();
        const end = new Date(a.endsAt).getTime();
        return a.status === 'scheduled' && start <= now && now < end;
      }) ||
      null,
    [rows],
  );
  const filtered = useMemo(() => {
    const now = new Date();
    const week = new Date(now);
    week.setDate(week.getDate() + 7);
    if (filter === 'today') return rows.filter((a) => a.status !== 'cancelled');
    if (filter === 'week') return rangeRows.filter((a) => new Date(a.startsAt) <= week && a.status !== 'cancelled');
    if (filter === 'overdue') return overdueRows;
    if (filter === 'missed') return missedRows;
    return upcomingRows.filter(
      (a) => ['scheduled', 'arrived', 'in_progress'].includes(a.status) && new Date(a.startsAt) >= now,
    );
  }, [filter, rows, rangeRows, upcomingRows, missedRows, overdueRows]);
  const followFiltered = useMemo(() => {
    const now = new Date();
    const three = new Date(now);
    three.setMonth(three.getMonth() + 3);
    if (filter === 'renewal') return followUps.filter((x) => x.visitType === 'renewal');
    if (filter === 'threeMonths') return followUps.filter((x) => new Date(x.followUpAt) >= three);
    return [];
  }, [filter, followUps]);
  const special = filter === 'renewal' || filter === 'threeMonths';
  const alertCount = alerts.length + overdueRows.length + inactive.length;

  return (
    <>
      <header>
        <div>
          <h1>لوحة التحكم</h1>
          <p>كل ما تحتاجه لإدارة عمل اليوم في مكان واحد.</p>
        </div>
        <div className="headActions">
          <div className="alertsWrap">
            <button
              className="icon"
              type="button"
              aria-label="التنبيهات"
              aria-expanded={showAlerts}
              aria-controls="dashboard-alerts"
              onClick={() => setShowAlerts((v) => !v)}
            >
              <Bell aria-hidden="true" />
              {alertCount > 0 && <span className="alertBadge">{alertCount}</span>}
            </button>
            {showAlerts && (
              <div id="dashboard-alerts" className="alertsPopover" role="region" aria-label="تنبيهات لوحة التحكم">
                <div className="panelHead">
                  <div>
                    <h2>التنبيهات</h2>
                    <p>المواعيد القريبة والملفات التي تحتاج مراجعة</p>
                  </div>
                </div>
                {alertCount === 0 ? (
                  <div className="empty compact">
                    <Bell aria-hidden="true" />
                    <h3>لا توجد تنبيهات</h3>
                    <p>لا توجد مواعيد قريبة أو ملفات غير نشطة منذ 10 سنوات.</p>
                  </div>
                ) : (
                  <div className="alertList">
                    {alerts.slice(0, 8).map((a) => (
                      <button key={`appointment-${a.id}`} type="button" onClick={() => onPatient(a.fileNo)}>
                        <CalendarDays aria-hidden="true" />
                        <span>
                          <b>{a.patientName}</b>
                          <small>
                            موعد قريب • ملف <bdi>{a.fileNo}</bdi> •{' '}
                            {fullGregorianDateTime(a.startsAt)}
                          </small>
                        </span>
                      </button>
                    ))}
                    {overdueRows.slice(0, 8).map((a) => (
                      <button key={`overdue-${a.id}`} type="button" onClick={() => onPatient(a.fileNo)}>
                        <Clock3 aria-hidden="true" />
                        <span><b>{a.patientName}</b><small>موعد فائت • ملف <bdi>{a.fileNo}</bdi> • {fullGregorianDateTime(a.startsAt)}</small></span>
                      </button>
                    ))}
                    {inactive.slice(0, 8).map((p) => (
                      <button key={`inactive-${p.id}`} type="button" onClick={() => onPatient(p.fileNo)}>
                        <Archive aria-hidden="true" />
                        <span>
                          <b>{p.fullName}</b>
                          <small>
                            ملف <bdi>{p.fileNo}</bdi> • غير نشط منذ 10 سنوات — للمراجعة فقط
                          </small>
                        </span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
          <button className="primary" type="button" onClick={onPatientAdd}>
            <UserPlus aria-hidden="true" /> مريض جديد
          </button>
        </div>
      </header>

      {error && <div className="error" role="alert">{error}</div>}

      {showUpcomingReminder && alerts.length > 0 && (
        <section className="panel upcomingReminder noPrint" role="dialog" aria-modal="false" aria-labelledby="two-day-reminder-title">
          <div className="panelHead">
            <div>
              <h2 id="two-day-reminder-title"><Bell aria-hidden="true" /> تنبيه صامت للمواعيد القادمة</h2>
              <p>لديك {alerts.length} موعد/مواعيد خلال اليومين القادمين. يظهر هذا التنبيه داخل التطبيق فقط وبدون صوت.</p>
            </div>
            <button className="icon" type="button" aria-label="إغلاق تنبيه المواعيد القادمة" onClick={() => setShowUpcomingReminder(false)}><X aria-hidden="true" /></button>
          </div>
          <div className="alertList">
            {alerts.slice(0, 8).map((a) => (
              <button key={`two-day-${a.id}`} type="button" onClick={() => onPatient(a.fileNo)}>
                <CalendarDays aria-hidden="true" />
                <span>
                  <b>{a.patientName}</b>
                  <small>ملف <bdi>{a.fileNo}</bdi> • {fullGregorianDateTime(a.startsAt)}</small>
                </span>
              </button>
            ))}
          </div>
        </section>
      )}

      <section className="quick" aria-label="إجراءات سريعة">
        <button type="button" onClick={onPatientSearch} aria-keyshortcuts="F2">
          <Search aria-hidden="true" />
          <span><b>بحث سريع</b><small>اسم، رقم ملف أو هوية</small></span>
          <kbd>F2</kbd>
        </button>
        <button type="button" onClick={onPatientAdd}>
          <UserPlus aria-hidden="true" />
          <span><b>إضافة مريض</b><small>إنشاء ملف جديد</small></span>
        </button>
        <button type="button" onClick={onAppointmentAdd}>
          <CalendarDays aria-hidden="true" />
          <span><b>موعد جديد</b><small>حجز موعد للمراجع</small></span>
        </button>
      </section>

      <section className="stats" aria-label="ملخص اليوم">
        <article><span className="statIcon"><CalendarDays aria-hidden="true" /></span><div><small>مواعيد اليوم</small><strong>{rows.filter((a) => a.status !== 'cancelled').length}</strong></div></article>
        <article><span className="statIcon"><Clock3 aria-hidden="true" /></span><div><small>بانتظار الحضور</small><strong>{waiting}</strong></div></article>
        <article><span className="statIcon"><Users aria-hidden="true" /></span><div><small>إجمالي المرضى النشطين</small><strong>{patientCount ?? '—'}</strong></div></article>
        <article><span className="statIcon"><Archive aria-hidden="true" /></span><div><small>غير نشطين 10 سنوات</small><strong>{inactive.length}</strong></div></article>
      </section>

      <section className="workspace">
        <div className="panel schedule">
          <div className="panelHead">
            <div><h2>المواعيد</h2><p>فلترة تشغيلية سريعة للمواعيد</p></div>
            <button type="button" onClick={onAppointments}>عرض المواعيد</button>
          </div>
          <div className="appointmentFilters" aria-label="فلاتر المواعيد">
            {(Object.keys(filterLabels) as Filter[]).map((key) => (
              <button key={key} type="button" className={filter === key ? 'active' : ''} aria-pressed={filter === key} onClick={() => setFilter(key)}>
                {filterLabels[key]}
              </button>
            ))}
          </div>
          {special ? (
            followFiltered.length === 0 ? (
              <div className="empty"><CalendarDays aria-hidden="true" /><h3>لا توجد متابعات ضمن هذا الفلتر</h3><p>ستظهر المتابعات والتجديدات المسجلة هنا.</p></div>
            ) : (
              <div className="dashboardAppointments">
                {followFiltered.slice(0, 10).map((a) => (
                  <button key={`follow-${a.appointmentId}`} type="button" onClick={() => onPatient(a.fileNo)}>
                    <time dir="ltr">{fullGregorianDateTime(a.followUpAt)}</time>
                    <span><b>{a.patientName}</b><small>ملف <bdi>{a.fileNo}</bdi> • {a.visitType === 'renewal' ? 'تجديد' : 'متابعة'}</small></span>
                  </button>
                ))}
              </div>
            )
          ) : filtered.length === 0 ? (
            <div className="empty"><CalendarDays aria-hidden="true" /><h3>لا توجد مواعيد ضمن هذا الفلتر</h3><p>اختر فلترًا آخر أو أضف موعدًا جديدًا.</p></div>
          ) : (
            <div className="dashboardAppointments">
              {filtered.slice(0, 10).map((a) => (
                <button key={a.id} type="button" onClick={() => onPatient(a.fileNo)}>
                  <time dir="ltr">{fullGregorianDateTime(a.startsAt)}</time>
                  <span><b>{a.patientName}</b><small>ملف <bdi>{a.fileNo}</bdi> • {a.clinicName || a.doctorName || 'بدون تحديد'}</small></span>
                  <em className={appointmentDisplayClass(a)}>{appointmentDisplayLabel(a)}</em>
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="panel now">
          <div className="panelHead"><div><h2>الآن</h2><p>المراجع الحالي</p></div><span className="live">{current ? 'نشط' : 'جاهز'}</span></div>
          {current ? (
            <button className="currentPatient" type="button" onClick={() => onPatient(current.fileNo)}>
              <Clock3 aria-hidden="true" />
              <span><strong>{current.patientName}</strong><small>ملف <bdi>{current.fileNo}</bdi> • {current.doctorName || current.clinicName || 'بدون تحديد'}</small></span>
            </button>
          ) : (
            <div className="empty compact"><Clock3 aria-hidden="true" /><h3>لا يوجد مراجع حاليًا</h3><p>سيظهر الموعد الحالي هنا تلقائيًا.</p></div>
          )}
        </div>
      </section>

      <section className="setup">
        <div><Building2 aria-hidden="true" /><span><b>إعداد العيادة</b><small>لا توجد عيادات أو أطباء افتراضيون. أضف بيانات منشأتك عند الاستعداد.</small></span></div>
        <button type="button" onClick={onDirectory}>إضافة عيادة أو طبيب</button>
      </section>
    </>
  );
}
