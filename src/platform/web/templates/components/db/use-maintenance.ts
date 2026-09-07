import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";

/**
 * Compacting and WAL-syncing a local store.
 *
 * Only engines that declare the maintenance capability route here; the hook
 * is inert without an API to call, so the page can hold it unconditionally.
 */
export function useMaintenance(maintenanceApi) {
  const [health, setHealth] = useState(null);
  const [report, setReport] = useState(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("Idle");
  const [pendingAction, setPendingAction] = useState("");

  async function reloadHealth({ silent = false } = {}) {
    if (!maintenanceApi) return null;
    if (!silent) {
      setBusy(true);
      setStatus("Checking store…");
    }
    try {
      const payload = await requestJson(`${maintenanceApi}/health`);
      const next = payload?.health || null;
      setHealth(next);
      setStatus(next ? `Checked · ${Number(next?.duration_ms || 0)} ms` : "No health data returned");
      return next;
    } catch (error) {
      setStatus(`Error · ${String(error?.message || error)}`);
      return null;
    } finally {
      if (!silent) setBusy(false);
    }
  }

  async function runOperation(operation) {
    if (!maintenanceApi) return;
    setBusy(true);
    setReport(null);
    setStatus(operation === "compact" ? "Compacting store…" : "Syncing WAL…");
    try {
      const payload = await requestJson(`${maintenanceApi}/${operation}`, { method: "POST" });
      const next = payload?.[operation] || null;
      setReport(next);
      setHealth(next?.after || null);
      setStatus(`${operation === "compact" ? "Compacted" : "Synced"} · ${Number(next?.duration_ms || 0)} ms`);
    } catch (error) {
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
      setPendingAction("");
    }
  }

  return { health, report, busy, status, pendingAction, setPendingAction, reloadHealth, runOperation };
}
