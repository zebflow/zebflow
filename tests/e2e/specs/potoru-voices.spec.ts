import { test, expect } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext, Page } from "@playwright/test";

/**
 * Voices in `zeb/potoru`, end to end, in a throwaway project: a story whose
 * line has no baked audio speaks through the voice the viewer chooses.
 * PotoPlayer's `voices` (the site's catalog) and `voiceFallback` props reach
 * the player; `zeb:potoru:voice-needed` reaches the page; the registry opens
 * the Voices sheet (which lists the catalog) and sets a voice; the story then
 * speaks its line through that voice. The voice is an OpenAI-compatible
 * endpoint answered by the test itself, so nothing leaves the machine and no
 * voice model is needed.
 */

/** A Potoru format v4 source folder: one Indonesian line for `voice:narrator`, without baked audio. */
const STORY: Record<string, string> = {
  "potoru.lock.yml": "lockVersion: 1\npackages: []\n",
  "potoru.project.yml": [
    "format: potoru", "kind: story", "assets: []", "composition:", "  id: composition-main", '  name: "Voices"',
    "entry: scene-main", "metadata:", "  id: voices", '  name: "Voices"', '  updatedAt: "2000-01-01T00:00:00.000Z"',
    "objects: []", "requires:", "  - vector2d", "  - timeline", "  - tick-time", "  - speech", "  - string-catalog",
    "scenes:", "  - scene-main", "timebase:", "  tickRate: 1000", "versions:", "  runtime: 1", "  schema: 5", "  scoreLang: 1", "",
  ].join("\n"),
  "scenes/scene-main/scene.yml": [
    "name: Main", "durationTicks: 3000", "fps: 30", "root: inline", "stage:", "  width: 320", "  height: 180",
    "  background:", "    kind: color", '    color: "#0000ff"', "  clipContent: true", "",
  ].join("\n"),
  "scenes/scene-main/root/object.yml": [
    "initialState: main", "parts:", "  - id: main-box", "    name: box", "    x: 20", "    y: 60", "    width: 60", "    height: 60",
    '    drawable: {kind: vector, geometry: {kind: rect}, paint: {fill: "#ff0000", stroke: transparent, strokeWidth: 0}}',
    "states:", "  - main", "",
  ].join("\n"),
  "scenes/scene-main/root/states/main.yml": [
    "name: main", 'summary: ""', "durationTicks: 3000", "fps: 30", "tracks:", "  - id: main-speech", "    label: Speech",
    "    kind: speech", "    clips:",
    '      - {id: main-speech-1, label: line-1, startTicks: 300, durationTicks: 1500, speechRef: "speech:line-1", tone: purple}', "",
  ].join("\n"),
  "speech/lines.yml": 'lines:\n  - {id: line-1, text: "string:line-1", locale: id, voice: "voice:narrator"}\n',
  "strings/id.yml": 'line-1: "Halo, aku narator."\n',
};

/** A catalog with one voice; the spec never downloads it, the sheet only lists it. */
const CATALOG = JSON.stringify({ voices: [{ id: "test-voice", name: "Test voice", lang: "id", size: 1_000_000, model: "test-voice.onnx", config: "test-voice.onnx.json" }] });

const PAGES: Record<string, string> = {
  "pages/make.tsx": `import { PotoEditor } from "zeb/potoru";
const FILES = ${JSON.stringify(STORY)};
export default function Make() { return <main><PotoEditor id="lab" yaml={FILES} height="420px" /></main>; }
`,
  "pages/listen.tsx": `import { PotoPlayer } from "zeb/potoru";
export default function Listen() { return <main><PotoPlayer id="talk" src="/_files/stories/voices.poto" voices="/_files/voices/voices.json" voiceFallback="none" controls captions /></main>; }
`,
};

const PIPELINES: Record<string, string> = {
  "pipelines/make": "| trigger.webhook --route /make --method GET | web.response.send --template pages/make.tsx",
  "pipelines/listen": "| trigger.webhook --route /listen --method GET | web.response.send --template pages/listen.tsx",
};

const SPEECH = "https://tts.example.com/v1/audio/speech";

/** One second of a quiet tone as a WAV file: what the test's speech endpoint answers. */
function toneWav(): Buffer {
  const rate = 16000;
  const samples = Buffer.alloc(rate * 2);
  for (let i = 0; i < rate; i++) samples.writeInt16LE(Math.round(2000 * Math.sin((2 * Math.PI * 440 * i) / rate)), i * 2);
  const header = Buffer.alloc(44);
  header.write("RIFF", 0); header.writeUInt32LE(36 + samples.length, 4); header.write("WAVE", 8);
  header.write("fmt ", 12); header.writeUInt32LE(16, 16); header.writeUInt16LE(1, 20); header.writeUInt16LE(1, 22);
  header.writeUInt32LE(rate, 24); header.writeUInt32LE(rate * 2, 28); header.writeUInt16LE(2, 32); header.writeUInt16LE(16, 34);
  header.write("data", 36); header.writeUInt32LE(samples.length, 40);
  return Buffer.concat([header, samples]);
}

const player = (page: Page) => page.evaluate(() => (window as any).__zebPotoru?.get("talk"));

test.describe.serial("zeb/potoru: viewer-chosen voices", () => {
  const project = `e2e-voices-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  const base = new URL(BASE_URL);
  const host = `${base.protocol}//${project}.${OWNER}.localhost:${base.port}`;
  let request: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Voices check" }, maxRedirects: 0 });
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
    for (const path of ["stories", "voices"]) {
      const access = await request.put(`${api}/files/access`, { data: { path, access: "public_read", scope: "prefix" } });
      expect(access.ok(), await access.text()).toBeTruthy();
    }
    const catalog = await request.post(`${api}/files/upload?path=voices`, {
      multipart: { file: { name: "voices.json", mimeType: "application/json", buffer: Buffer.from(CATALOG) } },
    });
    expect(catalog.ok(), await catalog.text()).toBeTruthy();
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, { data: { project_name: project, password: PASSWORD } });
    expect(removed.ok(), `remove ${project}`).toBeTruthy();
    await request.dispose();
  });

  test("a story with a live line is compiled and stored", async ({ page, consoleErrors }) => {
    await page.goto(`${host}/make`);
    await expect.poll(() => page.evaluate(() => (window as any).__zebPotoEditor?.get("lab")?.lastResult?.ok ?? null), { timeout: 20_000 }).toBe(true);
    const bytes = await page.evaluate(() => Array.from((window as any).__zebPotoEditor.get("lab").lastResult.poto as Uint8Array));
    const uploaded = await request.post(`${api}/files/upload?path=stories`, {
      multipart: { file: { name: "voices.poto", mimeType: "application/octet-stream", buffer: Buffer.from(bytes) } },
    });
    expect(uploaded.ok(), await uploaded.text()).toBeTruthy();
    expect(consoleErrors).toEqual([]);
  });

  test("the player gets the catalog and fallback, tells the page which voice is unset, and speaks through the chosen one", async ({ page, consoleErrors }) => {
    const raw = await (await request.get(`${host}/listen`)).text();
    expect(raw).toContain(`"voices":"/_files/voices/voices.json","voiceFallback":"none"`.replace(/"/g, "&quot;"));

    const spoken: string[] = [];
    await page.route(SPEECH, async (route) => {
      if (route.request().method() === "OPTIONS") {
        await route.fulfill({ status: 204, headers: { "access-control-allow-origin": "*", "access-control-allow-headers": "*", "access-control-allow-methods": "POST" } });
        return;
      }
      spoken.push(String(JSON.parse(route.request().postData() || "{}").input));
      await route.fulfill({ status: 200, contentType: "audio/wav", body: toneWav(), headers: { "access-control-allow-origin": "*" } });
    });
    await page.addInitScript(() => {
      (window as any).__voiceNeeded = [];
      document.addEventListener("zeb:potoru:voice-needed", (event) => (window as any).__voiceNeeded.push((event as CustomEvent).detail));
      // Silent: the audio plays, muted.
      const play = HTMLMediaElement.prototype.play;
      HTMLMediaElement.prototype.play = function () { this.muted = true; (window as any).__played = ((window as any).__played ?? 0) + 1; return play.call(this); };
    });
    await page.goto(`${host}/listen`);
    await expect.poll(() => page.evaluate(() => (window as any).__voiceNeeded.length), { timeout: 20_000 }).toBe(1);
    const needed = await page.evaluate(() => (window as any).__voiceNeeded[0]);
    expect(needed).toMatchObject({ id: "talk", aliases: ["narrator"], missing: ["narrator"], fallback: "none" });
    expect(await page.evaluate(() => (window as any).__zebPotoru.get("talk").voicesRequired)).toEqual(["narrator"]);

    // The sheet, through the registry: it lists the site's catalog voice.
    await page.evaluate(() => (window as any).__zebPotoru.get("talk").openVoices());
    const sheet = page.locator("[data-zeb-lib=potoru] potoru-player .voices-sheet");
    await expect(sheet.locator('select[data-voice-choice="narrator"]')).toBeVisible({ timeout: 15_000 });
    await expect(sheet.locator('[data-catalog-voice="test-voice"]')).toContainText("Test voice");
    await page.evaluate(() => (window as any).__zebPotoru.get("talk").closeVoices());

    // A voice chosen through the registry speaks the story's line.
    await page.evaluate(() => (window as any).__zebPotoru.get("talk").setVoice("narrator", { provider: "openai", endpoint: "https://tts.example.com/v1", model: "test", voice: "test" }));
    await expect.poll(() => page.evaluate(() => (window as any).__zebPotoru.get("talk").voicesUnset), { timeout: 10_000 }).toEqual([]);
    await page.evaluate(() => (window as any).__zebPotoru.get("talk").play());
    await expect.poll(() => spoken, { timeout: 15_000 }).toEqual(["Halo, aku narator."]);
    await expect.poll(() => page.evaluate(() => (window as any).__played ?? 0), { timeout: 15_000 }).toBeGreaterThan(0);
    expect(consoleErrors).toEqual([]);
    expect(await player(page)).toBeTruthy();
  });
});
