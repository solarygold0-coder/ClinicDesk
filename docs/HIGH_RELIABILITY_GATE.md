# ClinicDesk high-reliability production gate

Network mode is not production-ready until every mandatory gate below passes.

## Data integrity
- Every multi-step write is atomic: commit all effects or roll back all effects.
- Patient file numbers and national IDs cannot duplicate under concurrency.
- Protected appointment slots cannot double-book under concurrency.
- Optimistic version checks reject silent lost updates.
- Mandatory audit writes are in the same transaction as the clinical/administrative mutation.

## Failure recovery
- Simulate client disconnect during writes and attachment uploads.
- Simulate server restart and database interruption.
- Simulate timeouts and connection-pool exhaustion.
- Verify retries never replay unsafe writes.
- Automated backups must have a tested restore procedure; backup existence alone is not acceptance.

## Concurrency and capacity
- 10 simultaneous clients performing representative reads and writes.
- 10,000-patient dataset plus realistic appointments and attachment metadata.
- Stress, load, spike, volume, endurance/soak, scalability and race/concurrency tests.
- No duplicate file numbers, protected double bookings, silent lost updates, authorization bypasses or silent data loss.

## Quality gate
- Rust tests pass on Windows and Linux.
- Clippy passes with warnings denied; useful warnings are never suppressed to make CI green.
- rustfmt passes.
- TypeScript strict checks and production frontend build pass.
- CodeQL and dependency/security checks pass.
- Migration tests cover clean install and supported upgrades.
- Integration/E2E tests pass before release packaging.

## Release safety
- Network credentials and patient data use TLS in transit.
- Network mode requires authenticated identities and backend authorization.
- Local single-PC mode must not regain a forced startup login.
- Release artifacts are versioned, checksummed and digitally signed when the signing identity is available.
- Upgrade and rollback/restore procedures are tested before production deployment.
