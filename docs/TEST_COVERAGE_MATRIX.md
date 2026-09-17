# ClinicDesk requirement test coverage matrix

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

## Additional coverage found during full-suite audit

- Authorization role tests now use the exact-four password policy instead of legacy long passwords.
- Patient search is executable-tested by name, file number, national ID (including Arabic-Indic input normalization), and phone.
- Follow-up dates are backend-validated and tested at both 1950 and 2050 boundaries.
- Clinic/doctor deactivation is tested to remain blocked while future active appointments exist.
- Patient birth date and closure date now have tests on both sides of the shared 1950-2050 policy.

The employee actor identity path for ordinary business mutations remains OPEN until actor/session data is propagated from authorization into those transactional audit writes.
