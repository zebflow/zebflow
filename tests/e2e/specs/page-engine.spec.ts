import { test, expect } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext, Page } from "@playwright/test";

/**
 * What a project page gets from the engine, on a project host, in a throwaway
 * project:
 *
 * - its scripts come from the host's own `/_static/`, so the public page names
 *   neither its owner nor its project (`addressing.md` §1);
 * - a `#fragment` link scrolls to the fragment and refetches nothing;
 * - `cx` lets a class passed last replace one that sets the same property, on
 *   the server and after hydration alike (`<Button className="px-0">`);
 * - a bare `border` is drawn in the theme's border colour;
 * - `bg-overlay` dims in light and dark, and on a page whose theme predates
 *   the `overlay` role.
 */

const DEMO = `import "@/globals.css";
import { cx, useState } from "zeb/react";
export default function Demo() {
  const [count, setCount] = useState(0);
  return (
    <main>
      <button id="count" onClick={() => setCount(count + 1)}>{count}</button>
      <a id="jump" href="#far">far</a>
      <div id="merged" className={cx("px-4 text-primary", "px-0 text-muted-foreground")}>merged</div>
      <div id="ringed" className={cx("focus-visible:ring-[3px] ring-[#f00]", "focus-visible:ring-0 ring-blue-500")}>ringed</div>
      <div id="bordered" className="border">bordered</div>
      <div id="probe" style={{ color: "var(--border)" }}>probe</div>
      <div id="scrim" className="bg-overlay">scrim</div>
      <div className="dark"><div id="scrim-dark" className="bg-overlay">dark scrim</div></div>
      <div style={{ height: "3000px" }} />
      <h2 id="far">far</h2>
    </main>
  );
}
`;

// No globals.css: a theme written before the overlay role existed.
const BARE = `export default function Bare() {
  return <main><div id="scrim" className="bg-overlay">scrim</div></main>;
}
`;

test.describe("page engine", () => {
  let request: APIRequestContext;
  const project = `e2e-engine-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  const base = new URL(BASE_URL);
  const host = `${base.protocol}//${project}.${OWNER}.localhost:${base.port}`;

  async function hydrated(page: Page, path: string) {
    await page.goto(`${host}${path}`);
    await page.waitForFunction(() => typeof (window as any).rweNavigate === "function");
  }

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Page engine" }, maxRedirects: 0 });
    expect(created.status(), `create ${project}`).toBeLessThan(400);
    for (const [rel, source] of [["pages/demo.tsx", DEMO], ["pages/bare.tsx", BARE]]) {
      const put = await request.put(`${api}/repo/file?path=${rel}`, { data: source, headers: { "Content-Type": "text/plain" } });
      expect(put.ok(), `write ${rel}`).toBeTruthy();
    }
    for (const name of ["demo", "bare"]) {
      for (const dsl of [
        `register pipelines/sandbox/${name} -- | trigger.webhook --route /${name} --method GET | web.response.send --template pages/${name}.tsx`,
        `activate pipeline pipelines/sandbox/${name}`,
      ]) {
        const res = await request.post(`${api}/pipelines/dsl`, { data: { dsl } });
        expect(JSON.stringify(await res.json()), dsl).not.toContain('"ok":false');
      }
    }
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, { data: { project_name: project, password: PASSWORD } });
    expect(removed.ok(), `remove ${project}`).toBeTruthy();
    await request.dispose();
  });

  test("a page on a project host names neither owner nor project", async ({ page, consoleErrors }) => {
    const html = await (await request.get(`${host}/demo`)).text();
    expect(html).toContain('src="/_static/_rwe/scripts/');
    expect(html).not.toContain(`/static/${OWNER}/${project}`);
    const platform = await (await request.get(`/wh/${OWNER}/${project}/demo`)).text();
    expect(platform, "the platform address keeps the platform form").toContain(`/static/${OWNER}/${project}/_rwe/scripts/`);
    await hydrated(page, "/demo");
    expect(consoleErrors).toEqual([]);
  });

  test("a #fragment link scrolls to its section and refetches nothing", async ({ page, consoleErrors }) => {
    await hydrated(page, "/demo");
    const refetched: string[] = [];
    page.on("request", (r) => { if (new URL(r.url()).pathname === "/demo") refetched.push(r.resourceType()); });
    await page.locator("#jump").click();
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(1000);
    await expect(page.locator("#far")).toBeInViewport();
    expect(page.url()).toMatch(/#far$/);
    await page.waitForTimeout(300);
    expect(refetched, "no request for the page itself").toEqual([]);
    expect(consoleErrors).toEqual([]);
  });

  test("cx lets the class passed last win, on the server and in the browser", async ({ page, consoleErrors }) => {
    const html = await (await request.get(`${host}/demo`)).text();
    const served = /id="merged" class="([^"]*)"|class="([^"]*)" id="merged"/.exec(html);
    const classes = (served?.[1] ?? served?.[2] ?? "").split(/\s+/);
    expect(classes).toEqual(expect.arrayContaining(["px-0", "text-muted-foreground"]));
    expect(classes).not.toContain("px-4");
    expect(classes).not.toContain("text-primary");
    // A ring width or colour written in brackets is replaced like any other.
    const ringed = /id="ringed" class="([^"]*)"|class="([^"]*)" id="ringed"/.exec(html);
    expect(ringed?.[1] ?? ringed?.[2]).toBe("focus-visible:ring-0 ring-blue-500");
    await hydrated(page, "/demo");
    await expect(page.locator("#merged")).toHaveClass("px-0 text-muted-foreground");
    await expect(page.locator("#ringed")).toHaveClass("focus-visible:ring-0 ring-blue-500");
    expect(consoleErrors).toEqual([]);
  });

  test("a bare border takes the theme colour and the overlay dims in both modes", async ({ page, consoleErrors }) => {
    await hydrated(page, "/demo");
    const look = await page.evaluate(() => {
      const css = (id: string) => getComputedStyle(document.getElementById(id)!);
      return {
        border: css("bordered").borderTopColor,
        text: css("bordered").color,
        theme: css("probe").color,
        scrim: css("scrim").backgroundColor,
        scrimDark: css("scrim-dark").backgroundColor,
      };
    });
    expect(look.border).toBe(look.theme);
    expect(look.border, "not the text colour").not.toBe(look.text);
    expect(look.scrim).toBe("rgba(0, 0, 0, 0.5)");
    expect(look.scrimDark).toBe("rgba(0, 0, 0, 0.5)");
    await page.goto(`${host}/bare`);
    expect(await page.evaluate(() => getComputedStyle(document.getElementById("scrim")!).backgroundColor)).toBe("rgba(0, 0, 0, 0.5)");
    expect(consoleErrors).toEqual([]);
  });
});
