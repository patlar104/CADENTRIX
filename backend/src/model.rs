use std::collections::BTreeMap;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

pub const ROLES: [&str; 4] = ["Planner", "Builder", "Verifier", "Reviewer"];
pub fn now() -> String { Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true) }
pub fn id() -> String { uuid::Uuid::new_v4().to_string() }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stage {
    pub role: String,
    pub state: String,
    pub work: String,
    pub tools: String,
}
pub fn stages() -> Vec<Stage> {
    [
        ("Validate configured inspection scope", "workspace metadata read · permitted"),
        ("Inventory project file metadata", "directory metadata read · project scope"),
        ("Verify inventory consistency", "report validation · permitted"),
        ("Produce the project report", "report formatting · permitted"),
    ].iter().enumerate().map(|(i, (work, tools))| Stage { role: ROLES[i].into(), state: "idle".into(), work: (*work).into(), tools: (*tools).into() }).collect()
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    pub id: String,
    pub timestamp: String,
    pub status: String,
    pub title: String,
    pub detail: String,
    pub run_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub objective: String,
    pub priority: String,
    pub job_type: String,
    pub status: String,
    pub step: i32,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
    pub run_id: Option<String>,
    pub stages: Vec<Stage>,
    pub events: Vec<Event>,
    pub report: Option<Report>,
    pub error: Option<String>,
}
impl Task {
    pub fn event(&mut self, status: &str, title: &str, detail: &str) {
        self.updated_at = now();
        self.events.push(Event { id: id(), timestamp: self.updated_at.clone(), status: status.into(), title: title.into(), detail: detail.into(), run_id: self.run_id.clone() });
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct TaskSummary {
    pub id: String,
    pub name: String,
    pub priority: String,
    pub status: String,
    pub updated_at: String,
}
impl From<Task> for TaskSummary {
    fn from(task: Task) -> Self { Self { id: task.id, name: task.name, priority: task.priority, status: task.status, updated_at: task.updated_at } }
}
#[derive(Clone, Debug)]
pub struct Run { pub id: String, pub task_id: String }
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Report {
    pub workspace_name: String,
    pub generated_at: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub file_types: BTreeMap<String, u64>,
    pub top_directories: BTreeMap<String, u64>,
    pub manifests: Vec<String>,
    pub sample_files: Vec<String>,
    pub partial: bool,
    pub notes: Vec<String>,
    pub markdown: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTask {
    pub name: String,
    pub objective: String,
    pub priority: String,
}
