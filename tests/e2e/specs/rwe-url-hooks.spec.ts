import { test, expect, clickUntil } from "../fixtures";
import { OWNER, PASSWORD } from "../config";

/** The actual webhook boundary must supply the same URL to SSR and hydration. */
test("URL hooks preserve queries and namespace exports across hydration and navigation", async ({ page, request, consoleErrors }) => {
  const project = `e2e-rwe-url-${Date.now()}`;
  const base = `/api/projects/${OWNER}/${project}`;
  const route = `/wh/${OWNER}/${project}/probe`;
  const source = `
import * as UI from "zeb/react";
import { useState, useRef, useSearchParams } from "zeb/react";
export default function Page() {
  const params = useSearchParams();
  const [count, setCount] = useState(0);
  const previous = useRef(null);
  const stable = previous.current === null || previous.current === params;
  previous.current = params;
  let readonly = false;
  try { params.set("q", "mutated"); } catch (_) { readonly = true; }
  return <main><p id="route">{UI.usePathname()}</p>
    <p id="query">{params.get("q")}|{params.getAll("tag").join(",")}|{params.get("bad")}</p>
    <p id="checks">{stable ? "stable" : "changed"}:{typeof UI.useRouter === "function" ? "namespace" : "missing"}:{readonly ? "readonly" : "mutable"}</p>
    <button onClick={() => setCount(n => n + 1)}>Count {count}</button>
    <UI.Link href="?q=next&amp;tag=three">Next query</UI.Link></main>;
}`;
  try {
    const created = await request.post("/home/projects/create", { form: { project, title: "RWE URL regression" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);
    expect((await request.put(`${base}/repo/file?path=query_probe.tsx`, {
      data: source, headers: { "Content-Type": "text/plain" },
    })).ok()).toBeTruthy();
    const graph = {
      id: "query_probe", entry_nodes: ["trigger"],
      nodes: [
        { id: "trigger", kind: "n.trigger.webhook", input_pins: [], output_pins: ["out"], config: { path: "/probe", method: "GET" } },
        { id: "page", kind: "n.web.response", input_pins: ["in"], output_pins: ["out"], config: { template: "query_probe.tsx" } },
      ],
      edges: [{ from_node: "trigger", from_pin: "out", to_node: "page", to_pin: "in" }],
    };
    expect((await request.post(`${base}/pipelines/definition`, { data: {
      file_rel_path: "query_probe.zf.json", title: "Query Probe", trigger_kind: "webhook",
      source: JSON.stringify({ apiVersion: "zebflow.com/v1", kind: "Pipeline", metadata: { name: "query_probe" }, spec: graph }),
    } })).ok()).toBeTruthy();
    expect((await request.post(`${base}/pipelines/activate`, { data: { file_rel_path: "query_probe.zf.json" } })).ok()).toBeTruthy();

    const url = `${route}?q=caf%C3%A9+tea&tag=one&tag=two&bad=%FF`;
    const raw = await request.get(url);
    const html = await raw.text();
    expect(raw.ok()).toBeTruthy();
    expect(html).not.toContain("RWE component error");
    expect(html).toContain(`<p id="route">${route}</p>`);
    expect(html).toContain('<p id="query">café tea|one,two|�</p>');
    expect(html).toContain('<p id="checks">stable:namespace:readonly</p>');

    await page.goto(url);
    await clickUntil(page, "button", async () => {
      await expect(page.getByRole("button", { name: /Count [1-9]/ })).toBeVisible({ timeout: 3000 });
    });
    await expect(page.locator("#route")).toHaveText(route);
    await expect(page.locator("#query")).toHaveText("café tea|one,two|�");
    await expect(page.locator("#checks")).toHaveText("stable:namespace:readonly");
    await page.getByRole("link", { name: "Next query" }).click();
    await expect(page.locator("#query")).toHaveText("next|three|");
    await expect(page.locator("#route")).toHaveText(route);
    await expect(page.locator("#checks")).toHaveText(/:namespace:readonly$/);
    await page.getByRole("button", { name: /Count/ }).click();
    await expect(page.locator("#checks")).toHaveText("stable:namespace:readonly");
    await page.goBack();
    await expect(page.locator("#query")).toHaveText("café tea|one,two|�");
    expect(consoleErrors).toEqual([]);
  } finally {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, {
      data: { project_name: project, password: PASSWORD },
    });
    expect(removed.ok(), "remove isolated URL test project").toBeTruthy();
  }
});
