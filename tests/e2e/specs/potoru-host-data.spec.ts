import { test, expect } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext } from "@playwright/test";

/**
 * Host data in `zeb/potoru`, end to end, in a throwaway project: a page drives a story from its
 * own state. PotoPlayer's `collections` prop (React state) spawns and removes the story's dots
 * by id; a tap on a dot comes back through `onReport` with the dot it hit; `usePotoPlayer(id)
 * .apply` reaches the same player, and data the story does not declare comes back through
 * `onError` as `host-input` (without covering the stage), also a `__proto__` key sent before the
 * player was ready. None of it is in the server markup.
 */

/** A Potoru format v4 source folder: a 320 x 180 board whose dots the page owns. */
const STORY: Record<string, string> = {
  "potoru.lock.yml": "lockVersion: 1\npackages: []\n",
  "potoru.project.yml": [
    "format: potoru", "kind: story", "assets: []", "composition:", "  id: composition-main", '  name: "Board"',
    "entry: scene-board", "metadata:", "  id: board", '  name: "Board"', '  updatedAt: "2000-01-01T00:00:00.000Z"',
    "objects:", "  - dot", "requires:", "  - vector2d", "  - timeline", "  - objects", "  - tick-time", "  - string-catalog",
    "  - actions-v1", "  - hit-areas", "  - spawn", "  - state-machine", "  - host-data",
    "scenes:", "  - scene-board", "timebase:", "  tickRate: 1000", "versions:", "  runtime: 1", "  schema: 5", "  scoreLang: 1", "",
  ].join("\n"),
  "scenes/scene-board/scene.yml": [
    "name: Board", "durationTicks: 1000", "fps: 30", "root: inline", "stage:", "  width: 320", "  height: 180",
    "  background:", "    kind: color", '    color: "#20232a"', "  clipContent: true", "",
  ].join("\n"),
  "scenes/scene-board/root/object.yml": [
    "spawn:", "  dots:", "    host: true", "    max: 10", "    object: dot",
    "events:", "  pick:", "    report: true",
    "initialState: main", '"on":', "  dots.click:", "    - emit: pick", "      payload: payload",
    "parts:", "  - id: board-floor", "    name: floor", "    width: 320", "    height: 180",
    '    drawable: {kind: vector, geometry: {kind: rect}, paint: {fill: "#20232a", stroke: transparent, strokeWidth: 0}}',
    "    interactive: false", "states:", "  - main", "",
  ].join("\n"),
  "scenes/scene-board/root/states/main.yml": 'name: main\nsummary: ""\ndurationTicks: 1000\nfps: 30\ntracks: []\n',
  "objects/dot/object.yml": [
    "name: Dot", "kind: prop", 'summary: "40 x 40, centred where the host places it"',
    "data:", "  x:", "    type: number", "    default: 0", "    input: true", "  y:", "    type: number", "    default: 0", "    input: true",
    "  label:", "    type: string", '    default: ""', "    input: true", "    max: 4",
    "parts:", "  - id: dot-box", "    name: box", "    width: 40", "    height: 40",
    '    drawable: {kind: vector, geometry: {kind: ellipse}, paint: {fill: "#ffd166", stroke: "#11131c", strokeWidth: 2}}',
    "  - id: dot-label", "    name: label", "    y: 10", "    width: 40", "    height: 20", "    bind:", '      text: "{label}"',
    '    drawable: {kind: text, content: "string:dot-label", paint: {fill: "#11131c", stroke: transparent, strokeWidth: 0}, style: {align: center, fontFamily: "system-ui, Arial, sans-serif", fontSize: 12, fontWeight: 700}}',
    "    interactive: false", "states:", "  - still", "",
  ].join("\n"),
  "objects/dot/states/still.yml": 'name: still\nsummary: ""\ndurationTicks: 1000\nfps: 30\nloop: true\ntracks: []\n',
  "strings/und.yml": 'dot-label: ""\n',
};

const PAGES: Record<string, string> = {
  "pages/make.tsx": `import { PotoEditor } from "zeb/potoru";
const FILES = ${JSON.stringify(STORY)};
export default function Make() { return <main><PotoEditor id="lab" yaml={FILES} height="420px" /></main>; }
`,
  "pages/board.tsx": `import { useState } from "zeb/react";
import { PotoPlayer, usePotoPlayer } from "zeb/potoru";
export default function Board() {
  const [dots, setDots] = useState([{ id: "a", x: 80, y: 90, label: "A" }]);
  const [ready, setReady] = useState(false);
  const [reports, setReports] = useState([]);
  const [errors, setErrors] = useState([]);
  const board = usePotoPlayer("board");
  // Sent while nothing is mounted yet (so it waits, merged): a key the story can never declare.
  useState(() => { board.apply(JSON.parse('{"inputs":{"__proto__":{"polluted":1}}}')); return true; });
  return (
    <main style={{ width: "640px" }}>
      <PotoPlayer id="board" src="/_files/stories/board.poto" aspect="16 / 9"
        collections={{ dots }}
        onReady={() => setReady(true)}
        onReport={(r) => setReports((all) => [...all, r.event + ":" + String(r.payload && r.payload.target)])}
        onError={(e) => setErrors((all) => [...all, e.code + ":" + e.message])} />
      <button id="add" onClick={() => setDots([{ id: "a", x: 80, y: 90, label: "A" }, { id: "b", x: 240, y: 90, label: "B" }])}>add</button>
      <button id="clear" onClick={() => setDots([])}>clear</button>
      <button id="bad" onClick={() => board.apply({ inputs: { $root: { secret: 1 } } })}>bad</button>
      <p id="ready">{ready ? "ready" : "loading"}</p>
      <p id="reports">{reports.join("|")}</p>
      <p id="errors">{errors.join("|")}</p>
    </main>
  );
}
`,
};

const PIPELINES: Record<string, string> = {
  "pipelines/make": "| trigger.webhook --route /make --method GET | web.response.send --template pages/make.tsx",
  "pipelines/board": "| trigger.webhook --route /board --method GET | web.response.send --template pages/board.tsx",
};

test.describe.serial("zeb/potoru: host data from page state", () => {
  const project = `e2e-host-data-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  const base = new URL(BASE_URL);
  const host = `${base.protocol}//${project}.${OWNER}.localhost:${base.port}`;
  let request: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Host data check" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);
    const enabled = await request.post(`${api}/rwe/libraries/enable`, { data: { name: "zeb/potoru", version: "", source: "hub" } });
    expect(enabled.ok(), await enabled.text()).toBeTruthy();
    for (const [rel, source] of Object.entries(PAGES)) {
      const put = await request.put(`${api}/repo/file?path=${rel}`, { data: source, headers: { "Content-Type": "text/plain" } });
      expect(put.ok(), `${rel}: ${await put.text()}`).toBeTruthy();
    }
    for (const [path, body] of Object.entries(PIPELINES)) {
      for (const dsl of [`register ${path} -- ${body}`, `activate pipeline ${path}`]) {
        const res = await request.post(`${api}/pipelines/dsl`, { data: { dsl } });
        expect(res.ok(), `${dsl}\n${await res.text()}`).toBeTruthy();
      }
    }
    const addressing = await request.put(`${api}/settings/addressing`, { data: { data: {
      hosts: [], routes: [], disabled: ["mcp", "ms"], api_on_hosts: false, errors: "hidden",
    } } });
    expect(addressing.ok(), await addressing.text()).toBeTruthy();
    const access = await request.put(`${api}/files/access`, { data: { path: "stories", access: "public_read", scope: "prefix" } });
    expect(access.ok(), await access.text()).toBeTruthy();
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, { data: { project_name: project, password: PASSWORD } });
    expect(removed.ok(), `remove ${project}`).toBeTruthy();
    await request.dispose();
  });

  test("a host-data story is compiled and stored", async ({ page, consoleErrors }) => {
    await page.goto(`${host}/make`);
    await expect.poll(() => page.evaluate(() => (window as any).__zebPotoEditor?.get("lab")?.lastResult?.ok ?? null), { timeout: 20_000 }).toBe(true);
    const bytes = await page.evaluate(() => Array.from((window as any).__zebPotoEditor.get("lab").lastResult.poto as Uint8Array));
    const uploaded = await request.post(`${api}/files/upload?path=stories`, {
      multipart: { file: { name: "board.poto", mimeType: "application/octet-stream", buffer: Buffer.from(bytes) } },
    });
    expect(uploaded.ok(), await uploaded.text()).toBeTruthy();
    expect(consoleErrors).toEqual([]);
  });

  test("collections from page state spawn and remove dots; taps come back through onReport; refusals through onError", async ({ page, consoleErrors }) => {
    // The server markup carries the player's config only: no page data, no handlers.
    const raw = await (await request.get(`${host}/board`)).text();
    expect(raw).toContain('data-zeb-wrapper="PotoPlayer"');
    expect(raw).not.toContain("collections");

    await page.goto(`${host}/board`);
    await expect(page.locator("#ready")).toHaveText("ready", { timeout: 20_000 });
    // The __proto__ patch applied before ready reaches the player as a plain key and is refused.
    await expect(page.locator("#errors")).toContainText("host-input:", { timeout: 10_000 });
    await expect(page.locator("#errors")).toContainText("__proto__");
    expect(await page.evaluate(() => ({} as any).polluted)).toBeUndefined();
    const stage = page.locator("[data-zeb-lib=potoru] potoru-player");
    const box = (await stage.boundingBox())!;
    const at = (x: number, y: number) => ({ x: box.x + (box.width * x) / 320, y: box.y + (box.height * y) / 180 });
    const tap = async (x: number, y: number) => { const p = at(x, y); await page.mouse.click(p.x, p.y); };

    // Dot a (from the first render's state) is there: a tap on it reports pick with its id.
    await expect.poll(async () => { await tap(80, 90); return page.locator("#reports").textContent(); }, { timeout: 15_000 }).toMatch(/pick:~dots@a\//);

    // New state: dot b appears.
    await page.click("#add");
    await expect.poll(async () => { await tap(240, 90); return page.locator("#reports").textContent(); }, { timeout: 10_000 }).toMatch(/pick:~dots@b\//);

    // An empty list removes both: taps where they stood report nothing more.
    await page.click("#clear");
    await page.waitForTimeout(300);
    const before = await page.locator("#reports").textContent();
    await tap(80, 90); await tap(240, 90);
    await page.waitForTimeout(400);
    expect(await page.locator("#reports").textContent()).toBe(before);

    // usePotoPlayer reaches the same player; what the story does not declare is refused.
    await page.click("#bad");
    await expect(page.locator("#errors")).toContainText("host-input:", { timeout: 10_000 });
    await expect(page.locator("#errors")).toContainText("secret");
    await expect(stage.locator(".message:not([hidden])")).toHaveCount(0);
    expect(consoleErrors).toEqual([]);
  });
});
