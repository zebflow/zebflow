import { useMemo } from "zeb/react";
import { data } from "zeb/world";
import StudioMap, { geometryLayers } from "@/components/ui/studio-map";
import { geometryBox } from "@/components/lib/geometry-box";
import { geoLabel } from "@/components/db/geo-shape";

/**
 * Spatial preview for any engine that stores geometry.
 *
 * Declared by `capabilities.geo`, so PostGIS through PostgreSQL renders the
 * same preview as sekejap geometry. Nothing here is engine-specific: the
 * input is a GeoJSON geometry and the output is a map of it, the view fitted
 * to it, over the basemap that ships with Zebflow (nothing from outside).
 */

const ORANGE = "#ff6a00";
const LOOK = {
  area: { color: ORANGE, opacity: 0.4, outline: { color: ORANGE, width: 2 } },
  line: { color: ORANGE, width: 3, opacity: 0.94 },
  points: { color: ORANGE, size: 16, opacity: 0.7, outline: { color: ORANGE, width: 2 } },
};

/** The geometry as features: one, or one per part of a GeometryCollection. */
function featuresOf(geometry) {
  const parts = geometry.type === "GeometryCollection" ? geometry.geometries || [] : [geometry];
  return parts.map((part, index) => ({ type: "Feature", properties: { id: index }, geometry: part }));
}

export default function GeoPreviewMap({ geometry }) {
  const shown = geometry && (geometry.coordinates || geometry.geometries) ? geometry : null;
  const key = shown ? JSON.stringify(shown) : "";
  const elements = useMemo(() => {
    if (!shown) return [];
    const source = data.geojson({ type: "FeatureCollection", features: featuresOf(shown) }, { id: "id" });
    return geometryLayers("geo", source, LOOK);
  }, [key]);
  const fit = useMemo(() => (shown ? geometryBox(shown) : null), [key]);
  if (!shown) return null;
  return (
    <div className="space-y-1">
      <div className="overflow-hidden rounded-md border border-border/70">
        <StudioMap className="h-[180px] w-full" elements={elements} fit={fit} />
      </div>
      <p className="text-[0.68rem] text-muted-foreground">{geoLabel(shown)}</p>
    </div>
  );
}
