import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Badge from "@/components/ui/badge";
import LayerAttributes from "@/pages/project-studio/connections/db/mapserver/components/layer-attributes";
import LayerPreview from "@/pages/project-studio/connections/db/mapserver/components/layer-preview";
import { formatNumber } from "@/pages/project-studio/connections/db/mapserver/components/format-number";

/** One published layer: its stats, its attribute columns and a map of it. */
export default function LayerDetail({ layer, api, onBack }) {
  const [stats, setStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  const layerPublicUrl = layer?.path
    ? `${api.base_public}${layer.path.startsWith("/") ? "" : "/"}${layer.path}`
    : null;

  // The public /stats endpoint reports only the columns this layer exposes.
  // Choosing what to expose needs the whole source, so the operator view is
  // read from the project API instead.
  const layerStatsUrl = layer?.layer_id
    ? `${api.layers}/${encodeURIComponent(layer.layer_id)}/stats`
    : null;

  useEffect(() => {
    if (!layerStatsUrl) return;
    setLoading(true);
    setError(null);
    requestJson(layerStatsUrl)
      .then((data) => { setStats(data); setLoading(false); })
      .catch((err) => { setError(String(err?.message || err)); setLoading(false); });
  }, [layerStatsUrl]);

  const columns = stats?.columns ?? [];
  const geoCol = columns.find((c) => c.data_type === "geometry" || c.name === "geometry" || c.name === "geom");
  const attrColumns = columns.filter((c) => c !== geoCol);
  const exposed = stats?.allowed_properties ?? layer?.allowed_properties ?? [];

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

      <div className="rounded-lg border border-border/80 p-4">
        <div className="flex items-start justify-between gap-4 mb-3">
          <div>
            <p className="text-base font-semibold text-foreground">{layer.layer_id}</p>
            <p className="text-xs text-muted-foreground mt-0.5">
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
          <p className="text-sm text-muted-foreground">Loading stats...</p>
        ) : error ? (
          <p className="text-sm text-rose-400">{error}</p>
        ) : (
          <div className="grid grid-cols-3 gap-3 text-sm">
            <div className="rounded border border-border/60 px-3 py-2">
              <p className="text-muted-foreground text-[11px] uppercase tracking-wider">Rows</p>
              <p className="text-foreground font-medium">{formatNumber(stats?.row_count)}</p>
            </div>
            <div className="rounded border border-border/60 px-3 py-2">
              <p className="text-muted-foreground text-[11px] uppercase tracking-wider">Fields</p>
              <p className="text-foreground font-medium">{attrColumns.length}</p>
            </div>
            <div className="rounded border border-border/60 px-3 py-2">
              <p className="text-muted-foreground text-[11px] uppercase tracking-wider">Source</p>
              <p className="text-foreground font-medium truncate">{layer.source_path || "—"}</p>
            </div>
            <div className="rounded border border-border/60 px-3 py-2 col-span-3">
              <p className="text-muted-foreground text-[11px] uppercase tracking-wider">Public properties</p>
              {exposed.length ? (
                <div className="flex flex-wrap gap-1 mt-1">
                  {exposed.map((name) => (
                    <Badge key={`exposed-${name}`} variant="secondary" label={name} />
                  ))}
                </div>
              ) : (
                <p className="text-foreground font-medium">Geometry only — no properties are served</p>
              )}
            </div>
          </div>
        )}
      </div>

      {!loading && !error && attrColumns.length > 0 ? (
        <LayerAttributes columns={attrColumns} exposed={exposed} />
      ) : null}

      <LayerPreview layer={layer} publicUrl={layerPublicUrl} />
    </div>
  );
}
