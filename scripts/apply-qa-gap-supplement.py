from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text(encoding='utf-8')


def write(path: str, text: str):
    Path(path).write_text(text, encoding='utf-8')


def replace_once(path: str, old: str, new: str, label: str):
    text = read(path)
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    write(path, text.replace(old, new, 1))


def insert_before_last_brace(path: str, snippet: str):
    text = read(path)
    pos = text.rfind('\n}')
    if pos < 0:
        raise SystemExit(f"{path}: final brace not found")
    write(path, text[:pos] + '\n' + snippet.rstrip() + text[pos:])

# Full-suite regression: authorization tests must obey the exact-four policy.
replace_once(
    'src-tauri/src/authorization.rs',
    '        let password = format!("Password-{n}-Strong");\n',
    '        let password = format!("{n:04}");\n',
    'authorization test password',
)

# Every follow-up date is part of the global 1950-2050 Gregorian policy.
replace_once(
    'src-tauri/src/visit_tracking.rs',
    'use rusqlite::{params, Connection};\n',
    'use chrono::{Datelike, NaiveDateTime};\nuse rusqlite::{params, Connection};\n',
    'visit tracking chrono import',
)
replace_once(
    'src-tauri/src/visit_tracking.rs',
    '''    if input\n        .follow_up_at\n        .as_deref()\n        .is_some_and(|value| value.trim().is_empty())\n    {\n        return Err("تاريخ المتابعة غير صالح".into());\n    }\n''',
    '''    if let Some(value) = input.follow_up_at.as_deref() {\n        let value = value.trim();\n        if value.is_empty() {\n            return Err("تاريخ المتابعة غير صالح".into());\n        }\n        let date = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")\n            .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M"))\n            .map_err(|_| "تاريخ المتابعة غير صالح".to_string())?;\n        if !(1950..=2050).contains(&date.year()) {\n            return Err("تاريخ المتابعة يجب أن يكون بين 1950 و2050".into());\n        }\n    }\n''',
    'follow-up Gregorian range validation',
)
insert_before_last_brace(
    'src-tauri/src/visit_tracking.rs',
    r'''    #[test]
    fn follow_up_date_range_is_enforced() {
        let c = db();
        for value in ["1949-12-31T10:00:00", "2051-01-01T10:00:00"] {
            assert!(update(
                &c,
                1,
                VisitTrackingInput {
                    visit_type: "follow_up".into(),
                    visit_stage: "scheduled".into(),
                    follow_up_at: Some(value.into()),
                },
            )
            .is_err());
        }
        for value in ["1950-01-01T10:00:00", "2050-12-31T10:00:00"] {
            assert!(update(
                &c,
                1,
                VisitTrackingInput {
                    visit_type: "follow_up".into(),
                    visit_stage: "scheduled".into(),
                    follow_up_at: Some(value.into()),
                },
            )
            .is_ok());
        }
    }
''',
)

# Protect all four approved patient search keys.
insert_before_last_brace(
    'src-tauri/src/patients.rs',
    r'''    #[test]
    fn patient_search_covers_name_file_national_id_and_phone() {
        let mut c = db();
        let mut data = input("مريض البحث الكامل", "1234567890");
        data.phone = Some("0555123456".into());
        let patient = create(&mut c, data).unwrap();

        for query in [
            "البحث الكامل".to_string(),
            patient.file_no.to_string(),
            "١٢٣٤٥٦٧٨٩٠".to_string(),
            "0555123456".to_string(),
        ] {
            let rows = list(&c, Some(query), 50).unwrap();
            assert!(rows.iter().any(|row| row.id == patient.id));
        }
    }

    #[test]
    fn birth_date_upper_boundary_is_enforced() {
        let mut c = db();
        let mut data = input("مريض تاريخ علوي", "1234567890");
        data.birth_date = Some("2051-01-01".into());
        assert!(create(&mut c, data).is_err());
    }
''',
)

# Directory deactivation protections need regression tests, not only happy-path tests.
insert_before_last_brace(
    'src-tauri/src/directory.rs',
    r'''    #[test]
    fn future_active_appointment_blocks_doctor_and_clinic_deactivation() {
        let c = db();
        let clinic = create_clinic(
            &c,
            ClinicInput {
                name: "عيادة مرتبطة".into(),
                phone: None,
                address: None,
            },
        )
        .unwrap();
        let doctor = create_doctor(
            &c,
            DoctorInput {
                clinic_id: Some(clinic.id),
                name: "طبيب مرتبط".into(),
                specialty: None,
                phone: None,
            },
        )
        .unwrap();
        c.execute("INSERT INTO patients(file_no,full_name) VALUES(1,'مريض مرتبط')", [])
            .unwrap();
        c.execute(
            "INSERT INTO appointments(patient_id,clinic_id,doctor_id,starts_at,ends_at,status) VALUES(1,?1,?2,'2099-01-01T10:00:00','2099-01-01T10:30:00','scheduled')",
            rusqlite::params![clinic.id, doctor.id],
        )
        .unwrap();

        assert!(deactivate_doctor(&c, doctor.id).is_err());
        assert!(deactivate_clinic(&c, clinic.id).is_err());
        assert_eq!(list_doctors(&c).unwrap().len(), 1);
        assert_eq!(list_clinics(&c).unwrap().len(), 1);
    }
''',
)

# Closure date policy covers both ends of the shared date range.
insert_before_last_brace(
    'src-tauri/src/scheduling.rs',
    r'''    #[test]
    fn closure_lower_range_is_enforced_in_backend() {
        let c = db();
        assert!(add_closure(
            &c,
            ClosureInput {
                closure_date: "1949-12-31".into(),
                reason: None
            }
        )
        .is_err());
    }
''',
)

# Keep the generated coverage inventory honest about the newly executable coverage.
p = Path('docs/TEST_COVERAGE_MATRIX.md')
s = read(str(p))
s += '''\n## Additional coverage found during full-suite audit\n\n- Authorization role tests now use the exact-four password policy instead of legacy long passwords.\n- Patient search is executable-tested by name, file number, national ID (including Arabic-Indic input normalization), and phone.\n- Follow-up dates are backend-validated and tested at both 1950 and 2050 boundaries.\n- Clinic/doctor deactivation is tested to remain blocked while future active appointments exist.\n- Patient birth date and closure date now have tests on both sides of the shared 1950-2050 policy.\n\nThe employee actor identity path for ordinary business mutations remains OPEN until actor/session data is propagated from authorization into those transactional audit writes.\n'''
write(str(p), s)
