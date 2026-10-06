import { test, expect, clickUntil } from "../fixtures";
import { OWNER, PASSWORD } from "../config";

/**
 * Zeb React's `onChange` is React's: on a text input or textarea it fires per
 * keystroke, so a controlled field and a live search update while typing — the
 * field keeps focus throughout, so no blur is involved. Checkboxes and selects
 * fire on change, and a save hung on `onBlur` runs once, not per keystroke.
 */
test("onChange on a controlled text field updates state on every keystroke", async ({ page, request, consoleErrors }) => {
  const project = `e2e-rwe-onchange-${Date.now()}`;
  const base = `/api/projects/${OWNER}/${project}`;
  const route = `/wh/${OWNER}/${project}/typing`;
  const source = `
import { useState } from "zeb/react";
export default function Page() {
  const [q, setQ] = useState("");
  const [note, setNote] = useState("");
  const [changes, setChanges] = useState(0);
  const [saves, setSaves] = useState(0);
  const [on, setOn] = useState(false);
  const [pick, setPick] = useState("a");
  const [ready, setReady] = useState(false);
  const items = ["alpha", "beta", "gamma"].filter((item) => item.includes(q));
  return <main>
    <button id="ready" onClick={() => setReady(true)}>{ready ? "hydrated" : "ready"}</button>
    <input id="q" value={q} onChange={(e) => { setQ(e.target.value); setChanges((n) => n + 1); }} onBlur={() => setSaves((n) => n + 1)} />
    <textarea id="note" value={note} onChange={(e) => setNote(e.target.value)} />
    <input id="on" type="checkbox" checked={on} onChange={(e) => setOn(e.target.checked)} />
    <select id="pick" value={pick} onChange={(e) => setPick(e.target.value)}><option value="a">A</option><option value="b">B</option></select>
    <p id="mirror">{q}|{note}|{changes}|{saves}|{on ? "on" : "off"}|{pick}</p>
    <ul id="results">{items.map((item) => <li key={item}>{item}</li>)}</ul>
  </main>;
}`;
  try {
    const created = await request.post("/home/projects/create", { form: { project, title: "RWE onChange" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);
    expect((await request.put(`${base}/repo/file?path=typing.tsx`, {
      data: source, headers: { "Content-Type": "text/plain" },
    })).ok()).toBeTruthy();
    const graph = {
      id: "typing", entry_nodes: ["trigger"],
      nodes: [
        { id: "trigger", kind: "trigger.webhook", input_pins: [], output_pins: ["out"], config: { route: "/typing", method: "GET" } },
        { id: "page", kind: "web.response.send", input_pins: ["in"], output_pins: ["out"], config: { template: "typing.tsx" } },
      ],
      edges: [{ from_node: "trigger", from_pin: "out", to_node: "page", to_pin: "in" }],
    };
    expect((await request.post(`${base}/pipelines/definition`, { data: {
      file_rel_path: "typing.zf.json", title: "Typing", trigger_kind: "webhook",
      source: JSON.stringify({ apiVersion: "zebflow.com/v1", kind: "Pipeline", metadata: { name: "typing" }, spec: graph }),
    } })).ok()).toBeTruthy();
    expect((await request.post(`${base}/pipelines/activate`, { data: { file_rel_path: "typing.zf.json" } })).ok()).toBeTruthy();

    await page.goto(route);
    // Hydrated once a click changes state.
    await clickUntil(page, "#ready", async () => {
      await expect(page.locator("#ready")).toHaveText("hydrated", { timeout: 3000 });
    });
    await page.locator("#on").check();
    await expect(page.locator("#mirror")).toHaveText("||0|0|on|a");

    const q = page.locator("#q");
    await q.focus();
    for (const [index, key] of ["b", "e", "t"].entries()) {
      await page.keyboard.type(key);
      const typed = "bet".slice(0, index + 1);
      // Still focused: the update came from the keystroke, not from a blur.
      await expect(q).toBeFocused();
      await expect(page.locator("#mirror")).toHaveText(`${typed}||${index + 1}|0|on|a`);
    }
    await expect(page.locator("#results li")).toHaveText(["beta"]);
    await page.keyboard.press("Backspace");
    await expect(page.locator("#mirror")).toHaveText("be||4|0|on|a");

    // Leaving the field is one save, and no extra change.
    await page.locator("#note").focus();
    await page.keyboard.type("hi");
    await expect(page.locator("#mirror")).toHaveText("be|hi|4|1|on|a");

    await page.locator("#on").uncheck();
    await page.locator("#pick").selectOption("b");
    await expect(page.locator("#mirror")).toHaveText("be|hi|4|1|off|b");
    expect(consoleErrors).toEqual([]);
  } finally {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, {
      data: { project_name: project, password: PASSWORD },
    });
    expect(removed.ok(), "remove isolated onChange test project").toBeTruthy();
  }
});
