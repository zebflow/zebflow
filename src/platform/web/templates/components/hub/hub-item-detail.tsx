import { cx, useState } from "zeb/react";
import Button from "@/components/ui/button";
import FolderPicker from "@/components/ui/folder-picker";
import { kindOf, needsDestination, verbOf } from "@/components/hub/hub-kinds";

/** One labelled fact about the package. */
function Fact({ label, children }) {
  return (
    <div className="flex gap-3 text-[0.74rem]">
      <span className="w-20 shrink-0 text-body-soft">{label}</span>
      <span className="min-w-0 text-body">{children}</span>
    </div>
  );
}

/**
 * What this package would do to the project, and what it needs from the reader.
 *
 * The consequence is stated before the button, not after: an add copies files
 * into the repository and they become the reader's own, so there is no update
 * path afterwards. That is not discoverable by trying it.
 */
export default function HubItemDetail({ item, owner, project, destination, onDestinationChange, busy, onAct }) {
  const [picking, setPicking] = useState(false);
  if (!item) {
    return (
      <div className="flex h-full items-center justify-center px-6 text-center text-[0.78rem] text-body-soft">
        Choose a package to see what it would do to this project.
      </div>
    );
  }

  const kind = kindOf(item.asset_kind);
  const verb = verbOf(item.asset_kind);
  const asks = needsDestination(item.asset_kind);

  return (
    <div className="flex flex-col gap-4 px-4 py-4">
      <div>
        <p className="text-[0.9rem] font-semibold text-body">{item.title || item.package_id}</p>
        <p className="mt-1 text-[0.78rem] leading-[1.5] text-body-soft">
          {item.description || item.summary || `A ${kind.one}.`}
        </p>
      </div>

      <div className="flex flex-col gap-1.5 border-t border-border pt-3">
        <Fact label="Kind">{kind.label}</Fact>
        <Fact label="Version">{item.packed_version || item.latest_version || "—"}</Fact>
        <Fact label="From">{item.repository_title || "Local"}</Fact>
        <Fact label="Publisher">{item.publisher_display_name || item.publisher_id || "—"}</Fact>
      </div>

      {verb === "add" ? (
        <div className="border border-dark-accent4/40 bg-dark-accent4/5 px-3 py-2.5">
          <p className="text-[0.74rem] font-medium text-dark-accent4">
            Adding copies these files into your project
          </p>
          <p className="mt-1 text-[0.72rem] leading-[1.45] text-body-soft">
            They become your files — you can edit them freely. Nothing records where
            they came from, so the Hub cannot update them later.
          </p>
        </div>
      ) : (
        <div className="border border-border bg-ui-bg-muted/30 px-3 py-2.5">
          <p className="text-[0.74rem] font-medium text-body">Installing keeps this managed</p>
          <p className="mt-1 text-[0.72rem] leading-[1.45] text-body-soft">
            Recorded in <code className="font-mono">zeb.lock</code> with its exact version, so it
            can be updated and removed.
          </p>
        </div>
      )}

      {asks ? (
        <div className="flex flex-col gap-1.5">
          <span className="text-[0.7rem] font-medium uppercase tracking-[0.08em] text-body-soft">
            Destination
          </span>
          <button
            type="button"
            data-destination-toggle="true"
            onClick={() => setPicking((open) => !open)}
            className="flex items-center justify-between gap-2 rounded-md border border-ui-border bg-ui-bg px-3 py-2 text-left text-sm text-ui-text hover:border-ui-border/80"
          >
            <span className={cx("truncate", destination ? "" : "text-ui-text-soft")}>
              {destination || "Project root"}
            </span>
            <span className="shrink-0 text-[0.7rem] text-ui-text-soft">{picking ? "done" : "change"}</span>
          </button>
          {picking ? (
            <FolderPicker
              owner={owner}
              project={project}
              value={destination}
              rootLabel="Project root"
              onChange={(next) => {
                onDestinationChange(next);
                setPicking(false);
              }}
            />
          ) : null}
        </div>
      ) : null}

      {/* The action sits with what it acts on, not pinned to the bottom of a
          tall pane where it reads as belonging to nothing. */}
      <div className="flex items-center justify-end gap-2 border-t border-border pt-3">
        {/* Only an installed managed package can be removed. An added one was
            copied into the repository and became the writer's own file — the
            hub has no claim on it, which is the whole difference between the
            two verbs. */}
        {item.installed && verb === "install" ? (
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => onAct({ ...item, intent: "remove" })}
          >
            Remove
          </Button>
        ) : null}
        <Button type="button" size="sm" disabled={busy} onClick={() => onAct(item)}>
          {busy
            ? "Working…"
            : verb === "add"
              ? "Add to project"
              : item.installed
                ? item.updatable
                  ? "Update"
                  : "Reinstall"
                : "Install"}
        </Button>
      </div>
    </div>
  );
}
