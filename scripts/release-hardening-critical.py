from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text(encoding='utf-8')
    if old not in text:
        raise SystemExit(f'missing expected marker in {path}: {old[:100]!r}')
    if text.count(old) != 1:
        raise SystemExit(f'expected one marker in {path}, found {text.count(old)}')
    p.write_text(text.replace(old, new, 1), encoding='utf-8')

# Runtime: initialize dialog plugin and use Rust-side opener for attachments.
replace_once(
    'src-tauri/src/lib.rs',
    'use tauri::Manager;\n',
    'use tauri::Manager;\nuse tauri_plugin_opener::OpenerExt;\n',
)
replace_once(
    'src-tauri/src/lib.rs',
    '    tauri::Builder::default()\n        .plugin(tauri_plugin_opener::init())\n',
    '    tauri::Builder::default()\n        .plugin(tauri_plugin_dialog::init())\n        .plugin(tauri_plugin_opener::init())\n',
)
replace_once(
    'src-tauri/src/lib.rs',
    '            auth_commands::auth_validate_session,\n',
    '            auth_commands::auth_validate_session,\n            auth_commands::auth_change_password,\n',
)

attachment_import_marker = '''#[tauri::command]\nfn attachment_archive(\n'''
attachment_open = '''#[tauri::command]\nfn attachment_open(\n    app: tauri::AppHandle,\n    db: tauri::State<Db>,\n    actor_token: Option<String>,\n    id: i64,\n) -> Result<(), String> {\n    authorize_command(&db, actor_token.as_deref(), authorization::ATTACHMENT_READ)?;\n    let root = attachment_root(&app)?;\n    let path = with_db(&db, |c| {\n        let (stored_name, size_bytes, expected_sha): (String, i64, String) = c\n            .query_row(\n                "SELECT stored_name,size_bytes,sha256 FROM attachments WHERE id=?1 AND deleted_at IS NULL",\n                [id],\n                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),\n            )\n            .map_err(|_| "المرفق غير موجود أو مؤرشف".to_string())?;\n        if stored_name.is_empty()\n            || Path::new(&stored_name).file_name().and_then(|value| value.to_str())\n                != Some(stored_name.as_str())\n        {\n            return Err("اسم المرفق المخزن غير صالح".into());\n        }\n        let path = root.join(&stored_name);\n        let metadata = fs::metadata(&path).map_err(|_| "ملف المرفق مفقود من مجلد البرنامج".to_string())?;\n        if !metadata.is_file() || metadata.len() != size_bytes as u64 {\n            return Err("حجم ملف المرفق لا يطابق السجل؛ لن يتم فتحه".into());\n        }\n        if backup::sha256_file(&path)? != expected_sha.to_ascii_lowercase() {\n            return Err("فشل التحقق من بصمة المرفق؛ لن يتم فتحه".into());\n        }\n        Ok(path)\n    })?;\n    let path_string = path\n        .to_str()\n        .ok_or_else(|| "مسار المرفق غير صالح".to_string())?\n        .to_owned();\n    app.opener()\n        .open_path(path_string, None::<&str>)\n        .map_err(|e| format!("تعذر فتح المرفق بواسطة Windows: {e}"))\n}\n\n#[tauri::command]\nfn attachment_archive(\n'''
replace_once('src-tauri/src/lib.rs', attachment_import_marker, attachment_open)
replace_once(
    'src-tauri/src/lib.rs',
    '            attachment_import,\n            attachment_archive,\n',
    '            attachment_import,\n            attachment_open,\n            attachment_archive,\n',
)

# Backend password policy: minimum four characters everywhere.
replace_once(
    'src-tauri/src/auth.rs',
    'pub fn hash_password(password: &str) -> Result<String, String> {\n    let password = required(password, "كلمة المرور")?;\n',
    'pub fn hash_password(password: &str) -> Result<String, String> {\n    let password = required(password, "كلمة المرور")?;\n    if password.chars().count() < 4 {\n        return Err("كلمة المرور يجب ألا تقل عن 4 خانات".into());\n    }\n',
)
for old, new in [
    ('if temporary_password.chars().count() < 10 {', 'if temporary_password.chars().count() < 4 {'),
    ('كلمة المرور المؤقتة يجب ألا تقل عن 10 أحرف', 'كلمة المرور المؤقتة يجب ألا تقل عن 4 خانات'),
    ('if new_password.chars().count() < 10 {', 'if new_password.chars().count() < 4 {'),
    ('كلمة المرور الجديدة يجب ألا تقل عن 10 أحرف', 'كلمة المرور الجديدة يجب ألا تقل عن 4 خانات'),
]:
    replace_once('src-tauri/src/auth_commands.rs', old, new)

# Frontend password policy and dedicated command.
replace_once(
    'src/api.ts',
    "changePassword:(token:string,currentPassword:string,newPassword:string)=>invoke<UserSummary>('auth_validate_session',{token,currentPassword,newPassword}),",
    "changePassword:(token:string,currentPassword:string,newPassword:string)=>invoke<UserSummary>('auth_change_password',{actorToken:token,currentPassword,newPassword}),",
)
replace_once(
    'src/api.ts',
    "importAttachment:(patientId:number,sourcePath:string,displayName?:string,category?:string)=>authed<number>('attachment_import',{patientId,sourcePath,displayName,category}),archiveAttachment:",
    "importAttachment:(patientId:number,sourcePath:string,displayName?:string,category?:string)=>authed<number>('attachment_import',{patientId,sourcePath,displayName,category}),openAttachment:(id:number)=>authed<void>('attachment_open',{id}),archiveAttachment:",
)
for path in ['src/AuthGate.tsx', 'src/UsersPage.tsx']:
    p = Path(path)
    text = p.read_text(encoding='utf-8')
    text = text.replace('length < 10', 'length < 4')
    text = text.replace('10 أحرف', '4 خانات')
    p.write_text(text, encoding='utf-8')

# Frontend no longer reconstructs app-data paths itself.
p = Path('src/PatientAttachments.tsx')
text = p.read_text(encoding='utf-8')
text = text.replace("import { openPath } from '@tauri-apps/plugin-opener';\n", '')
text = text.replace("import { appDataDir, join } from '@tauri-apps/api/path';\n", '')
old_open = '''    try {\n      const root = await appDataDir();\n      await openPath(await join(root, 'attachments', item.storedName));\n    } catch (e) {\n'''
new_open = '''    try {\n      await api.openAttachment(item.id);\n    } catch (e) {\n'''
if old_open not in text:
    raise SystemExit('attachment open-path frontend marker missing')
text = text.replace(old_open, new_open, 1)
p.write_text(text, encoding='utf-8')

# Permanent regression contract for real desktop runtime wiring.
Path('scripts/runtime-hardening-contract.mjs').write_text(r'''import { readFileSync } from 'node:fs';
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
''', encoding='utf-8')
