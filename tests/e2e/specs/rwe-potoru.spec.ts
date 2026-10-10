import { test, expect, countColour } from "../fixtures";
import { STORY } from "../potoru-story";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext, Page } from "@playwright/test";

/**
 * zeb/potoru end to end, in a throwaway project: the library is enabled the
 * way the Studio enables it (a hub install, `.wasm` engines included), a
 * story is authored in the browser in both source modes (YAML and Script),
 * and the player and the snapshot draw the bytes the compiler made — fetched
 * same-origin from the project's own host.
 *
 * A placeholder that never mounts still answers 200 with correct markup, so
 * every case asserts what the runtime did: a compile result, a ready event,
 * painted pixels.
 */

/** Script mode: authoring-API JavaScript that returns a project. */
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
  "yaml.tsx": `
import { PotoEditor } from "zeb/potoru";
const FILES = ${JSON.stringify(STORY)};
export default function Page() {
  return <main><PotoEditor id="lab" yaml={FILES} height="420px" /></main>;
}`,
  "script.tsx": `
import { PotoEditor } from "zeb/potoru";
const SCRIPT = ${JSON.stringify(SCRIPT)};
export default function Page() {
  return <main><PotoEditor id="lab" mode="script" script={SCRIPT} height="420px" /></main>;
}`,
  "player.tsx": `
import { PotoPlayer } from "zeb/potoru";
export default function Page() {
  return <main style={{ width: "640px" }}><PotoPlayer id="square" src="/_files/stories/square.poto" autoplay muted loop /></main>;
}`,
  "fullscreen.tsx": `
import { PotoPlayer } from "zeb/potoru";
export default function Page() {
  return <main style={{ width: "640px" }}><PotoPlayer id="fs" src="/_files/stories/square.poto" controls autoplay muted loop /></main>;
}`,
  "still.tsx": `
import { PotoPlayer } from "zeb/potoru";
export default function Page() {
  return <main style={{ width: "640px" }}><PotoPlayer id="frame" src="/_files/stories/square.poto" still time={1.5} /></main>;
}`,
  "snapshot.tsx": `
import { PotoSnapshot } from "zeb/potoru";
export default function Page() {
  return <main style={{ width: "640px" }}><PotoSnapshot id="thumb" src="/_files/stories/square.poto" time={1} aspect="16 / 9" /></main>;
}`,
};

/** The compile result the editor with this id last produced, or null. */
async function lastResult(page: Page, id: string) {
  return page.evaluate((id) => {
    const w = (window as any).__zebPotoEditor?.get(id);
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

  test("YAML: the editor compiles a source folder and its preview plays", async ({ page, consoleErrors }) => {
    const response = await page.goto(`${host}/yaml`);
    expect(response?.status()).toBe(200);
    expect(await page.content()).not.toContain("RWE component error");

    // The editor compiles on mount; the preview paints the blue stage and the red square.
    await expect.poll(() => lastResult(page, "lab"), { timeout: 20_000 }).toMatchObject({ ok: true });
    expect((await lastResult(page, "lab"))!.size).toBeGreaterThan(0);
    await expect.poll(() => countColour(page, "#lab", [0, 0, 255]), { timeout: 10_000 }).toBeGreaterThan(2000);
    await expect.poll(() => countColour(page, "#lab", [255, 0, 0]), { timeout: 10_000 }).toBeGreaterThan(200);

    // Keep the bytes: the player case plays exactly what the browser compiled.
    const bytes = await page.evaluate(() => Array.from((window as any).__zebPotoEditor.get("lab").lastResult.poto as Uint8Array));
    const uploaded = await request.post(`${api}/files/upload?path=stories`, {
      multipart: { file: { name: "square.poto", mimeType: "application/octet-stream", buffer: Buffer.from(bytes) } },
    });
    expect(uploaded.ok(), await uploaded.text()).toBeTruthy();
    expect(consoleErrors).toEqual([]);
  });

  test("YAML: broken YAML is reported with its file and line", async ({ page, consoleErrors }) => {
    await page.goto(`${host}/yaml`);
    await expect.poll(() => lastResult(page, "lab"), { timeout: 20_000 }).toMatchObject({ ok: true });

    const result = await page.evaluate(async () => {
      const potoru = (window as any).potoru;
      const editor = (window as any).__zebPotoEditor.get("lab");
      const yaml = { ...editor.getFiles() };
      yaml["scenes/scene-main/scene.yml"] += "\n  : not: [valid\n";
      const r = await potoru.compile({ yaml });
      return { ok: r.ok, diagnostics: r.diagnostics.map((d: any) => ({ file: d.file, line: d.line, code: d.code })) };
    });
    expect(result.ok).toBe(false);
    expect(result.diagnostics).toContainEqual(
      expect.objectContaining({ file: "scenes/scene-main/scene.yml", code: "invalid-yaml", line: expect.any(Number) }),
    );
    expect(consoleErrors).toEqual([]);
  });

  test("PotoPlayer plays a story served from the project's files", async ({ page, consoleErrors }) => {
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

  // The player lives in the placeholder's shadow root, where
  // `document.fullscreenElement` is the outer placeholder and never the
  // player — so the player has to ask its own root. Asking `document` let it
  // enter fullscreen and never see itself there: the button kept reading
  // "Fullscreen" and the second click did nothing. Esc always worked, which
  // is why it looked like a label bug rather than a stuck state.
  // The player lives in the placeholder's shadow root, where
  // `document.fullscreenElement` is the outer placeholder and never the
  // player — so the player has to ask its own root. Asking `document` let it
  // enter fullscreen and never see itself there: the button kept reading
  // "Fullscreen" and the second click did nothing. Esc always worked, which
  // is why it looked like a label bug rather than a stuck state.
  //
  // The hover is not decoration: a playing story hides its controls, and a
  // hidden button has no box to click. This is the reader's own path in.
  // The player lives in the placeholder's shadow root, where
  // `document.fullscreenElement` is the outer placeholder and never the
  // player — so the player has to ask its own root. Asking `document` let it
  // enter fullscreen and never see itself there: the button kept reading
  // "Fullscreen" and the second click did nothing. Esc always worked, which
  // is why it read as a label bug rather than a stuck state.
  //
  // Its own page, because the bar is `display: none` without `controls` and
  // a button with no box cannot be clicked. The wake() is the reader's path
  // in: a playing story fades its bar until a pointer moves over it.
  test("fullscreen is entered and left again from the same button", async ({ page, consoleErrors }) => {
    await page.addInitScript(() => {
      document.addEventListener("zeb:potoru:ready", (e: any) => ((window as any).__ready ||= []).push(e.detail));
    });
    expect((await page.goto(`${host}/fullscreen`))?.status()).toBe(200);
    await expect.poll(() => page.evaluate(() => (window as any).__ready?.length ?? 0), { timeout: 20_000 }).toBe(1);

    const player = page.locator("#fs");
    const button = page.locator('[data-action="fullscreen"]');
    // What the document sees: the placeholder, never the player. That it is
    // set at all is proof the browser really went fullscreen.
    const inFullscreen = () => page.evaluate(() => !!document.fullscreenElement);

    await player.hover();
    await expect(button).toHaveAttribute("aria-label", "Fullscreen");
    expect(await inFullscreen()).toBe(false);

    await button.click();
    await expect(button).toHaveAttribute("aria-label", "Exit fullscreen");
    // The label is the proof that matters: it is drawn from the player's own
    // root, which is the getter that was wrong.
    expect(await inFullscreen()).toBe(true);

    // No second hover: in fullscreen the placeholder is the top-layer element
    // and cannot be hovered, and the click above already woke the bar.
    await button.click();
    await expect(button).toHaveAttribute("aria-label", "Fullscreen");
    expect(await inFullscreen()).toBe(false);
    expect(consoleErrors).toEqual([]);
  });

  test("PotoPlayer still shows one frame and does not play", async ({ page, consoleErrors }) => {
    await page.addInitScript(() => {
      document.addEventListener("zeb:potoru:ready", (e: any) => ((window as any).__ready ||= []).push(e.detail));
      document.addEventListener("zeb:potoru:error", (e: any) => ((window as any).__errors ||= []).push(e.detail));
    });
    const response = await page.goto(`${host}/still`);
    expect(response?.status()).toBe(200);
    expect(await page.content()).not.toContain("RWE component error");

    await expect.poll(() => page.evaluate(() => (window as any).__ready?.length ?? 0), { timeout: 20_000 }).toBe(1);
    expect(await page.evaluate(() => (window as any).__errors ?? [])).toEqual([]);
    await expect.poll(() => countColour(page, "#frame", [255, 0, 0]), { timeout: 10_000 }).toBeGreaterThan(500);
    // Still: the clock stays where `time` put it.
    const at = () => page.evaluate(() => {
      const p = (window as any).__zebPotoru.get("frame");
      return { time: p.currentTime(), paused: p.paused };
    });
    expect(await at()).toEqual({ time: 1.5, paused: true });
    await page.waitForTimeout(500);
    expect(await at()).toEqual({ time: 1.5, paused: true });
    expect(consoleErrors).toEqual([]);
  });

  test("PotoSnapshot paints one frame without the action or score engines", async ({ page, consoleErrors }) => {
    const fetched: string[] = [];
    page.on("request", (r) => fetched.push(new URL(r.url()).pathname));
    await page.addInitScript(() => {
      document.addEventListener("zeb:potoru:ready", (e: any) => ((window as any).__ready ||= []).push(e.detail));
      document.addEventListener("zeb:potoru:error", (e: any) => ((window as any).__errors ||= []).push(e.detail));
    });
    const response = await page.goto(`${host}/snapshot`);
    expect(response?.status()).toBe(200);
    expect(await page.content()).not.toContain("RWE component error");

    await expect.poll(() => page.evaluate(() => (window as any).__ready?.length ?? 0), { timeout: 20_000 }).toBe(1);
    expect(await page.evaluate(() => (window as any).__ready[0])).toMatchObject({ id: "thumb", kind: "snapshot", time: 1, duration: 2 });
    expect(await page.evaluate(() => (window as any).__errors ?? [])).toEqual([]);
    await expect.poll(() => countColour(page, "#thumb", [0, 0, 255]), { timeout: 10_000 }).toBeGreaterThan(5000);
    await expect.poll(() => countColour(page, "#thumb", [255, 0, 0]), { timeout: 10_000 }).toBeGreaterThan(500);
    expect(await page.evaluate(() => (window as any).__zebPotoru.get("thumb").kind)).toBe("snapshot");

    // The snapshot code came, and nothing of the player's engines or the compiler.
    expect(fetched.some((p) => /\/potoru\/0\.1\/runtime\/snapshot-[^/]+\.mjs$/.test(p))).toBe(true);
    expect(fetched.filter((p) => /action-core-|score-wasm-|score-worker|\.wasm$|compiler-|potoru-authoring-sandbox/.test(p))).toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("Script: an authoring-API script compiles in the sandbox and plays", async ({ page, consoleErrors }) => {
    const response = await page.goto(`${host}/script`);
    expect(response?.status()).toBe(200);
    expect(await page.content()).not.toContain("RWE component error");

    await expect.poll(() => lastResult(page, "lab"), { timeout: 30_000 }).toMatchObject({ ok: true, diagnostics: [] });
    expect((await lastResult(page, "lab"))!.size).toBeGreaterThan(0);
    await expect.poll(() => countColour(page, "#lab", [0, 255, 0]), { timeout: 10_000 }).toBeGreaterThan(200);
    expect(consoleErrors).toEqual([]);
  });
});
