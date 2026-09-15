# ClinicDesk network mode — 10 clients

## Supported target

ClinicDesk is designed to support up to 10 concurrent Windows client devices on the clinic LAN.

## Topology

- One dedicated clinic server/service owns the central database and attachment storage.
- Up to 10 Windows clients run the Tauri/React desktop UI.
- Clients communicate with the service through an authenticated HTTPS API.
- Clients never open a shared SQLite file over SMB/network shares.
- Network mode requires authentication and backend-enforced permissions.
- Local single-PC mode remains available and authentication remains optional there.

## Persistence

The current local edition continues to use SQLite behind Rust repository/service boundaries. Network mode must use a server-owned central database suitable for concurrent clients (target: PostgreSQL). The frontend must not depend on SQLite-specific details.

## Concurrency requirements

- Patient file-number allocation is atomic on the server.
- Appointment conflict checks and appointment creation/rescheduling occur in one server-side transaction.
- Concurrent edits use an optimistic concurrency/version check so one workstation cannot silently overwrite another workstation's newer change.
- Attachments are uploaded/downloaded through the service rather than a client-visible shared folder.
- Audit events include authenticated user and workstation/session identity.

## Availability and safety

- Server health endpoint and client connection-state indicator.
- Automatic reconnect for transient LAN loss without replaying unsafe writes.
- Server-side scheduled backup with restore verification.
- No offline write mode in the first network release; this avoids split-brain/merge corruption.
- TLS is required even on the LAN for credentials and patient data.

## Acceptance gate for 10 clients

Before network mode is called production-ready, automated/integration tests must cover 10 concurrent clients performing reads and representative writes, including simultaneous patient creation, appointment booking conflicts, edits, attachment metadata operations, authentication/authorization and audit logging. No duplicate file numbers, double-booked protected slots, silent lost updates, or authorization bypasses are acceptable.
