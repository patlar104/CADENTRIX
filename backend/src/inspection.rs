use std::{collections::BTreeMap, path::{Component, Path}, time::{Duration, Instant}};
use walkdir::{DirEntry, WalkDir};
use crate::{AppError, Report, Result, Run, Store};
use crate::model::now;

const MAX_ENTRIES: usize = 10_000;
const MAX_DEPTH: usize = 12;
const EXCLUDED: [&str; 10] = ["node_modules", "target", "dist", "build", "coverage", "vendor", "qa", "__pycache__", "venv", "data"];
fn include(entry: &DirEntry) -> bool {
    if entry.depth() == 0 { return true; }
    let name = entry.file_name().to_string_lossy();
    !name.starts_with('.') && !EXCLUDED.contains(&name.as_ref()) && !entry.file_type().is_symlink()
}
fn validate_workspace_scope(root: &Path) -> Result<()> {
    let unavailable = || AppError::job("The configured workspace is no longer available. No report was produced.");
    let metadata = root.symlink_metadata().map_err(|_| unavailable())?;
    let canonical = root.canonicalize().map_err(|_| unavailable())?;
    // Store binds a canonical path at startup. Re-resolve it without adopting a
    // new target, including when a parent directory has become a symlink.
    if metadata.file_type().is_symlink() || canonical != root {
        return Err(AppError::job("The configured workspace boundary changed. No report was produced."));
    }
    if !metadata.is_dir() { return Err(unavailable()); }
    Ok(())
}
/// Inventory the canonical workspace path bound by `Store::open`.
pub fn inspect_workspace(root: &Path) -> Result<Report> {
    validate_workspace_scope(root)?;
    let started = Instant::now();
    let mut report = Report { workspace_name: root.file_name().unwrap_or_default().to_string_lossy().into_owned(), generated_at: now(), file_count: 0, total_bytes: 0, file_types: BTreeMap::new(), top_directories: BTreeMap::new(), manifests: vec![], sample_files: vec![], partial: false, notes: vec!["Read-only native worker. File contents were not read; project commands and tests were not executed.".into(), "Hidden paths, symlinks, dependencies, build output, backend data and QA captures were excluded.".into()], markdown: String::new() };
    for (count, entry) in WalkDir::new(root).follow_links(false).follow_root_links(false).max_depth(MAX_DEPTH).sort_by_file_name().into_iter().filter_entry(include).enumerate() {
        if count >= MAX_ENTRIES { report.partial = true; report.notes.push(format!("Inventory stopped at the {MAX_ENTRIES}-entry limit.")); break; }
        if started.elapsed() > Duration::from_secs(15) { return Err(AppError::job("Inspection exceeded its cooperative time budget. No complete report was published.")); }
        let entry = entry.map_err(|_| AppError::job("An entry in the configured workspace could not be read. No complete report was published."))?;
        if entry.depth() == 0 && !entry.file_type().is_dir() { return Err(AppError::job("The configured workspace boundary changed. No report was produced.")); }
        if entry.file_type().is_dir() {
            if entry.depth() == MAX_DEPTH { report.partial = true; }
            continue;
        }
        if !entry.file_type().is_file() { continue; }
        let metadata = entry.metadata().map_err(|_| AppError::job("File metadata became unavailable during inspection. Please retry."))?;
        let relative = entry.path().strip_prefix(root).map_err(AppError::internal)?;
        if !relative.components().all(|part| matches!(part, Component::Normal(_))) { return Err(AppError::job("A path escaped the configured workspace; inspection stopped.")); }
        let path = relative.to_string_lossy().into_owned();
        report.file_count += 1;
        report.total_bytes = report.total_bytes.checked_add(metadata.len()).ok_or_else(|| AppError::job("The inventory exceeded the supported byte count."))?;
        let extension = relative.extension().map(|value| value.to_string_lossy().to_lowercase()).unwrap_or_else(|| "(no extension)".into());
        *report.file_types.entry(extension).or_default() += 1;
        let group = if relative.components().count() > 1 { relative.components().next().map(|part| part.as_os_str().to_string_lossy().into_owned()).unwrap_or_default() } else { "(root)".into() };
        *report.top_directories.entry(group).or_default() += 1;
        if ["package.json", "Cargo.toml", "pyproject.toml", "go.mod"].contains(&entry.file_name().to_string_lossy().as_ref()) { report.manifests.push(path.clone()); }
        if report.sample_files.len() < 30 { report.sample_files.push(path); }
    }
    if report.partial && !report.notes.iter().any(|note| note.contains("entry limit")) { report.notes.push(format!("Directory depth was limited to {MAX_DEPTH}; this is a partial inventory.")); }
    validate_workspace_scope(root)?;
    verify(&report)?;
    Ok(report)
}
fn verify(report: &Report) -> Result<()> {
    if report.file_types.values().sum::<u64>() != report.file_count || report.top_directories.values().sum::<u64>() != report.file_count { return Err(AppError::job("Inventory totals did not agree. The report was rejected.")); }
    Ok(())
}
fn markdown(report: &Report, objective: &str) -> String {
    // A download is plain text; escape line breaks/backticks in file names to keep the report readable.
    let clean = |value: &str| value.replace(['\r', '\n', '`'], " ");
    let mut text = format!("# Project inspection: {}\n\nGenerated: {}\n\nObjective: {}\n\nScope: {}\n\n## Inventory\n\n- Files inspected: {}\n- Total file bytes: {}\n\n## File types\n\n", clean(&report.workspace_name), report.generated_at, clean(objective), if report.partial { "Partial bounded inventory" } else { "Configured workspace, with documented exclusions" }, report.file_count, report.total_bytes);
    for (extension, count) in &report.file_types { text.push_str(&format!("- {}: {}\n", clean(extension), count)); }
    text.push_str("\n## Top-level groups\n\n");
    for (directory, count) in &report.top_directories { text.push_str(&format!("- {}: {} files\n", clean(directory), count)); }
    text.push_str("\n## Detected manifests\n\n");
    if report.manifests.is_empty() { text.push_str("No supported manifests found within scope.\n"); }
    for path in &report.manifests { text.push_str(&format!("- {}\n", clean(path))); }
    text.push_str("\n## Sample file paths (up to 30)\n\n");
    for path in &report.sample_files { text.push_str(&format!("- {}\n", clean(path))); }
    text.push_str("\n## Notes and limits\n\n");
    for note in &report.notes { text.push_str(&format!("- {note}\n")); }
    text.push_str("- This report is an inventory, not a code audit or a claim that the project builds or passes tests.\n- Counts describe a live filesystem scan, not a point-in-time filesystem snapshot.\n- Planner, Builder, Verifier and Reviewer are native stages; no AI model was invoked.\n");
    text
}
pub fn execute_run(store: &Store, run: &Run) -> Result<()> {
    let result: Result<()> = (|| {
        validate_workspace_scope(store.workspace())?;
        store.finish_stage(run, 0, "Configured read-only workspace scope validated.", None)?;
        let mut report = inspect_workspace(store.workspace())?;
        store.finish_stage(run, 1, &format!("Inventoried {} files ({} bytes){}.", report.file_count, report.total_bytes, if report.partial { "; partial bounded inventory" } else { "" }), None)?;
        verify(&report)?;
        store.finish_stage(run, 2, "File-type and directory totals match the inventory. No project tests were executed.", None)?;
        let task = store.get_task(&run.task_id)?;
        report.markdown = markdown(&report, &task.objective);
        validate_workspace_scope(store.workspace())?;
        store.finish_stage(run, 3, "Project inspection report saved to SQLite. Ready to view or download.", Some(report))?;
        Ok(())
    })();
    if let Err(error) = &result { store.fail_run(run, &error.message)?; }
    result
}
