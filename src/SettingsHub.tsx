import { useState } from 'react';
import { DatabaseBackup, FileClock, Info, Settings2, UserCog } from 'lucide-react';
import { SchedulingPage } from './SchedulingPage';
import { UsersPage } from './UsersPage';
import { AuditPage } from './AuditPage';
import type { UserSummary } from './api';

type SettingsTab = 'operations' | 'users' | 'audit' | 'about';

export function SettingsHub({
  currentUser,
  canManage,
}: {
  currentUser: UserSummary;
  canManage: boolean;
}) {
  const [tab, setTab] = useState<SettingsTab>('operations');
  const [auditEmployee, setAuditEmployee] = useState('');

  const openLifecycle = (employeeCode: string) => {
    setAuditEmployee(employeeCode);
    setTab('audit');
  };

  return (
    <section className="page settingsHub" dir="rtl">
      <div className="pageTitle">
        <div>
          <h1>الإعدادات</h1>
          <p>مركز واحد لإعدادات التشغيل والمستخدمين والصلاحيات والسجل ومعلومات النسخة.</p>
        </div>
      </div>
      <div className="settingsTabs" role="tablist" aria-label="أقسام الإعدادات">
        <button type="button" className={tab === 'operations' ? 'active' : ''} onClick={() => setTab('operations')}><Settings2 aria-hidden="true" />التشغيل والنسخ</button>
        {canManage && <button type="button" className={tab === 'users' ? 'active' : ''} onClick={() => setTab('users')}><UserCog aria-hidden="true" />المستخدمون والصلاحيات</button>}
        {canManage && <button type="button" className={tab === 'audit' ? 'active' : ''} onClick={() => { setAuditEmployee(''); setTab('audit'); }}><FileClock aria-hidden="true" />سجل العمليات</button>}
        <button type="button" className={tab === 'about' ? 'active' : ''} onClick={() => setTab('about')}><Info aria-hidden="true" />حول البرنامج</button>
      </div>

      {tab === 'operations' && <SchedulingPage embedded />}
      {tab === 'users' && canManage && <UsersPage currentUser={currentUser} onLifecycle={openLifecycle} />}
      {tab === 'audit' && canManage && <AuditPage initialEmployeeCode={auditEmployee} />}
      {tab === 'about' && (
        <div className="aboutEdition card">
          <div className="cardHeading"><DatabaseBackup aria-hidden="true" /><div><h2>ClinicDesk</h2><p>نظام إدارة السجلات والمواعيد للعيادة</p></div></div>
          <div className="aboutGrid">
            <div><small>الإصدار</small><strong>5.1.0</strong></div>
            <div><small>نوع النسخة</small><strong>النسخة المجانية</strong></div>
            <div><small>هوية الموظف الحالي</small><strong><bdi>{currentUser.employeeCode}</bdi></strong></div>
            <div><small>وضع التشغيل الحالي</small><strong>جهاز رئيسي محلي</strong></div>
          </div>
          <p className="notice">وضع الشبكة الجاري اعتماده يستخدم خدمة ClinicDesk Server مركزية على الجهاز الرئيسي، والعملاء يتصلون عبر API داخل الشبكة المحلية دون فتح SQLite أو مجلد المرفقات مباشرة.</p>
        </div>
      )}
    </section>
  );
}
