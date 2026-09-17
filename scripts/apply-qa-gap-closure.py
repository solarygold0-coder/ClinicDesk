from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str):
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    p.write_text(text.replace(old, new, 1))


def insert_before_last_brace(path: str, snippet: str):
    p = Path(path)
    text = p.read_text()
    pos = text.rfind('\n}')
    if pos < 0:
        raise SystemExit(f"{path}: final brace not found")
    p.write_text(text[:pos] + '\n' + snippet.rstrip() + text[pos:])

# 1) The approved product capacity is 10,000 active patient files.
replace_once(
    'src-tauri/src/patients.rs',
    'use serde::{Deserialize, Serialize};\n',
    'use serde::{Deserialize, Serialize};\n\npub const MAX_ACTIVE_PATIENTS: i64 = 10_000;\n',
    'patient capacity constant',
)
replace_once(
    'src-tauri/src/patients.rs',
    'pub fn create(conn: &mut Connection, input: PatientInput) -> Result<Patient, String> {\n    let (name, nid, phone) = validate(&input)?;\n',
    '''pub fn create(conn: &mut Connection, input: PatientInput) -> Result<Patient, String> {\n    let (name, nid, phone) = validate(&input)?;\n    let active_total: i64 = conn\n        .query_row(\n            "SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL",\n            [],\n            |r| r.get(0),\n        )\n        .map_err(|e| e.to_string())?;\n    if active_total >= MAX_ACTIVE_PATIENTS {\n        return Err("تم بلوغ الحد الأقصى لملفات المرضى النشطة (10,000 مريض)".into());\n    }\n''',
    'patient capacity enforcement',
)
insert_before_last_brace(
    'src-tauri/src/patients.rs',
    r'''    #[test]
    fn ten_thousand_active_patient_limit_is_enforced_and_archive_frees_capacity() {
        let mut c = db();
        {
            let tx = c.transaction().unwrap();
            for n in 1..=MAX_ACTIVE_PATIENTS {
                tx.execute(
                    "INSERT INTO patients(file_no,full_name) VALUES(?1,'مريض سعة')",
                    [n],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }
        let err = create(&mut c, input("مريض زائد", "1234567890")).unwrap_err();
        assert!(err.contains("10,000"));

        c.execute(
            "UPDATE patients SET deleted_at=CURRENT_TIMESTAMP WHERE file_no=1",
            [],
        )
        .unwrap();
        c.execute(
            "UPDATE app_meta SET value='10001' WHERE key='next_patient_file_no'",
            [],
        )
        .unwrap();
        let accepted = create(&mut c, input("مريض بديل", "1234567890")).unwrap();
        assert_eq!(accepted.file_no, 10001);
    }
''',
)

# 2) A patient cannot hold overlapping active appointments even with different doctors.
replace_once(
    'src-tauri/src/appointments.rs',
    '''fn ensure_no_conflict(\n    tx: &Transaction,\n    i: &AppointmentInput,\n    starts: &str,\n    ends: &str,\n    exclude_id: Option<i64>,\n) -> Result<(), String> {\n    let sql = format!(\n        "SELECT id FROM appointments WHERE status IN {ACTIVE} AND starts_at<?1 AND ends_at>?2 AND ((?3 IS NOT NULL AND doctor_id=?3) OR (?3 IS NULL AND ?4 IS NOT NULL AND doctor_id IS NULL AND clinic_id=?4)) AND (?5 IS NULL OR id<>?5) LIMIT 1"\n    );\n    let conflict: Option<i64> = tx\n        .query_row(\n            &sql,\n            params![ends, starts, i.doctor_id, i.clinic_id, exclude_id],\n            |r| r.get(0),\n        )\n''',
    '''fn ensure_no_conflict(\n    tx: &Transaction,\n    i: &AppointmentInput,\n    patient_id: i64,\n    starts: &str,\n    ends: &str,\n    exclude_id: Option<i64>,\n) -> Result<(), String> {\n    let sql = format!(\n        "SELECT id FROM appointments WHERE status IN {ACTIVE} AND starts_at<?1 AND ends_at>?2 AND (((?3 IS NOT NULL AND doctor_id=?3) OR (?3 IS NULL AND ?4 IS NOT NULL AND doctor_id IS NULL AND clinic_id=?4)) OR patient_id=?5) AND (?6 IS NULL OR id<>?6) LIMIT 1"\n    );\n    let conflict: Option<i64> = tx\n        .query_row(\n            &sql,\n            params![ends, starts, i.doctor_id, i.clinic_id, patient_id, exclude_id],\n            |r| r.get(0),\n        )\n''',
    'appointment patient conflict query',
)
text = Path('src-tauri/src/appointments.rs').read_text()
text = text.replace('ensure_no_conflict(&tx, &i, &starts, &ends, None)?;', 'ensure_no_conflict(&tx, &i, pid, &starts, &ends, None)?;')
text = text.replace('ensure_no_conflict(&tx, &i, &starts, &ends, Some(id))?;', 'ensure_no_conflict(&tx, &i, pid, &starts, &ends, Some(id))?;')
if 'ensure_no_conflict(&tx, &i, &starts' in text:
    raise SystemExit('appointment conflict call was not fully updated')
Path('src-tauri/src/appointments.rs').write_text(text)
replace_once(
    'src-tauri/src/appointments.rs',
    '''        c.execute(\n            "INSERT INTO doctors(clinic_id,name)VALUES(1,'طبيب ثان')",\n            [],\n        )\n        .unwrap();\n        create(&mut c, i("2026-09-20T10:00")).unwrap();\n        let mut other = i("2026-09-20T10:15");\n        other.doctor_id = Some(2);\n        assert!(create(&mut c, other).is_ok())\n''',
    '''        c.execute(\n            "INSERT INTO doctors(clinic_id,name)VALUES(1,'طبيب ثان')",\n            [],\n        )\n        .unwrap();\n        c.execute(\n            "INSERT INTO patients(file_no,full_name)VALUES(2,'مريض ثان')",\n            [],\n        )\n        .unwrap();\n        create(&mut c, i("2026-09-20T10:00")).unwrap();\n        let mut other = i("2026-09-20T10:15");\n        other.patient_file_no = 2;\n        other.doctor_id = Some(2);\n        assert!(create(&mut c, other).is_ok())\n''',
    'different doctors overlap test uses different patient',
)
insert_before_last_brace(
    'src-tauri/src/appointments.rs',
    r'''    #[test]
    fn same_patient_cannot_overlap_across_different_doctors() {
        let mut c = db();
        c.execute(
            "INSERT INTO doctors(clinic_id,name)VALUES(1,'طبيب ثان')",
            [],
        )
        .unwrap();
        create(&mut c, i("2026-09-20T10:00")).unwrap();
        let mut other = i("2026-09-20T10:15");
        other.doctor_id = Some(2);
        assert!(create(&mut c, other).is_err());
    }

    #[test]
    fn appointment_lower_date_boundary_is_enforced() {
        assert!(normalized(&i("1949-12-29T10:00")).is_err());
    }
''',
)

# 3) Exact-four policy: update stale tests and keep legacy long hashes login-compatible once.
p = Path('src-tauri/src/auth.rs')
s = p.read_text().replace('StrongPassword1', 'A1!z').replace('TemporaryPassword2', 'B2@y')
p.write_text(s)
insert_before_last_brace(
    'src-tauri/src/auth.rs',
    r'''    #[test]
    fn legacy_long_hash_can_login_for_forced_password_change() {
        let db = db();
        let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes()).unwrap();
        let legacy_hash = Argon2::default()
            .hash_password(b"LegacyPassword1", &salt)
            .unwrap()
            .to_string();
        db.execute(
            "INSERT INTO users(username,display_name,password_hash,employee_code,role_type,must_change_password) VALUES('legacy','موظف قديم',?1,'U01','ordinary_employee',1)",
            [legacy_hash],
        )
        .unwrap();
        let session = authenticate(&db, "legacy".into(), "LegacyPassword1".into()).unwrap();
        assert!(session.user.must_change_password);
        assert!(hash_password("123").is_err());
        assert!(hash_password("12345").is_err());
        assert!(hash_password("1234").is_ok());
    }
''',
)
replace_once(
    'src-tauri/src/auth_commands.rs',
    '''    if temporary_password.chars().count() < 4 {\n        return Err("كلمة المرور المؤقتة يجب ألا تقل عن 4 خانات".into());\n    }\n''',
    '''    if temporary_password.chars().count() != 4 {\n        return Err("كلمة المرور المؤقتة يجب أن تتكون من 4 خانات بالضبط".into());\n    }\n''',
    'temporary password command validation',
)
replace_once(
    'src-tauri/src/auth_commands.rs',
    '''    if new_password.chars().count() < 4 {\n        return Err("كلمة المرور الجديدة يجب ألا تقل عن 4 خانات".into());\n    }\n''',
    '''    if new_password.chars().count() != 4 {\n        return Err("كلمة المرور الجديدة يجب أن تتكون من 4 خانات بالضبط".into());\n    }\n''',
    'self password command validation',
)

# Migration 18 must change actual active accounts, not only register metadata.
p = Path('src-tauri/src/migration_tests.rs')
s = p.read_text()
s += r'''

#[test]
fn exact_four_password_migration_forces_only_active_accounts_to_change() {
    let db = fresh();
    migrate_db(&db).unwrap();
    db.execute(
        "INSERT INTO users(username,display_name,password_hash,employee_code,role_type,account_status,is_active,must_change_password) VALUES('active_old','نشط قديم','legacy','U61','ordinary_employee','ACTIVE',1,0)",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO users(username,display_name,password_hash,employee_code,role_type,account_status,is_active,must_change_password) VALUES('retired_old','متقاعد قديم','legacy','U62','ordinary_employee','RETIRED',0,0)",
        [],
    )
    .unwrap();
    db.execute_batch(include_str!("../migrations/018_exact_four_password_policy.sql"))
        .unwrap();
    let active: i64 = db
        .query_row(
            "SELECT must_change_password FROM users WHERE username='active_old'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let retired: i64 = db
        .query_row(
            "SELECT must_change_password FROM users WHERE username='retired_old'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(active, 1);
    assert_eq!(retired, 0);
}
'''
p.write_text(s)

# 4) Permanent CI must build/test the real server and explicitly cover authentication policy.
p = Path('.github/workflows/build.yml')
s = p.read_text()
anchor = '''  gate:\n    name: All checks passed\n'''
server_jobs = '''  authentication-policy-tests:\n    name: Authentication policy tests\n    runs-on: windows-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: dtolnay/rust-toolchain@stable\n      - run: cargo test --manifest-path src-tauri/Cargo.toml auth::tests -- --nocapture\n      - run: cargo test --manifest-path src-tauri/Cargo.toml --test auth_gate -- --nocapture\n\n  server-tests:\n    name: ClinicDesk Server build / tests\n    runs-on: windows-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: dtolnay/rust-toolchain@stable\n      - run: cargo check -p clinicdesk-server\n      - run: cargo test -p clinicdesk-server\n\n'''
if anchor not in s:
    raise SystemExit('build gate anchor not found')
s = s.replace(anchor, server_jobs + anchor, 1)
s = s.replace('      - security-tests\n', '      - security-tests\n      - authentication-policy-tests\n      - server-tests\n', 1)
s = s.replace('            ${{ needs.security-tests.result }}\n', '            ${{ needs.security-tests.result }}\n            ${{ needs.authentication-policy-tests.result }}\n            ${{ needs.server-tests.result }}\n', 1)
p.write_text(s)

# Workspace formatting repair must stage server files too.
replace_once(
    '.github/workflows/format-repair-current.yml',
    '      - run: cargo fmt --manifest-path src-tauri/Cargo.toml --all\n',
    '      - run: cargo fmt --all\n',
    'workspace fmt command',
)
replace_once(
    '.github/workflows/format-repair-current.yml',
    '          git add src-tauri/src src-tauri/tests\n',
    '          git add src-tauri/src src-tauri/tests server/src\n',
    'workspace fmt staging',
)

# Persistent coverage inventory: no requirement may be called complete without its evidence level.
Path('docs/TEST_COVERAGE_MATRIX.md').write_text(r'''# ClinicDesk requirement test coverage matrix

Status meanings:
- **AUTOMATED**: behavior is exercised by executable tests or a CI contract.
- **COVERED IN THIS SLICE**: an approved requirement that previously lacked a complete test now has one.
- **OPEN**: not safe to call complete yet.

## Automated / covered

| Requirement | Evidence |
|---|---|
| Maximum 62 permanent user accounts; U01-U62 never reused | `users::tests::refuses_more_than_sixty_two_permanent_accounts` and permanent-code tests |
| Maximum 20 active attachments per patient; archive frees a slot; restore respects limit | `attachments::tests` |
| Dangerous attachment extensions and path traversal blocked | `attachments::tests` |
| Friday and Saturday cannot be booked | `appointments::tests` |
| Doctor/clinic time conflicts, adjacent slots, hours, breaks and manual closures | `appointments::tests` |
| Patient same-time overlap across different doctors | **COVERED IN THIS SLICE**: `same_patient_cannot_overlap_across_different_doctors` |
| Patient capacity is 10,000 active files | **COVERED IN THIS SLICE**: `ten_thousand_active_patient_limit_is_enforced_and_archive_frees_capacity` |
| Patient file numbers are sequential and not reused after archive | `patients::tests::numbering_is_not_reused` |
| Duplicate national ID rejected | `patients::tests::duplicate_nid_rejected` |
| Patient inactivity review does not delete records | `patients::tests::inactive_patients_are_detected_without_deletion` |
| Gregorian year floor/ceiling enforced in backend | patient / appointment / scheduling tests; lower appointment edge added in this slice |
| Provider unavailable batch operations are atomic | `provider_unavailability::tests::batch_resolution_is_atomic` |
| Backup / restore integrity and rollback gate | `backup_restore_gate` |
| Four-character new-password policy | `auth_gate`, `auth::tests`; stale long-password tests fixed in this slice |
| Legacy long password hash can still authenticate once for forced migration | **COVERED IN THIS SLICE**: `legacy_long_hash_can_login_for_forced_password_change` |
| Migration 18 forces active old accounts to change password, not retired accounts | **COVERED IN THIS SLICE**: migration test added in this slice |
| ClinicDesk Server compiles/tests on every permanent Build | **COVERED IN THIS SLICE**: `server-tests` job is part of the final gate |
| RTL/UI, visible approved features, two-day reminder and date display source contracts | permanent UI contract workflow |

## OPEN — do not call these complete yet

1. **Installed Windows print/Notepad end-to-end behavior.** Source protection exists, but there is no installed-app automation proving that closing an external viewer/print dialog cannot terminate ClinicDesk.
2. **10,000-patient performance benchmark.** Capacity is now enforced/tested, but realistic search/open/save latency with 10,000 fully populated records and attachments is not benchmarked.
3. **10 concurrent LAN clients.** The server skeleton builds, but patient/appointment/attachment/user APIs have not all moved to the central server, so a real 10-client concurrency test is not yet possible.
4. **PostgreSQL integration test.** `clinicdesk-server` compiles/tests, but CI does not yet start a real PostgreSQL service and execute its migrations and HTTP health/auth flows against it.
5. **Digital signing / SmartScreen reputation.** No valid code-signing certificate is installed in CI; unsigned package tests cannot prove trusted-publisher behavior.
6. **Audit atomicity for every mutation.** Appointment create/update and attachment lifecycle are transactional, but patient update/archive, visit tracking, directory mutations and scheduling mutations still require explicit transaction/rollback tests.
7. **Saudi advisory behavior as an end-to-end booking test.** Source contracts cover advisory wiring, but Ramadan/Hajj/holiday warnings still need rendered interaction tests plus administrator override scenarios.
8. **Rendered UI E2E.** Current UI gates are mostly source/contract checks plus compile; there is no full click-through desktop E2E suite for every role and screen.

No Release Candidate label should be applied while OPEN items that affect the requested release scope remain unresolved.
''')
