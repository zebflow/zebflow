import { test, expect, clickUntil, countColour } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import { STORY } from "../potoru-story";
import type { APIRequestContext, Page } from "@playwright/test";

/**
 * A Potoru story as a block of an editor document, end to end, in a
 * throwaway project with `zeb/potoru` enabled. A story is compiled in the
 * browser and uploaded; the editor inserts it from a project route and
 * previews it live; the saved JSON renders on the server as PotoPlayer's
 * placeholder, and the page hydrates it into a player that plays.
 *
 * Neither the editor page nor the view page imports `zeb/potoru`: each
 * downloads it only once a block is on it, and a page of documents without
 * one never does.
 *
 * Libraries: the page's extension list gives every block a default library;
 * the block adds its own in the editor (its components: the live preview and
 * the library list); the player — in the editor and on the published page —
 * receives both. The story links no library, so it plays without fetching
 * them.
 */

// The entry bundle, wherever the page serves libraries from (`/assets/libraries/…` or the project's `/static/…/_rwe/lib/…`).
const BUNDLE = "zeb/potoru/0.1/runtime/potoru.bundle.mjs";

const EXTENSIONS = `import { potoruExtension } from "zeb/ui/editor-potoru";
import { mentionExtension } from "zeb/ui/editor-mention";

export const EXTENSIONS = [
  potoruExtension({ route: "/stories-search", libraries: ["/_files/libs/basic.potolib"] }),
  mentionExtension({ name: "person", route: "/people-search" }),
];
`;

const PAGES: Record<string, string> = {
  // Compiles the story in the browser; the spec uploads the bytes.
  "pages/make.tsx": `import { PotoEditor } from "zeb/potoru";
const FILES = ${JSON.stringify(STORY)};
export default function Make() { return <main><PotoEditor id="lab" yaml={FILES} height="420px" /></main>; }
`,
  "pages/write.tsx": `import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { EXTENSIONS } from "@/shared/editor/extensions/index";
export default function Write() {
  const [doc, setDoc] = useState(null);
  const [saved, setSaved] = useState("not saved");
  async function save() {
    const response = await fetch("/doc-save", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ doc }) });
    setSaved(response.ok ? "saved" : "failed " + response.status);
  }
  return <main className="mx-auto max-w-3xl p-8">
    <Editor value={doc} onChange={setDoc} extensions={EXTENSIONS} />
    <button id="save" onClick={save}>Save</button>
    <p id="saved">{saved}</p>
    <pre id="doc-json">{doc ? JSON.stringify(doc) : ""}</pre>
  </main>;
}
`,
  "pages/view.tsx": `import { DocumentView } from "zeb/ui/editor-render";
import { EXTENSIONS } from "@/shared/editor/extensions/index";
export default function View(input) {
  return <main className="mx-auto max-w-3xl p-8"><article id="server"><DocumentView doc={input.entry && input.entry.value} extensions={EXTENSIONS} /></article></main>;
}
`,
  // Documents, but no story extension: nothing here may load zeb/potoru.
  "pages/plain.tsx": `import { DocumentView } from "zeb/ui/editor-render";
import { mentionExtension } from "zeb/ui/editor-mention";
const LIST = [mentionExtension({ name: "person", route: "/people-search" })];
const DOC = { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "No stories here." }] }] };
export default function Plain() { return <main><DocumentView doc={DOC} extensions={LIST} /></main>; }
`,
};

const STORIES = "[{ id: 'square', label: 'Square', href: '/_files/stories/square.poto', snapshot: { description: 'A red square crossing' } }]";
const list = (items: string) => `javascript.script.run -- "const q = String((input.webhook.query || {}).q || '').toLowerCase(); return ${items}.filter((item) => item.label.toLowerCase().includes(q))"`;

const PIPELINES: Record<string, string> = {
  "pipelines/make": "| trigger.webhook --route /make --method GET | web.response.send --template pages/make.tsx",
  "pipelines/write": "| trigger.webhook --route /write --method GET | web.response.send --template pages/write.tsx",
  "pipelines/plain": "| trigger.webhook --route /plain --method GET | web.response.send --template pages/plain.tsx",
  "pipelines/view": "| trigger.webhook --route /view --method GET | kv.entry.get --key story-doc --durable | web.response.send --template pages/view.tsx",
  "pipelines/save": "| trigger.webhook --route /doc-save --method POST | kv.entry.put --key story-doc --value \"{{ input.webhook.body.doc }}\" --durable | web.response.send --body \"{{ input.webhook.body.doc }}\"",
  "pipelines/stories": `| trigger.webhook --route /stories-search --method GET | ${list(STORIES)} | web.response.send --body "{{ input.script }}"`,
  "pipelines/people": `| trigger.webhook --route /people-search --method GET | ${list("[]")} | web.response.send --body "{{ input.script }}"`,
};

/** Every request the page makes for the zeb/potoru entry bundle, from now on. */
function bundleRequests(page: Page): string[] {
  const seen: string[] = [];
  page.on("request", (req) => { if (req.url().includes(BUNDLE)) seen.push(req.url()); });
  return seen;
}

const BASIC = "/_files/libs/basic.potolib";
const SHAPES = "/_files/libs/shapes.potolib";
const LIBRARIES = `"libraries":["${BASIC}","${SHAPES}"]`;

/** The `libraries` the mounted player's element was given, space-separated. */
const playerLibraries = (page: Page, id: string) => page.evaluate((id) => (window as any).__zebPotoru.get(id).element.getAttribute("libraries"), id);

/** The ids of the mounted Potoru players on the page. */
const players = (page: Page) => page.evaluate(() => ((window as any).__zebPotoru ? (window as any).__zebPotoru.ids() : []));

test.describe.serial("editor: Potoru story block", () => {
  const project = `e2e-story-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  // The project's own host: the `.poto` is fetched from `/_files/…` there, same-origin.
  const base = new URL(BASE_URL);
  const host = `${base.protocol}//${project}.${OWNER}.localhost:${base.port}`;
  let request: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Story block check" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);
    for (const name of ["zeb/prosemirror", "zeb/potoru"]) {
      const enabled = await request.post(`${api}/rwe/libraries/enable`, { data: { name, version: "", source: "hub" } });
      expect(enabled.ok(), `${name}: ${await enabled.text()}`).toBeTruthy();
    }
    for (const [rel, source] of Object.entries({ "shared/editor/extensions/index.tsx": EXTENSIONS, ...PAGES })) {
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

  test("a story is compiled and stored in the project", async ({ page, consoleErrors }) => {
    await page.goto(`${host}/make`);
    await expect.poll(() => page.evaluate(() => !!(window as any).__zebPotoEditor?.get("lab")?.lastResult?.ok), { timeout: 20_000 }).toBe(true);
    const bytes = await page.evaluate(() => Array.from((window as any).__zebPotoEditor.get("lab").lastResult.poto as Uint8Array));
    const uploaded = await request.post(`${api}/files/upload?path=stories`, {
      multipart: { file: { name: "square.poto", mimeType: "application/octet-stream", buffer: Buffer.from(bytes) } },
    });
    expect(uploaded.ok(), await uploaded.text()).toBeTruthy();
    expect(consoleErrors).toEqual([]);
  });

  test("a page of documents without a story never loads zeb/potoru", async ({ page, consoleErrors }) => {
    const seen = bundleRequests(page);
    await page.goto(`${host}/plain`);
    await expect(page.getByText("No stories here.")).toBeVisible();
    await page.waitForLoadState("networkidle");
    expect(seen).toEqual([]);
    expect(await page.evaluate(() => typeof (window as any).__zebPotoru)).toBe("undefined");
    expect(consoleErrors).toEqual([]);
  });

  test("the editor inserts a story from the route and previews it live", async ({ page, consoleErrors }) => {
    const seen = bundleRequests(page);
    await page.goto(`${host}/write`);
    const editor = page.locator("[data-slot=editor] .ProseMirror");
    await clickUntil(page, "[data-slot=editor] .ProseMirror", async () => {
      await expect(editor).toBeFocused({ timeout: 2000 });
    });
    await page.waitForLoadState("networkidle");
    expect(seen, "no story yet, so no library yet").toEqual([]);

    await page.keyboard.type("A story: ");
    await page.keyboard.press("Enter");
    await page.keyboard.type("/potoru");
    await expect(page.locator("[data-slot=editor-slash] [role=option]").first()).toContainText("Potoru story");
    await page.keyboard.press("Enter");
    const picker = page.locator("[data-slot=editor-picker]");
    await expect(picker.locator("input")).toBeFocused();
    await page.keyboard.type("squ");
    await expect(picker.locator("[role=option]")).toHaveText(["SquareA red square crossing"]);
    await page.keyboard.press("Enter");

    const block = editor.locator("[data-potoru-block=potoru]");
    await expect(block.locator("[data-zeb-lib=potoru]")).toHaveAttribute("data-config", `{"src":"/_files/stories/square.poto","libraries":["${BASIC}"],"controls":true}`);
    await expect(block.locator("figcaption")).toHaveText("Square");
    // The block put the placeholder on the page: now, and only now, the library loads and the preview draws.
    await expect.poll(() => seen.length, { timeout: 10_000 }).toBeGreaterThan(0);
    await expect.poll(() => players(page), { timeout: 15_000 }).toHaveLength(1);
    await expect.poll(() => countColour(page, "[data-slot=editor] [data-zeb-lib=potoru]", [0, 0, 255]), { timeout: 15_000 }).toBeGreaterThan(2000);

    // The block's library list (its editComponent): the page's default, then one of its own.
    const libraries = editor.locator("[data-node-view=potoru] [data-slot=potoru-libraries]");
    await expect(libraries.locator("[data-library-default]")).toHaveText(["basic.potolib"]);
    const address = libraries.locator("input[name=potoru-library]");
    await address.fill("http://example.com/libs/shapes.potolib");
    await address.press("Enter");
    await expect(libraries.locator("[role=alert]")).toContainText("https://");
    await expect(libraries.locator("[data-library]")).toHaveCount(0);
    await address.fill(SHAPES);
    await libraries.getByRole("button", { name: "Add library" }).click();
    await expect(libraries.locator("[data-library]")).toHaveText(["shapes.potolib×"]);
    await expect(block.locator("[data-zeb-lib=potoru]")).toHaveAttribute("data-config", `{"src":"/_files/stories/square.poto",${LIBRARIES},"controls":true}`);
    await expect(page.locator("#doc-json")).toContainText(`"libraries":["${SHAPES}"]`);
    // The preview player follows its placeholder: it now holds both.
    const preview = (await players(page))[0];
    await expect.poll(() => playerLibraries(page, preview), { timeout: 10_000 }).toBe(`${BASIC} ${SHAPES}`);

    // The panel edits the block; the preview follows.
    const panel = page.locator("[data-slot=editor-panel][data-extension=potoru]");
    await block.locator("figcaption").click();
    await expect(panel).toBeVisible();
    await panel.locator("input[name=still]").check();
    await panel.locator("input[name=time]").fill("1");
    await expect(block.locator("[data-zeb-lib=potoru]")).toHaveAttribute("data-config", `{"src":"/_files/stories/square.poto",${LIBRARIES},"controls":true,"still":true,"time":1}`);
    await panel.locator("input[name=still]").uncheck();
    await expect(block.locator("[data-zeb-lib=potoru]")).toHaveAttribute("data-config", `{"src":"/_files/stories/square.poto",${LIBRARIES},"controls":true}`);

    await clickUntil(page, "#save", async () => {
      await expect(page.locator("#saved")).toHaveText("saved", { timeout: 3000 });
    });
    expect(consoleErrors).toEqual([]);
  });

  test("the saved document renders the player's placeholder on the server and plays in the page", async ({ page, consoleErrors }) => {
    const raw = await (await request.get(`${host}/view`)).text();
    expect(raw).not.toContain("RWE component error");
    const config = `{"src":"/_files/stories/square.poto",${LIBRARIES},"controls":true}`.replace(/"/g, "&quot;");
    expect(raw).toContain(`<figure data-potoru-block="potoru" data-key="square" class="my-4"><div data-zeb-lib="potoru" data-zeb-wrapper="PotoPlayer" data-config="${config}"`);
    expect(raw, "the editor's library list never reaches the page").not.toContain("potoru-libraries");
    expect(raw).not.toMatch(/await import\('[^']*zeb\/potoru\//);

    const seen = bundleRequests(page);
    await page.goto(`${host}/view`);
    await expect.poll(() => seen.length, { timeout: 10_000 }).toBeGreaterThan(0);
    await expect.poll(() => players(page), { timeout: 15_000 }).toHaveLength(1);
    // It plays: started by its own controls' API, time moves on and the square paints.
    const id = (await players(page))[0];
    await expect.poll(() => page.evaluate((id) => { const p = (window as any).__zebPotoru.get(id); return p.duration(); }, id), { timeout: 15_000 }).toBeGreaterThan(0);
    await page.evaluate((id) => (window as any).__zebPotoru.get(id).play(), id);
    await expect.poll(() => page.evaluate((id) => (window as any).__zebPotoru.get(id).currentTime(), id), { timeout: 10_000 }).toBeGreaterThan(0.2);
    await expect.poll(() => countColour(page, "#server [data-zeb-lib=potoru]", [255, 0, 0]), { timeout: 10_000 }).toBeGreaterThan(200);
    await expect.poll(() => countColour(page, "#server [data-zeb-lib=potoru]", [0, 0, 255]), { timeout: 10_000 }).toBeGreaterThan(2000);
    // The player received the page's default library and the block's own.
    expect(await playerLibraries(page, id)).toBe(`${BASIC} ${SHAPES}`);
    expect(consoleErrors).toEqual([]);
  });
});
