import type { APIRequestContext, Page } from "@playwright/test";
import { test, expect, visit, clickUntil } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";

/**
 * The Studio's own maps: the database geo preview (a geometry cell of a
 * table) and the MapServer layer preview. Both draw with zeb/world over the
 * earth-lite basemap that ships inside Zebflow, so an office with no internet
 * still has them, and so neither page may ask anything from outside while it
 * loads: every request is recorded and one that leaves the dev server fails.
 *
 * The map says what its world says (`data-world-api`, `data-world-elements`,
 * `data-world-settled`): a real graphics API ran, the data layers are in the
 * world, and `w.settled()` resolved. The spec works in a throwaway project it
 * creates through the APIs and removes afterwards.
 *
 * Headless Chromium draws WebGL2 in software, so the waits are generous.
 */

const PROJECT = `e2e-maps-${Date.now()}`;
const SLUG = "geo-store";
const api = `/api/projects/${OWNER}/${PROJECT}`;

const AREA = { type: "Polygon", coordinates: [[[144.95, -37.82], [144.98, -37.82], [144.98, -37.8], [144.95, -37.8], [144.95, -37.82]]] };
const SITES = {
  type: "FeatureCollection",
  features: [
    { type: "Feature", properties: { name: "Area A", kind: "zone" }, geometry: { type: "Polygon", coordinates: [[[144.9, -37.85], [145.0, -37.85], [145.0, -37.78], [144.9, -37.78], [144.9, -37.85]]] } },
    { type: "Feature", properties: { name: "Route B", kind: "path" }, geometry: { type: "LineString", coordinates: [[144.85, -37.9], [145.05, -37.75]] } },
    { type: "Feature", properties: { name: "Point C", kind: "site" }, geometry: { type: "Point", coordinates: [144.88, -37.88] } },
    { type: "Feature", properties: { name: "Point D", kind: "site" }, geometry: { type: "Point", coordinates: [145.04, -37.76] } },
  ],
};

async function sql(request: APIRequestContext, connectionId: string, statement: string) {
  const response = await request.post(`${api}/db/connections/${connectionId}/query`, { data: { sql: statement, read_only: false } });
  expect(response.ok(), `${statement}: ${await response.text()}`).toBeTruthy();
}

test.beforeAll(async ({ playwright }) => {
  const request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
  const created = await request.post("/home/projects/create", { form: { project: PROJECT, title: "E2E Maps" }, maxRedirects: 0 });
  expect(created.status(), "create project").toBe(303);

  // A sekejap table with one polygon: the geo preview's cell.
  const connection = await request.post(`${api}/db/connections`, {
    data: { connection_slug: SLUG, connection_label: "Geo store", database_kind: "sekejap", config: {} },
  });
  expect(connection.ok(), `create connection: ${await connection.text()}`).toBeTruthy();
  const connectionId = (await connection.json()).connection.connection_id;
  await sql(request, connectionId, "CREATE TABLE places (_key TEXT PRIMARY KEY, name TEXT, location GEOMETRY(Polygon,4326))");
  await sql(request, connectionId, `INSERT INTO places (_key, name, location) VALUES ('a', 'Site A', '${JSON.stringify(AREA)}')`);

  // A published MapServer layer of every kind of geometry, with its tiles switched on.
  const uploaded = await request.post(`${api}/files/upload?path=mapserver`, {
    multipart: { file: { name: "sites.geojson", mimeType: "application/geo+json", buffer: Buffer.from(JSON.stringify(SITES)) } },
  });
  expect(uploaded.ok(), `upload: ${await uploaded.text()}`).toBeTruthy();
  const published = await request.post(`${api}/mapserver/default-mapserver/layers`, {
    data: { layer_id: "sites", path: "/layers/sites", source_path: "mapserver/sites.geojson", min_zoom: null, max_zoom: null, bbox_required: true, max_features: 1000, allowed_properties: ["name", "kind"] },
  });
  expect(published.ok(), `publish: ${await published.text()}`).toBeTruthy();
  const addressing = await request.get(`${api}/settings/addressing`);
  const config = (await addressing.json()).data.config;
  const surfaces = await request.put(`${api}/settings/addressing`, {
    data: { data: { ...config, disabled: (config.disabled ?? []).filter((key: string) => key !== "ms") } },
  });
  expect(surfaces.ok(), "switch the map tiles surface on").toBeTruthy();
  await request.dispose();
});

test.afterAll(async ({ playwright }) => {
  const request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
  await request.delete(`${api}/db/connections/${SLUG}`);
  await request.delete(`/api/users/${OWNER}/projects/${PROJECT}`, { data: { project_name: PROJECT, password: PASSWORD } });
  await request.dispose();
});

/** Every request the page makes that leaves the dev server, and every one of ours that failed. */
function watchRequests(page: Page) {
  const outside: string[] = [];
  const failed: string[] = [];
  const local = new URL(BASE_URL).host;
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.protocol === "data:" || url.protocol === "blob:") return;
    if (url.host !== local) outside.push(request.url());
  });
  page.on("response", (response) => {
    if (response.status() >= 400) failed.push(`${response.status()} ${response.url()}`);
  });
  return { outside, failed };
}

/** Software-rendered graphics and a GPU stall warning are not defects. */
const realErrors = (errors: string[]) => errors.filter((e) => !/GL Driver Message|WebGL: INVALID|GPU stall/.test(e));

/** The map's world: a real graphics API, these layers in it, and settled. */
async function expectSettledWorld(page: Page, layers: string[]) {
  const map = page.locator("[data-studio-map]");
  await expect(map).toHaveCount(1);
  await expect(map).toHaveAttribute("data-world-settled", "true", { timeout: 120_000 });
  expect(["webgpu", "webgl2"]).toContain(await map.getAttribute("data-world-api"));
  const elements = (await map.getAttribute("data-world-elements"))!.split(" ");
  for (const id of layers) expect(elements, `${id} is in w.list`).toContain(id);
  await expect(map.locator("canvas")).toHaveCount(1);
}

test("the geo preview draws a table's geometry with nothing from outside", async ({ page, consoleErrors }) => {
  test.setTimeout(180_000);
  const seen = watchRequests(page);
  await visit(page, `/projects/${OWNER}/${PROJECT}/db/sekejap/${SLUG}/tables?table=places`);
  expect(await page.content()).not.toContain("RWE component error");
  // The geometry cell opens the inspector, and the inspector draws the map.
  await clickUntil(page, 'td:has-text("Polygon")', async () => {
    await expect(page.locator("[data-studio-map]")).toHaveCount(1, { timeout: 5_000 });
  });
  await expectSettledWorld(page, ["geo-areas"]);
  await expect(page.getByText(/^Polygon · 5 vertices$/)).toBeVisible();
  expect(seen.outside, "no request leaves the dev server").toEqual([]);
  expect(seen.failed).toEqual([]);
  expect(realErrors(consoleErrors)).toEqual([]);
});

test("the MapServer preview draws a layer from its own vector tiles with nothing from outside", async ({ page, consoleErrors }) => {
  test.setTimeout(180_000);
  const seen = watchRequests(page);
  const tiles: string[] = [];
  page.on("request", (request) => { if (/\/layers\/sites\/\d+\/\d+\/\d+\.mvt$/.test(request.url())) tiles.push(request.url()); });
  await visit(page, `/projects/${OWNER}/${PROJECT}/db/mapserver/default-mapserver/layers`);
  expect(await page.content()).not.toContain("RWE component error");
  await clickUntil(page, 'tr:has-text("sites")', async () => {
    await expect(page.getByText("Back to layers")).toBeVisible({ timeout: 5_000 });
  });
  await expectSettledWorld(page, ["layer-areas", "layer-lines", "layer-points"]);
  expect(tiles.length, "the layer is drawn from its MVT tiles").toBeGreaterThan(0);
  // Hovering a feature shows its public properties. The view is fitted to the
  // layer, so a little above the middle is inside the area and clear of the route.
  const map = page.locator("[data-studio-map]");
  await map.scrollIntoViewIfNeeded();
  const box = (await map.boundingBox())!;
  await expect(async () => {
    await page.mouse.move(box.x + box.width / 2 - 4, box.y + box.height / 2 - 34);
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 - 30, { steps: 4 });
    await expect(page.locator(".aslnut-tooltip")).toContainText("name: Area A", { timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
  await expect(page.locator(".aslnut-tooltip")).toContainText("kind: zone");
  expect(seen.outside, "no request leaves the dev server").toEqual([]);
  expect(seen.failed).toEqual([]);
  expect(realErrors(consoleErrors)).toEqual([]);
});
