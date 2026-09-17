import { useEffect, useMemo, useState } from 'react';
import { FileClock, RefreshCcw, Search } from 'lucide-react';
import { api, AuditEntry } from './api';

const eventLabels: Record<string, string> = {
  create: 'إنشاء', update: 'تعديل', delete: 'أرشفة/حذف', status: 'تغيير حالة',
  user_created: 'إنشاء مستخدم', user_updated: 'تعديل مستخدم', user_status_changed: 'تغيير حالة مستخدم',
  user_password_reset: 'إعادة تعيين كلمة المرور', system_bootstrap: 'تهيئة النظام',
  attachment_added: 'إضافة مرفق', attachment_archived: 'أرشفة مرفق', attachment_restored: 'استعادة مرفق',
  backup_created: 'إنشاء نسخة احتياطية', restore_started: 'بدء استعادة', restore_completed: 'اكتمال استعادة',
  deputy_restore_permission_changed: 'تعديل صلاحية الاستعادة', clinic_created: 'إضافة عيادة', clinic_updated: 'تعديل عيادة',
  clinic_deactivated: 'تعطيل عيادة', doctor_created: 'إضافة طبيب', doctor_updated: 'تعديل طبيب', doctor_deactivated: 'تعطيل طبيب',
  scheduling_updated: 'تعديل ساعات العمل', closure_created: 'إضافة يوم إغلاق', closure_deleted: 'حذف يوم إغلاق',
};
const entityLabels: Record<string, string> = { patient:'مريض', appointment:'موعد', user:'مستخدم', database:'قاعدة البيانات', clinic:'عيادة', doctor:'طبيب', attachment:'مرفق', scheduling:'الإعدادات', closure:'يوم إغلاق', security:'الأمان' };

export function AuditPage({ initialEmployeeCode = '' }: { initialEmployeeCode?: string }) {
  const [rows, setRows] = useState<AuditEntry[]>([]);
  const [query, setQuery] = useState(initialEmployeeCode);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const load = async () => { setBusy(true); setError(''); try { setRows(await api.audit(500)); } catch (e) { setError(String(e)); } finally { setBusy(false); } };
  useEffect(() => { void load(); }, []);
  useEffect(() => { setQuery(initialEmployeeCode); }, [initialEmployeeCode]);
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((row) => [row.actorDisplayName,row.actorEmployeeCode,row.eventType,row.entityType,row.entityId?.toString(),row.reason,row.detailsJson].filter(Boolean).join(' ').toLowerCase().includes(q));
  }, [rows, query]);
  return <section className="page auditPage" dir="rtl" aria-busy={busy}>
    <div className="pageTitle"><div><h1>دورة حياة الموظف وسجل العمليات</h1><p>يكشف من قام بأي إجراء، ومتى، وعلى أي سجل، مع القيم قبل وبعد عندما تكون متاحة.</p></div><button type="button" disabled={busy} onClick={() => void load()}><RefreshCcw aria-hidden="true" />تحديث</button></div>
    {error && <div className="notice errorText" role="alert">{error}</div>}
    <div className="searchbar"><Search aria-hidden="true" /><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="ابحث باسم الموظف، Uxx، نوع العملية أو رقم الكيان" /></div>
    <div className="tableCard">
      {filtered.length === 0 ? <div className="empty"><FileClock aria-hidden="true" /><h3>لا توجد عمليات مطابقة</h3><p>ستظهر إجراءات الموظفين هنا فور تسجيلها.</p></div> : <table><thead><tr><th>التاريخ والوقت</th><th>الموظف</th><th>العملية</th><th>الكيان</th><th>السبب/التفاصيل</th></tr></thead><tbody>{filtered.map((row) => <tr key={row.id}>
        <td>{new Date(row.createdAt).toLocaleString('ar-SA-u-ca-gregory',{weekday:'long',year:'numeric',month:'long',day:'numeric',hour:'2-digit',minute:'2-digit'})}</td>
        <td>{row.actorDisplayName || 'نظام/قديم'}{row.actorEmployeeCode ? <small className="block"><bdi>{row.actorEmployeeCode}</bdi></small> : null}</td>
        <td>{eventLabels[row.eventType] || row.eventType}</td>
        <td>{entityLabels[row.entityType] || row.entityType}{row.entityId != null ? <> • <bdi>#{row.entityId}</bdi></> : null}</td>
        <td>{row.reason || '—'}{(row.beforeJson || row.afterJson || row.detailsJson) && <details><summary>عرض التفاصيل</summary>{row.beforeJson && <div><b>قبل:</b><pre>{row.beforeJson}</pre></div>}{row.afterJson && <div><b>بعد:</b><pre>{row.afterJson}</pre></div>}{row.detailsJson && <div><b>بيانات:</b><pre>{row.detailsJson}</pre></div>}</details>}</td>
      </tr>)}</tbody></table>}
    </div>
  </section>;
}
