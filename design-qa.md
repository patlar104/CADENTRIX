# CADENTRIX design QA

final result: passed

## Comparison target and evidence

- Source visual truth: `qa/design-reference.png`, copied unchanged from the second displayed generated concept (`exec-7026ad8d-69e3-4ae5-97b3-5737de42ce73.png`).
- Original source pixels: 1487 × 1058. Normalized reference: `qa/source-normalized.jpg`, 1440 × 1024. The source is scaled down by approximately 0.968; aspect rounding is under one pixel.
- Implementation: http://127.0.0.1:4173/, captured in the Codex in-app browser.
- Implementation screenshot: `qa/desktop-final.jpg`, 1440 × 1024 pixels at a 1440 × 1024 CSS viewport. Screenshot density is 1 pixel per CSS pixel; no browser chrome or device frame.
- State: Prepare release candidate selected; Builder selected; dark theme; page scroll at top; no dialogs open.
- Full-view comparison, with both artifacts in one image: `qa/comparison-final.jpg`.
- Focused detail and timeline comparison, both artifacts in one image: `qa/detail-comparison-final.jpg`.
- Additional browser evidence: `qa/admission-held.jpg`, `qa/create-task.jpg`, `qa/workflow-complete.jpg`, `qa/mobile.jpg`, `qa/mobile-form.jpg`, and `qa/tablet.jpg`.
- Responsive viewports checked: 1024 × 768 and 390 × 844. DOM inspection confirms viewport and document scroll widths match at 1024 and 390; there is no horizontal overflow. Mobile and tablet captures use explicit viewport clips because the browser's generic surface screenshot includes unused surrounding space under an override.

## Findings and fidelity surfaces

No actionable P0/P1/P2 findings remain after the final comparison.

- **Fonts and typography:** Locally bundled Inter preserves the sans-serif hierarchy, readable body copy, display weight, and label wrapping. The 40px heading, 21px task selector and graph labels, 18px detail copy, and 16px event descriptions closely match the normalized reference. Minor glyph, rasterization, and optical weight differences from the generated mock are P3.
- **Spacing and layout rhythm:** The 216px sidebar, left content inset, task toolbar, overview split, four-column execution map, details table and timeline match the reference's major proportions and order. Adjusted the header, heading, overview, and detail padding after the first comparison. Final detail and timeline dividers align closely with the normalized source. The map stacks vertically at phone width; persistent controls remain usable.
- **Colors and tokens:** Dark neutral base, cyan running/primary states, green completion, amber waiting, gray inactive, restrained sidebar selection and thin dividers preserve the reference. The generated mock's subtle background lighting is represented by a clean solid surface; this is an acceptable P3 difference, with no decorative CSS artwork.
- **Image quality and assets:** The source contains text and standard interface icons, with no raster illustrations, product photos, or avatars. Phosphor supplies the navigation, status, action, and arrow icons. The running symbol combines unmodified Circle and Play library icons to match the outlined ring and filled triangle. Graph edges and timeline rules are structural UI separators. No handcrafted SVG, raster screenshot-as-UI, placeholder art, or fake asset is used. Inter is self-hosted; runtime media does not depend on external requests.
- **Copy and content:** The default task, objective, stages, permission labels, ARC note and three initial events reproduce the selected concept. Additional states stay explicitly simulated. Unknown memory is shown as unknown and holds admission; illustrative slot values are labeled. Broader architecture and production policy defaults are not claimed.
- **Interaction and accessibility:** Semantic buttons/selects, labeled inputs, native modal focus containment and Escape handling, visible focus outlines, live status announcements and reduced-motion support are present. The held Verifier's accessible name now matches its visible admission state. The new-task dialog focuses the task-name field. No comprehensive assistive-technology certification is claimed.

## Comparison history

### Initial comparison — blocked

Evidence: `qa/desktop-initial.jpg`, `qa/comparison-initial.jpg`, `qa/detail-comparison-initial.jpg`.

1. [P2] Vertical rhythm drift: toolbar around 5px lower, overview divider around 10px lower, detail/timeline around 14px lower than source. Fixed by reducing header/heading gaps, overview bottom padding and detail bottom padding.
2. [P2] Running state icon: undersized circle and hollow play symbol. Fixed by using the library Circle and filled Play symbols, optical scaling, and slightly larger timeline status icons.
3. [P2] Accessible admission label: visible state said Admission held while the accessible name said Waiting for handoff. Fixed by deriving both labels from the same status-label function.
4. [P3] Inter glyph shapes vary slightly from the raster mock. Accepted as a close, freely available typeface match.

### Post-fix comparison — passed

Recaptured the same task and selected agent at 1440 × 1024 with scroll at top. Reviewed `qa/comparison-final.jpg` and `qa/detail-comparison-final.jpg` together with the source. The earlier spacing and icon differences are resolved to non-blocking optical differences. Browser inspection independently confirmed the corrected `Verifier: Admission held` accessible label. No further visual fixes were required after this pass.

## Primary interaction checks

- Switching sample tasks updates objective, graph states, selected agent, permissions, ARC context, and activity.
- Clicking an agent in the graph changes its selected detail panel.
- Unknown-memory recovery scenario holds admission and exposes the reason.
- New task accepts a name, objective and priority, adds it to the selector and selects it.
- Starting the new task and advancing each step produces Planner → Builder → Verifier → Reviewer → Complete, updates details, and appends activity events. Completed workflow has no advance action.
- Agents navigation opens the role list; choosing Planner closes the dialog and selects Planner.
- Activity navigation displays the current task's events.
- Policies navigation displays read-only illustrative limits and execution boundaries.
- Modal close, Cancel and Escape work. Mobile creation dialog fits within the viewport and focuses the task-name field.
- Final browser console check returned no warnings or errors.
- Final production build completed successfully through Vite and the preserved Sites preparation script.

## Open questions and limits

- This is a frontend prototype. Data resets on reload; there is no backend, host telemetry, external integration, real agent execution or production policy engine.
- The role sequence, capacities and sample tasks are illustrative. Broader platform requirements remain separate from ARC subsystem decisions.
- Browser checks cover the named desktop, tablet and phone viewports. Cross-browser, screen-reader and production integration testing remain future work.

## Follow-up polish

- [P3] A later brand pass may refine typography and subtle surface colors after the product direction is confirmed.

## Implementation checklist

- [x] Recreate the selected visual and copy.
- [x] Resolve initial P2 findings and compare fresh evidence.
- [x] Exercise the core demo journey and navigation.
- [x] Check narrow layouts, focus and console errors.
- [x] Build successfully and keep the local preview available.
