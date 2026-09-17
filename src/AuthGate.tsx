import { FormEvent, useEffect, useState } from 'react';
import { KeyRound, LockKeyhole, ShieldCheck, UserRoundPlus } from 'lucide-react';
import { api, AuthSession, setActorToken } from './api';

type Mode = 'checking' | 'setup' | 'login' | 'changePassword';

export function AuthGate({ onAuthenticated }: { onAuthenticated: (session: AuthSession) => void }) {
  const [mode, setMode] = useState<Mode>('checking');
  const [username, setUsername] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [pendingSession, setPendingSession] = useState<AuthSession | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const finish = (session: AuthSession) => {
    setActorToken(session.token);
    sessionStorage.setItem('clinicdesk.session', session.token);
    onAuthenticated(session);
  };

  const acceptOrRequirePasswordChange = (session: AuthSession, currentPassword = '') => {
    if (session.user.mustChangePassword) {
      setPendingSession(session);
      setPassword(currentPassword);
      setNewPassword('');
      setConfirmPassword('');
      setMode('changePassword');
      return;
    }
    finish(session);
  };

  useEffect(() => {
    const restore = async () => {
      const saved = sessionStorage.getItem('clinicdesk.session');
      if (saved) {
        try {
          const user = await api.validateSession(saved);
          const restored = { token: saved, expiresAt: '', user };
          if (user.mustChangePassword) {
            sessionStorage.removeItem('clinicdesk.session');
            setActorToken(null);
            setMode('login');
            setError('يجب تسجيل الدخول بكلمة المرور المؤقتة ثم تعيين كلمة مرور جديدة.');
            return;
          }
          setActorToken(saved);
          finish(restored);
          return;
        } catch {
          sessionStorage.removeItem('clinicdesk.session');
          setActorToken(null);
        }
      }
      try {
        const users = await api.users();
        setMode(users.length === 0 ? 'setup' : 'login');
      } catch (e) {
        const message = String(e);
        if (message.includes('تسجيل الدخول') || message.includes('الجلسة')) setMode('login');
        else {
          setError(message);
          setMode('login');
        }
      }
    };
    void restore();
  }, [onAuthenticated]);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError('');
    try {
      if (mode === 'changePassword') {
        if (!pendingSession) throw new Error('جلسة تغيير كلمة المرور غير متاحة؛ سجل الدخول مجددًا');
        if (newPassword.length < 4) throw new Error('كلمة المرور الجديدة يجب ألا تقل عن 4 خانات');
        if (newPassword !== confirmPassword) throw new Error('تأكيد كلمة المرور الجديدة غير مطابق');
        const user = await api.changePassword(pendingSession.token, password, newPassword);
        finish({ ...pendingSession, user });
        return;
      }
      if (mode === 'setup') {
        if (displayName.trim().length < 2) throw new Error('اسم المدير مطلوب');
        if (password.length < 4) throw new Error('كلمة المرور الأولى يجب ألا تقل عن 4 خانات');
        if (password !== confirmPassword) throw new Error('تأكيد كلمة المرور غير مطابق');
        await api.createUser(username.trim(), displayName.trim(), password, 'general_manager');
      }
      const session = await api.login(username.trim(), password);
      acceptOrRequirePasswordChange(session, password);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  if (mode === 'checking') {
    return <main className="authShell" dir="rtl"><div className="authCard"><ShieldCheck aria-hidden="true" /><h1>ClinicDesk</h1><p>جارٍ التحقق من إعدادات الأمان…</p></div></main>;
  }

  const changing = mode === 'changePassword';
  return (
    <main className="authShell" dir="rtl">
      <form className="authCard" onSubmit={submit} aria-busy={busy}>
        <div className="authMark">{changing ? <KeyRound aria-hidden="true" /> : mode === 'setup' ? <UserRoundPlus aria-hidden="true" /> : <LockKeyhole aria-hidden="true" />}</div>
        <h1>{changing ? 'تغيير كلمة المرور المؤقتة' : mode === 'setup' ? 'تهيئة المستخدم الأول' : 'تسجيل الدخول'}</h1>
        <p>{changing ? 'يجب تعيين كلمة مرور شخصية جديدة قبل الوصول إلى بيانات النظام.' : mode === 'setup' ? 'أنشئ حساب المدير العام. بعد ذلك يصبح تسجيل الدخول إلزاميًا لجميع العمليات.' : 'أدخل حساب الموظف للمتابعة إلى ClinicDesk.'}</p>
        {error && <div className="notice errorText" role="alert">{error}</div>}
        {changing ? <>
          <label>كلمة المرور المؤقتة<span>*</span><input required dir="ltr" type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="current-password" /></label>
          <label>كلمة المرور الجديدة<span>*</span><input autoFocus required dir="ltr" type="password" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} autoComplete="new-password" /></label>
          <label>تأكيد كلمة المرور الجديدة<span>*</span><input required dir="ltr" type="password" value={confirmPassword} onChange={(e) => setConfirmPassword(e.target.value)} autoComplete="new-password" /></label>
        </> : <>
          {mode === 'setup' && <label>اسم الموظف<span>*</span><input autoFocus required value={displayName} onChange={(e) => setDisplayName(e.target.value)} autoComplete="name" /></label>}
          <label>اسم المستخدم<span>*</span><input autoFocus={mode === 'login'} required dir="ltr" value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" /></label>
          <label>كلمة المرور<span>*</span><input required dir="ltr" type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete={mode === 'setup' ? 'new-password' : 'current-password'} /></label>
          {mode === 'setup' && <label>تأكيد كلمة المرور<span>*</span><input required dir="ltr" type="password" value={confirmPassword} onChange={(e) => setConfirmPassword(e.target.value)} autoComplete="new-password" /></label>}
        </>}
        <button className="primary" type="submit" disabled={busy}>{busy ? 'جارٍ التحقق…' : changing ? 'حفظ كلمة المرور والدخول' : mode === 'setup' ? 'إنشاء المدير وبدء النظام' : 'دخول'}</button>
        {mode === 'setup' && <small>الحساب الأول يُنشأ كمدير عام فقط، ولا يمكن إنشاء حساب أول بصلاحيات أقل.</small>}
      </form>
    </main>
  );
}
