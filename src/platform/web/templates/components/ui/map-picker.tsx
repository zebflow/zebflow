import { useEffect, useRef, useState, cx } from "zeb";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";

const TILE_SIZE = 256;
const INITIAL_CENTER = [144.9631, -37.8136];

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function lngLatToWorld(lng, lat, zoom) {
  const scale = TILE_SIZE * Math.pow(2, zoom);
  const clippedLat = clamp(lat, -85.05112878, 85.05112878);
  const sin = Math.sin((clippedLat * Math.PI) / 180);
  return [
    ((lng + 180) / 360) * scale,
    (0.5 - Math.log((1 + sin) / (1 - sin)) / (4 * Math.PI)) * scale,
  ];
}

function worldToLngLat(x, y, zoom) {
  const scale = TILE_SIZE * Math.pow(2, zoom);
  const lng = (x / scale) * 360 - 180;
  const n = Math.PI - (2 * Math.PI * y) / scale;
  const lat = (180 / Math.PI) * Math.atan(0.5 * (Math.exp(n) - Math.exp(-n)));
  return [Number(lng.toFixed(6)), Number(lat.toFixed(6))];
}

function parseGeometry(value) {
  if (!value) return null;
  let geometry = value;
  if (typeof value === "string") {
    const text = value.trim();
    if (!text) return null;
    try {
      geometry = JSON.parse(text);
    } catch (_) {
      return null;
    }
  }
  if (!geometry || typeof geometry !== "object") return null;
  if (geometry.type === "Point" && Array.isArray(geometry.coordinates)) {
    return { mode: "Point", points: [geometry.coordinates.slice(0, 2)] };
  }
  if (geometry.type === "LineString" && Array.isArray(geometry.coordinates)) {
    return { mode: "LineString", points: geometry.coordinates.map((p) => p.slice(0, 2)) };
  }
  if (geometry.type === "Polygon" && Array.isArray(geometry.coordinates) && Array.isArray(geometry.coordinates[0])) {
    const ring = geometry.coordinates[0].map((p) => p.slice(0, 2));
    if (ring.length > 1) {
      const first = ring[0];
      const last = ring[ring.length - 1];
      if (first?.[0] === last?.[0] && first?.[1] === last?.[1]) ring.pop();
    }
    return { mode: "Polygon", points: ring };
  }
  return null;
}

function geometryCenter(points) {
  if (!points?.length) return INITIAL_CENTER;
  let minLng = Infinity;
  let maxLng = -Infinity;
  let minLat = Infinity;
  let maxLat = -Infinity;
  for (const [lng, lat] of points) {
    minLng = Math.min(minLng, lng);
    maxLng = Math.max(maxLng, lng);
    minLat = Math.min(minLat, lat);
    maxLat = Math.max(maxLat, lat);
  }
  return [Number(((minLng + maxLng) / 2).toFixed(6)), Number(((minLat + maxLat) / 2).toFixed(6))];
}

function geometryFromPoints(mode, points) {
  const clean = (points || [])
    .filter((p) => Array.isArray(p) && Number.isFinite(Number(p[0])) && Number.isFinite(Number(p[1])))
    .map((p) => [Number(p[0]), Number(p[1])]);

  if (mode === "Point") {
    if (!clean.length) return null;
    return { type: "Point", coordinates: clean[clean.length - 1] };
  }
  if (mode === "LineString") {
    if (clean.length < 2) return null;
    return { type: "LineString", coordinates: clean };
  }
  if (mode === "Polygon") {
    if (clean.length < 3) return null;
    const first = clean[0];
    const last = clean[clean.length - 1];
    const ring = first[0] === last[0] && first[1] === last[1] ? clean : [...clean, first];
    return { type: "Polygon", coordinates: [ring] };
  }
  return null;
}

function formatPoint(point) {
  if (!point) return "";
  return `${Number(point[1]).toFixed(6)}, ${Number(point[0]).toFixed(6)}`;
}

function MapCanvas({ center, zoom, points, mode, onPick, onZoom, onPan }) {
  const ref = useRef(null);
  const dragRef = useRef(null);
  const [size, setSize] = useState({ width: 720, height: 420 });
  const [dragCursor, setDragCursor] = useState(false);

  useEffect(() => {
    function measure() {
      if (!ref.current) return;
      const rect = ref.current.getBoundingClientRect();
      setSize({ width: Math.max(320, rect.width), height: Math.max(280, rect.height) });
    }
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);

  const centerWorld = lngLatToWorld(center[0], center[1], zoom);
  const topLeft = [centerWorld[0] - size.width / 2, centerWorld[1] - size.height / 2];
  const minTileX = Math.floor(topLeft[0] / TILE_SIZE);
  const maxTileX = Math.floor((topLeft[0] + size.width) / TILE_SIZE);
  const minTileY = Math.floor(topLeft[1] / TILE_SIZE);
  const maxTileY = Math.floor((topLeft[1] + size.height) / TILE_SIZE);
  const tileCount = Math.pow(2, zoom);
  const tiles = [];
  for (let x = minTileX; x <= maxTileX; x += 1) {
    for (let y = minTileY; y <= maxTileY; y += 1) {
      if (y < 0 || y >= tileCount) continue;
      const wrappedX = ((x % tileCount) + tileCount) % tileCount;
      tiles.push({
        key: `${zoom}-${x}-${y}`,
        url: `https://tile.openstreetmap.org/${zoom}/${wrappedX}/${y}.png`,
        left: x * TILE_SIZE - topLeft[0],
        top: y * TILE_SIZE - topLeft[1],
      });
    }
  }

  function pointToScreen(point) {
    const world = lngLatToWorld(point[0], point[1], zoom);
    return [world[0] - topLeft[0], world[1] - topLeft[1]];
  }

  const screenPoints = (points || []).map(pointToScreen);
  const pointString = screenPoints.map((p) => `${p[0]},${p[1]}`).join(" ");
  const polygonString = mode === "Polygon" && screenPoints.length >= 3
    ? [...screenPoints, screenPoints[0]].map((p) => `${p[0]},${p[1]}`).join(" ")
    : "";

  function pickFromEvent(event) {
    const rect = ref.current?.getBoundingClientRect?.();
    if (!rect) return;
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    onPick(worldToLngLat(topLeft[0] + x, topLeft[1] + y, zoom));
  }

  function handlePointerDown(event) {
    if (event.button !== undefined && event.button !== 0) return;
    event.preventDefault();
    ref.current?.setPointerCapture?.(event.pointerId);
    dragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      centerWorld,
      moved: false,
    };
  }

  function handlePointerMove(event) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const dx = event.clientX - drag.startX;
    const dy = event.clientY - drag.startY;
    if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true;
    if (!drag.moved) return;
    event.preventDefault();
    setDragCursor(true);
    onPan(0, 0, worldToLngLat(drag.centerWorld[0] - dx, drag.centerWorld[1] - dy, zoom));
  }

  function handlePointerUp(event) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    ref.current?.releasePointerCapture?.(event.pointerId);
    dragRef.current = null;
    setDragCursor(false);
    if (!drag.moved) pickFromEvent(event);
  }

  function handleWheel(event) {
    event.preventDefault();
    onZoom(event.deltaY < 0 ? 1 : -1);
  }

  return (
    <div
      className={cx("relative overflow-hidden rounded-lg border border-ui-border/80 bg-slate-950 select-none", dragCursor ? "cursor-grabbing" : "cursor-crosshair")}
      style={{ height: 420, touchAction: "none" }}
      ref={ref}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={() => { dragRef.current = null; setDragCursor(false); }}
      onWheel={handleWheel}
    >
      {tiles.map((tile) => (
        <img
          key={tile.key}
          src={tile.url}
          alt=""
          className="absolute h-[256px] w-[256px] select-none"
          draggable={false}
          style={{ left: tile.left, top: tile.top }}
        />
      ))}
      <svg className="pointer-events-none absolute inset-0 h-full w-full">
        {polygonString ? <polygon points={polygonString} fill="rgba(246,134,60,0.22)" stroke="#f6863c" strokeWidth="3" /> : null}
        {pointString && mode === "LineString" ? <polyline points={pointString} fill="none" stroke="#f6863c" strokeWidth="4" strokeLinecap="round" strokeLinejoin="round" /> : null}
        {screenPoints.map((point, index) => (
          <g key={`point-${index}`}>
            <circle cx={point[0]} cy={point[1]} r="7" fill="#ed752e" stroke="white" strokeWidth="2" />
            <text x={point[0] + 10} y={point[1] - 10} fill="white" fontSize="11" fontWeight="700">{index + 1}</text>
          </g>
        ))}
      </svg>
      <div className="absolute left-3 top-3 flex flex-col overflow-hidden rounded-md border border-white/20 bg-slate-950/80 text-white shadow-lg">
        <button type="button" className="h-8 w-8 hover:bg-white/15" onPointerDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); onZoom(1); }}>+</button>
        <button type="button" className="h-8 w-8 border-t border-white/15 hover:bg-white/15" onPointerDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); onZoom(-1); }}>-</button>
      </div>
      <div className="absolute right-3 top-3 grid grid-cols-3 overflow-hidden rounded-md border border-white/20 bg-slate-950/80 text-white shadow-lg">
        {[
          ["↖", -1, -1], ["↑", 0, -1], ["↗", 1, -1],
          ["←", -1, 0], ["•", 0, 0], ["→", 1, 0],
          ["↙", -1, 1], ["↓", 0, 1], ["↘", 1, 1],
        ].map(([label, dx, dy]) => (
          <button
            key={label}
            type="button"
            className="h-7 w-7 text-xs hover:bg-white/15"
            onPointerDown={(event) => event.stopPropagation()}
            onClick={(event) => { event.stopPropagation(); onPan(dx, dy); }}
          >
            {label}
          </button>
        ))}
      </div>
      <div className="absolute bottom-3 left-3 rounded-md bg-slate-950/80 px-2 py-1 text-[11px] text-white">
        Drag to pan. Click to {mode === "Point" ? "place point" : mode === "LineString" ? "add line vertex" : "add polygon vertex"}.
      </div>
    </div>
  );
}

export default function MapPicker({ open, onOpenChange, value, title = "Pick Geometry", onSave, onClear }) {
  const parsed = parseGeometry(value);
  const [mode, setMode] = useState(parsed?.mode || "Point");
  const [points, setPoints] = useState(parsed?.points || []);
  const [center, setCenter] = useState(geometryCenter(parsed?.points || []));
  const [zoom, setZoom] = useState(parsed?.points?.length ? 13 : 11);
  const [status, setStatus] = useState("Choose one geometry type, then click on the map.");

  useEffect(() => {
    if (!open) return;
    const next = parseGeometry(value);
    setMode(next?.mode || "Point");
    setPoints(next?.points || []);
    setCenter(geometryCenter(next?.points || []));
    setZoom(next?.points?.length ? 13 : 11);
    setStatus("Choose one geometry type, then click on the map.");
  }, [open, value]);

  function handleMode(nextMode) {
    setMode(nextMode);
    setPoints((current) => nextMode === "Point" && current.length ? [current[current.length - 1]] : current);
  }

  function handlePick(point) {
    setCenter(point);
    setPoints((current) => mode === "Point" ? [point] : [...current, point]);
    setStatus(`${mode} point added · ${formatPoint(point)}`);
  }

  function useCurrentPosition() {
    if (!navigator?.geolocation) {
      setStatus("Current location is not available in this browser.");
      return;
    }
    setStatus("Requesting current location…");
    navigator.geolocation.getCurrentPosition(
      (position) => {
        const point = [Number(position.coords.longitude.toFixed(6)), Number(position.coords.latitude.toFixed(6))];
        setMode("Point");
        setPoints([point]);
        setCenter(point);
        setZoom(16);
        setStatus(`Current location selected · ${formatPoint(point)}`);
      },
      (error) => setStatus(`Location unavailable · ${error?.message || "permission denied"}`),
      { enableHighAccuracy: true, timeout: 12000, maximumAge: 30000 },
    );
  }

  function saveGeometry() {
    const geometry = geometryFromPoints(mode, points);
    if (!geometry) {
      setStatus(mode === "Point" ? "Point needs one coordinate." : mode === "LineString" ? "LineString needs at least two vertices." : "Polygon needs at least three vertices.");
      return;
    }
    onSave?.(geometry);
    onOpenChange?.(false);
  }

  const previewGeometry = geometryFromPoints(mode, points);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="full" className="border-border bg-surface text-body">
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>{title}</DialogTitle>
          <p className="text-sm text-body-soft">Create one GeoJSON geometry for the selected spatial field.</p>
        </DialogHeader>
        <div className="grid gap-4 px-6 py-4 lg:grid-cols-[minmax(0,1fr)_20rem]">
          <div className="space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              {["Point", "LineString", "Polygon"].map((item) => (
                <button
                  key={item}
                  type="button"
                  className={cx(
                    "rounded-md border px-3 py-1.5 text-xs font-medium",
                    mode === item ? "border-[#f6863c] bg-[#f6863c]/15 text-[#f6863c]" : "border-ui-border bg-ui-bg text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text"
                  )}
                  onClick={() => handleMode(item)}
                >
                  {item}
                </button>
              ))}
              <Button type="button" variant="outline" size="sm" onClick={useCurrentPosition}>Current GPS</Button>
              <Button type="button" variant="ghost" size="sm" disabled={!points.length} onClick={() => setPoints((current) => current.slice(0, -1))}>Undo</Button>
              <Button type="button" variant="ghost" size="sm" disabled={!points.length} onClick={() => setPoints([])}>Clear</Button>
            </div>
            <MapCanvas
              center={center}
              zoom={zoom}
              points={points}
              mode={mode}
              onPick={handlePick}
              onZoom={(delta) => setZoom((current) => clamp(current + delta, 2, 19))}
              onPan={(dx, dy, directCenter) => {
                if (directCenter) {
                  setCenter(directCenter);
                  return;
                }
                if (!dx && !dy) {
                  setCenter(geometryCenter(points));
                  return;
                }
                const world = lngLatToWorld(center[0], center[1], zoom);
                const step = 140;
                setCenter(worldToLngLat(world[0] + dx * step, world[1] + dy * step, zoom));
              }}
            />
            <p className="text-xs text-body-soft">{status}</p>
          </div>
          <div className="space-y-3">
            <div className="rounded-lg border border-ui-border/80 bg-ui-bg-muted/20 p-3">
              <p className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">Vertices</p>
              <div className="max-h-40 space-y-1 overflow-auto pr-1 text-xs text-ui-text-soft">
                {points.length ? points.map((point, index) => (
                  <div key={`coord-${index}`} className="flex justify-between gap-3 rounded border border-ui-border/60 bg-ui-bg px-2 py-1 font-mono">
                    <span>{index + 1}</span>
                    <span>{formatPoint(point)}</span>
                  </div>
                )) : <p>No vertices yet.</p>}
              </div>
            </div>
            <div className="rounded-lg border border-ui-border/80 bg-ui-bg-muted/20 p-3">
              <p className="mb-2 text-xs font-medium uppercase tracking-[0.14em] text-ui-text-soft">GeoJSON</p>
              <pre className="max-h-72 overflow-auto rounded border border-ui-border/60 bg-ui-bg p-2 text-[11px] text-ui-text">
                {previewGeometry ? JSON.stringify(previewGeometry, null, 2) : "No valid geometry yet."}
              </pre>
            </div>
          </div>
        </div>
        <DialogFooter className="px-6 pb-6">
          <Button type="button" variant="ghost" onClick={() => onOpenChange?.(false)}>Cancel</Button>
          <Button type="button" variant="outline" onClick={() => { onClear?.(); onOpenChange?.(false); }}>Clear Value</Button>
          <Button type="button" onClick={saveGeometry}>Save Geometry</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
