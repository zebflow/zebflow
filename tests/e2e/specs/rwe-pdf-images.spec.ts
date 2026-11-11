import { test, expect } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext } from "@playwright/test";

/**
 * zeb/pdf places images as they are, in the browser, in a throwaway project:
 *
 * - two JPEGs whose data URLs share their first 242 characters (and their
 *   length) stay two images, each with its own pixel size;
 * - a PNG with transparency gets its alpha as a separate /SMask image;
 * - webp is refused with a message that says what to do.
 */

// An 8x6 and a 12x10 JPEG from the same encoder, a 6x4 PNG whose left half is transparent, a 4x4 webp.
const JPEG_A = "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAYEBQYFBAYGBQYHBwYIChAKCgkJChQODwwQFxQYGBcUFhYaHSUfGhsjHBYWICwgIyYnKSopGR8tMC0oMCUoKSj/2wBDAQcHBwoIChMKChMoGhYaKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCj/wAARCAAGAAgDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAUEAEAAAAAAAAAAAAAAAAAAAAA/8QAFQEBAQAAAAAAAAAAAAAAAAAABgf/xAAUEQEAAAAAAAAAAAAAAAAAAAAA/9oADAMBAAIRAxEAPwCKAUMRf//Z";
const JPEG_B = "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAYEBQYFBAYGBQYHBwYIChAKCgkJChQODwwQFxQYGBcUFhYaHSUfGhsjHBYWICwgIyYnKSopGR8tMC0oMCUoKSj/2wBDAQcHBwoIChMKChMoGhYaKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCj/wAARCAAKAAwDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAf/xAAUEAEAAAAAAAAAAAAAAAAAAAAA/8QAFQEBAQAAAAAAAAAAAAAAAAAABQf/xAAUEQEAAAAAAAAAAAAAAAAAAAAA/9oADAMBAAIRAxEAPwCfgAFbf//Z";
const PNG_HALF = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAYAAAAECAYAAACtBE5DAAAAFElEQVR42mNgQAIXBAT+wzAD9SQA+jMXNa7AF4gAAAAASUVORK5CYII=";
const WEBP = "data:image/webp;base64,UklGRiQAAABXRUJQVlA4IBgAAABQAQCdASoEAAQAAgA0JaQABHQAAORAAAA=";

const PAGE = `import { useState } from "zeb/react";
import { createDocument } from "zeb/pdf";

function images(bytes) {
  let text = "";
  for (let i = 0; i < bytes.length; i++) text += String.fromCharCode(bytes[i]);
  return text.match(new RegExp("/Subtype /Image[^>]*", "g")) || [];
}

export default function PdfImages() {
  const [out, setOut] = useState("");
  function run() {
    const found = images(createDocument().page()
      .image({ data: "${JPEG_A}", x: 10, y: 10, width: 50, height: 50 })
      .image({ data: "${JPEG_B}", x: 70, y: 10, width: 50, height: 50 })
      .image({ data: "${PNG_HALF}", x: 130, y: 10, width: 60 })
      .toBytes());
    let refused = "";
    try { createDocument().page().image({ data: "${WEBP}", x: 0, y: 0, width: 10 }).toBytes(); } catch (e) { refused = e.message; }
    setOut(JSON.stringify({ found, refused }));
  }
  return <main><button id="run" onClick={run}>run</button><pre id="out">{out}</pre></main>;
}
`;

test.describe("zeb/pdf images", () => {
  let request: APIRequestContext;
  const project = `e2e-pdf-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  const base = new URL(BASE_URL);
  const host = `${base.protocol}//${project}.${OWNER}.localhost:${base.port}`;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "zeb/pdf images" }, maxRedirects: 0 });
    expect(created.status(), `create ${project}`).toBeLessThan(400);
    const put = await request.put(`${api}/repo/file?path=pages/pdf.tsx`, { data: PAGE, headers: { "Content-Type": "text/plain" } });
    expect(put.ok(), "write pages/pdf.tsx").toBeTruthy();
    for (const dsl of [
      "register pipelines/sandbox/pdf -- | trigger.webhook --route /pdf --method GET | web.response.send --template pages/pdf.tsx",
      "activate pipeline pipelines/sandbox/pdf",
    ]) {
      const res = await request.post(`${api}/pipelines/dsl`, { data: { dsl } });
      expect(JSON.stringify(await res.json()), dsl).not.toContain('"ok":false');
    }
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, { data: { project_name: project, password: PASSWORD } });
    expect(removed.ok(), `remove ${project}`).toBeTruthy();
    await request.dispose();
  });

  test("each image keeps its own bytes and size; PNG transparency becomes a mask; webp is refused", async ({ page, consoleErrors }) => {
    await page.goto(`${host}/pdf`);
    await page.waitForFunction(() => typeof (window as any).rweNavigate === "function");
    await page.locator("#run").click();
    await expect(page.locator("#out")).not.toBeEmpty();
    const { found, refused } = JSON.parse(await page.locator("#out").textContent() ?? "{}");
    const dicts: string[] = found.map((d: string) => d.replace(/\s+/g, " "));
    expect(dicts, "two JPEGs, the PNG and its mask").toHaveLength(4);
    expect(dicts.filter((d) => d.includes("/Width 8 /Height 6") && d.includes("/DCTDecode"))).toHaveLength(1);
    expect(dicts.filter((d) => d.includes("/Width 12 /Height 10") && d.includes("/DCTDecode"))).toHaveLength(1);
    expect(dicts.filter((d) => d.includes("/Width 6 /Height 4") && /\/SMask \d+ 0 R/.test(d))).toHaveLength(1);
    expect(dicts.filter((d) => d.includes("/Width 6 /Height 4") && d.includes("/DeviceGray"))).toHaveLength(1);
    expect(refused).toContain("convert the image to PNG or JPEG");
    expect(consoleErrors).toEqual([]);
  });
});
