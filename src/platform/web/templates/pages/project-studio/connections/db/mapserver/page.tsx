import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { useEffect, useState } from "zeb";

export const page = {
  head: {
    links: [{ rel: "stylesheet", href: "/assets/platform/db-suite.css" }],
  },
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}

function requestJson(url, options = {}) {
  return fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  }).then(async (response) => {
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      throw new Error(payload?.error || payload?.message || `${response.status} ${response.statusText}`);
    }
    return payload;
  });
}

export default function Page(input) {
  const navLinks = input?.nav?.links ?? {};
  const suiteTabs = Array.isArray(input?.suite_tabs) ? input.suite_tabs : [];
  const tabFlags = input?.tab_flags ?? {};
  const connection = input?.connection ?? {};
  const api = input?.mapserver_api ?? {};
  const initialLayers = Array.isArray(input?.layers) ? input.layers : [];
  const initialSources = Array.isArray(input?.sources) ? input.sources : [];
  const [layers, setLayers] = useState(initialLayers);
  const [sources, setSources] = useState(initialSources);
  const [status, setStatus] = useState("Ready");
  const [uploading, setUploading] = useState(false);
  const [pendingDelete, setPendingDelete] = useState(null);
  const [form, setForm] = useState({
    layer_id: "",
    path: "",
    source_path: initialSources[0]?.path || "",
    min_zoom: "",
    max_zoom: "",
    bbox_required: true,
    max_features: 1000,
    allowed_properties_csv: "",
  });

  async function refresh() {
    const [layerRes, sourceRes] = await Promise.all([
      requestJson(api.layers),
      requestJson(api.sources),
    ]);
    setLayers(Array.isArray(layerRes?.items) ? layerRes.items : []);
    setSources(Array.isArray(sourceRes?.items) ? sourceRes.items : []);
  }

  useEffect(() => {
    refresh().catch((err) => setStatus(String(err?.message || err)));
  }, []);

  async function onUpload(event) {
    const file = event?.target?.files?.[0];
    if (!file) return;
    const formData = new FormData();
    formData.append("file", file);
    setUploading(true);
    setStatus("Uploading...");
    try {
      const response = await fetch(api.upload, { method: "POST", body: formData });
      const payload = await response.json().catch(() => null);
      if (!response.ok) throw new Error(payload?.error || `${response.status} ${response.statusText}`);
      await refresh();
      setForm((prev) => ({ ...prev, source_path: payload?.path || prev.source_path }));
      setStatus(`Uploaded ${payload?.path || file.name}`);
    } catch (err) {
      setStatus(String(err?.message || err));
    } finally {
      setUploading(false);
      if (event?.target) event.target.value = "";
    }
  }

  async function publishLayer(event) {
    event?.preventDefault?.();
    setStatus("Publishing...");
    try {
      const payload = await requestJson(api.layers, {
        method: "POST",
        body: JSON.stringify({
          layer_id: form.layer_id,
          path: form.path,
          source_path: form.source_path,
          min_zoom: form.min_zoom === "" ? null : Number(form.min_zoom),
          max_zoom: form.max_zoom === "" ? null : Number(form.max_zoom),
          bbox_required: !!form.bbox_required,
          max_features: Number(form.max_features || 1000),
          allowed_properties: String(form.allowed_properties_csv || "")
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean),
        }),
      });
      await refresh();
      setStatus(`Published ${payload?.item?.layer_id || form.layer_id}`);
    } catch (err) {
      setStatus(String(err?.message || err));
    }
  }

  async function removeLayer(layerId) {
    setStatus(`Deleting ${layerId}...`);
    try {
      await requestJson(`${api.layers}/${encodeURIComponent(layerId)}`, { method: "DELETE" });
      await refresh();
      setPendingDelete(null);
      setStatus(`Deleted ${layerId}`);
    } catch (err) {
      setStatus(String(err?.message || err));
    }
  }

  return (
    <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu={`Databases / ${connection.slug || "mapserver"}`}
        owner={input.owner}
        project={input.project}
        nav={input.nav}
      >
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
          <StudioTabNav>
            <StudioTabLink href={navLinks.db_connections ?? "#"}>Connections</StudioTabLink>
            {suiteTabs.map((item, index) => (
              <StudioTabLink key={`${item?.href ?? "tab"}-${index}`} href={item?.href ?? "#"} active={item?.classes === "is-active"}>
                {item?.label}
              </StudioTabLink>
            ))}
          </StudioTabNav>
          <section
            className="db-suite-page flex min-h-0 flex-1 flex-col overflow-auto bg-bg"
            data-db-suite="true"
            data-owner={input.owner}
            data-project={input.project}
            data-db-kind={connection.kind ?? ""}
            data-connection-slug={connection.slug ?? ""}
            data-connection-id={connection.id ?? ""}
          >
            <header className="db-suite-header">
              <p className="db-suite-panel-title">{connection.name}</p>
              <span className="project-inline-chip">
                <i className={`zf-devicon ${connection.icon_class || ""}`} aria-hidden="true"></i>
                <span>kind: {connection.kind} | slug: {connection.slug}</span>
              </span>
            </header>

            <section className="db-suite-shell">
              <div className="db-suite-main">
                <section className="db-suite-panel db-suite-panel-fill">
                  <div className="space-y-4 p-4 md:p-5">
                    <div className="db-suite-side-actions">
                      <div>
                        <p className="db-suite-side-title">Status</p>
                        <p className="text-sm text-ui-text-soft">{status}</p>
                      </div>
                      <Button type="button" variant="outline" onClick={() => refresh().then(() => setStatus("Refreshed")).catch((err) => setStatus(String(err?.message || err)))}>
                        Refresh
                      </Button>
                    </div>

                    {pendingDelete ? (
                      <div className="rounded-lg border border-rose-400/40 bg-rose-500/10 p-3 flex items-center justify-between gap-3">
                        <div>
                          <p className="text-sm font-medium text-rose-100">Delete layer</p>
                          <p className="text-xs text-rose-100/80">Remove <code>{pendingDelete}</code> from this mapserver?</p>
                        </div>
                        <div className="flex items-center gap-2">
                          <Button type="button" variant="ghost" size="sm" onClick={() => setPendingDelete(null)}>
                            Cancel
                          </Button>
                          <Button type="button" variant="outline" size="sm" onClick={() => removeLayer(pendingDelete)}>
                            Delete
                          </Button>
                        </div>
                      </div>
                    ) : null}

                    {tabFlags?.layers ? (
                      <StudioTable>
                        <StudioThead>
                          <tr>
                            <StudioTh>Layer</StudioTh>
                            <StudioTh>Path</StudioTh>
                            <StudioTh>Zoom</StudioTh>
                            <StudioTh>Source</StudioTh>
                            <StudioTh>Max</StudioTh>
                            <StudioTh>Action</StudioTh>
                          </tr>
                        </StudioThead>
                        <tbody>
                          {layers.map((item, index) => (
                            <tr key={`${item?.layer_id ?? "layer"}-${index}`}>
                              <StudioTd>{item?.layer_id}</StudioTd>
                              <StudioTd>
                                <a href={`${api.base_public}${item?.path || ""}`} target="_blank">
                                  {item?.path}
                                </a>
                              </StudioTd>
                              <StudioTd>
                                {(item?.min_zoom ?? item?.max_zoom ?? null) == null
                                  ? "all"
                                  : `${item?.min_zoom ?? 0} - ${item?.max_zoom ?? "∞"}`}
                              </StudioTd>
                              <StudioTd>{item?.source_path}</StudioTd>
                              <StudioTd>{item?.max_features}</StudioTd>
                              <StudioTd>
                                <Button type="button" variant="ghost" size="sm" onClick={() => setPendingDelete(item?.layer_id)}>
                                  Delete
                                </Button>
                              </StudioTd>
                            </tr>
                          ))}
                          {!layers.length ? (
                            <tr>
                              <StudioTd colSpan={6}>No published layers yet.</StudioTd>
                            </tr>
                          ) : null}
                        </tbody>
                      </StudioTable>
                    ) : null}

                    {tabFlags?.publish ? (
                      <div className="space-y-4">
                        <div className="rounded-lg border border-ui-border/80 p-4">
                          <div className="db-suite-side-actions mb-3">
                            <p className="db-suite-side-title">Upload GeoJSON</p>
                          </div>
                          <input type="file" accept=".geojson,.json,application/geo+json,application/json" onChange={onUpload} disabled={uploading} />
                          <p className="text-xs text-ui-text-muted mt-2">Files are stored under <code>files/private/mapserver</code>.</p>
                        </div>
                        <form className="rounded-lg border border-ui-border/80 p-4 space-y-3" onSubmit={publishLayer}>
                          <div className="db-suite-side-actions">
                            <div>
                              <p className="db-suite-side-title">Publish Layer</p>
                              <p className="text-xs text-ui-text-muted">Bind a private GeoJSON source into a public mapserver layer.</p>
                            </div>
                          </div>
                          <Field label="Layer ID">
                            <Input value={form.layer_id} onInput={(e) => setForm((prev) => ({ ...prev, layer_id: e.target.value }))} placeholder="admin_province" />
                          </Field>
                          <Field label="Public Path">
                            <Input value={form.path} onInput={(e) => setForm((prev) => ({ ...prev, path: e.target.value }))} placeholder="/layers/admin-province" />
                          </Field>
                          <Field label="Source GeoJSON">
                            <select className="w-full rounded-md border border-ui-border bg-ui-bg px-3 py-2" value={form.source_path} onChange={(e) => setForm((prev) => ({ ...prev, source_path: e.target.value }))}>
                              <option value="">Select source file</option>
                              {sources.map((item, index) => (
                                <option key={`${item?.path ?? "src"}-${index}`} value={item?.path}>{item?.path}</option>
                              ))}
                            </select>
                          </Field>
                          <div className="grid gap-3 md:grid-cols-2">
                            <Field label="Min Zoom">
                              <Input type="number" value={String(form.min_zoom)} onInput={(e) => setForm((prev) => ({ ...prev, min_zoom: e.target.value }))} placeholder="10" />
                            </Field>
                            <Field label="Max Zoom">
                              <Input type="number" value={String(form.max_zoom)} onInput={(e) => setForm((prev) => ({ ...prev, max_zoom: e.target.value }))} placeholder="14" />
                            </Field>
                          </div>
                          <div className="grid gap-3 md:grid-cols-2">
                            <Field label="Max Features">
                              <Input type="number" value={String(form.max_features)} onInput={(e) => setForm((prev) => ({ ...prev, max_features: e.target.value }))} />
                            </Field>
                            <Field label="Allowed Properties">
                              <Input value={form.allowed_properties_csv} onInput={(e) => setForm((prev) => ({ ...prev, allowed_properties_csv: e.target.value }))} placeholder="ADM1_EN,ADM1_PCODE" />
                            </Field>
                          </div>
                          <label className="inline-flex items-center gap-2 text-sm">
                            <input type="checkbox" checked={!!form.bbox_required} onChange={(e) => setForm((prev) => ({ ...prev, bbox_required: !!e.target.checked }))} />
                            <span>Require bbox</span>
                          </label>
                          <div>
                            <Button type="submit">Publish Layer</Button>
                          </div>
                        </form>
                      </div>
                    ) : null}

                    {tabFlags?.test ? (
                      <StudioTable>
                        <StudioThead>
                          <tr>
                            <StudioTh>Layer</StudioTh>
                            <StudioTh>Example URL</StudioTh>
                          </tr>
                        </StudioThead>
                        <tbody>
                          {layers.map((item, index) => (
                            <tr key={`${item?.layer_id ?? "test"}-${index}`}>
                              <StudioTd>{item?.layer_id}</StudioTd>
                              <StudioTd>
                                <a href={`${api.base_public}${item?.path || ""}?bbox=95,-11,141,6&limit=25`} target="_blank">
                                  {`${api.base_public}${item?.path || ""}?bbox=95,-11,141,6&limit=25`}
                                </a>
                              </StudioTd>
                            </tr>
                          ))}
                          {!layers.length ? (
                            <tr>
                              <StudioTd colSpan={2}>Publish a layer first.</StudioTd>
                            </tr>
                          ) : null}
                        </tbody>
                      </StudioTable>
                    ) : null}
                  </div>
                </section>
              </div>
            </section>
          </section>
        </div>
    </ProjectStudioShell>
  );
}
