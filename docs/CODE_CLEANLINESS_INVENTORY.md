# ClinicDesk Code Cleanliness Inventory

This inventory is the first execution slice of the adopted code-cleanliness gate. It identifies responsibility boundaries before behavior-preserving refactors. No product behavior is intentionally changed by this slice.

## Frontend priority map

### `src/PatientsPage.tsx` — highest cleanup priority
Current responsibilities are concentrated in one page component:
- patient search and result loading;
- initial file-number navigation;
- create/edit patient form state and submission;
- patient deletion preconditions and confirmation;
- patient record/details modal;
- appointment-history loading and rendering;
- attachment section composition;
- medical-summary sections;
- patient table and row actions.

Safe target boundaries:
1. `PatientSearchBar`
2. `PatientTable`
3. `PatientRecordModal`
4. `PatientFormModal`
5. patient-form mapping/normalization helpers
6. appointment-history presentation

Refactor rule: extract presentation first; keep API calls and state ownership in `PatientsPage` until extracted components are covered by UI tests. Do not change deletion, file-number, persistence, or appointment semantics while splitting the UI.

### `src/AppointmentsPage.tsx` — high cleanup priority
Known mixed responsibilities include appointment list/table presentation, patient lookup, date/time editing, status actions, validation, and modal state. Split only after the patient-page slice is green so failures remain attributable.

### `src/Dashboard.tsx` — high cleanup priority
Separate alert/query orchestration from dashboard cards and appointment/alert presentation. Preserve the current silent in-app alert behavior.

### `src/DirectoryPage.tsx` — medium cleanup priority
Keep clinic/doctor CRUD behavior intact and preserve the required empty first-run state with no default clinic or doctor records.

### `src/SchedulingPage.tsx` — medium cleanup priority
Separate schedule form/presentation helpers without changing working-day, break, slot, closure, Friday, or Saturday rules.

### `src/PatientAttachments.tsx` — focused component
Keep as a dedicated boundary. Audit error handling, file lifecycle, accessibility, and duplicate logic before considering further extraction.

### `src/App.tsx` — composition boundary
Keep navigation and top-level page routing/composition here. Do not move database-specific behavior into React while preparing for future network mode.

## Cross-cutting cleanup targets

- Replace compressed/minified-like source formatting with reviewable formatting in isolated commits.
- Consolidate only genuinely repeated date/time/status/modal helpers.
- Remove dead imports and unreachable/duplicate logic only after verification.
- Keep Arabic RTL semantics explicit and testable.
- Add accessible names to icon-only controls where missing.
- Keep API/database concerns behind the existing API boundary.
- Avoid new abstractions that have only one trivial caller unless they create a clear responsibility boundary.

## Execution order

1. Reformat/refactor `PatientsPage.tsx` without behavior changes.
2. Extract patient presentation components in small slices.
3. Gate each behavior-affecting slice through frontend build/type checks and the full required CI/security gate.
4. Repeat for appointments, dashboard, directory, and scheduling.
5. Perform dead-code/dependency/Rust/SQL audit only after UI responsibilities are readable enough to review reliably.

## Acceptance for each cleanup slice

A cleanup slice is accepted only when:
- observable behavior is intentionally unchanged unless the commit explicitly declares a product change;
- TypeScript/frontend build passes;
- required Rust, migration, database, and security checks remain green;
- strict Clippy remains green without suppressing valid warnings;
- CodeQL remains green;
- no screenshot/visual baseline is blindly updated to hide a regression.
