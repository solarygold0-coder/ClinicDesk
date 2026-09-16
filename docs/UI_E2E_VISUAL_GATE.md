# ClinicDesk UI, E2E, and Visual Regression Gate

This is a mandatory quality gate for ClinicDesk. A green compile/build alone is not sufficient evidence that the desktop application works correctly for a real user.

## Required layers

### 1. Component/UI tests
- Verify important React components render expected controls and states.
- Verify validation messages and disabled/enabled actions.
- Verify Arabic RTL behavior for every major page.
- Verify keyboard focus and basic accessibility behavior.

### 2. End-to-end workflows
Automate representative user journeys against a real application/database environment, including:
- launch the application;
- create, edit, search, open, and delete/disable records according to product rules;
- create and edit appointments;
- enforce Friday/Saturday and scheduling constraints;
- detect appointment conflicts;
- open a patient from appointments and alerts;
- exercise clinic/doctor setup from an empty first-run state;
- add/read/delete attachments according to permissions and product rules;
- close and reopen the application and verify persisted data;
- verify upgrade/migration paths preserve existing clinical data.

### 3. Visual regression
Maintain approved screenshots for major states and compare them after UI changes. At minimum cover:
- dashboard;
- patients list, patient details, and patient editor;
- appointments list and editor;
- clinics/doctors directory;
- scheduling/settings;
- empty, loading, validation-error, modal, and representative populated states.

Visual checks must detect unintended layout shifts, missing/overlapping controls, broken RTL, clipped content, and modal/window regressions. Baselines must only be updated after the UI change is intentionally reviewed; never update screenshots merely to make CI green.

### 4. Accessibility checks
Check semantic names, keyboard navigation, focus visibility/order, form labels, and automated accessibility rules where practical. Automated accessibility checks supplement rather than replace manual review.

### 5. Real Windows runtime gate
Before a release candidate is accepted:
- build the actual Windows application/package;
- install it in a clean/reproducible Windows test environment;
- launch the installed application;
- execute critical E2E workflows against the installed build;
- restart Windows/application where relevant and verify persistence;
- test installation/upgrade/uninstall behavior and detect stale-version interference.

### 6. Network-mode gate (when implemented)
The network release must additionally exercise independently installed Windows clients against the central service/database, including 10 concurrent clients and integrity/concurrency rules.

### 7. Performance/reliability release gate
Before final release run the previously required Stress, Load, Endurance/Soak, Spike, Volume, and Scalability tests. Passing means preserving integrity and acceptable behavior, not merely avoiding a crash.

## Non-negotiable rules
- Do not treat CodeQL, Clippy, TypeScript, or a successful build as proof of UI correctness.
- Do not suppress a valid warning or weaken an assertion to make a gate green.
- Do not blindly regenerate visual baselines after a failure.
- Any reproducible functional, data-integrity, security, or material visual regression blocks the next release stage.
- Tests should run in parallel where independent, while the final gate requires every required check to pass.

## Adoption sequence
1. Complete the current code-cleanliness refactor in small green slices.
2. Add the UI/component test harness.
3. Add E2E journeys for critical local-mode workflows.
4. Add deterministic visual-regression baselines.
5. Add Windows installed-build E2E execution.
6. Extend the same framework to network mode and 10-client concurrency when the server architecture exists.
7. Run the full suite again on the final release candidate.
