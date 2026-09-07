import { cx, useState } from "zeb/react";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";
import HubItemRow from "@/components/hub/hub-item-row";
import HubItemDetail from "@/components/hub/hub-item-detail";
import { HUB_KINDS, HUB_KIND_CLASS_HINT, kindOf } from "@/components/hub/hub-kinds";

/**
 * Which packages the reader is looking at.
 *
 * State comes first because it is the question people actually arrive with —
 * "do I already have this" before "what type is it". Only installed kinds can
 * answer it: added content is copied into the repository and recorded nowhere,
 * so it is always simply available.
 */
const STATES = [
  { id: "project", label: "In project" },
  { id: "available", label: "Available" },
  { id: "updatable", label: "Updatable" },
  { id: "problems", label: "Problems" },
];

function matchesState(item, state) {
  if (state === "project") return !!item?.installed;
  if (state === "updatable") return !!item?.updatable;
  if (state === "problems") return !!item?.status && item.status !== "resolved";
  return !item?.installed;
}

function matchesSearch(item, text) {
  if (!text) return true;
  const needle = text.toLowerCase();
  return [item?.title, item?.package_id, item?.summary, item?.description]
    .filter(Boolean)
    .some((field) => String(field).toLowerCase().includes(needle));
}

/**
 * The one catalogue.
 *
 * The Hub page and the editor's "Add from hub" dialog are the same list, the
 * same filters and the same consequences — they differ only in where they open
 * and whether a destination is already known. Building it twice is how they
 * drifted into two vocabularies for one idea.
 */
export default function HubBrowser({ items, owner, project, initialState, destination, onDestinationChange, onAct, busy }) {
  const [state, setState] = useState(initialState || "available");
  const [kind, setKind] = useState("");
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState("");

  const all = Array.isArray(items) ? items : [];
  const visible = all.filter(
    (item) =>
      matchesState(item, state) &&
      (!kind || item?.asset_kind === kind) &&
      matchesSearch(item, search),
  );
  // The reader's choice survives a filter that stops matching it. Installing a
  // package moves it out of "Available", and looking only at `visible` meant
  // the pane silently switched to an unrelated package the moment the install
  // finished — so the Remove action, which had just become available, appeared
  // to be missing.
  const selected =
    all.find((item) => item?.package_id === selectedId) || visible[0] || null;
  const problemCount = all.filter((item) => item?.status && item.status !== "resolved").length;

  return (
    <div className="flex min-h-0 flex-1 flex-col" data-hub-browser="true">
      {/* tw-variants hint: per-kind colours are chosen at runtime from the kind
          table, so they never appear in scanned markup on their own. */}
      <span hidden tw-variants={HUB_KIND_CLASS_HINT} />

      <div className="flex flex-wrap items-center gap-2 border-b border-border px-3 py-2.5">
        <Input
          className="min-w-[14rem] flex-1"
          value={search}
          placeholder="Search packages…"
          onInput={(event) => setSearch(event.currentTarget.value)}
        />
        <div className="flex items-center gap-1">
          {STATES.map((entry) => (
            <button
              key={entry.id}
              type="button"
              data-hub-state={entry.id}
              onClick={() => setState(entry.id)}
              className={cx(
                "rounded px-2.5 py-1 text-[0.72rem] font-medium transition-colors",
                state === entry.id
                  ? "bg-ui-bg-muted text-body"
                  : "text-body-soft hover:bg-ui-bg-muted/60 hover:text-body",
              )}
            >
              {entry.label}
              {entry.id === "problems" && problemCount ? (
                <span className="ml-1 text-dark-accent4">{problemCount}</span>
              ) : null}
            </button>
          ))}
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-1.5 border-b border-border px-3 py-2">
        <button
          type="button"
          onClick={() => setKind("")}
          className={cx(
            "rounded-full border px-2.5 py-[3px] text-[0.7rem] transition-colors",
            kind ? "border-border text-body-soft hover:text-body" : "border-border bg-ui-bg-muted text-body",
          )}
        >
          All
        </button>
        {HUB_KINDS.map((entry) => (
          <button
            key={entry.id}
            type="button"
            data-hub-kind-chip={entry.id}
            onClick={() => setKind(kind === entry.id ? "" : entry.id)}
            className={cx(
              "flex items-center gap-1.5 rounded-full border px-2.5 py-[3px] text-[0.7rem] transition-colors",
              kind === entry.id ? entry.chipOn : "border-border text-body-soft hover:text-body",
            )}
          >
            <span className={cx("leading-none", kind === entry.id ? "" : entry.tone)} aria-hidden="true">
              {entry.glyph}
            </span>
            {entry.label}
          </button>
        ))}
      </div>

      {state === "problems" && problemCount ? (
        <div className="flex items-center justify-between gap-3 border-b border-dark-accent4/40 bg-dark-accent4/5 px-3 py-2">
          <p className="text-[0.74rem] text-body-soft">
            <span className="font-medium text-dark-accent4">{problemCount}</span> installed package(s)
            do not match <code className="font-mono">zeb.lock</code> — missing, or the bytes changed.
          </p>
          <Button type="button" variant="outline" size="sm" onClick={() => onAct({ repair_all: true })}>
            Repair all
          </Button>
        </div>
      ) : null}

      <div className="grid flex-1 items-start grid-cols-[minmax(0,1fr)_minmax(0,22rem)]">
        <div className="border-r border-border">
          {visible.length ? (
            visible.map((item) => (
              <HubItemRow
                key={item?.package_id}
                item={item}
                selected={selected?.package_id === item?.package_id}
                onSelect={(next) => setSelectedId(next?.package_id)}
              />
            ))
          ) : (
            <p className="px-3 py-6 text-center text-[0.76rem] text-body-soft">
              {kind ? `No ${kindOf(kind).label.toLowerCase()} here.` : "Nothing matches."}
            </p>
          )}
        </div>

        <div className="min-h-0 self-start sticky top-0 max-h-[calc(100dvh-8rem)] overflow-y-auto">
          <HubItemDetail
            item={selected}
            owner={owner}
            project={project}
            destination={destination}
            onDestinationChange={onDestinationChange}
            busy={busy}
            onAct={onAct}
          />
        </div>
      </div>
    </div>
  );
}
