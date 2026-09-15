# Optional authentication and permissions architecture

## Non-negotiable startup behavior

ClinicDesk continues to start without a username/password prompt by default. Authentication is opt-in and must never recreate the old forced startup Admin/login behavior.

## Operating modes

- Local simple mode (default): authentication disabled; the application opens directly to the dashboard.
- Protected local mode: the user explicitly enables users and permissions from Settings. Login is required only after this setting has been enabled and at least one recovery-capable administrator account has been created successfully.
- Future network mode: authentication is mandatory and identities/permissions are validated by the server/API rather than trusted from the client.

## Safe enablement sequence

1. User opens Settings > Security and permissions.
2. User chooses Enable users and permissions.
3. ClinicDesk requires creation of the first administrator and verifies credentials can be persisted securely.
4. Only after successful verification is `auth_enabled` committed as enabled.
5. If setup is cancelled or fails, authentication remains disabled and the next launch still opens directly to the dashboard.

Disabling authentication later must require an authorized administrator while protected mode is active and must be recorded in the audit log.

## Permission model

Permissions are capabilities, not UI-only hiding. Backend commands must enforce them when protected mode is active.

Initial capabilities:

- patients.view
- patients.create
- patients.update
- patients.delete
- appointments.view
- appointments.create
- appointments.update
- appointments.status
- attachments.view
- attachments.manage
- reports.view
- backup.create
- backup.restore
- directory.manage
- settings.manage
- users.manage
- audit.view

Roles are collections of capabilities. Custom roles may be added later without changing patient or appointment records.

## Security requirements

- Never store plaintext passwords.
- Store a memory-hard password hash with a unique salt; credentials never belong in React/localStorage.
- Session state and authorization decisions live in the Rust/backend layer.
- Every protected backend command checks its capability when auth is enabled.
- Sensitive actions and permission changes are audit logged with user identity and timestamp.
- UI permission checks improve usability but are not the security boundary.
- Database backups must include the security metadata needed for a consistent restore.

## Network-ready boundary

React must continue to call the existing application API layer rather than SQLite directly. Rust owns local persistence behind repository/service boundaries. A future network edition can replace the local implementation with authenticated HTTPS API calls backed by a central database such as PostgreSQL while preserving most React workflows.

Do not place SQLite-specific assumptions in React components. Do not expose database paths or raw SQL to the frontend.

## Migration plan

1. Add security settings schema with authentication disabled by default.
2. Add users, roles, role-capabilities, sessions and security audit metadata through additive migrations.
3. Add Rust authentication/authorization services and tests without changing default startup behavior.
4. Add Settings UI for enabling protected mode and managing users/roles.
5. Add login screen that is reachable at startup only when protected mode is already enabled.
6. Add integration tests proving fresh install and failed/cancelled setup never show forced login.
7. Keep repository/service interfaces suitable for a future server-backed implementation.

## Required regression tests

- Fresh database: auth disabled and dashboard startup allowed.
- No users exist on fresh install.
- Enabling auth cannot commit before the first administrator is valid and persisted.
- Cancelled/failed enablement leaves auth disabled.
- Protected mode rejects unauthenticated protected commands.
- Permission denial is enforced in Rust even if a frontend control is manipulated.
- Protected mode restart requires login.
- Local simple mode restart does not require login.
- Audit records identify the acting user when auth is enabled.
