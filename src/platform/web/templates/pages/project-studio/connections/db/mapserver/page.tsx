import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Button from "@/components/ui/button";
import Badge from "@/components/ui/badge";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import DeckMap from "zeb/deckgl";
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

function formatNumber(n) {
  if (n == null) return "—";
  if (typeof n === "number") return n.toLocaleString();
  return String(n);
}

function LayerDetail({ layer, api, onBack }) {
  const [stats, setStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [previewData, setPreviewData] = useState(null);

  const layerPublicUrl = layer?.path
    ? `${api.base_public}${layer.path.startsWith("/") ? "" : "/"}${layer.path}`
    : null;

  useEffect(() => {
    if (!layerPublicUrl) return;
    setLoading(true);
    setError(null);
    requestJson(`${layerPublicUrl}/stats`)
      .then((data) => { setStats(data); setLoading(false); })
      .catch((err) => { setError(String(err?.message || err)); setLoading(false); });
  }, [layerPublicUrl]);

  useEffect(() => {
    if (!layerPublicUrl) return;
    const featureUrl = `${layerPublicUrl}?limit=500&bbox=-180,-90,180,90`;
    fetch(featureUrl, { headers: { Accept: "application/json" } })
      .then((r) => r.json())
      .then((geojson) => { if (geojson?.features) setPreviewData(geojson); })
      .catch(() => {});
  }, [layerPublicUrl]);

  const columns = stats?.columns ?? [];
  const geoCol = columns.find((c) => c.data_type === "geometry" || c.name === "geometry" || c.name === "geom");
  const attrColumns = columns.filter((c) => c !== geoCol);

  const mapLayers = [];
  if (previewData?.features?.length) {
    const pts = [];
    const polys = [];
    const lines = [];
    for (const f of previewData.features) {
      const gt = f?.geometry?.type;
      if (gt === "Point" || gt === "MultiPoint") pts.push(f);
      else if (gt === "LineString" || gt === "MultiLineString") lines.push(f);
      else polys.push(f);
    }
    if (polys.length) {
      mapLayers.push({
        type: "GeoJsonLayer",
        id: "polygons",
        data: { type: "FeatureCollection", features: polys },
        getFillColor: [59, 130, 246, 60],
        getLineColor: [59, 130, 246, 200],
        getLineWidth: 1,
        lineWidthMinPixels: 1,
        pickable: true,
      });
    }
    if (lines.length) {
      mapLayers.push({
        type: "GeoJsonLayer",
        id: "lines",
        data: { type: "FeatureCollection", features: lines },
        getLineColor: [234, 179, 8, 220],
        getLineWidth: 2,
        lineWidthMinPixels: 1,
        pickable: true,
      });
    }
    if (pts.length) {
      mapLayers.push({
        type: "GeoJsonLayer",
        id: "points",
        data: { type: "FeatureCollection", features: pts },
        getFillColor: [16, 185, 129, 200],
        getLineColor: [255, 255, 255, 180],
        pointRadiusMinPixels: 4,
        pointRadiusMaxPixels: 12,
        getPointRadius: 100,
        lineWidthMinPixels: 1,
        pickable: true,
      });
    }
  }

  let initialView = { longitude: 106.85, latitude: -6.2, zoom: 5 };
  if (previewData?.features?.length) {
    let minLon = 180, maxLon = -180, minLat = 90, maxLat = -90;
    for (const f of previewData.features) {
      const coords = extractCoords(f?.geometry);
      for (const [lon, lat] of coords) {
        if (lon < minLon) minLon = lon;
        if (lon > maxLon) maxLon = lon;
        if (lat < minLat) minLat = lat;
        if (lat > maxLat) maxLat = lat;
      }
    }
    if (minLon <= maxLon && minLat <= maxLat) {
      initialView = {
        longitude: (minLon + maxLon) / 2,
        latitude: (minLat + maxLat) / 2,
        zoom: estimateZoom(maxLon - minLon, maxLat - minLat),
      };
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <Button type="button" variant="ghost" size="sm" onClick={onBack}>
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" className="w-4 h-4">
            <path fillRule="evenodd" d="M9.78 4.22a.75.75 0 0 1 0 1.06L7.06 8l2.72 2.72a.75.75 0 1 1-1.06 1.06L5.22 8.53a.75.75 0 0 1 0-1.06l3.5-3.5a.75.75 0 0 1 1.06 0Z" clipRule="evenodd" />
          </svg>
          Back to layers
        </Button>
      </div>

      <div className="rounded-lg border border-ui-border/80 p-4">
        <div className="flex items-start justify-between gap-4 mb-3">
          <div>
            <p className="text-base font-semibold text-ui-text">{layer.layer_id}</p>
            <p className="text-xs text-ui-text-muted mt-0.5">
              {layer.path}
            </p>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {stats?.source_kind ? <Badge variant="secondary" label={stats.source_kind} /> : null}
            {stats?.capabilities?.mvt ? <Badge variant="secondary" label="MVT" /> : null}
            {stats?.capabilities?.png ? <Badge variant="secondary" label="PNG" /> : null}
            {stats?.capabilities?.geojson ? <Badge variant="secondary" label="GeoJSON" /> : null}
          </div>
        </div>
        {loading ? (
          <p className="text-sm text-ui-text-soft">Loading stats...</p>
        ) : error ? (
          <p className="text-sm text-rose-400">{error}</p>
        ) : (
          <div className="grid grid-cols-3 gap-3 text-sm">
            <div className="rounded border border-ui-border/60 px-3 py-2">
              <p className="text-ui-text-muted text-[11px] uppercase tracking-wider">Rows</p>
              <p className="text-ui-text font-medium">{formatNumber(stats?.row_count)}</p>
            </div>
            <div className="rounded border border-ui-border/60 px-3 py-2">
              <p className="text-ui-text-muted text-[11px] uppercase tracking-wider">Fields</p>
              <p className="text-ui-text font-medium">{attrColumns.length}</p>
            </div>
            <div className="rounded border border-ui-border/60 px-3 py-2">
              <p className="text-ui-text-muted text-[11px] uppercase tracking-wider">Source</p>
              <p className="text-ui-text font-medium truncate">{layer.source_path || "—"}</p>
            </div>
          </div>
        )}
      </div>

      {!loading && !error && attrColumns.length > 0 ? (
        <div className="rounded-lg border border-ui-border/80">
          <div className="px-4 py-2.5 border-b border-ui-border/60">
            <p className="text-sm font-medium text-ui-text">Attributes</p>
          </div>
          <div className="overflow-x-auto">
            <StudioTable>
              <StudioThead>
                <tr>
                  <StudioTh>Field</StudioTh>
                  <StudioTh>Type</StudioTh>
                  <StudioTh>Cardinality</StudioTh>
                  <StudioTh>Min</StudioTh>
                  <StudioTh>Max</StudioTh>
                  <StudioTh>Nulls</StudioTh>
                  <StudioTh>Top Values</StudioTh>
                </tr>
              </StudioThead>
              <tbody>
                {attrColumns.map((col, i) => (
                  <tr key={`col-${col.name}-${i}`}>
                    <StudioTd>
                      <span className="font-mono text-xs">{col.name}</span>
                    </StudioTd>
                    <StudioTd>
                      <Badge variant="secondary" label={col.data_type || "unknown"} />
                    </StudioTd>
                    <StudioTd>{formatNumber(col.cardinality)}</StudioTd>
                    <StudioTd>{col.min != null ? String(col.min) : "—"}</StudioTd>
                    <StudioTd>{col.max != null ? String(col.max) : "—"}</StudioTd>
                    <StudioTd>{formatNumber(col.null_count)}</StudioTd>
                    <StudioTd>
                      <div className="flex flex-wrap gap-1 max-w-xs">
                        {(col.top_values || []).slice(0, 5).map(([val, count], j) => (
                          <span key={`tv-${i}-${j}`} className="inline-flex items-center gap-1 rounded bg-ui-bg-muted px-1.5 py-0.5 text-[11px] text-ui-text-soft">
                            <span className="truncate max-w-[100px]">{val}</span>
                            <span className="text-ui-text-muted">({count})</span>
                          </span>
                        ))}
                        {(col.top_values?.length || 0) > 5 ? (
                          <span className="text-[11px] text-ui-text-muted">+{col.top_values.length - 5}</span>
                        ) : null}
                      </div>
                    </StudioTd>
                  </tr>
                ))}
              </tbody>
            </StudioTable>
          </div>
        </div>
      ) : null}

      <div className="rounded-lg border border-ui-border/80">
        <div className="px-4 py-2.5 border-b border-ui-border/60">
          <p className="text-sm font-medium text-ui-text">Preview</p>
        </div>
        <div className="p-0">
          {previewData?.features?.length ? (
            <DeckMap
              height="400px"
              initialViewState={initialView}
              layers={mapLayers}
              tooltip={true}
              controller={true}
            />
          ) : (
            <div className="flex items-center justify-center h-[200px] text-sm text-ui-text-muted">
              {previewData === null ? "Loading preview..." : "No features to preview"}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function extractCoords(geom) {
  if (!geom) return [];
  const t = geom.type;
  if (t === "Point") return [geom.coordinates];
  if (t === "MultiPoint" || t === "LineString") return geom.coordinates;
  if (t === "MultiLineString" || t === "Polygon") return geom.coordinates.flat();
  if (t === "MultiPolygon") return geom.coordinates.flat(2);
  if (t === "GeometryCollection") return (geom.geometries || []).flatMap(extractCoords);
  return [];
}

function estimateZoom(lonSpan, latSpan) {
  const span = Math.max(lonSpan, latSpan);
  if (span > 100) return 2;
  if (span > 50) return 3;
  if (span > 20) return 5;
  if (span > 10) return 6;
  if (span > 5) return 7;
  if (span > 2) return 8;
  if (span > 1) return 9;
  if (span > 0.5) return 10;
  if (span > 0.1) return 12;
  return 14;
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
  const [selectedLayer, setSelectedLayer] = useState(null);
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

                    {tabFlags?.layers && selectedLayer ? (
                      <LayerDetail layer={selectedLayer} api={api} onBack={() => setSelectedLayer(null)} />
                    ) : null}

                    {tabFlags?.layers && !selectedLayer ? (
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
                            <tr key={`${item?.layer_id ?? "layer"}-${index}`} className="cursor-pointer hover:bg-ui-bg-muted/50" onClick={() => setSelectedLayer(item)}>
                              <StudioTd>
                                <span className="font-medium text-blue-400 hover:underline">{item?.layer_id}</span>
                              </StudioTd>
                              <StudioTd>{item?.path}</StudioTd>
                              <StudioTd>
                                {(item?.min_zoom ?? item?.max_zoom ?? null) == null
                                  ? "all"
                                  : `${item?.min_zoom ?? 0} - ${item?.max_zoom ?? "∞"}`}
                              </StudioTd>
                              <StudioTd>{item?.source_path}</StudioTd>
                              <StudioTd>{item?.max_features}</StudioTd>
                              <StudioTd>
                                <Button type="button" variant="ghost" size="sm" onClick={(e) => { e.stopPropagation(); setPendingDelete(item?.layer_id); }}>
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
                          <p className="text-xs text-ui-text-muted mt-2">Files are stored under <code>mapserver/</code> in Zebflow FS.</p>
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
