import { Database, ShieldCheck, Stethoscope } from 'lucide-react';

export function WelcomeScreen({ onContinue }: { onContinue: () => void }) {
  return (
    <main className="welcomeScreen" dir="rtl">
      <section className="welcomeCard" aria-labelledby="clinicdesk-welcome-title">
        <div className="welcomeLogo"><Stethoscope aria-hidden="true" /></div>
        <p className="welcomeEyebrow">ClinicDesk</p>
        <h1 id="clinicdesk-welcome-title">مرحبًا بك في ClinicDesk</h1>
        <p className="welcomeLead">نظام إدارة ملفات المرضى والمواعيد والتشغيل اليومي للعيادة.</p>
        <div className="welcomeFacts">
          <span><ShieldCheck aria-hidden="true" />النسخة المجانية</span>
          <span><Database aria-hidden="true" />الإصدار 5.1.0</span>
        </div>
        <button type="button" className="primary welcomeContinue" onClick={onContinue}>بدء العمل</button>
      </section>
    </main>
  );
}
