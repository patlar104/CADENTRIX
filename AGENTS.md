# Prototype Instructions

Run the local server yourself and open the preview in the browser available to this environment. Do not give the user server-start instructions when you can run it.

Before making substantial visual changes, use the Product Design plugin's `get-context` skill when the visual source is unclear or no longer matches the current goal. When the user gives durable prototype-specific design feedback, preferences, or decisions, record them in `AGENTS.md`.

When implementing from a selected generated mock, treat that image as the source of truth for layout, component anatomy, density, spacing, color, typography, visible content, and hierarchy.

Build app UI in `src/`. Keep `.openai/hosting.json`, `worker/index.js`, `scripts/prepare-sites-build.mjs`, and `tests/sites-worker.test.mjs` intact so the same local prototype can be handed to Sites. Before a Sites handoff, run `npm run build` and `npm run test:sites`; the build must leave `dist/client/index.html`, `dist/server/index.js`, and `dist/.openai/hosting.json`.

## Selected prototype direction

The user selected the second displayed concept: the dark execution map. Use `qa/design-reference.png` as the visual target. Preserve its task selector, four-agent handoff graph, selected-agent details, activity timeline, dark neutral surface, and cyan/green/amber status palette. ARC is a supporting subsystem, not the identity of the whole platform. CADENTRIX is the current working label. The user subsequently authorized a real local backend and frontend integration: tasks, native inspection stages, activity and reports now come from Rust/SQLite. Do not restore demo records, manual simulated stage advancement or invented telemetry. The four roles currently represent native read-only worker stages; no AI model is connected. The one-slot local admission limit is an implementation choice, not a confirmed ARC controller policy default.
