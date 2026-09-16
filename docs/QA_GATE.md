# QA gate

Before merging this branch:

- `npm run build` succeeds.
- `cargo test --manifest-path src-tauri/Cargo.toml` succeeds.
- `cargo check --manifest-path src-tauri/Cargo.toml` succeeds on Windows.
- Fresh install opens directly to dashboard without authentication.
- Fresh database contains zero clinics, zero doctors, zero patients and zero appointments.
- Root document and every routed page remain RTL.
- SQLite foreign keys and WAL are active.
