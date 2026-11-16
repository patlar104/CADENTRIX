# CADENTRIX local workspace

A React task-and-worker interface connected to a local Rust service and SQLite. The first real job is a bounded, read-only project inspection. The selected dark execution-map layout is preserved.

## Use it

Open http://127.0.0.1:4173/ while the development frontend and backend are running.

1. Create an inspection task with a name, objective and priority.
2. Choose **Run inspection**. The backend validates the workspace, inventories file metadata, verifies the totals and saves a Markdown report.
3. Use **View report** or **Download report** when execution completes.
4. Switch between saved tasks. The selected task is recorded in the URL so a refresh reopens it.

Task data, runs, timestamped activity and reports live in SQLite, not browser storage. They survive browser refreshes and server restarts. The interface polls server state every 1.2 seconds; fast inspections may finish between polls, while every stage transition remains in the activity timeline.

These four stages are native Rust worker steps, not AI agents. No model provider is connected. Objectives label the report; they are not executable instructions. Priority is recorded, but this first service uses explicit starts and one local worker slot rather than priority-based queue scheduling.

## Run locally

Use the project working directory and the existing mise environment. No project-specific tool versions override the user's configured tools.

Backend, from /Users/patricklarocque/Documents/ChatGPT/CADENTRIX:

```sh
MISE_AUTO_INSTALL=0 /Users/patricklarocque/.local/bin/mise exec -- cargo run --locked --manifest-path /Users/patricklarocque/Documents/ChatGPT/CADENTRIX/backend/Cargo.toml
```

Frontend, from the same directory in a second terminal:

```sh
MISE_AUTO_INSTALL=0 /Users/patricklarocque/.local/bin/mise exec -- npm run dev -- --host 127.0.0.1 --port 4173 --strictPort
```

Vite proxies relative /api requests to http://127.0.0.1:4180/. The Rust service also serves the built frontend at that address after a production build.

## Data and execution scope

- Default database: .cadentrix/state.sqlite3 under the configured project. Keep the associated SQLite WAL files with the live database; do not treat copying only the main file as a live backup.
- CADENTRIX_WORKSPACE optionally supplies the startup workspace. CADENTRIX_DATA_DIR optionally supplies the data directory. Configure these in the shell before starting the service.
- The database stores the original canonical workspace binding and rejects a different workspace. Equivalent dot-segment or symlink aliases are accepted. Existing task databases with no binding fail closed.
- The native worker reads directory entries and file metadata. It does not read file contents, run project commands, change source files or make external requests.
- Hidden paths, symlinks, dependency/build directories, backend data and QA captures are excluded. Entry and depth limits produce an explicitly partial inventory; a cooperative time limit stops excessive scanning.
- The single-instance lock and atomic admission gate prevent duplicate active ownership. Interrupted runs require an explicit retry after restart.
- CPU and memory telemetry are unknown. The full ARC reactive/predictive controller, persistent worker checkpoints and distributed execution are not implemented.
- The server listens on loopback, checks Host/Origin and requires JSON mutations. This is a single-user local application, not a network deployment with authentication.

The data directory and build/dependency outputs are ignored by Git. The application runs locally; no deployment has been created.

## Checks

Run from the project directory:

```sh
MISE_AUTO_INSTALL=0 /Users/patricklarocque/.local/bin/mise exec -- npm run test:client
MISE_AUTO_INSTALL=0 /Users/patricklarocque/.local/bin/mise exec -- cargo test --locked --manifest-path /Users/patricklarocque/Documents/ChatGPT/CADENTRIX/backend/Cargo.toml
MISE_AUTO_INSTALL=0 /Users/patricklarocque/.local/bin/mise exec -- npm run build
```

Backend tests cover persistent records and reports, API requests, admission ownership, origin restrictions, workspace binding, scanner exclusions, interrupted-run recovery and failure reporting. Client tests cover committed responses and error handling without automatic mutation retries.

Browser verification covers task creation, real execution, report view/download, task switching, refresh persistence, backend stop/reconnect/restart persistence, and desktop/mobile layouts. The report downloaded through the UI was compared with the displayed saved report; the complete task record and activity history were compared before and after a real service restart.

## Design and hosting

The visual source remains qa/design-reference.png. Existing design-qa.md and qa captures document the earlier simulated prototype; current behavior is described here.

The Product Design starter's static Sites runtime files remain intact. A static Sites deployment alone would not host this Rust/SQLite backend; publishing is outside the current scope.

Font: locally bundled [Inter](https://github.com/rsms/inter). Icons: [Phosphor](https://github.com/phosphor-icons/react).
