# Local backend, first usable slice

The user delegated the choice of the first real job. Implementation choice: a read-only project inspection worker, with a Rust HTTP service and local SQLite storage behind the existing UI. This is application/backend work, not a change to the approved ARC roadmap.

## Why this slice

A database-only service would make the demo persistent but would still leave execution simulated. Connecting a model provider immediately would introduce provider, cost, tool-access and execution-boundary choices before the task lifecycle is established. A bounded native inspection worker exercises real execution, persistence, reporting and failure handling on this project without requiring those choices.

## Behavior

- Create tasks with name, objective and recorded priority. The job type is explicitly project inspection; objectives are labels and do not become shell commands or model instructions.
- One server-owned local worker slot; explicit starts rather than a pretend adaptive scheduler. The one-slot limit is an implementation default, not a confirmed ARC policy threshold.
- Planner validates the configured root; Builder inventories project metadata; Verifier checks report consistency; Reviewer formats the report. These are native worker stages, not AI agents.
- Persist tasks, runs, stages, timestamped events and JSON/Markdown results in SQLite. A refreshed browser or restarted service sees committed records.
- Atomic admission prevents duplicate starts and concurrent ownership. A single-instance database lock prevents two processes from recovering each other's live runs.
- On restart, running attempts become interrupted, with an event; no automatic replay. A user may explicitly retry the read-only job.
- Unknown host telemetry remains unknown. This slice implements a local admission gate, not the planned ARC reactive/predictive controller, durable task checkpoints, or distributed workers.

## Execution boundary

Server configuration fixes the workspace root to this project by default. API clients cannot choose paths or executables. The worker reads directory entries and file metadata only; it does not read file contents, execute project commands, write project files, follow symlinks, or call external services. Exclude hidden paths, dependencies, build output, backend data and QA captures. Bound depth, entries and execution time. Report partial inventory when depth/entry limits are reached; fail clearly on unavailable/unreadable filesystem entries.

The service listens on loopback. Check Host and Origin, reject cross-site mutations, require JSON mutation requests, and do not enable wildcard CORS. This is a single-user local service, not an authenticated network/multi-user deployment.

## API and frontend

GET health/workspace/policies, GET/POST tasks, GET task details, POST task runs, GET saved Markdown report. Return structured errors and real timestamps. The React frontend uses relative API requests through Vite's local proxy, polls while open, exposes loading/error/empty states, and preserves the selected visual hierarchy. Replace simulation labels and manual step controls with real worker state and Run inspection/View report controls.

Keep the existing Sites static runtime files intact. Static Sites hosting by itself does not host the Rust service; deployment is out of this slice.

## Validation

Write a persistence regression first and observe its failure before implementing. Then cover validation, API errors and request-origin restrictions, persisted reports, duplicate/capacity admission, process lock, interrupted-run recovery, scanner exclusions, bounded scans and failure reporting. Use disposable fixture directories for worker tests. Exercise create/run/report/refresh in the browser and restart the service to prove persistence. Run Rust tests, rustfmt, Clippy and the frontend production build.
