import { test, expect, clickUntil } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import { STORY } from "../potoru-story";
import type { APIRequestContext, Page } from "@playwright/test";

/**
 * Stored document HTML (`help("web/editor")` § Stored HTML), end to end, in a
 * throwaway project. The editor page saves a document holding a figure and
 * a Potoru story with `renderDocumentHtml(doc, { extensions })` beside its
 * JSON; the save pipeline stores both in a table; a page shows the stored
 * HTML with `<DocumentHtml>` — the same markup the editor rendered, the
 * story's player hydrating into a player that plays. A row someone tampered
 * with runs nothing.
 *
 * The view page imports `zeb/ui/editor-html` only: no extension list, no
 * editor, and no `zeb/potoru` until a block is on it.
 */

const BUNDLE = "zeb/potoru/0.1/runtime/potoru.bundle.mjs";
const PNG = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==", "base64");

const EXTENSIONS = `import { figureExtension } from "zeb/ui/editor-figure";
import { potoruExtension } from "zeb/ui/editor-potoru";

export const EXTENSIONS = [figureExtension(), potoruExtension()];
`;

const DOC = {
  type: "doc",
  content: [
    { type: "paragraph", content: [{ type: "text", text: "Field notes." }] },
    { type: "figure", attrs: { src: "/_files/stories/figure.png", alt: "A figure", ref: "stories/figure.png", credit: "Photo: Example" }, content: [{ type: "text", text: "The demo site" }] },
    { type: "potoru", attrs: { key: "square", src: "/_files/stories/square.poto", snapshot: { title: "Square" } } },
  ],
};

const PAGES: Record<string, string> = {
  "pages/make.tsx": `import { PotoEditor } from "zeb/potoru";
const FILES = ${JSON.stringify(STORY)};
export default function Make() { return <main><PotoEditor id="lab" yaml={FILES} height="420px" /></main>; }
`,
  "pages/write.tsx": `import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { renderDocumentHtml } from "zeb/ui/editor-render";
import { EXTENSIONS } from "@/shared/editor/extensions/index";
const DOC = ${JSON.stringify(DOC)};
export default function Write() {
  const [doc, setDoc] = useState(DOC);
  const [saved, setSaved] = useState("not saved");
  const [html, setHtml] = useState("");
  async function save() {
    const body_html = renderDocumentHtml(doc, { extensions: EXTENSIONS });
    const response = await fetch("/doc-save", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ key: "notes", doc, body_html }) });
    setHtml(body_html);
    setSaved(response.ok ? "saved" : "failed " + response.status);
  }
  return <main className="mx-auto max-w-3xl p-8">
    <Editor value={doc} onChange={setDoc} extensions={EXTENSIONS} />
    <button id="save" onClick={save}>Save</button>
    <p id="saved">{saved}</p>
    <pre id="html">{html}</pre>
  </main>;
}
`,
  "pages/view.tsx": `import { DocumentHtml } from "zeb/ui/editor-html";
export default function View(input) {
  const row = ((input.query && input.query.rows) || [])[0];
  return <main className="mx-auto max-w-3xl p-8"><article id="server"><DocumentHtml html={row && row.body_html} /></article></main>;
}
`,
};

const PIPELINES: Record<string, string> = {
  "pipelines/setup": `| trigger.webhook --route /setup --method POST | sekejap.query.run --write -- "CREATE TABLE IF NOT EXISTS docs (_key TEXT PRIMARY KEY, body JSON, body_html TEXT)" | web.response.send --body "{{ { ok: true } }}"`,
  "pipelines/make": "| trigger.webhook --route /make --method GET | web.response.send --template pages/make.tsx",
  "pipelines/write": "| trigger.webhook --route /write --method GET | web.response.send --template pages/write.tsx",
  "pipelines/save": `| trigger.webhook --route /doc-save --method POST | sekejap.query.run --write --param "1={{ input.webhook.body.key }}" --param "2={{ input.webhook.body.doc }}" --param "3={{ input.webhook.body.body_html }}" -- "INSERT INTO docs (_key, body, body_html) VALUES ($1, $2, $3)" | web.response.send --body "{{ { ok: true } }}"`,
  "pipelines/view": `| trigger.webhook --route /view/:key --method GET | sekejap.query.run --param "1={{ input.webhook.params.key }}" -- "SELECT body_html FROM docs WHERE _key = $1" | web.response.send --template pages/view.tsx`,
};

function bundleRequests(page: Page): string[] {
  const seen: string[] = [];
  page.on("request", (req) => { if (req.url().includes(BUNDLE)) seen.push(req.url()); });
  return seen;
}

const players = (page: Page) => page.evaluate(() => ((window as any).__zebPotoru ? (window as any).__zebPotoru.ids() : []));

/** The document slot of a page's server HTML. */
function documentSlot(html: string): string {
  const open = '<article id="server"><div data-slot="document">';
  const start = html.indexOf(open);
  expect(start, html).toBeGreaterThanOrEqual(0);
  const end = html.indexOf("</div></article>", start);
  return html.slice(start + open.length, end);
}

test.describe.serial("editor: stored document HTML", () => {
  const project = `e2e-html-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  const base = new URL(BASE_URL);
  const host = `${base.protocol}//${project}.${OWNER}.localhost:${base.port}`;
  let request: APIRequestContext;
  let stored = "";

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Stored HTML check" }, maxRedirects: 0 });
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
    const table = await request.post(`${host}/setup`);
    expect(table.ok(), await table.text()).toBeTruthy();
    const figure = await request.post(`${api}/files/upload?path=stories`, {
      multipart: { file: { name: "figure.png", mimeType: "image/png", buffer: PNG } },
    });
    expect(figure.ok(), await figure.text()).toBeTruthy();
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

  test("the editor saves the document's HTML beside its JSON", async ({ page, consoleErrors }) => {
    await page.goto(`${host}/write`);
    const editor = page.locator("[data-slot=editor] .ProseMirror");
    await clickUntil(page, "[data-slot=editor] .ProseMirror p", async () => {
      await expect(editor).toBeFocused({ timeout: 2000 });
    });
    await page.keyboard.press("End");
    await page.keyboard.type(" Edited.");
    await clickUntil(page, "#save", async () => {
      await expect(page.locator("#saved")).toHaveText("saved", { timeout: 3000 });
    });
    stored = (await page.locator("#html").textContent()) || "";
    expect(stored).toContain("Field notes. Edited.");
    expect(stored).toContain('<figure data-figure=""');
    expect(stored).toContain('data-zeb-lib="potoru" data-zeb-wrapper="PotoPlayer"');
    expect(consoleErrors).toEqual([]);
  });

  test("the page shows the stored HTML as it was rendered, and the player hydrates and plays", async ({ page, consoleErrors }) => {
    expect(stored, "the save test ran").not.toBe("");
    const raw = await (await request.get(`${host}/view/notes`)).text();
    expect(raw).not.toContain("RWE component error");
    expect(documentSlot(raw), "the server shows exactly what the editor rendered").toBe(stored);
    expect(raw, "the page does not import the player up front").not.toMatch(/await import\('[^']*zeb\/potoru\//);

    const seen = bundleRequests(page);
    await page.goto(`${host}/view/notes`);
    await expect(page.locator("#server figure[data-figure] img")).toHaveAttribute("src", "/_files/stories/figure.png");
    await expect(page.locator("#server figure[data-figure] figcaption")).toContainText("The demo site");
    await expect.poll(() => seen.length, { timeout: 10_000 }).toBeGreaterThan(0);
    await expect.poll(() => players(page), { timeout: 15_000 }).toHaveLength(1);
    const id = (await players(page))[0];
    await expect.poll(() => page.evaluate((id) => (window as any).__zebPotoru.get(id).duration(), id), { timeout: 15_000 }).toBeGreaterThan(0);
    await page.evaluate((id) => (window as any).__zebPotoru.get(id).play(), id);
    await expect.poll(() => page.evaluate((id) => (window as any).__zebPotoru.get(id).currentTime(), id), { timeout: 10_000 }).toBeGreaterThan(0.2);
    // Hydration kept the server's markup: the figure in the live page is the stored one.
    const live = await page.locator("#server figure[data-figure]").evaluate((el) => el.outerHTML);
    expect(stored).toContain(live);
    expect(consoleErrors).toEqual([]);
  });

  test("a tampered row runs nothing", async ({ page, consoleErrors }) => {
    const tampered = [
      '<p onclick="window.__pwned=1">Safe text<script>window.__pwned=2</script></p>',
      '<img src="/_files/stories/figure.png" onload="window.__pwned=3">',
      '<a href="javascript:window.__pwned=4">link</a>',
      '<iframe src="javascript:parent.__pwned=5"></iframe>',
      '<p style="display:none">styled</p>',
    ].join("");
    const saved = await request.post(`${host}/doc-save`, { data: { key: "tampered", doc: {}, body_html: tampered } });
    expect(saved.ok(), await saved.text()).toBeTruthy();
    const raw = documentSlot(await (await request.get(`${host}/view/tampered`)).text());
    expect(raw).toBe('<p>Safe text</p><img src="/_files/stories/figure.png"><a href="#">link</a><p>styled</p>');

    const seen = bundleRequests(page);
    await page.goto(`${host}/view/tampered`);
    await expect(page.getByText("Safe text")).toBeVisible();
    await page.getByText("Safe text").click();
    await page.locator("#server a").click();
    await page.waitForLoadState("networkidle");
    expect(await page.evaluate(() => (window as any).__pwned)).toBeUndefined();
    expect(seen, "no story block, no player").toEqual([]);
    expect(consoleErrors).toEqual([]);
  });
});
