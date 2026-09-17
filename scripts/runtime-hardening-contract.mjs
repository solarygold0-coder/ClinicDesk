import { readFileSync } from 'node:fs';
const read = (p) => readFileSync(p, 'utf8');
const runtime = read('src-tauri/src/lib.rs');
const api = read('src/api.ts');
const auth = read('src-tauri/src/auth.rs');
const authCommands = read('src-tauri/src/auth_commands.rs');
const authUi = read('src/AuthGate.tsx');
const usersUi = read('src/UsersPage.tsx');
const attachments = read('src/PatientAttachments.tsx');
const checks = [
  ['dialog plugin is initialized in desktop runtime', runtime.includes('.plugin(tauri_plugin_dialog::init())')],
  ['opener plugin is initialized in desktop runtime', runtime.includes('.plugin(tauri_plugin_opener::init())')],
  ['dedicated change-password command is registered', runtime.includes('auth_commands::auth_change_password') && api.includes("'auth_change_password' in") === false && api.includes("invoke<UserSummary>('auth_change_password'")],
  ['password backend minimum is four characters', auth.includes('password.chars().count() < 4') && authCommands.includes('temporary_password.chars().count() < 4') && authCommands.includes('new_password.chars().count() < 4')],
  ['password UI minimum is four characters', authUi.includes('length < 4') && usersUi.includes('length < 4') && !authUi.includes('10 أحرف') && !usersUi.includes('10 أحرف')],
  ['attachment open command is registered', runtime.includes('fn attachment_open(') && runtime.includes('attachment_open,')],
  ['attachment open verifies integrity before Windows association', runtime.includes('backup::sha256_file(&path)?') && runtime.includes('app.opener()') && runtime.includes('open_path(path_string, None::<&str>)')],
  ['attachment frontend delegates opening to authenticated backend', api.includes("openAttachment:(id:number)=>authed<void>('attachment_open',{id})") && attachments.includes('await api.openAttachment(item.id)') && !attachments.includes('appDataDir') && !attachments.includes('openPath')],
];
const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
if (failed.length) process.exit(1);
console.log(`Desktop runtime hardening contract passed: ${checks.length}/${checks.length}`);
