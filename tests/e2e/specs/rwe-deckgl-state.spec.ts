import { test, expect } from "../fixtures";
import { OWNER, PASSWORD } from "../config";

/**
 * The deck.gl runtime mounts the server-rendered host before hydration; the
 * hydrated DeckMap must adopt that instance so a layer built from page state
 * reaches the deck. It once did not: the DOM changed, the map never did.
 */
test("a deck.gl layer built from page state reaches the deck after hydration", async ({ page, request, consoleErrors }) => {
  const project = `e2e-deck-${Date.now()}`;
  const base = `/api/projects/${OWNER}/${project}`;
  const route = `/wh/${OWNER}/${project}/deck`;
  const source = `
import { useState, useEffect } from "zeb/react";
import DeckMap from "zeb/deckgl";
export default function Page() {
  const [pts, setPts] = useState([]);
  useEffect(() => { const t = setTimeout(() => setPts([{ p: [151.2, -33.9] }, { p: [115.9, -31.9] }]), 200); return () => clearTimeout(t); }, []);
  const square = { type: "FeatureCollection", features: [{ type: "Feature", properties: {}, geometry: { type: "Polygon", coordinates: [[[120, -30], [140, -30], [140, -20], [120, -20], [120, -30]]] } }] };
  const layers = [
    { type: "ScatterplotLayer", id: "pts", data: pts, getPosition: "p", getFillColor: [255, 0, 0, 255], radiusMinPixels: 10 },
    { type: "GeoJsonLayer", id: "geo", data: square, filled: true, stroked: true, getFillColor: [0, 0, 255, 255] },
  ];
  return <main><p id="count">{pts.length}</p>
    <DeckMap id="deck" height="300px" initialViewState={{ longitude: 130, latitude: -25, zoom: 3 }} controller={false} layers={layers} /></main>;
}`;
  try {
    const created = await request.post("/home/projects/create", { form: { project, title: "deck.gl state regression" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);
    expect((await request.put(`${base}/repo/file?path=deck.tsx`, {
      data: source, headers: { "Content-Type": "text/plain" },
    })).ok()).toBeTruthy();
    const graph = {
      id: "deck", entry_nodes: ["trigger"],
      nodes: [
        { id: "trigger", kind: "n.trigger.webhook", input_pins: [], output_pins: ["out"], config: { path: "/deck", method: "GET" } },
        { id: "page", kind: "n.web.response", input_pins: ["in"], output_pins: ["out"], config: { template: "deck.tsx" } },
      ],
      edges: [{ from_node: "trigger", from_pin: "out", to_node: "page", to_pin: "in" }],
    };
    expect((await request.post(`${base}/pipelines/definition`, { data: {
      file_rel_path: "deck.zf.json", title: "Deck", trigger_kind: "webhook",
      source: JSON.stringify({ apiVersion: "zebflow.com/v1", kind: "Pipeline", metadata: { name: "deck" }, spec: graph }),
    } })).ok()).toBeTruthy();
    expect((await request.post(`${base}/pipelines/activate`, { data: { file_rel_path: "deck.zf.json" } })).ok()).toBeTruthy();

    await page.goto(route);
    await expect(page.locator("#count")).toHaveText("2");
    // The DOM changed; the map must have followed. Read the deck's own layer
    // list rather than trusting the render: the bug was exactly that the two
    // disagreed.
    await expect.poll(async () => page.evaluate(() => {
      const host = document.getElementById("deck") as any;
      const layers = host?._zdInstance?.deck?.props?.layers ?? [];
      return layers.map((l: any) => `${l.id}:${l.props.data.length}`);
    }), { timeout: 5000 }).toEqual(["pts:2", "geo__polygons:1"]);
    // A GeoJSON spec that leaves optional props unset must not hand deck.gl
    // an own `undefined`: it shadows the default, and a polygon whose
    // opacity is undefined draws nothing.
    expect(await page.evaluate(() => {
      const host = document.getElementById("deck") as any;
      const poly = host._zdInstance.deck.props.layers.find((l: any) => l.id === "geo__polygons");
      return { opacity: poly.props.opacity, extruded: poly.props.extruded, own: Object.keys(poly.props).filter((k) => poly.props[k] === undefined) };
    })).toEqual({ opacity: 1, extruded: false, own: [] });
    expect(consoleErrors.filter((e) => !/GL Driver Message/.test(e))).toEqual([]);
  } finally {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, {
      data: { project_name: project, password: PASSWORD },
    });
    expect(removed.ok(), "remove deck.gl test project").toBeTruthy();
  }
});
