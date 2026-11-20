import { useCallback, useEffect, useState } from "react";
import { apiRequest } from "./api.js";

export function useWorkspace() {
  const [workspace, setWorkspace] = useState(null);
  const [activeId, setActiveId] = useState(() =>
    new URLSearchParams(window.location.search).get("task"),
  );
  const [task, setTask] = useState(null);
  const [connected, setConnected] = useState(false);
  const [connectionError, setConnectionError] = useState("");
  const [refreshVersion, setRefreshVersion] = useState(0);
  const refresh = useCallback(
    () => setRefreshVersion((version) => version + 1),
    [],
  );

  useEffect(() => {
    const url = new URL(window.location.href);
    if (activeId) url.searchParams.set("task", activeId);
    else url.searchParams.delete("task");
    window.history.replaceState(null, "", url);
  }, [activeId]);

  useEffect(() => {
    let controller = null;
    let stopped = false;
    let loading = false;
    async function load() {
      if (loading) return;
      loading = true;
      controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 10000);
      try {
        const snapshot = await apiRequest("/api/workspace", {
          signal: controller.signal,
        });
        const selectedId = snapshot.tasks.some((item) => item.id === activeId)
          ? activeId
          : snapshot.tasks[0]?.id || null;
        const selectedTask = selectedId
          ? await apiRequest("/api/tasks/" + encodeURIComponent(selectedId), {
              signal: controller.signal,
            })
          : null;
        if (stopped) return;
        setWorkspace(snapshot);
        setTask(selectedTask);
        setConnected(true);
        setConnectionError("");
        if (selectedId !== activeId) setActiveId(selectedId);
      } catch (error) {
        if (stopped) return;
        setConnected(false);
        setConnectionError(
          error.name === "AbortError"
            ? "The local backend took too long to respond. Retry the connection."
            : "Cannot reach the local backend. " + error.message,
        );
      } finally {
        clearTimeout(timeout);
        loading = false;
      }
    }
    load();
    const timer = setInterval(load, 1200);
    return () => {
      stopped = true;
      clearInterval(timer);
      controller?.abort();
    };
  }, [activeId, refreshVersion]);

  function selectTask(id) {
    setTask(null);
    setActiveId(id);
  }
  function acceptTask(nextTask) {
    setTask(nextTask);
    setActiveId(nextTask.id);
    setWorkspace((previous) =>
      previous
        ? {
            ...previous,
            tasks: [
              { id: nextTask.id, name: nextTask.name, status: nextTask.status },
              ...previous.tasks.filter((item) => item.id !== nextTask.id),
            ],
          }
        : previous,
    );
    refresh();
  }
  return {
    workspace,
    activeId,
    task,
    connected,
    connectionError,
    selectTask,
    acceptTask,
    refresh,
  };
}
