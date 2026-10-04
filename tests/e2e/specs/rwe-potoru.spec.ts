import { test, expect } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext, Page } from "@playwright/test";

/**
 * zeb/potoru end to end, in a throwaway project: the library is enabled the
 * way the Studio enables it (a hub install, `.wasm` engines included), a
 * story is authored in the browser both ways, and the player plays the bytes
 * the compiler made — fetched same-origin from the project's own host.
 *
 * A placeholder that never mounts still answers 200 with correct markup, so
 * every case asserts what the runtime did: a compile result, a ready event,
 * painted pixels.
 */

/** A Potoru format v4 source folder: a red square crossing a blue stage. */
const STORY: Record<string, string> = {
  "potoru.lock.yml": "lockVersion: 1\npackages: []\n",
  "potoru.project.yml": [
    "format: potoru", "kind: story", "assets: []", "composition:", "  id: composition-main", '  name: "Square"',
    "entry: scene-main", "metadata:", "  id: square", '  name: "Square"', '  updatedAt: "2000-01-01T00:00:00.000Z"',
    "objects: []", "requires:", "  - vector2d", "  - timeline", "  - tick-time", "scenes:", "  - scene-main",
    "timebase:", "  tickRate: 1000", "versions:", "  runtime: 1", "  schema: 5", "  scoreLang: 1", "",
  ].join("\n"),
  "scenes/scene-main/scene.yml": [
    "name: Square", "durationTicks: 2000", "fps: 30", "root: inline", "stage:", "  width: 320", "  height: 180",
    "  background:", "    kind: color", '    color: "#0000ff"', "  clipContent: true", "",
  ].join("\n"),
  "scenes/scene-main/root/object.yml": [
    "initialState: main", "parts:", "  - id: main-box", "    name: box", "    x: 20", "    y: 60", "    width: 60",
    "    height: 60",
    '    drawable: {kind: vector, geometry: {kind: rect}, paint: {fill: "#ff0000", stroke: transparent, strokeWidth: 0}}',
    "states:", "  - main", "",
  ].join("\n"),
  "scenes/scene-main/root/states/main.yml": [
    "name: main", 'summary: ""', "durationTicks: 2000", "fps: 30", "tracks:", "  - id: main-box-x",
    "    label: main-box.x", "    kind: property", "    targetId: main-box", "    clips:",
    "      - {id: main-box-x-1, label: x, property: x, startTicks: 0, durationTicks: 2000, easing: linear, from: 20, playback: once, to: 240, tone: blue}",
    "",
  ].join("\n"),
};

/** Way B: authoring-API JavaScript that returns a project. */
const SCRIPT = [
  'const project = Project.create({ id: "dot", name: "Dot", format: 4 });',
  'project.scene("main", { stage: { width: 320, height: 180 }, duration: 1 }, (s) => {',
  '  const dot = s.ellipse("dot", { at: [20, 60], size: [40, 40], fill: "#00ff00" });',
  '  s.state("main", { duration: 1 }, (t) => t.tween(dot, "x", { from: 20, to: 260, duration: 1 }));',
  "});",
  "return project;",
  "",
].join("\n");

const PAGES: Record<string, string> = {
  "files.tsx": `
import { PotoruCompiler } from "zeb/potoru";
const FILES = ${JSON.stringify(STORY)};
export default function Page() {
  return <main><PotoruCompiler id="lab" files={FILES} height="420px" /></main>;
}`,
  "script.tsx": `
import { PotoruCompiler } from "zeb/potoru";
const SCRIPT = ${JSON.stringify(SCRIPT)};
export default function Page() {
  return <main><PotoruCompiler id="lab" mode="script" script={SCRIPT} height="420px" /></main>;
}`,
  "player.tsx": `
import { PotoruPlayer } from "zeb/potoru";
export default function Page() {
  return <main style={{ width: "640px" }}><PotoruPlayer id="square" src="/_files/stories/square.poto" autoplay muted loop /></main>;
}`,
};

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

/** The compile result the widget with this id last produced, or null. */
async function lastResult(page: Page, id: string) {
  return page.evaluate((id) => {
    const w = (window as any).__zebPotoruCompiler?.get(id);
    const r = w?.lastResult;
    return r ? { ok: !!r.ok, size: r.poto ? r.poto.length : 0, diagnostics: r.diagnostics ?? [] } : null;
  }, id);
}

test.describe.serial("zeb/potoru", () => {
  const project = `e2e-potoru-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  // The project's own host: the `.poto` is fetched from `/_files/…` there, same-origin.
  const base = new URL(BASE_URL);
  const host = `${base.protocol}//${project}.${OWNER}.localhost:${base.port}`;
  let request: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Potoru check" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);

    const enabled = await request.post(`${api}/rwe/libraries/enable`, { data: { name: "zeb/potoru", version: "", source: "hub" } });
    expect(enabled.ok(), await enabled.text()).toBeTruthy();

    for (const [name, source] of Object.entries(PAGES)) {
      const rel = `pages/${name}`;
      expect((await request.put(`${api}/repo/file?path=${rel}`, { data: source, headers: { "Content-Type": "text/plain" } })).ok()).toBeTruthy();
      const id = name.replace(/\.tsx$/, "");
      const graph = {
        id, entry_nodes: ["trigger"],
        nodes: [
          { id: "trigger", kind: "trigger.webhook", input_pins: [], output_pins: ["out"], config: { route: `/${id}`, method: "GET" } },
          { id: "page", kind: "web.response.send", input_pins: ["in"], output_pins: ["out"], config: { template: rel } },
        ],
        edges: [{ from_node: "trigger", from_pin: "out", to_node: "page", to_pin: "in" }],
      };
      const file = `${id}.zf.json`;
      expect((await request.post(`${api}/pipelines/definition`, { data: {
        file_rel_path: file, title: id, trigger_kind: "webhook",
        source: JSON.stringify({ apiVersion: "zebflow.com/v1", kind: "Pipeline", metadata: { name: id }, spec: graph }),
      } })).ok()).toBeTruthy();
      expect((await request.post(`${api}/pipelines/activate`, { data: { file_rel_path: file } })).ok()).toBeTruthy();
    }

    // Exposed files answer at /_files/ on the project's host once the surface
    // is on and the path is public_read; nothing else in this project is exposed.
    const addressing = await request.put(`${api}/settings/addressing`, { data: { data: {
      hosts: [], routes: [], disabled: ["mcp", "ms"], api_on_hosts: false, errors: "hidden",
    } } });
    expect(addressing.ok(), await addressing.text()).toBeTruthy();
    const access = await request.put(`${api}/files/access`, { data: { path: "stories", access: "public_read", scope: "prefix" } });
    expect(access.ok(), await access.text()).toBeTruthy();
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, {
      data: { project_name: project, password: PASSWORD },
    });
    expect(removed.ok(), `remove ${project}`).toBeTruthy();
    await request.dispose();
  });

  test("Way A: the widget compiles a source folder and its preview plays", async ({ page, consoleErrors }) => {
    const response = await page.goto(`${host}/files`);
    expect(response?.status()).toBe(200);
    expect(await page.content()).not.toContain("RWE component error");

    // The widget compiles on mount; the preview paints the blue stage and the red square.
    await expect.poll(() => lastResult(page, "lab"), { timeout: 20_000 }).toMatchObject({ ok: true });
    expect((await lastResult(page, "lab"))!.size).toBeGreaterThan(0);
    await expect.poll(() => countColour(page, "#lab", [0, 0, 255]), { timeout: 10_000 }).toBeGreaterThan(2000);
    await expect.poll(() => countColour(page, "#lab", [255, 0, 0]), { timeout: 10_000 }).toBeGreaterThan(200);

    // Keep the bytes: the player case plays exactly what the browser compiled.
    const bytes = await page.evaluate(() => Array.from((window as any).__zebPotoruCompiler.get("lab").lastResult.poto as Uint8Array));
    const uploaded = await request.post(`${api}/files/upload?path=stories`, {
      multipart: { file: { name: "square.poto", mimeType: "application/octet-stream", buffer: Buffer.from(bytes) } },
    });
    expect(uploaded.ok(), await uploaded.text()).toBeTruthy();
    expect(consoleErrors).toEqual([]);
  });

  test("Way A: broken YAML is reported with its file and line", async ({ page, consoleErrors }) => {
    await page.goto(`${host}/files`);
    await expect.poll(() => lastResult(page, "lab"), { timeout: 20_000 }).toMatchObject({ ok: true });

    const result = await page.evaluate(async () => {
      const potoru = (window as any).potoru;
      const widget = (window as any).__zebPotoruCompiler.get("lab");
      const files = { ...widget.getFiles() };
      files["scenes/scene-main/scene.yml"] += "\n  : not: [valid\n";
      const r = await potoru.compile({ files });
      return { ok: r.ok, diagnostics: r.diagnostics.map((d: any) => ({ file: d.file, line: d.line, code: d.code })) };
    });
    expect(result.ok).toBe(false);
    expect(result.diagnostics).toContainEqual(
      expect.objectContaining({ file: "scenes/scene-main/scene.yml", code: "invalid-yaml", line: expect.any(Number) }),
    );
    expect(consoleErrors).toEqual([]);
  });

  test("PotoruPlayer plays a story served from the project's files", async ({ page, consoleErrors }) => {
    // Recorded from the first script on, so a ready event that fires before
    // the test looks is still seen.
    await page.addInitScript(() => {
      document.addEventListener("zeb:potoru:ready", (e: any) => ((window as any).__ready ||= []).push(e.detail));
      document.addEventListener("zeb:potoru:error", (e: any) => ((window as any).__errors ||= []).push(e.detail));
    });
    const response = await page.goto(`${host}/player`);
    expect(response?.status()).toBe(200);
    expect(await page.content()).not.toContain("RWE component error");

    await expect.poll(() => page.evaluate(() => (window as any).__ready?.length ?? 0), { timeout: 20_000 }).toBe(1);
    const ready = await page.evaluate(() => (window as any).__ready[0]);
    expect(ready).toMatchObject({ id: "square", mode: "play", duration: 2 });
    expect(await page.evaluate(() => (window as any).__errors ?? [])).toEqual([]);
    await expect.poll(() => countColour(page, "#square", [0, 0, 255]), { timeout: 10_000 }).toBeGreaterThan(5000);
    await expect.poll(() => countColour(page, "#square", [255, 0, 0]), { timeout: 10_000 }).toBeGreaterThan(500);
    expect(consoleErrors).toEqual([]);
  });

  test("Way B: an authoring-API script compiles in the sandbox and plays", async ({ page, consoleErrors }) => {
    const response = await page.goto(`${host}/script`);
    expect(response?.status()).toBe(200);
    expect(await page.content()).not.toContain("RWE component error");

    await expect.poll(() => lastResult(page, "lab"), { timeout: 30_000 }).toMatchObject({ ok: true, diagnostics: [] });
    expect((await lastResult(page, "lab"))!.size).toBeGreaterThan(0);
    await expect.poll(() => countColour(page, "#lab", [0, 255, 0]), { timeout: 10_000 }).toBeGreaterThan(200);
    expect(consoleErrors).toEqual([]);
  });
});
