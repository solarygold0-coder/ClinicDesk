import { FormEvent, useEffect, useState } from 'react';
import { KeyRound, RefreshCcw, Shield, UserPlus, UsersRound } from 'lucide-react';
import { AccountStatus, api, RoleType, UserSummary } from './api';

const roleLabels: Record<RoleType, string> = {
  general_manager: 'المدير العام',
  deputy_manager: 'نائب المدير',
  ordinary_employee: 'موظف عادي',
  doctor: 'طبيب',
  specialist: 'أخصائي',
};
const statusLabels: Record<AccountStatus, string> = {
  ACTIVE: 'نشط',
  SUSPENDED: 'موقوف',
  CLOSED_INACTIVITY: 'مغلق لعدم النشاط',
  ENDED_SERVICE: 'انتهاء خدمة',
  RETIRED: 'متقاعد',
};
const blank = { username: '', displayName: '', password: '', roleType: 'ordinary_employee' as RoleType };

export function UsersPage({ currentUser, onLifecycle }: { currentUser: UserSummary; onLifecycle: (employeeCode: string) => void }) {
  const [rows, setRows] = useState<UserSummary[]>([]);
  const [form, setForm] = useState(blank);
  const [deputyRestore, setDeputyRestore] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [ok, setOk] = useState('');

  const load = async () => {
    const users = await api.users();
    setRows(users);
    if (currentUser.roleType === 'general_manager') {
      setDeputyRestore(await api.deputyRestorePermission());
    }
  };
  useEffect(() => { void load().catch((e) => setError(String(e))); }, []);

  async function create(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true); setError(''); setOk('');
    try {
      if ([...form.password].length !== 4) throw new Error('كلمة المرور يجب أن تتكون من 4 خانات بالضبط');
      await api.createUser(form.username.trim(), form.displayName.trim(), form.password, form.roleType);
      setForm(blank);
      await load();
      setOk('تم إنشاء المستخدم');
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }

  async function changeStatus(user: UserSummary, status: AccountStatus) {
    if (busy || status === user.accountStatus) return;
    const reason = status === 'ACTIVE' ? undefined : window.prompt('سبب تغيير حالة الحساب', '')?.trim();
    if (status !== 'ACTIVE' && !reason) return;
    setBusy(true); setError(''); setOk('');
    try {
      await api.setUserStatus(user.id, status, reason);
      await load();
      setOk('تم تحديث حالة المستخدم');
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }

  async function resetPassword(user: UserSummary) {
    if (busy) return;
    const password = window.prompt(`كلمة المرور الجديدة للمستخدم ${user.displayName}`, '') || '';
    if (!password) return;
    if ([...password].length !== 4) { setError('كلمة المرور يجب أن تتكون من 4 خانات بالضبط'); return; }
    setBusy(true); setError(''); setOk('');
    try {
      await api.resetUserPassword(user.id, password);
      await load();
      setOk('تم تعيين كلمة المرور وإلغاء الجلسات السابقة للمستخدم');
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }

  async function changeRole(user: UserSummary, roleType: RoleType) {
    if (busy || roleType === user.roleType) return;
    setBusy(true); setError(''); setOk('');
    try {
      await api.updateUser(user.id, user.username, user.displayName, roleType);
      await load();
      setOk('تم تحديث الدور الوظيفي');
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }

  async function toggleDeputyRestore() {
    if (busy || currentUser.roleType !== 'general_manager') return;
    setBusy(true); setError(''); setOk('');
    try {
      await api.setDeputyRestorePermission(!deputyRestore);
      setDeputyRestore(!deputyRestore);
      setOk('تم تحديث صلاحية استعادة النسخ لنائب المدير');
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }

  return (
    <section className="page usersPage" dir="rtl" aria-busy={busy}>
      <div className="pageTitle"><div><h1>المستخدمون والصلاحيات</h1><p>إدارة حسابات الموظفين ودورة حياتهم وصلاحياتهم التشغيلية.</p></div></div>
      {error && <div className="notice errorText" role="alert">{error}</div>}
      {ok && <div className="notice successText" role="status">{ok}</div>}
      <div className="settingsGrid">
        <form className="card" onSubmit={create}>
          <div className="cardHeading"><UserPlus aria-hidden="true" /><div><h2>إضافة مستخدم</h2><p>الرمز الوظيفي Uxx يُنشأ تلقائيًا ولا يعاد استخدامه.</p></div></div>
          <div className="formGrid">
            <label>اسم الموظف<span>*</span><input required disabled={busy} value={form.displayName} onChange={(e) => setForm({ ...form, displayName: e.target.value })} /></label>
            <label>اسم المستخدم<span>*</span><input required disabled={busy} dir="ltr" value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} /></label>
            <label>كلمة المرور<span>*</span><input required minLength={4} maxLength={4} disabled={busy} dir="ltr" type="password" value={form.password} onChange={(e) => setForm({ ...form, password: e.target.value })} /></label>
            <label>الدور<select disabled={busy} value={form.roleType} onChange={(e) => setForm({ ...form, roleType: e.target.value as RoleType })}>{(Object.keys(roleLabels) as RoleType[]).map((r) => <option key={r} value={r}>{roleLabels[r]}</option>)}</select></label>
          </div>
          <button className="primary" type="submit" disabled={busy}><UserPlus aria-hidden="true" />إضافة المستخدم</button>
        </form>
        {currentUser.roleType === 'general_manager' && <div className="card">
          <div className="cardHeading"><Shield aria-hidden="true" /><div><h2>صلاحية الاستعادة الحساسة</h2><p>الاستعادة للمدير العام فقط، ويمكن منحها لنائب المدير صراحة.</p></div></div>
          <button type="button" disabled={busy} className={deputyRestore ? 'primary' : ''} onClick={() => void toggleDeputyRestore()}>{deputyRestore ? 'إلغاء استعادة النسخ لنائب المدير' : 'منح استعادة النسخ لنائب المدير'}</button>
        </div>}
      </div>
      <div className="tableCard">
        <div className="cardHead"><div><h2><UsersRound aria-hidden="true" /> جميع المستخدمين</h2><p>{rows.length} حسابًا</p></div><button type="button" disabled={busy} onClick={() => void load()}><RefreshCcw aria-hidden="true" />تحديث</button></div>
        <table><thead><tr><th>الرمز</th><th>الموظف</th><th>المستخدم</th><th>الدور</th><th>الحالة</th><th>إجراءات</th></tr></thead><tbody>{rows.map((user) => <tr key={user.id}>
          <td><bdi>{user.employeeCode}</bdi></td><td>{user.displayName}{user.id === currentUser.id ? ' • أنت' : ''}</td><td dir="ltr">{user.username}</td>
          <td><select disabled={busy || user.id === currentUser.id} value={user.roleType} onChange={(e) => void changeRole(user, e.target.value as RoleType)}>{(Object.keys(roleLabels) as RoleType[]).map((r) => <option key={r} value={r}>{roleLabels[r]}</option>)}</select></td>
          <td><select disabled={busy || user.id === currentUser.id} value={user.accountStatus} onChange={(e) => void changeStatus(user, e.target.value as AccountStatus)}>{(Object.keys(statusLabels) as AccountStatus[]).map((s) => <option key={s} value={s}>{statusLabels[s]}</option>)}</select></td>
          <td><div className="rowActions"><button type="button" disabled={busy} onClick={() => onLifecycle(user.employeeCode)}><UsersRound aria-hidden="true" />دورة الحياة</button><button type="button" disabled={busy} onClick={() => void resetPassword(user)}><KeyRound aria-hidden="true" />كلمة المرور</button></div></td>
        </tr>)}</tbody></table>
      </div>
    </section>
  );
}
