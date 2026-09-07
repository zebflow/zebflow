import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import { requestJson } from "@/components/lib/http";

const PRIMARY = "!rounded-none";
const OUTLINE =
  "!rounded-none !border !border-dark-border !bg-transparent !text-body hover:!bg-dark-border";

const KINDS = [
  { key: "bundle", label: "Bundle", variant: "primary" },
  { key: "files", label: "Files", variant: "outline" },
  { key: "full", label: "Full", variant: "outline" },
];

/**
 * Every way an archive leaves or enters the project.
 *
 * Owns the chosen files and which request is in flight; the panel above only
 * hears that the operation list moved and what to say about it. Threading
 * that state upward instead would have cost eight props for no reader.
 */
export default function TransferActionsGrid({ api, onOperationsChanged, onStatus }) {
  const [picked, setPicked] = useState({ bundle: null, files: null, full: null });
  const [busyKey, setBusyKey] = useState("");

  async function send(direction, kind) {
    const url = api?.[`${direction}_${kind}`];
    const file = picked[kind];
    if (!url || (direction === "import" && !file)) return;

    const label = KINDS.find((k) => k.key === kind)?.label ?? kind;
    setBusyKey(`${direction}:${kind}`);
    onStatus(direction === "export" ? `Preparing ${kind} export…` : `Importing ${kind} archive…`, "info");
    try {
      let body;
      if (direction === "import") {
        body = new FormData();
        body.append("archive", file);
      }
      const payload = await requestJson(url, { method: "POST", body });
      if (direction === "export" && payload?.download_url) {
        window.location.href = payload.download_url;
      }
      if (direction === "import") {
        setPicked((prev) => ({ ...prev, [kind]: null }));
      }
      onStatus(`${label} ${direction === "export" ? "export ready." : "import applied."}`, "ok");
    } catch (err) {
      onStatus(
        `${direction === "export" ? "Export" : "Import"} failed: ${err?.message || String(err)}`,
        "error",
      );
    } finally {
      setBusyKey("");
      onOperationsChanged();
    }
  }

  return (
    <div className="grid gap-3 md:grid-cols-2">
      <section className="bg-dark-border px-4 py-4">
        <p className="text-[0.8rem] font-medium text-body">Export</p>
        <p className="mt-2 text-[0.78rem] leading-[1.45] text-body-soft">
          Credentials and DB connections stay platform-managed. Caches, installed hub content, logs,
          and recovery copies stay home and rebuild or regenerate.
        </p>
        <div className="mt-4 flex flex-wrap gap-2">
          {KINDS.map((kind) => (
            <Button
              key={kind.key}
              type="button"
              variant={kind.variant}
              size="sm"
              className={kind.variant === "primary" ? PRIMARY : OUTLINE}
              disabled={busyKey !== ""}
              onClick={() => send("export", kind.key)}
            >
              {busyKey === `export:${kind.key}` ? "Preparing…" : `Export ${kind.label}`}
            </Button>
          ))}
        </div>
      </section>

      <section className="bg-dark-border px-4 py-4">
        <p className="text-[0.8rem] font-medium text-body">Import</p>
        <p className="mt-2 text-[0.78rem] leading-[1.45] text-body-soft">
          Apply one archive at a time to the current project. This replaces that scope on the target
          project.
        </p>
        <div className="mt-4 grid gap-3">
          {KINDS.map((kind) => (
            <div key={kind.key} className="grid gap-3">
              <Field label={`${kind.label} archive`}>
                <input
                  type="file"
                  accept=".tar"
                  onChange={(event) => {
                    const files = event?.target?.files;
                    setPicked((prev) => ({ ...prev, [kind.key]: files && files[0] ? files[0] : null }));
                  }}
                />
              </Field>
              <Button
                type="button"
                variant={kind.variant}
                size="sm"
                className={kind.variant === "primary" ? PRIMARY : OUTLINE}
                disabled={!picked[kind.key] || busyKey !== ""}
                onClick={() => send("import", kind.key)}
              >
                {busyKey === `import:${kind.key}` ? "Importing…" : `Import ${kind.label}`}
              </Button>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
