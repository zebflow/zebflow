import { test, expect, clickUntil } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext } from "@playwright/test";

/**
 * An editor extension drawn by Zeb React components, end to end, in a
 * throwaway project. The project's extension (`shared/editor/extensions/`)
 * declares a `component` for the page and an `editComponent` for the editor,
 * configured with `options`.
 *
 * In the editor the node is its editComponent, mounted live (a node view):
 * its select changes the node through `update`, and selecting the node
 * reaches the component. On the published page the node is its component:
 * the server's HTML equals the browser's renderDocumentHtml, the editor's
 * select never appears, and the page hydrates it — its click handler runs.
 */

const project = `e2e-editor-comp-${Date.now()}`;
const api = `/api/projects/${OWNER}/${project}`;
const wh = `/wh/${OWNER}/${project}`;

const EXTENSIONS = `import { useState } from "zeb/react";
import { defineExtension } from "zeb/ui/editor-extension";

function StatusBadge({ attrs, options }) {
  const [open, setOpen] = useState(false);
  return <span data-status={attrs.state} data-open={open ? "true" : undefined} onClick={() => setOpen(!open)} className="rounded-full bg-muted px-2 text-xs">{options.labels[attrs.state] || attrs.state}</span>;
}

function StatusEditor(props) {
  const { attrs, options, update, selected } = props;
  return <span data-status-editor={selected ? "selected" : "idle"} className="inline-flex items-center gap-1">
    <StatusBadge {...props} />
    <select aria-label="Status" value={attrs.state} onChange={(e) => update({ state: e.target.value })}>
      {Object.keys(options.labels).map((state) => <option key={state} value={state}>{options.labels[state]}</option>)}
    </select>
  </span>;
}

export const EXTENSIONS = [defineExtension({
  name: "status",
  node: { group: "inline", inline: true, atom: true, attrs: { state: { default: "draft" } } },
  options: { labels: { draft: "Draft", review: "In review", done: "Done" } },
  component: StatusBadge,
  editComponent: StatusEditor,
  text: (node) => node.attrs.state,
  insert: [{ label: "Status", hint: "A status badge", keys: "status", run: (api) => api.insert("status") }],
})];
`;

const WRITE = `import { useState } from "zeb/react";
import { Editor } from "zeb/ui/editor";
import { EXTENSIONS } from "@/shared/editor/extensions/index";

export default function Write() {
  const [doc, setDoc] = useState(null);
  const [saved, setSaved] = useState("not saved");
  async function save() {
    const response = await fetch("${wh}/api/doc", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ doc }) });
    setSaved(response.ok ? "saved" : "failed " + response.status);
  }
  return <main className="mx-auto max-w-3xl p-8">
    <Editor value={doc} onChange={setDoc} extensions={EXTENSIONS} />
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
  useEffect(() => { setClient(renderDocumentHtml(doc, { extensions: EXTENSIONS })); }, []);
  return <main className="mx-auto max-w-3xl p-8">
    <article id="server"><DocumentView doc={doc} extensions={EXTENSIONS} /></article>
    <textarea id="client-html" readOnly value={client} />
  </main>;
}
`;

const PIPELINES: Record<string, string> = {
  "pipelines/write": "| trigger.webhook --route /write --method GET | web.response.send --template pages/write.tsx",
  "pipelines/view": "| trigger.webhook --route /view --method GET | kv.entry.get --key comp-doc --durable | web.response.send --template pages/view.tsx",
  "pipelines/save": "| trigger.webhook --route /api/doc --method POST | kv.entry.put --key comp-doc --value \"{{ input.webhook.body.doc }}\" --durable | web.response.send --body \"{{ input.webhook.body.doc }}\"",
};

const SLOT = '<article id="server"><div data-slot="document">';
function documentSlot(html: string): string {
  const start = html.indexOf(SLOT);
  expect(start, "the server rendered the document slot").toBeGreaterThan(-1);
  return html.slice(start + SLOT.length, html.indexOf("</div></article>", start));
}

test.describe.serial("editor: extensions drawn by components", () => {
  let request: APIRequestContext;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Editor components check" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);
    const enabled = await request.post(`${api}/rwe/libraries/enable`, { data: { name: "zeb/prosemirror", version: "", source: "hub" } });
    expect(enabled.ok(), await enabled.text()).toBeTruthy();
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
    expect(removed.ok(), "remove the test project").toBeTruthy();
    await request.dispose();
  });

  test("the editor mounts the editComponent as the node and changes it through update", async ({ page, consoleErrors }) => {
    await page.goto(`${wh}/write`);
    const editor = page.locator("[data-slot=editor] .ProseMirror");
    await clickUntil(page, "[data-slot=editor] .ProseMirror", async () => {
      await expect(editor).toBeFocused({ timeout: 2000 });
    });
    await page.keyboard.type("Survey ");
    await page.keyboard.type("/status");
    await expect(page.locator("[data-slot=editor-slash] [role=option]").first()).toContainText("Status");
    await page.keyboard.press("Enter");

    const view = editor.locator("[data-node-view=status]");
    await expect(view).toHaveCount(1);
    await expect(view).toHaveAttribute("contenteditable", "false");
    await expect(view.locator("[data-status]")).toHaveText("Draft");
    await expect(page.locator("#doc-json")).toContainText('{"type":"status","attrs":{"state":"draft"}}');

    // The component's own control: ProseMirror leaves its events alone, update() writes the attr.
    await view.locator("select").selectOption("review");
    await expect(page.locator("#doc-json")).toContainText('{"type":"status","attrs":{"state":"review"}}');
    await expect(view.locator("[data-status]")).toHaveText("In review");
    await expect(view.locator("select")).toHaveValue("review");

    // Selecting the node reaches the component.
    await expect(view.locator("[data-status-editor]")).toHaveAttribute("data-status-editor", "idle");
    await view.locator("[data-status]").click();
    await expect(view).toHaveAttribute("data-selected", "");
    await expect(view.locator("[data-status-editor]")).toHaveAttribute("data-status-editor", "selected");

    await clickUntil(page, "#save", async () => {
      await expect(page.locator("#saved")).toHaveText("saved", { timeout: 3000 });
    });
    expect(consoleErrors).toEqual([]);
  });

  test("the page renders the component on the server, the same bytes as in the browser, and hydrates it", async ({ page, consoleErrors }) => {
    const raw = await (await request.get(`${wh}/view`)).text();
    expect(raw).not.toContain("RWE component error");
    const server = documentSlot(raw);
    expect(server).toBe('<p class="my-1.5">Survey <span data-status="review" class="rounded-full bg-muted px-2 text-xs">In review</span> </p>');
    expect(server, "the editor's component never reaches the page").not.toContain("<select");

    await page.goto(`${wh}/view`);
    await expect(page.locator("#client-html")).not.toHaveValue("");
    expect(await page.locator("#client-html").inputValue()).toBe(server);
    // Hydrated: the component's click handler (kept by the guard, never printed) runs.
    const badge = page.locator("#server [data-status=review]");
    await clickUntil(page, "#server [data-status=review]", async () => {
      await expect(badge).toHaveAttribute("data-open", "true", { timeout: 2000 });
    });
    expect(consoleErrors).toEqual([]);
  });
});
