import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import AddressingHosts from "@/pages/project-studio/settings/components/addressing/addressing-hosts";
import AddressingRoutes from "@/pages/project-studio/settings/components/addressing/addressing-routes";
import AddressingConfigs from "@/pages/project-studio/settings/components/addressing/addressing-configs";

/**
 * Settings → Addressing (`docs/contracts/addressing.md` §6; the reference
 * implementation of `ux.md` §4: why, inputs, output, verify).
 *
 * Owns the section data and every change to it. Each change is saved at
 * once — hosts and routes are instance configuration, not `zebflow.yaml`, so
 * there is no commit step — and the server answers with the regenerated
 * section, which is what the panels below render.
 */
export default function AddressingPanel({ api, checkApi, initialData }) {
  const [data, setData] = useState(initialData ?? {});
  const [checks, setChecks] = useState({});
  const [checking, setChecking] = useState("");
  const [busy, setBusy] = useState(false);
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");

  const config = data?.config ?? { hosts: [], routes: [], disabled: [] };
  const hosts = Array.isArray(config.hosts) ? config.hosts : [];
  const routes = Array.isArray(config.routes) ? config.routes : [];
  const disabled = Array.isArray(config.disabled) ? config.disabled : [];

  async function save(next, done) {
    setBusy(true);
    try {
      const resp = await requestJson(api, { method: "PUT", body: JSON.stringify({ data: next }) });
      if (resp?.ok === false) throw new Error(resp?.error?.message ?? resp?.error ?? "save failed");
      setData(resp?.data ?? data);
      setStatusMsg(done);
      setStatusTone("ok");
    } catch (err) {
      setStatusMsg(err?.message || String(err));
      setStatusTone("error");
    } finally {
      setBusy(false);
    }
  }

  function addHost(host) {
    if (hosts.includes(host)) return;
    save({ ...config, hosts: [...hosts, host] }, `Added ${host}. Point its DNS at this instance, paste the server configuration below, then Verify.`);
  }

  function removeHost(host) {
    const ok = typeof window === "undefined" ? true : window.confirm(`Remove ${host}? Browsers on it get 404 from this instance until you add it back; routes on it are removed too.`);
    if (!ok) return;
    save(
      { ...config, hosts: hosts.filter((name) => name !== host), routes: routes.filter((r) => r.host !== host) },
      `Removed ${host}.`,
    );
  }

  function addRoute(route) {
    save({ ...config, routes: [...routes, route] }, `Route added: ${route.host}${route.path} → ${route.surface}.`);
  }

  function removeRoute(index) {
    save({ ...config, routes: routes.filter((_, i) => i !== index) }, "Route removed.");
  }

  function toggleSurface(key) {
    const next = disabled.includes(key) ? disabled.filter((k) => k !== key) : [...disabled, key];
    save({ ...config, disabled: next }, disabled.includes(key) ? `${key} switched on.` : `${key} switched off — it answers 404 on every host now.`);
  }

  async function check(host) {
    setChecking(host);
    try {
      const resp = await requestJson(checkApi, { method: "POST", body: JSON.stringify({ host }) });
      setChecks({ ...checks, [host]: resp });
      setStatusMsg(resp?.verify?.ok ? `${host} answers from this project.` : `${host}: ${resp?.verify?.reason ?? "not reachable yet"}`);
      setStatusTone(resp?.verify?.ok ? "ok" : "error");
    } catch (err) {
      setStatusMsg(err?.message || String(err));
      setStatusTone("error");
    } finally {
      setChecking("");
    }
  }

  return (
    <SettingsSection
      id="addressing"
      title="Addressing"
      description="Where this project answers. The project never stores its own address — hosts live on this instance, so the project moves between machines as it is."
      tag="instance"
    >
      <div className="flex flex-col gap-8">
        <AddressingHosts
          data={data}
          hosts={hosts}
          checks={checks}
          checking={checking}
          onAdd={addHost}
          onRemove={removeHost}
          onCheck={check}
          busy={busy}
        />
        <AddressingRoutes
          data={data}
          hosts={hosts}
          routes={routes}
          disabled={disabled}
          onAddRoute={addRoute}
          onRemoveRoute={removeRoute}
          onToggleSurface={toggleSurface}
          busy={busy}
        />
        <AddressingConfigs configs={data?.configs ?? []} />
        <p className={`text-[0.76rem] ${settingsStatusToneClass(statusTone)}`} aria-live="polite">{statusMsg}</p>
      </div>
    </SettingsSection>
  );
}
