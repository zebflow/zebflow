import { useState } from "zeb/react";
import ChromeHeader from "@/pages/home/components/chrome-header";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardDescription from "@/components/ui/card-description";
import CardTitle from "@/components/ui/card-title";
import { formatBytes } from "@/components/lib/format";
import { requestJson } from "@/components/lib/http";
import MemoryShareField from "@/pages/settings/components/memory-share-field";

export const page = {
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-background text-foreground font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "Settings",
      description: input?.seo?.description ?? "This office's own settings",
    },
  };
}

/** The page's sections. Later ones, such as an outbound allow-list, join here. */
const SECTIONS = [{ id: "runtime", label: "Runtime" }];

export default function Page(input) {
  const [effective, setEffective] = useState(input?.effective ?? {});
  const [heap, setHeap] = useState(input?.settings?.javascript?.heap ?? "auto");
  const [queue, setQueue] = useState(input?.settings?.runs?.record_queue ?? "auto");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [tone, setTone] = useState("muted");
  const api = input?.settings_api ?? "/api/platform/settings";
  const memory = effective?.memory ?? {};
  const limitFrom = memory?.source === "container" ? "container limit" : "machine memory";

  const save = async (event) => {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    setTone("muted");
    try {
      const data = await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({
          javascript: { heap: heap.trim() },
          runs: { record_queue: queue.trim() },
        }),
      });
      if (!data) return;
      setEffective(data.effective ?? {});
      setMessage("Saved. Each runtime takes the new limit before its next run.");
      setTone("ok");
    } catch (err) {
      setMessage(String(err?.message ?? "Failed to save the settings."));
      setTone("error");
    } finally {
      setBusy(false);
    }
  };

  const messageClass =
    tone === "ok" ? "text-emerald-700" : tone === "error" ? "text-red-600" : "text-gray-500";

  return (
    <>
      <ChromeHeader superadmin />
      <main className="pb-16 pt-24">
        <section className="mx-auto max-w-4xl px-6">
          <header className="mb-8 border-b border-gray-200 pb-4">
            <h1 className="text-3xl font-black tracking-tighter text-gray-900">Settings</h1>
            <p className="mt-2 text-sm text-gray-500">
              This office's own settings. Each office has its own, because each has its own memory.
            </p>
          </header>

          <div className="grid gap-6 md:grid-cols-[10rem_1fr]">
            <nav className="flex flex-col gap-1 text-sm">
              {SECTIONS.map((section) => (
                <a
                  key={section.id}
                  href={`#${section.id}`}
                  className="rounded-md bg-gray-100 px-3 py-2 font-medium text-gray-900"
                >
                  {section.label}
                </a>
              ))}
            </nav>

            <section id="runtime">
              <Card>
                <CardContent className="py-5">
                  <CardTitle className="text-lg">Runtime</CardTitle>
                  <CardDescription className="mt-1">
                    Memory limit: {formatBytes(memory?.bytes)} ({limitFrom}). A share written as
                    auto is taken from it; a size is written with its unit, such as 512MiB.
                  </CardDescription>
                  <form className="mt-5 flex flex-col gap-5" onSubmit={save}>
                    <MemoryShareField
                      id="settings-javascript-heap"
                      label="JavaScript heap"
                      description="What every JavaScript runtime's heap may reach together: the script nodes' and the page renderer's. A script past its share fails alone. auto is a quarter of the memory limit."
                      value={heap}
                      onChange={setHeap}
                      comesTo={`${formatBytes(effective?.javascript_heap)} across ${effective?.javascript_runtimes ?? 0} runtimes, ${formatBytes(effective?.javascript_heap_per_runtime)} each`}
                    />
                    <MemoryShareField
                      id="settings-record-queue"
                      label="Run-record queue"
                      description="What the run records waiting to be written may hold together. When it is full a successful run's record is not kept and the run is counted in its history; failures have twice the room. auto is a hundredth of the memory limit."
                      value={queue}
                      onChange={setQueue}
                      comesTo={formatBytes(effective?.record_queue)}
                    />
                    <div className="flex flex-wrap items-center gap-3 pt-1">
                      <Button type="submit" variant="primary" disabled={busy}>
                        {busy ? "Saving..." : "Save settings"}
                      </Button>
                      {message ? <span className={`text-sm ${messageClass}`}>{message}</span> : null}
                    </div>
                  </form>
                </CardContent>
              </Card>
            </section>
          </div>
        </section>
      </main>
    </>
  );
}
