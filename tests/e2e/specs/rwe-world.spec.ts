import { test, expect } from "../fixtures";
import { OWNER, PASSWORD } from "../config";
import type { APIRequestContext } from "@playwright/test";

/**
 * zeb/world is aslnut, a WebGPU / WebGL2 engine, wrapped by aslnut's own React
 * shell on top of Zeb React. The page below uses no network data: a flat floor
 * (`plane()`) and one layer over rows written in the page. It checks the four
 * things a wrapper owes (aslnut's implementers guide, "Checking your wrapper"):
 * the server sends the box and nothing broken, the world exists after
 * hydration on a real graphics API, a restyle through props is a `setProps` on
 * the element already in the world and never a rebuild, and mounting and
 * unmounting the world leaves no graphics, buffers, textures or bytes behind.
 *
 * Headless Chromium draws WebGL2 in software, so every wait below is generous.
 */

/** A throwaway project serving `source` at /wh/{owner}/{project}/page. */
async function servePage(request: APIRequestContext, project: string, source: string) {
  const base = `/api/projects/${OWNER}/${project}`;
  const created = await request.post("/home/projects/create", { form: { project, title: "World check" }, maxRedirects: 0 });
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

/** Software-rendered graphics and a GPU stall warning are not defects. */
const realErrors = (errors: string[]) => errors.filter((e) => !/GL Driver Message|WebGL: INVALID|GPU stall/.test(e));

/**
 * The page. The source and the body are made once, outside the component: a
 * source made again in render is a new prop each render, which aslnut counts as
 * changed. `ready` hands the test the plain aslnut world and records what the
 * world says about its elements; `liveGraphics` is aslnut's own count of what
 * the graphics still hold.
 */
const SOURCE = `
import { useEffect, useMemo, useState } from "zeb/react";
import { World, data, layer, liveGraphics, plane } from "zeb/world";

const ROWS = data.rows([
  { id: "a", lon: -0.0004, lat: 0.0002, kind: "x" },
  { id: "b", lon: 0.0004, lat: -0.0002, kind: "y" },
]);

function ready(w) {
  const seen = globalThis;
  seen.__worldUnderTest = w;
  (seen.__worldsSeen = seen.__worldsSeen || []).push(w);
  seen.__worldsMade = (seen.__worldsMade || 0) + 1;
}

export default function Page() {
  const [warm, setWarm] = useState(false);
  const [shown, setShown] = useState(true);
  const options = useMemo(() => ({ body: plane(), view: { lon: 0, lat: 0, zoom: 16 } }), []);
  useEffect(() => { globalThis.__liveGraphicsUnderTest = liveGraphics; }, []);
  const elements = [layer({ id: "pts", data: ROWS, points: { size: 24, color: warm ? "#ff0000" : "#0000ff" } })];
  return (
    <main>
      <button id="restyle" onClick={() => setWarm((x) => !x)}>restyle</button>
      <button id="toggle" onClick={() => setShown((x) => !x)}>toggle</button>
      <p id="state">{warm ? "warm" : "cool"}:{shown ? "shown" : "hidden"}</p>
      {shown ? (
        <div id="stage">
          <World options={options} elements={elements} onReady={ready} style={{ width: "480px", height: "320px" }} />
        </div>
      ) : null}
    </main>
  );
}`;

type Probe = { __worldUnderTest?: any; __worldsSeen?: any[]; __worldsMade?: number; __liveGraphicsUnderTest?: () => Record<string, number>; __before?: unknown; __events?: string[] };

test("the server sends the world's box and nothing broken", async ({ request }) => {
  const project = `e2e-world-ssr-${Date.now()}`;
  try {
    const url = await servePage(request, project, SOURCE);
    const response = await request.get(url);
    expect(response.status()).toBe(200);
    const html = await response.text();
    expect(html).not.toContain("RWE component error");
    // The box the wrapper's World renders before it has a canvas, so hydration matches.
    expect(html).toMatch(/<div id="stage"><div style="position:relative;width:480px;height:320px"><div style="position:absolute;inset:0"><\/div>/);
    expect(html).toContain("cool:shown");
  } finally {
    await removeProject(request, project);
  }
});

test("a world exists after hydration, and a restyle through props is not a rebuild", async ({ page, request, consoleErrors }) => {
  test.setTimeout(180_000);
  const project = `e2e-world-restyle-${Date.now()}`;
  try {
    await page.goto(await servePage(request, project, SOURCE));
    await expect.poll(() => page.evaluate(() => !!(globalThis as Probe).__worldUnderTest), { timeout: 90_000, intervals: [500] }).toBe(true);
    // Real graphics ran: neither the headless "none" nor a failed start.
    const api = await page.evaluate(() => (globalThis as Probe).__worldUnderTest.api);
    expect(["webgpu", "webgl2"]).toContain(api);
    await expect(page.locator("#stage canvas")).toHaveCount(1);
    // The layer is in the world, and what the world says about its elements is recorded from here on.
    await expect.poll(() => page.evaluate(() => (globalThis as Probe).__worldUnderTest.list.some((e: any) => e.id === "pts")), { timeout: 30_000 }).toBe(true);
    await page.evaluate(() => {
      const probe = globalThis as Probe;
      const w = probe.__worldUnderTest;
      probe.__before = w.list.find((e: any) => e.id === "pts");
      probe.__events = [];
      w.on("elements", (e: any) => probe.__events!.push(`${e.what}:${e.ids.join(",")}`));
    });

    // The world is up, so the page is hydrated: one click is enough, and the state it produces is asserted.
    await page.locator("#restyle").click();
    await expect(page.locator("#state")).toHaveText("warm:shown");
    await expect.poll(() => page.evaluate(() => (globalThis as Probe).__events), { timeout: 30_000 }).toContain("props:pts");

    const after = await page.evaluate(() => {
      const probe = globalThis as Probe;
      return {
        sameElement: probe.__worldUnderTest.list.find((e: any) => e.id === "pts") === probe.__before,
        events: probe.__events,
        worlds: probe.__worldsMade,
      };
    });
    expect(after.sameElement, "the element in w.list is the object that was there before").toBe(true);
    expect(after.events!.filter((e) => /^(add|remove):/.test(e)), `no element was added or removed: ${after.events}`).toEqual([]);
    expect(after.worlds, "the restyle did not make another world").toBe(1);
    expect(realErrors(consoleErrors)).toEqual([]);
  } finally {
    await removeProject(request, project);
  }
});

test("mounting and unmounting the world ten times leaves no graphics behind", async ({ page, request, consoleErrors }) => {
  test.setTimeout(300_000);
  const project = `e2e-world-leak-${Date.now()}`;
  const TIMES = 10;
  try {
    await page.goto(await servePage(request, project, SOURCE));
    for (let round = 1; round <= TIMES; round++) {
      // Mounted: wait for the world of this round to be ready (the first mounts with the page).
      await expect.poll(() => page.evaluate(() => (globalThis as Probe).__worldsMade ?? 0), { timeout: 90_000, intervals: [250] }).toBe(round);
      // Unmount it, and wait for the world itself to say it is gone.
      await page.locator("#toggle").click();
      await expect(page.locator("#state")).toContainText(":hidden");
      await expect(page.locator("#stage")).toHaveCount(0);
      await expect.poll(() => page.evaluate(() => (globalThis as Probe).__worldUnderTest.isDisposed), { timeout: 30_000 }).toBe(true);
      if (round < TIMES) {
        await page.locator("#toggle").click();
        await expect(page.locator("#state")).toContainText(":shown");
      }
    }
    const result = await page.evaluate(() => {
      const probe = globalThis as Probe;
      return { live: probe.__liveGraphicsUnderTest!(), disposed: probe.__worldsSeen!.map((w) => !!w.isDisposed) };
    });
    expect(result.disposed, "every world that was made is disposed").toEqual(Array(TIMES).fill(true));
    // aslnut's own count: it opened one graphics per world, and holds none of it now.
    expect(result.live.made).toBeGreaterThanOrEqual(TIMES);
    expect({ graphics: result.live.graphics, buffers: result.live.buffers, textures: result.live.textures, bytes: result.live.bytes })
      .toEqual({ graphics: 0, buffers: 0, textures: 0, bytes: 0 });
    expect(realErrors(consoleErrors)).toEqual([]);
  } finally {
    await removeProject(request, project);
  }
});
