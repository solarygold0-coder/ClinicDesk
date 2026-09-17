from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str):
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    p.write_text(text.replace(old, new, 1))

# Product version is 5.1.0 everywhere, not just in visible labels.
replace_once('package.json', '"version":"5.0.0"', '"version":"5.1.0"', 'npm version')
replace_once('src-tauri/Cargo.toml', 'version="5.0.0"', 'version="5.1.0"', 'desktop cargo version')
replace_once('src-tauri/tauri.conf.json', '"version":"5.0.0"', '"version":"5.1.0"', 'tauri version')

# Welcome screen appears once per process launch before authentication.
Path('src/WelcomeScreen.tsx').write_text('''import { Database, ShieldCheck, Stethoscope } from 'lucide-react';\n\nexport function WelcomeScreen({ onContinue }: { onContinue: () => void }) {\n  return (\n    <main className="welcomeScreen" dir="rtl">\n      <section className="welcomeCard" aria-labelledby="clinicdesk-welcome-title">\n        <div className="welcomeLogo"><Stethoscope aria-hidden="true" /></div>\n        <p className="welcomeEyebrow">ClinicDesk</p>\n        <h1 id="clinicdesk-welcome-title">مرحبًا بك في ClinicDesk</h1>\n        <p className="welcomeLead">نظام إدارة ملفات المرضى والمواعيد والتشغيل اليومي للعيادة.</p>\n        <div className="welcomeFacts">\n          <span><ShieldCheck aria-hidden="true" />النسخة المجانية</span>\n          <span><Database aria-hidden="true" />الإصدار 5.1.0</span>\n        </div>\n        <button type="button" className="primary welcomeContinue" onClick={onContinue}>بدء العمل</button>\n      </section>\n    </main>\n  );\n}\n''')
Path('src/welcome.css').write_text('''.welcomeScreen{min-height:100vh;display:grid;place-items:center;padding:24px;background:radial-gradient(circle at top,#eef8fb 0,#f7fafc 45%,#edf2f5 100%)}.welcomeCard{width:min(560px,100%);background:#fff;border:1px solid #dfe8ed;border-radius:22px;padding:34px;box-shadow:0 18px 50px #19394b18;text-align:center}.welcomeLogo{width:72px;height:72px;margin:0 auto 14px;border-radius:20px;display:grid;place-items:center;background:#e5f4f8;color:#17667c}.welcomeLogo svg{width:38px;height:38px}.welcomeEyebrow{margin:0;color:#6a7a85;font-weight:800;letter-spacing:.08em}.welcomeCard h1{font-size:30px;margin:6px 0 10px}.welcomeLead{color:#64737e;margin:0 auto 22px;max-width:420px;line-height:1.8}.welcomeFacts{display:flex;justify-content:center;flex-wrap:wrap;gap:10px;margin-bottom:24px}.welcomeFacts span{display:flex;align-items:center;gap:7px;border:1px solid #dfe8ed;background:#f8fbfc;border-radius:999px;padding:8px 12px;color:#51616c}.welcomeFacts svg{width:17px}.welcomeContinue{min-width:180px;min-height:44px}@media(max-width:600px){.welcomeCard{padding:26px 18px}.welcomeCard h1{font-size:25px}}\n''')
replace_once('src/main.tsx', "import './appointment-status.css';\n", "import './appointment-status.css';\nimport './welcome.css';\n", 'welcome stylesheet')
replace_once('src/App.tsx', "import { SettingsHub } from './SettingsHub';\n", "import { SettingsHub } from './SettingsHub';\nimport { WelcomeScreen } from './WelcomeScreen';\n", 'welcome import')
replace_once('src/App.tsx', "  const [session,setSession]=useState<AuthSession|null>(null);\n", "  const [session,setSession]=useState<AuthSession|null>(null);\n  const [welcomed,setWelcomed]=useState(false);\n", 'welcome state')
replace_once('src/App.tsx', "  if(!session)return <AuthGate onAuthenticated={setSession}/>;\n", "  if(!welcomed)return <WelcomeScreen onContinue={()=>setWelcomed(true)}/>;\n  if(!session)return <AuthGate onAuthenticated={setSession}/>;\n", 'welcome render gate')

# Exact-four policy only applies to newly chosen passwords. Login keeps accepting legacy
# hashes so migrated users can authenticate once and then change their password.
replace_once('src-tauri/src/auth.rs', '    if password.chars().count() < 4 {\n        return Err("كلمة المرور يجب ألا تقل عن 4 خانات".into());\n    }', '    if password.chars().count() != 4 {\n        return Err("كلمة المرور يجب أن تتكون من 4 خانات بالضبط".into());\n    }', 'backend exact four policy')
replace_once('src/AuthGate.tsx', "        if (newPassword.length < 4) throw new Error('كلمة المرور الجديدة يجب ألا تقل عن 4 خانات');", "        if ([...newPassword].length !== 4) throw new Error('كلمة المرور الجديدة يجب أن تتكون من 4 خانات بالضبط');", 'change password exact four')
replace_once('src/AuthGate.tsx', "        if (password.length < 4) throw new Error('كلمة المرور الأولى يجب ألا تقل عن 4 خانات');", "        if ([...password].length !== 4) throw new Error('كلمة المرور الأولى يجب أن تتكون من 4 خانات بالضبط');", 'setup exact four')
replace_once('src/AuthGate.tsx', '<label>كلمة المرور الجديدة<span>*</span><input autoFocus required dir="ltr" type="password" value={newPassword}', '<label>كلمة المرور الجديدة<span>*</span><input autoFocus required minLength={4} maxLength={4} dir="ltr" type="password" value={newPassword}', 'new password input bounds')
replace_once('src/AuthGate.tsx', '<label>تأكيد كلمة المرور الجديدة<span>*</span><input required dir="ltr" type="password" value={confirmPassword}', '<label>تأكيد كلمة المرور الجديدة<span>*</span><input required minLength={4} maxLength={4} dir="ltr" type="password" value={confirmPassword}', 'confirm new password bounds')
replace_once('src/AuthGate.tsx', '<label>كلمة المرور<span>*</span><input required dir="ltr" type="password" value={password}', '<label>كلمة المرور<span>*</span><input required minLength={mode === \'setup\' ? 4 : undefined} maxLength={mode === \'setup\' ? 4 : undefined} dir="ltr" type="password" value={password}', 'setup password bounds')
replace_once('src/AuthGate.tsx', '<label>تأكيد كلمة المرور<span>*</span><input required dir="ltr" type="password" value={confirmPassword}', '<label>تأكيد كلمة المرور<span>*</span><input required minLength={4} maxLength={4} dir="ltr" type="password" value={confirmPassword}', 'setup confirm bounds')

replace_once('src/UsersPage.tsx', "      if (form.password.length < 4) throw new Error('كلمة المرور يجب ألا تقل عن 4 خانات');", "      if ([...form.password].length !== 4) throw new Error('كلمة المرور يجب أن تتكون من 4 خانات بالضبط');", 'create user exact four')
replace_once('src/UsersPage.tsx', "    if (password.length < 4) { setError('كلمة المرور يجب ألا تقل عن 4 خانات'); return; }", "    if ([...password].length !== 4) { setError('كلمة المرور يجب أن تتكون من 4 خانات بالضبط'); return; }", 'reset password exact four')
replace_once('src/UsersPage.tsx', '<label>كلمة المرور<span>*</span><input required disabled={busy} dir="ltr" type="password" value={form.password}', '<label>كلمة المرور<span>*</span><input required minLength={4} maxLength={4} disabled={busy} dir="ltr" type="password" value={form.password}', 'create password input bounds')

# Migration 18 forces existing active accounts through the already-implemented password-change flow.
Path('src-tauri/migrations/018_exact_four_password_policy.sql').write_text('''-- ClinicDesk 5.1 password policy: every newly chosen password is exactly four characters.\n-- Existing hashes remain valid for one login so users are not locked out; active accounts\n-- are forced through the existing must_change_password flow to adopt the new policy.\nUPDATE users\nSET must_change_password=1,\n    updated_at=CURRENT_TIMESTAMP\nWHERE account_status='ACTIVE';\n\nINSERT OR REPLACE INTO app_meta(key,value)\nVALUES('password_policy','exact-4-v1');\nUPDATE app_meta SET value='18' WHERE key='schema_version';\n''')
replace_once('src-tauri/src/lib.rs', 'const LATEST_SCHEMA_VERSION: i64 = 17;', 'const LATEST_SCHEMA_VERSION: i64 = 18;', 'latest schema')
replace_once('src-tauri/src/lib.rs', '''        (\n            17,\n            include_str!("../migrations/017_provider_unavailability.sql"),\n        ),\n''', '''        (\n            17,\n            include_str!("../migrations/017_provider_unavailability.sql"),\n        ),\n        (\n            18,\n            include_str!("../migrations/018_exact_four_password_policy.sql"),\n        ),\n''', 'migration 18 registration')
replace_once('src-tauri/src/migration_tests.rs', '    assert_eq!(schema_version(&db).unwrap(), 17);', '    assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);', 'provider migration version assertion')
Path('src-tauri/src/migration_tests.rs').write_text(Path('src-tauri/src/migration_tests.rs').read_text() + '''\n#[test]\nfn exact_four_password_policy_is_registered() {\n    let db = fresh();\n    migrate_db(&db).unwrap();\n    let policy: String = db\n        .query_row(\n            "SELECT value FROM app_meta WHERE key='password_policy'",\n            [],\n            |r| r.get(0),\n        )\n        .unwrap();\n    assert_eq!(policy, "exact-4-v1");\n    assert_eq!(schema_version(&db).unwrap(), LATEST_SCHEMA_VERSION);\n}\n''')

# Auth gate regression now proves 3/5 are rejected and 4 stays Argon2-hashed.
Path('src-tauri/tests/auth_gate.rs').write_text('''mod users {\n    pub use clinicdesk_lib::users::*;\n}\n\n#[allow(dead_code)]\n#[path = "../src/auth.rs"]\nmod auth;\n\n#[test]\nfn exact_four_password_policy_keeps_argon2_hashing() {\n    assert!(auth::hash_password("abc").is_err());\n    assert!(auth::hash_password("abcde").is_err());\n    assert!(auth::hash_password("A1!z").unwrap().starts_with("$argon2"));\n}\n''')

# Permanent visible contract protects welcome/free/version and exact-four UI.
p = Path('scripts/visible-feature-contract.mjs')
s = p.read_text()
s = s.replace("const files={", "const files={welcome:read('src/WelcomeScreen.tsx'),auth:read('src/AuthGate.tsx'),users:read('src/UsersPage.tsx'),")
s = s.replace("const checks=[", "const checks=[\n['welcome is visible before authentication',files.app.includes('<WelcomeScreen onContinue={()=>setWelcomed(true)}/>')&&files.welcome.includes('مرحبًا بك في ClinicDesk')],\n['welcome shows free edition and 5.1.0',files.welcome.includes('النسخة المجانية')&&files.welcome.includes('الإصدار 5.1.0')],\n['password UI enforces exactly four newly chosen characters',files.auth.includes('length !== 4')&&files.auth.includes('maxLength={4}')&&files.users.includes('length !== 4')&&files.users.includes('maxLength={4}')],")
p.write_text(s)
