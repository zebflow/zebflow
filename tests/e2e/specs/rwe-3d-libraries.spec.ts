import { test, expect } from "../fixtures";
import { OWNER, PASSWORD } from "../config";
import type { APIRequestContext, Page } from "@playwright/test";

/**
 * The bundled 3D and map libraries draw, not merely load. A library upgrade
 * can keep every export and still render nothing — deck.gl 9.4.0 with
 * luma.gl 9.4.0 left stale polygon triangles, a GeoJSON fill that props alone
 * never show — so each case counts the pixels it should have painted.
 */

/** Pixels of the element's screenshot within `tol` of the RGB colour. */
async function countColour(page: Page, selector: string, rgb: [number, number, number], tol = 40) {
  const png = (await page.locator(selector).screenshot()).toString("base64");
  return page.evaluate(async ({ png, rgb, tol }) => {
    const img = new Image();
    img.src = `data:image/png;base64,${png}`;
    await img.decode();
    const c = document.createElement("canvas");
    c.width = img.width; c.height = img.height;
    const ctx = c.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    let n = 0;
    for (let i = 0; i < d.length; i += 4) {
      if (Math.abs(d[i] - rgb[0]) < tol && Math.abs(d[i + 1] - rgb[1]) < tol && Math.abs(d[i + 2] - rgb[2]) < tol) n++;
    }
    return n;
  }, { png, rgb, tol });
}

/** A throwaway project serving `source` at /wh/{owner}/{project}/page. */
async function servePage(request: APIRequestContext, project: string, source: string) {
  const base = `/api/projects/${OWNER}/${project}`;
  const created = await request.post("/home/projects/create", { form: { project, title: "3D library check" }, maxRedirects: 0 });
  expect(created.status()).toBe(303);
  expect((await request.put(`${base}/repo/file?path=page.tsx`, {
    data: source, headers: { "Content-Type": "text/plain" },
  })).ok()).toBeTruthy();
  const graph = {
    id: "page", entry_nodes: ["trigger"],
    nodes: [
      { id: "trigger", kind: "trigger.webhook", input_pins: [], output_pins: ["out"], config: { route: "/page", method: "GET" } },
      { id: "page", kind: "web.response.send", input_pins: ["in"], output_pins: ["out"], config: { template: "page.tsx" } },
    ],
    edges: [{ from_node: "trigger", from_pin: "out", to_node: "page", to_pin: "in" }],
  };
  expect((await request.post(`${base}/pipelines/definition`, { data: {
    file_rel_path: "page.zf.json", title: "Page", trigger_kind: "webhook",
    source: JSON.stringify({ apiVersion: "zebflow.com/v1", kind: "Pipeline", metadata: { name: "page" }, spec: graph }),
  } })).ok()).toBeTruthy();
  expect((await request.post(`${base}/pipelines/activate`, { data: { file_rel_path: "page.zf.json" } })).ok()).toBeTruthy();
  return `/wh/${OWNER}/${project}/page`;
}

async function removeProject(request: APIRequestContext, project: string) {
  const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, {
    data: { project_name: project, password: PASSWORD },
  });
  expect(removed.ok(), `remove ${project}`).toBeTruthy();
}

const realErrors = (errors: string[]) => errors.filter((e) => !/GL Driver Message|WebGL: INVALID|GPU stall/.test(e));

test("a deck.gl GeoJSON polygon paints its fill", async ({ page, request, consoleErrors }) => {
  const project = `e2e-deckpx-${Date.now()}`;
  const source = `
import DeckMap from "zeb/deckgl";
export default function Page() {
  const square = { type: "FeatureCollection", features: [{ type: "Feature", properties: {}, geometry: { type: "Polygon", coordinates: [[[120, -30], [140, -30], [140, -20], [120, -20], [120, -30]]] } }] };
  const layers = [{ type: "GeoJsonLayer", id: "geo", data: square, filled: true, stroked: false, getFillColor: [0, 0, 255, 255] }];
  return <main><DeckMap id="deck" height="300px" initialViewState={{ longitude: 130, latitude: -25, zoom: 3 }} controller={false} layers={layers} /></main>;
}`;
  try {
    await page.goto(await servePage(request, project, source));
    await expect.poll(() => countColour(page, "#deck", [0, 0, 255]), { timeout: 8000 }).toBeGreaterThan(2000);
    // A map that does not pan must not claim the touch: on a phone the page
    // scrolls over it. deck.gl's own default is "none".
    const touch = await page.evaluate(() => {
      const el = document.querySelector("#deck canvas") as HTMLElement | null;
      return el ? getComputedStyle(el).touchAction : "no canvas";
    });
    expect(touch).toBe("auto");
    expect(realErrors(consoleErrors)).toEqual([]);
  } finally {
    await removeProject(request, project);
  }
});

test("zeb/threejs mounts a scene that renders", async ({ page, request, consoleErrors }) => {
  const project = `e2e-three-${Date.now()}`;
  const source = `
import { useRef, useEffect, useState } from "zeb/react";
import { mountThreeScene, REVISION } from "zeb/threejs";
export default function Page() {
  const host = useRef(null);
  const [rev, setRev] = useState("");
  useEffect(() => {
    const scene = mountThreeScene(host.current, { background: "#00ff00", width: 320, height: 240 });
    setRev(REVISION);
    return () => scene.destroy();
  }, []);
  return <main><p id="rev">{rev}</p><div id="three" ref={host} style={{ width: "320px", height: "240px" }} /></main>;
}`;
  try {
    await page.goto(await servePage(request, project, source));
    await expect(page.locator("#rev")).toHaveText(/^\d+$/);
    await expect(page.locator("#three canvas")).toHaveCount(1);
    // The background fills the frame; the spinning cube covers part of it.
    await expect.poll(() => countColour(page, "#three", [0, 255, 0]), { timeout: 8000 }).toBeGreaterThan(20000);
    expect(realErrors(consoleErrors)).toEqual([]);
  } finally {
    await removeProject(request, project);
  }
});

test("zeb/threejs-vrm mounts its viewer without a model", async ({ page, request, consoleErrors }) => {
  const project = `e2e-vrm-${Date.now()}`;
  const source = `
import VrmViewer from "zeb/threejs-vrm";
export default function Page() {
  return <main><VrmViewer id="vrm" height="240px" background="#ff00ff" /></main>;
}`;
  try {
    await page.goto(await servePage(request, project, source));
    await expect(page.locator("#vrm canvas")).toHaveCount(1, { timeout: 8000 });
    await expect.poll(() => countColour(page, "#vrm", [255, 0, 255]), { timeout: 8000 }).toBeGreaterThan(5000);
    expect(realErrors(consoleErrors)).toEqual([]);
  } finally {
    await removeProject(request, project);
  }
});
