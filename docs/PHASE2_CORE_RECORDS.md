# Phase 2 — Core records

Patient CRUD/search and clinic/doctor setup are implemented through Tauri commands backed by SQLite. File numbers are transactionally monotonic, national-ID digits are normalized, active duplicates are rejected, and patient deletion is soft. Windows CI must pass frontend build, Rust tests and cargo check before scheduling work begins.
