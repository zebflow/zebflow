import { useEffect, useMemo, useState } from "zeb/react";
import { data } from "zeb/world";
import StudioMap, { geometryLayers } from "@/components/ui/studio-map";
import { geometryBox } from "@/components/lib/geometry-box";

/**
 * A published layer drawn from MapServer's own vector tiles
 * (`{public url}/{z}/{x}/{y}.mvt`, the layer's zoom range), over the basemap
 * that ships with Zebflow: what a client of the layer gets, with nothing asked
 * from outside.
 *
 * The tiles hold only what the view needs, so where to look comes from a
 * sample of the layer's GeoJSON (up to 500 features over the world): the view
 * is fitted to it, and a layer with no features says so instead of a map.
 * Areas blue, lines yellow, points green; hovering a feature shows its public
 * properties. Fixed colours, no scale: a tiled source has no whole to read one
 * from.
 */

const LOOK = {
  area: { color: "#3b82f6", opacity: 0.24, outline: { color: "#3b82f6", width: 1, opacity: 0.8 } },
  line: { color: "#eab308", width: 2, opacity: 0.86 },
  points: { color: "#10b981", size: 8, opacity: 0.8, outline: { color: "#ffffff", width: 1 } },
  tooltip: (d) => {
    const lines = Object.entries(d?.props || {})
      .filter(([, value]) => value !== null && value !== undefined && typeof value !== "object")
      .map(([key, value]) => `${key}: ${value}`);
    return lines.length ? lines.join("\n") : null;
  },
};

export default function LayerPreview({ layer, publicUrl }) {
  const [sample, setSample] = useState(null);

  useEffect(() => {
    if (!publicUrl) return;
    setSample(null);
    fetch(`${publicUrl}?limit=500&bbox=-180,-90,180,90`, { headers: { Accept: "application/json" } })
      .then((r) => r.json())
      .then((geojson) => setSample(Array.isArray(geojson?.features) ? geojson.features : []))
      .catch(() => setSample([]));
  }, [publicUrl]);

  const minZoom = Number.isFinite(layer?.min_zoom) ? layer.min_zoom : 0;
  const maxZoom = Number.isFinite(layer?.max_zoom) ? layer.max_zoom : 14;
  const elements = useMemo(() => {
    if (!publicUrl) return [];
    const tiles = data.mvt(`${publicUrl}/{z}/{x}/{y}.mvt`, { minZoom, maxZoom });
    return geometryLayers("layer", tiles, LOOK);
  }, [publicUrl, minZoom, maxZoom]);
  const fit = useMemo(() => (sample?.length ? geometryBox(sample.map((f) => f?.geometry)) : null), [sample]);

  return (
    <div className="rounded-lg border border-border/80">
      <div className="px-4 py-2.5 border-b border-border/60">
        <p className="text-sm font-medium text-foreground">Preview</p>
      </div>
      <div className="p-0">
        {fit ? (
          <StudioMap className="h-[400px] w-full" elements={elements} fit={fit} />
        ) : (
          <div className="flex items-center justify-center h-[200px] text-sm text-muted-foreground">
            {sample === null ? "Loading preview..." : "No features to preview"}
          </div>
        )}
      </div>
    </div>
  );
}
