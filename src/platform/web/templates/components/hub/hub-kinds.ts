/**
 * The six kinds a hub package can be, in the order a reader looks for them:
 * the things you write and add first, the things you install last.
 *
 * The six kinds a hub package can be, from `docs/contracts/distribution.md §1`.
 *
 * One place decides what each kind is called, how it looks, and — the part that
 * matters most — which verb applies to it:
 *
 *   INSTALL  lands in `data/`, stays managed, is recorded in `zeb.lock`,
 *            can be updated and uninstalled.
 *   ADD      is copied into `repo/` and becomes the receiver's own file.
 *            It is not recorded anywhere, so **there is no update path**.
 *
 * That difference is the one thing a reader cannot discover by trying it, and
 * cannot undo afterwards, so every surface that shows a package shows it.
 *
 * `template_bundle` deliberately covers pages, components, `.ts` behaviour
 * scripts and stylesheets alike. The contract is explicit that scripts are not
 * a separate mechanism, so they are not a separate chip.
 */

export type HubVerb = "install" | "add";

export const HUB_KINDS = [
  {
    id: "template_bundle",
    label: "Templates",
    one: "template",
    verb: "add" as HubVerb,
    glyph: "◇",
    tone: "text-dark-accent4",
    chipOn: "border-dark-accent4 bg-dark-accent4/10 text-dark-accent4",
  },
  {
    id: "pipeline_bundle",
    label: "Pipelines",
    one: "pipeline",
    verb: "add" as HubVerb,
    glyph: "◆",
    tone: "text-dark-accent3",
    chipOn: "border-dark-accent3 bg-dark-accent3/10 text-dark-accent3",
  },
  {
    id: "folder_bundle",
    label: "Folders",
    one: "folder",
    verb: "add" as HubVerb,
    glyph: "▤",
    tone: "text-dark-accent5",
    chipOn: "border-dark-accent5 bg-dark-accent5/10 text-dark-accent5",
  },
  {
    id: "project_bundle",
    label: "Projects",
    one: "project",
    verb: "install" as HubVerb,
    glyph: "▣",
    tone: "text-body-soft",
    chipOn: "border-border bg-ui-bg-muted text-body",
  },
  {
    id: "node_bundle",
    label: "Nodes",
    one: "node bundle",
    verb: "install" as HubVerb,
    glyph: "⬢",
    tone: "text-dark-accent1",
    chipOn: "border-dark-accent1 bg-dark-accent1/10 text-dark-accent1",
  },
  {
    id: "rwe_library",
    label: "Libraries",
    one: "library",
    verb: "install" as HubVerb,
    glyph: "◈",
    tone: "text-dark-accent2",
    chipOn: "border-dark-accent2 bg-dark-accent2/10 text-dark-accent2",
  },
];

/** The kind record for an asset, or a neutral fallback for one we do not know. */
export function kindOf(assetKind: string) {
  return (
    HUB_KINDS.find((kind) => kind.id === assetKind) || {
      id: String(assetKind || "unknown"),
      label: String(assetKind || "Unknown"),
      one: "package",
      verb: "add" as HubVerb,
      glyph: "▫",
      tone: "text-body-soft",
      chipOn: "border-border bg-ui-bg-muted text-body",
    }
  );
}

/** What happens to this package if the reader accepts it. */
export function verbOf(assetKind: string): HubVerb {
  return kindOf(assetKind).verb;
}

/** Add-kinds land in the repository, so they need somewhere to land. */
export function needsDestination(assetKind: string): boolean {
  return verbOf(assetKind) === "add";
}

/**
 * Every colour class this module can produce, so the Tailwind engine emits
 * them. They are chosen at runtime from the table above and would otherwise
 * never appear in scanned markup.
 */
export const HUB_KIND_CLASS_HINT = HUB_KINDS.map((kind) => `${kind.tone} ${kind.chipOn}`).join(" ");
