import { test, expect, clickUntil } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext } from "@playwright/test";

/**
 * The composable editor end to end, in a throwaway project laid out the
 * documented way: extensions in `shared/editor/extensions/`, a picker route
 * answering `[{ id, label, href, snapshot }]`, the document saved as JSON and
 * shown by a page that renders it on the server.
 *
 * In the browser: type, insert a figure (upload → caption → credit in its
 * panel), mention a person through `@` and an organization through `+` —
 * two kinds, two project routes — cite a work and link another document from
 * the slash menu's search box, save. Then the view page: the server's HTML
 * holds every node, and the same document rendered in the browser is the
 * same bytes. Last, a hostile document saved straight to the route renders
 * with nothing that runs.
 */

const project = `e2e-editor-${Date.now()}`;
const api = `/api/projects/${OWNER}/${project}`;
const wh = `/wh/${OWNER}/${project}`;

const EXTENSIONS = `import { calloutExtension } from "zeb/ui/editor-callout";
import { tableExtension } from "zeb/ui/editor-table";
import { figureExtension } from "zeb/ui/editor-figure";
import { mentionExtension } from "zeb/ui/editor-mention";
import { citationExtension } from "zeb/ui/editor-citation";
import { referenceExtension } from "zeb/ui/editor-reference";

export const EXTENSIONS = [
  calloutExtension(),
  tableExtension(),
  figureExtension(),
  mentionExtension({ name: "person", route: "${wh}/api/people" }),
  mentionExtension({ name: "org", trigger: "+", route: "${wh}/api/orgs", label: "Organization" }),
  citationExtension({ route: "${wh}/api/works" }),
  referenceExtension({ name: "article", label: "Link to article", route: "${wh}/api/articles" }),
];
`;

const WRITE = `import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { EXTENSIONS } from "@/shared/editor/extensions/index";

// A stand-in upload: the figure only needs an address back — one the
// server really answers, so the page loads it without a 404.
async function uploadImage(file) {
  return { src: "/assets/node-icons/zebflow/ai.text.generate.svg", ref: "uploads/" + file.name, alt: file.name };
}

export default function Write() {
  const [doc, setDoc] = useState(null);
  const [saved, setSaved] = useState("not saved");
  async function save() {
    const response = await fetch("${wh}/api/doc", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ doc }) });
    setSaved(response.ok ? "saved" : "failed " + response.status);
  }
  return <main className="mx-auto max-w-3xl p-8">
    <Editor value={doc} onChange={setDoc} extensions={EXTENSIONS} uploadImage={uploadImage} />
    <button id="save" onClick={save}>Save</button>
    <p id="saved">{saved}</p>
    <pre id="doc-json">{doc ? JSON.stringify(doc) : ""}</pre>
  </main>;
}
`;

const VIEW = `import { useEffect, useState } from "zeb/react";
import { DocumentView, renderDocumentHtml } from "zeb/ui/editor-render";
import { EXTENSIONS } from "@/shared/editor/extensions/index";

export default function View(input) {
  const doc = input.entry && input.entry.value;
  const [client, setClient] = useState("");
  // The same walk, run in the browser after hydration.
  useEffect(() => { setClient(renderDocumentHtml(doc, { extensions: EXTENSIONS })); }, []);
  return <main className="mx-auto max-w-3xl p-8">
    <article id="server"><DocumentView doc={doc} extensions={EXTENSIONS} /></article>
    <textarea id="client-html" readOnly value={client} />
  </main>;
}
`;

const PEOPLE = "[{ id: 'p1', label: 'Alex Example', href: '/people/p1', snapshot: { description: 'Field notes' } }, { id: 'p2', label: 'Sam Sample', href: '/people/p2', snapshot: { description: 'Maps' } }]";
const WORKS = "[{ id: '10.5555/example.1', label: 'Sample 2021', href: 'https://doi.org/10.5555/example.1', snapshot: { authors: ['Sample, S.'], year: 2021, title: 'On sample documents', container: 'Journal of Examples', doi: '10.5555/example.1' } }]";
const ORGS = "[{ id: 'o1', label: 'Example Lab', href: '/orgs/o1', snapshot: { description: 'Sampleton' } }]";
const ARTICLES = "[{ id: 'a-1', label: 'Field guide', href: '/articles/field-guide', snapshot: { description: 'How we survey' } }, { id: 'a-2', label: 'Annual report', href: '/articles/annual', snapshot: {} }]";
const search = (list: string) => `javascript.script.run -- "const q = String((input.webhook.query || {}).q || '').toLowerCase(); return ${list}.filter((item) => item.label.toLowerCase().includes(q))"`;

const PIPELINES: Record<string, string> = {
  "pipelines/write": "| trigger.webhook --route /write --method GET | web.response.send --template pages/write.tsx",
  "pipelines/view": "| trigger.webhook --route /view --method GET | kv.entry.get --key editor-doc --durable | web.response.send --template pages/view.tsx",
  "pipelines/save": "| trigger.webhook --route /api/doc --method POST | kv.entry.put --key editor-doc --value \"{{ input.webhook.body.doc }}\" --durable | web.response.send --body \"{{ input.webhook.body.doc }}\"",
  "pipelines/people": `| trigger.webhook --route /api/people --method GET | ${search(PEOPLE)} | web.response.send --body "{{ input.script }}"`,
  "pipelines/orgs": `| trigger.webhook --route /api/orgs --method GET | ${search(ORGS)} | web.response.send --body "{{ input.script }}"`,
  "pipelines/articles": `| trigger.webhook --route /api/articles --method GET | ${search(ARTICLES)} | web.response.send --body "{{ input.script }}"`,
  "pipelines/works": `| trigger.webhook --route /api/works --method GET | ${search(WORKS)} | web.response.send --body "{{ input.script }}"`,
};

// A 1x1 PNG, enough for a file chooser.
const PNG = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==", "base64");

function documentSlot(html: string): string {
  const start = html.indexOf('<article id="server"><div data-slot="document">');
  expect(start, "the server rendered the document slot").toBeGreaterThan(-1);
  const from = start + '<article id="server"><div data-slot="document">'.length;
  return html.slice(from, html.indexOf("</div></article>", from));
}

test.describe("editor extensions", () => {
  let request: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Editor check" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);
    const enabled = await request.post(`${api}/rwe/libraries/enable`, { data: { name: "zeb/prosemirror", version: "", source: "hub" } });
    expect(enabled.ok(), await enabled.text()).toBeTruthy();
    expect(await enabled.text(), "the hub serves the engine with extensions").toContain("0.3.0");
    for (const [rel, source] of Object.entries({ "shared/editor/extensions/index.tsx": EXTENSIONS, "pages/write.tsx": WRITE, "pages/view.tsx": VIEW })) {
      const put = await request.put(`${api}/repo/file?path=${rel}`, { data: source, headers: { "Content-Type": "text/plain" } });
      expect(put.ok(), `${rel}: ${await put.text()}`).toBeTruthy();
    }
    for (const [path, body] of Object.entries(PIPELINES)) {
      for (const dsl of [`register ${path} -- ${body}`, `activate pipeline ${path}`]) {
        const res = await request.post(`${api}/pipelines/dsl`, { data: { dsl } });
        expect(res.ok(), `${dsl}\n${await res.text()}`).toBeTruthy();
      }
    }
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, { data: { project_name: project, password: PASSWORD } });
    expect(removed.ok(), "remove the editor test project").toBeTruthy();
    await request.dispose();
  });

  test("the picker route answers the documented shape", async () => {
    const res = await request.get(`${wh}/api/people?q=ale`);
    expect(res.ok()).toBeTruthy();
    expect(await res.json()).toEqual([{ id: "p1", label: "Alex Example", href: "/people/p1", snapshot: { description: "Field notes" } }]);
  });

  test("type, figure, mention, citation, save — then the page renders the JSON on the server", async ({ page, consoleErrors }) => {
    await page.goto(`${wh}/write`);
    const editor = page.locator("[data-slot=editor] .ProseMirror");
    await expect(editor).toBeVisible();
    // Hydrated and mounted once a click puts the caret in the engine's DOM.
    await clickUntil(page, "[data-slot=editor] .ProseMirror", async () => {
      await expect(editor).toBeFocused({ timeout: 2000 });
    });
    await page.keyboard.type("Field notes from site-a.");
    await page.keyboard.press("Enter");

    // Figure: the slash item uploads, the caret lands in the caption, the panel takes the credit.
    await page.keyboard.type("/figure");
    await expect(page.locator("[data-slot=editor-slash] [role=option]").first()).toContainText("Figure");
    const chooser = page.waitForEvent("filechooser");
    await page.keyboard.press("Enter");
    await (await chooser).setFiles({ name: "photo.png", mimeType: "image/png", buffer: PNG });
    await expect(editor.locator("figure img")).toHaveAttribute("src", "/assets/node-icons/zebflow/ai.text.generate.svg");
    await page.keyboard.type("The boardwalk at dawn");
    await expect(editor.locator("figure figcaption")).toHaveText("The boardwalk at dawn");
    const panel = page.locator("[data-slot=editor-panel][data-extension=figure]");
    await expect(panel).toBeVisible();
    await panel.locator("input[name=credit]").fill("Photo: Example");
    await expect(editor.locator("figure [data-credit]")).toHaveText("Photo: Example");

    // Back in the caption, Enter leaves the figure for a new paragraph.
    await editor.locator("figure figcaption").click();
    await page.keyboard.press("End");
    await page.keyboard.press("Enter");
    await page.keyboard.type("Thanks @ale");
    const picker = page.locator("[data-slot=editor-picker]");
    await expect(picker.locator("[role=option]")).toHaveText(["Alex ExampleField notes"]);
    await page.keyboard.press("Enter");
    await expect(editor.locator("[data-mention=person]")).toHaveText("@Alex Example");
    await expect(picker).toHaveCount(0);

    // A second kind on its own trigger and route: nothing about it is built in.
    await page.keyboard.type(" and +exa");
    await expect(picker.locator("[role=option]")).toHaveText(["Example LabSampleton"]);
    await page.keyboard.press("Enter");
    await expect(editor.locator("[data-mention=org]")).toHaveText("+Example Lab");
    await expect(picker).toHaveCount(0);
    await page.keyboard.type(" ");

    // Citation from the slash menu: the picker has its own search box.
    await page.keyboard.type("for the method /cite");
    await page.keyboard.press("Enter");
    await expect(picker.locator("input")).toBeFocused();
    await page.keyboard.type("sample");
    await expect(picker.locator("[role=option]")).toHaveText(["Sample 2021"]);
    await page.keyboard.press("Enter");
    await expect(editor.locator("[data-citation=citation]")).toHaveText("[Sample 2021]");

    // A reference: search the project's articles from the slash menu.
    await page.keyboard.type(", see /link");
    await expect(page.locator("[data-slot=editor-slash] [role=option]").first()).toContainText("Link to article");
    await page.keyboard.press("Enter");
    await expect(picker.locator("input")).toBeFocused();
    await page.keyboard.type("field");
    await expect(picker.locator("[role=option]")).toHaveText(["Field guideHow we survey"]);
    await page.keyboard.press("Enter");
    await expect(editor.locator("[data-reference=article]")).toHaveText("Field guide");

    await clickUntil(page, "#save", async () => {
      await expect(page.locator("#saved")).toHaveText("saved", { timeout: 3000 });
    });
    const saved = JSON.parse(await page.locator("#doc-json").textContent() || "{}");
    const types = JSON.stringify(saved);
    expect(saved.type).toBe("doc");
    expect(types).toContain('"type":"figure","attrs":{"src":"/assets/node-icons/zebflow/ai.text.generate.svg","alt":"photo.png","ref":"uploads/photo.png","credit":"Photo: Example"}');
    expect(types).toContain('"type":"person","attrs":{"id":"p1","label":"Alex Example","href":"/people/p1","snapshot":{"description":"Field notes"}}');
    expect(types).toContain('"type":"citation","attrs":{"id":"10.5555/example.1"');
    expect(types).toContain('"type":"org","attrs":{"id":"o1","label":"Example Lab","href":"/orgs/o1","snapshot":{"description":"Sampleton"}}');
    expect(types).toContain('"type":"article","attrs":{"key":"a-1","label":"Field guide","href":"/articles/field-guide","snapshot":{"description":"How we survey"}}');

    // The view page: HTML from the stored JSON, rendered by the server.
    const raw = await (await request.get(`${wh}/view`)).text();
    expect(raw).not.toContain("RWE component error");
    const server = documentSlot(raw);
    expect(server).toContain("Field notes from site-a.");
    expect(server).toContain('<figure data-figure="" class="my-5"><img src="/assets/node-icons/zebflow/ai.text.generate.svg" alt="photo.png" data-ref="uploads/photo.png" class="w-full rounded-lg"><figcaption class="mt-2 text-sm text-muted-foreground">The boardwalk at dawn</figcaption>');
    expect(server).toContain(">Photo: Example</p></figure>");
    expect(server).toContain('<a data-mention="person" data-id="p1" href="/people/p1"');
    expect(server).toContain('<a href="#citation-1" class="text-info no-underline">[1]</a>');
    expect(server).toContain('<a data-mention="org" data-id="o1" href="/orgs/o1"');
    expect(server).toContain('<a data-reference="article" data-key="a-1" title="How we survey" href="/articles/field-guide" class="text-info underline underline-offset-4">Field guide</a>');
    expect(server).toContain("<cite>On sample documents</cite>");
    expect(server).not.toContain("contenteditable");

    // The browser renders the same document to the same bytes.
    await page.goto(`${wh}/view`);
    await expect(page.locator("#client-html")).not.toHaveValue("");
    expect(await page.locator("#client-html").inputValue()).toBe(server);
    await expect(page.locator("#server figure figcaption")).toHaveText("The boardwalk at dawn");
    expect(consoleErrors).toEqual([]);
  });

  test("a hostile stored document renders with nothing that runs", async () => {
    const doc = {
      type: "doc",
      content: [
        { type: "script", content: [{ type: "text", text: "alert(1)" }] },
        { type: "paragraph", content: [{ type: "text", text: "x", marks: [{ type: "link", attrs: { href: "javascript:alert(2)" } }] }] },
        { type: "image", attrs: { src: "javascript:alert(3)", alt: "\" onerror=\"alert(4)" } },
        { type: "person", attrs: { id: "p", label: "<img src=x onerror=alert(5)>", href: "javascript:alert(6)" } },
        { type: "figure", attrs: { src: "/a.png", credit: "<script>alert(7)</script>" }, content: [{ type: "text", text: "c" }] },
      ],
    };
    expect((await request.post(`${wh}/api/doc`, { data: { doc } })).ok()).toBeTruthy();
    const server = documentSlot(await (await request.get(`${wh}/view`)).text());
    expect(server).toContain('data-unknown-node="script"');
    for (const banned of ["<script", "javascript:", 'onerror="', "<img src=x"]) {
      expect(server.toLowerCase(), banned).not.toContain(banned);
    }
  });
});
