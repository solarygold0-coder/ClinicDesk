# Architecture

## UI
React + TypeScript. Arabic RTL is structural, not page-specific. Logical CSS and `dir=rtl` are required for every view.

## Desktop boundary
Only explicit Tauri commands cross from the webview into Rust. The frontend must never receive a raw database path or execute SQL.

## Data
SQLite is local and offline-first. Foreign keys and WAL are enabled at initialization. Schema changes are migration based.

## Safety invariants
1. Patient file numbers are allocated transactionally from `app_meta.next_patient_file_no` and never decremented.
2. Patient deletion is soft-delete by default.
3. Appointment creation must validate working day and overlapping active slots inside the same transaction as insertion.
4. Clinics/doctors are empty on first launch.
5. There is no startup authentication/admin gate.

## Planned modules
`patients`, `appointments`, `clinics`, `doctors`, `attachments`, `backup`, `audit`, `settings`.
