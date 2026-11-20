import { useEffect, useRef, useState } from "react";
import {
  Stack,
  Users,
  Pulse,
  Shield,
  Plus,
  CaretDown,
  CheckCircle,
  PlayCircle,
  Play,
  PauseCircle,
  Circle,
  ArrowRight,
  X,
  Check,
  Info,
  FileText,
  DownloadSimple,
  XCircle,
  ArrowClockwise,
} from "@phosphor-icons/react";
import "@fontsource-variable/inter";
import { apiRequest } from "./api.js";
import { useWorkspace } from "./useWorkspace.js";

const roles = ["Planner", "Builder", "Verifier", "Reviewer"];
const dateFormatter = new Intl.DateTimeFormat("en-CA", {
  month: "short",
  day: "numeric",
  year: "numeric",
  hour: "numeric",
  minute: "2-digit",
  timeZone: "America/Toronto",
});
function formatDate(value) {
  return value ? dateFormatter.format(new Date(value)) : "Connecting…";
}
function stateFor(task, index) {
  return task?.stages[index]?.state || "idle";
}
function statusLabel(task, index) {
  const state = stateFor(task, index);
  return (
    {
      complete: "Complete",
      running: "Running",
      failed: "Failed",
      waiting:
        task?.status === "interrupted" ? "Interrupted" : "Waiting for handoff",
      idle: "Not started",
    }[state] || "Unknown"
  );
}
function StateIcon({ state, small = false }) {
  if (state === "running" && !small)
    return (
      <span className="state-icon running running-symbol" aria-hidden="true">
        <Circle size={60} weight="regular" />
        <Play size={29} weight="fill" />
      </span>
    );
  const Icon =
    {
      complete: CheckCircle,
      running: PlayCircle,
      waiting: PauseCircle,
      idle: Circle,
      failed: XCircle,
    }[state] || Circle;
  return (
    <Icon
      aria-hidden="true"
      className={"state-icon " + state + (small ? " small" : "")}
      size={small ? 36 : 60}
      weight={small || state === "complete" ? "fill" : "regular"}
    />
  );
}
function Timeline({ events = [] }) {
  if (!events.length)
    return (
      <p className="empty-message">
        No activity yet. Create and run an inspection to begin.
      </p>
    );
  return (
    <ol className="timeline">
      {events.map((event) => (
        <li key={event.id}>
          <StateIcon state={event.status} small />
          <div className="event-copy">
            <h3>{event.title}</h3>
            <p>{event.detail}</p>
          </div>
          <time dateTime={event.timestamp}>{formatDate(event.timestamp)}</time>
        </li>
      ))}
    </ol>
  );
}
function Modal({ title, onClose, children, wide = false, busy = false }) {
  const ref = useRef(null);
  useEffect(() => {
    ref.current.showModal();
    ref.current.querySelector("input")?.focus();
  }, []);
  return (
    <dialog
      ref={ref}
      className={wide ? "modal wide" : "modal"}
      aria-labelledby="modal-title"
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onClose();
      }}
      onClick={(event) => {
        if (event.target !== ref.current || busy) return;
        const bounds = ref.current.getBoundingClientRect();
        if (
          event.clientX < bounds.left ||
          event.clientX > bounds.right ||
          event.clientY < bounds.top ||
          event.clientY > bounds.bottom
        )
          onClose();
      }}
    >
      <div className="modal-heading">
        <div>
          <p className="eyebrow">CADENTRIX / LOCAL WORKSPACE</p>
          <h2 id="modal-title">{title}</h2>
        </div>
        <button
          className="icon-button"
          disabled={busy}
          onClick={onClose}
          aria-label="Close dialog"
        >
          <X size={23} />
        </button>
      </div>
      {children}
    </dialog>
  );
}
export function App() {
  const {
    workspace,
    activeId,
    task,
    connected,
    connectionError,
    selectTask,
    acceptTask,
    refresh,
  } = useWorkspace();
  const tasks = workspace?.tasks || [];
  const policy = workspace?.policy;
  const [agent, setAgent] = useState(0);
  const [modal, setModal] = useState(null);
  const [notice, setNotice] = useState("");
  const [title, setTitle] = useState("");
  const [objective, setObjective] = useState("");
  const [priority, setPriority] = useState("Standard");
  const [pending, setPending] = useState(false);
  const [actionError, setActionError] = useState("");
  const previousStatus = useRef(null);
  const returnFocus = useRef(null);

  useEffect(() => {
    if (task) setAgent(Math.max(0, Math.min(3, task.step)));
  }, [task?.id, task?.step]);
  useEffect(() => {
    if (task?.status === "completed" && previousStatus.current === "running")
      setNotice("Inspection complete. Your report has been saved.");
    previousStatus.current = task?.status || null;
  }, [task?.id, task?.status]);

  function openModal(name, event) {
    returnFocus.current = event?.currentTarget || document.activeElement;
    setActionError("");
    setModal(name);
  }
  function closeModal() {
    setModal(null);
    setActionError("");
    requestAnimationFrame(() => returnFocus.current?.focus());
  }
  async function addTask(event) {
    event.preventDefault();
    if (!title.trim() || !objective.trim()) {
      setActionError("Enter a task name and objective.");
      return;
    }
    setPending(true);
    setActionError("");
    try {
      const created = await apiRequest("/api/tasks", {
        method: "POST",
        body: JSON.stringify({
          name: title.trim(),
          objective: objective.trim(),
          priority,
        }),
        signal: AbortSignal.timeout(10000),
      });
      acceptTask(created);
      setAgent(0);
      closeModal();
      setTitle("");
      setObjective("");
      setPriority("Standard");
      setNotice("Task saved. Run the inspection when you are ready.");
    } catch (error) {
      setActionError(error.message + " Check the task list before retrying.");
      refresh();
    } finally {
      setPending(false);
    }
  }
  async function runInspection() {
    if (!task || pending) return;
    setPending(true);
    setActionError("");
    try {
      const started = await apiRequest(
        "/api/tasks/" + encodeURIComponent(task.id) + "/runs",
        {
          method: "POST",
          body: "{}",
          signal: AbortSignal.timeout(10000),
        },
      );
      acceptTask(started);
      setAgent(0);
      setNotice(
        "Inspection started. Progress is coming from the local worker.",
      );
    } catch (error) {
      setActionError(error.message);
      refresh();
    } finally {
      setPending(false);
    }
  }
  const currentRole = !task
    ? "Ready"
    : task.status === "completed"
      ? "Complete"
      : task.status === "failed"
        ? "Failed"
        : task.status === "interrupted"
          ? "Interrupted"
          : task.step < 0
            ? "Ready"
            : roles[task.step];
  const stageCopy = !task
    ? activeId
      ? "Loading task…"
      : "Create your first inspection task"
    : task.status === "completed"
      ? "Project report saved"
      : task.error ||
        (task.step < 0
          ? "Ready to inspect this project"
          : task.stages[task.step]?.work);
  const canStart =
    task && ["ready", "failed", "interrupted"].includes(task.status);
  const selectedStage = task?.stages[agent];

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a
          className="wordmark"
          aria-label="CADENTRIX"
          href="#"
          onClick={(event) => {
            event.preventDefault();
            closeModal();
          }}
        >
          CADENTRIX
        </a>
        <nav aria-label="Main navigation">
          {[
            { label: "Tasks", icon: Stack },
            { label: "Agents", icon: Users },
            { label: "Activity", icon: Pulse },
            { label: "Policies", icon: Shield },
          ].map(({ label, icon: Icon }) => (
            <button
              key={label}
              className={
                "nav-item " + ((modal || "Tasks") === label ? "active" : "")
              }
              disabled={["Agents", "Activity"].includes(label) && !task}
              onClick={(event) =>
                label === "Tasks" ? closeModal() : openModal(label, event)
              }
              aria-current={(modal || "Tasks") === label ? "page" : undefined}
            >
              <Icon size={26} weight="regular" />
              <span>{label}</span>
            </button>
          ))}
        </nav>
      </aside>
      <main aria-busy={!workspace && !connectionError}>
        <header className="workspace-header">
          <span>
            {connected
              ? "Local workspace · Backend connected"
              : workspace
                ? "Local workspace · Disconnected"
                : connectionError
                  ? "Local backend unavailable"
                  : "Connecting to local backend…"}
          </span>
          <time dateTime={workspace?.server_time}>
            {formatDate(workspace?.server_time)}{" "}
            <span className="timezone">(America/Toronto)</span>
          </time>
        </header>
        <h1>Trace the work</h1>
        {connectionError && (
          <div className="connection-banner" role="alert">
            <p>
              {connectionError} Saved work remains in SQLite; actions are
              unavailable until reconnected.
            </p>
            <button className="secondary-button" onClick={refresh}>
              <ArrowClockwise size={18} />
              Retry connection
            </button>
          </div>
        )}
        {actionError && modal !== "new" && (
          <p className="error-message" role="alert">
            {actionError}
          </p>
        )}
        <div className="task-toolbar">
          <div className="select-wrap">
            <select
              aria-label="Select task"
              value={activeId || ""}
              disabled={!tasks.length}
              onChange={(event) => {
                setActionError("");
                selectTask(event.target.value);
              }}
            >
              {!tasks.length && (
                <option value="">
                  {workspace ? "No tasks yet" : "Loading tasks…"}
                </option>
              )}
              {tasks.map((item) => (
                <option value={item.id} key={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
            <CaretDown size={23} aria-hidden="true" />
          </div>
          <button
            className="primary-button"
            disabled={!connected || pending}
            onClick={(event) => openModal("new", event)}
          >
            <Plus size={27} />
            New task
          </button>
        </div>
        <section className="task-overview" aria-label="Task overview">
          <div>
            <span className="field-label">Task objective</span>
            <p>
              {task?.objective ||
                (activeId
                  ? "Loading saved task…"
                  : "Create a task to inspect the structure of " +
                    (workspace?.workspace_name || "this project") +
                    ".")}
            </p>
          </div>
          <div>
            <span className="field-label">Current stage</span>
            <p>
              <strong>{currentRole}</strong>
              <span> — {stageCopy}</span>
            </p>
          </div>
        </section>
        <section className="execution" aria-label="Agent handoffs">
          <div className="execution-map">
            {roles.map((role, index) => {
              const state = stateFor(task, index);
              return (
                <div className="map-item" key={role}>
                  <button
                    className={"agent-node " + state}
                    disabled={!task}
                    aria-label={role + ": " + statusLabel(task, index)}
                    aria-pressed={agent === index}
                    onClick={() => setAgent(index)}
                  >
                    <StateIcon state={state} />
                    <strong>{role}</strong>
                    <span>{statusLabel(task, index)}</span>
                  </button>
                  {index < 3 && (
                    <div className="handoff-connector" aria-hidden="true">
                      <span />
                      <ArrowRight size={23} weight="regular" />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
          <p className="dependency">
            Dependency: <span>Planner → Builder → Verifier → Reviewer</span>{" "}
            <span>(sequential handoff)</span>
          </p>
        </section>
        <section className="agent-details" aria-labelledby="agent-heading">
          <h2 id="agent-heading">
            {roles[agent]} <span>(selected)</span>
          </h2>
          <dl>
            <dt>Work</dt>
            <dd>{selectedStage?.work || "Waiting for a task"}</dd>
            <dt>Tools</dt>
            <dd>{selectedStage?.tools || "Read-only project inspection"}</dd>
            <dt>ARC</dt>
            <dd>
              Host telemetry: {policy?.cpu_telemetry || "unknown"}
              <small>
                {policy
                  ? policy.active_runs +
                    " of " +
                    policy.max_concurrent_runs +
                    " local worker slots in use · Fixed admission limit"
                  : "Waiting for backend policy"}
              </small>
            </dd>
          </dl>
          <div className="simulation-controls inspection-controls">
            {canStart && (
              <button
                className="secondary-button"
                disabled={!connected || pending}
                onClick={runInspection}
              >
                <PlayCircle size={20} />
                {pending
                  ? "Starting inspection…"
                  : task.status === "ready"
                    ? "Run inspection"
                    : "Retry inspection"}
              </button>
            )}
            {task?.status === "running" && (
              <p className="running-note" role="status">
                <Pulse size={20} />
                Inspection running · Live worker state
              </p>
            )}
            {task?.status === "completed" && (
              <p className="completed-note">
                <Check size={20} />
                Inspection complete
              </p>
            )}
            {task?.report && (
              <button
                className="secondary-button"
                onClick={(event) => openModal("report", event)}
              >
                <FileText size={20} />
                View report
              </button>
            )}
            {task?.report && (
              <a
                className={
                  "secondary-button download-link" +
                  (!connected ? " unavailable" : "")
                }
                href={
                  connected
                    ? "/api/tasks/" + encodeURIComponent(task.id) + "/report"
                    : undefined
                }
                aria-disabled={!connected}
                download="project-inspection.md"
              >
                <DownloadSimple size={20} />
                Download report
              </a>
            )}
            <span>Native read-only worker · No AI model connected</span>
          </div>
        </section>
        <section
          className="activity-section"
          aria-labelledby="activity-heading"
        >
          <h2 id="activity-heading">Activity timeline</h2>
          <Timeline events={task?.events} />
        </section>
      </main>
      <div className="sr-only" role="status" aria-live="polite">
        {notice}
      </div>
      {modal === "new" && (
        <Modal
          title="Create an inspection task"
          onClose={closeModal}
          busy={pending}
        >
          <p className="modal-intro">
            Inspect the structure of {workspace?.workspace_name}. The worker
            reads file metadata and saves a project report.
          </p>
          <form onSubmit={addTask}>
            <label>
              Task name
              <input
                autoFocus
                required
                maxLength={80}
                value={title}
                disabled={pending}
                onChange={(event) => setTitle(event.target.value)}
                placeholder="e.g. Inspect project structure"
              />
            </label>
            <label>
              Objective
              <textarea
                required
                maxLength={240}
                rows={3}
                value={objective}
                disabled={pending}
                onChange={(event) => setObjective(event.target.value)}
                placeholder="What do you want this inventory to document?"
              />
            </label>
            <label>
              Priority
              <select
                value={priority}
                disabled={pending}
                onChange={(event) => setPriority(event.target.value)}
              >
                <option>Standard</option>
                <option>Urgent</option>
                <option>Background</option>
              </select>
            </label>
            <div className="form-note">
              <Info size={19} />
              <span>
                Job type: project inspection. Your objective labels the report;
                it does not execute instructions. Priority is recorded; starts
                are explicit.
              </span>
            </div>
            {actionError && (
              <p className="error-message" role="alert">
                {actionError}
              </p>
            )}
            <div className="modal-actions">
              <button
                type="button"
                className="secondary-button"
                disabled={pending}
                onClick={closeModal}
              >
                Cancel
              </button>
              <button
                className="primary-button"
                type="submit"
                disabled={pending || !connected}
              >
                <Plus size={20} />
                {pending ? "Saving task…" : "Create task"}
              </button>
            </div>
          </form>
        </Modal>
      )}
      {modal === "Agents" && task && (
        <Modal title="Workers in this task" onClose={closeModal}>
          <p className="modal-intro">
            Four native worker stages execute this inspection. No AI model is
            involved.
          </p>
          <div className="agent-list">
            {task.stages.map((stage, index) => (
              <button
                key={stage.role}
                onClick={() => {
                  setAgent(index);
                  closeModal();
                }}
              >
                <StateIcon state={stage.state} small />
                <span>
                  <strong>{stage.role}</strong>
                  <small>{stage.work}</small>
                </span>
                <ArrowRight size={20} />
              </button>
            ))}
          </div>
        </Modal>
      )}
      {modal === "Activity" && task && (
        <Modal title="Task activity" onClose={closeModal} wide>
          <p className="modal-intro">
            {task.name} · Recorded by the backend · Times in America/Toronto
          </p>
          <Timeline events={task.events} />
        </Modal>
      )}
      {modal === "Policies" && (
        <Modal title="Local execution boundaries" onClose={closeModal}>
          <p className="modal-intro">
            Backend configuration for{" "}
            {workspace?.workspace_name || "the local workspace"}. These are
            local service limits.
          </p>
          <dl className="policy-list">
            <dt>Concurrent capacity</dt>
            <dd>{policy?.max_concurrent_runs ?? "Unknown"} native worker</dd>
            <dt>Handoffs</dt>
            <dd>Sequential · One active owner per task</dd>
            <dt>Priority scheduling</dt>
            <dd>Priority recorded · Explicit starts</dd>
            <dt>CPU / memory</dt>
            <dd>
              {policy?.cpu_telemetry || "unknown"} /{" "}
              {policy?.memory_telemetry || "unknown"}
            </dd>
            <dt>Tool access</dt>
            <dd>Project directory and file metadata only</dd>
            <dt>Restart recovery</dt>
            <dd>Interrupted runs require an explicit retry</dd>
            <dt>Model providers</dt>
            <dd>No AI models connected</dd>
          </dl>
          <p className="policy-note">
            <Shield size={20} />
            The worker cannot run shell commands, change project files or choose
            a different workspace. The full ARC controller is not connected yet.
          </p>
        </Modal>
      )}
      {modal === "report" && task?.report && (
        <Modal title="Project inspection report" onClose={closeModal} wide>
          <p className="modal-intro">
            {task.report.workspace_name} ·{" "}
            {task.report.file_count.toLocaleString()} files ·{" "}
            {task.report.partial
              ? "Partial bounded inventory"
              : "Within configured scope"}{" "}
            · Saved {formatDate(task.report.generated_at)}
          </p>
          <pre
            className="report-content"
            tabIndex={0}
            aria-label="Saved project report"
          >
            {task.report.markdown}
          </pre>
          <div className="modal-actions">
            <button className="secondary-button" onClick={closeModal}>
              Close report
            </button>
            <a
              className={
                "secondary-button download-link" +
                (!connected ? " unavailable" : "")
              }
              href={
                connected
                  ? "/api/tasks/" + encodeURIComponent(task.id) + "/report"
                  : undefined
              }
              aria-disabled={!connected}
              download="project-inspection.md"
            >
              <DownloadSimple size={20} />
              Download report
            </a>
          </div>
        </Modal>
      )}
    </div>
  );
}
