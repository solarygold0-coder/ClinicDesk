import { FormEvent, useEffect, useState } from 'react';
import { LockKeyhole, ShieldCheck, UserRoundPlus } from 'lucide-react';
import { api, AuthSession, setActorToken } from './api';

type Mode = 'checking' | 'setup' | 'login';

export function AuthGate({ onAuthenticated }: { onAuthenticated: (session: AuthSession) => void }) {
  const [mode, setMode] = useState<Mode>('checking');
  const [username, setUsername] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    const restore = async () => {
      const saved = sessionStorage.getItem('clinicdesk.session');
      if (saved) {
        try {
          const user = await api.validateSession(saved);
          setActorToken(saved);
          onAuthenticated({ token: saved, expiresAt: '', user });
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

  const finish = (session: AuthSession) => {
    setActorToken(session.token);
    sessionStorage.setItem('clinicdesk.session', session.token);
    onAuthenticated(session);
  };

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError('');
    try {
      if (mode === 'setup') {
        if (displayName.trim().length < 2) throw new Error('اسم المدير مطلوب');
        if (password.length < 10) throw new Error('كلمة المرور الأولى يجب ألا تقل عن 10 أحرف');
        if (password !== confirmPassword) throw new Error('تأكيد كلمة المرور غير مطابق');
        await api.createUser(username.trim(), displayName.trim(), password, 'general_manager');
      }
      const session = await api.login(username.trim(), password);
      finish(session);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  if (mode === 'checking') {
    return <main className="authShell" dir="rtl"><div className="authCard"><ShieldCheck aria-hidden="true" /><h1>ClinicDesk</h1><p>جارٍ التحقق من إعدادات الأمان…</p></div></main>;
  }

  return (
    <main className="authShell" dir="rtl">
      <form className="authCard" onSubmit={submit} aria-busy={busy}>
        <div className="authMark">{mode === 'setup' ? <UserRoundPlus aria-hidden="true" /> : <LockKeyhole aria-hidden="true" />}</div>
        <h1>{mode === 'setup' ? 'تهيئة المستخدم الأول' : 'تسجيل الدخول'}</h1>
        <p>{mode === 'setup' ? 'أنشئ حساب المدير العام. بعد ذلك يصبح تسجيل الدخول إلزاميًا لجميع العمليات.' : 'أدخل حساب الموظف للمتابعة إلى ClinicDesk.'}</p>
        {error && <div className="notice errorText" role="alert">{error}</div>}
        {mode === 'setup' && <label>اسم الموظف<span>*</span><input autoFocus required value={displayName} onChange={(e) => setDisplayName(e.target.value)} autoComplete="name" /></label>}
        <label>اسم المستخدم<span>*</span><input autoFocus={mode === 'login'} required dir="ltr" value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" /></label>
        <label>كلمة المرور<span>*</span><input required dir="ltr" type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete={mode === 'setup' ? 'new-password' : 'current-password'} /></label>
        {mode === 'setup' && <label>تأكيد كلمة المرور<span>*</span><input required dir="ltr" type="password" value={confirmPassword} onChange={(e) => setConfirmPassword(e.target.value)} autoComplete="new-password" /></label>}
        <button className="primary" type="submit" disabled={busy}>{busy ? 'جارٍ التحقق…' : mode === 'setup' ? 'إنشاء المدير وبدء النظام' : 'دخول'}</button>
        {mode === 'setup' && <small>الحساب الأول يُنشأ كمدير عام فقط، ولا يمكن إنشاء حساب أول بصلاحيات أقل.</small>}
      </form>
    </main>
  );
}
