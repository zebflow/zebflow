import { cx } from "zeb/react";

/**
 * The bordered block every settings panel sits in.
 *
 * Flat and edge-to-edge, which is the Settings page's language. It is not
 * `StudioPanel` (rounded, composable) and it is not the Hub's card — dropping
 * it into a page of rounded cards makes it read as a block that lost its
 * container, which is exactly what happened when a Hub panel borrowed it.
 */

export const SETTINGS_SECTION_TAG_CLASS =
  "inline-flex items-center border border-border bg-border px-2 py-[0.38rem] text-[0.66rem] font-mono uppercase tracking-[0.12em] text-muted-foreground";

export default function SettingsSection({
  id,
  title,
  description,
  tag,
  tone = "default",
  children,
}: any) {
  const titleClass =
    tone === "danger"
      ? "text-destructive"
      : "text-foreground";

  return (
    <article
      id={id}
      className={cx(
        "border-b border-border",
        tone === "danger" && "border-warning",
      )}
    >
      <header className="flex items-start justify-between gap-3 px-4 py-3">
        <div className="min-w-0">
          <h3 className={cx("text-[0.83rem] font-semibold tracking-[0.01em]", titleClass)}>
            {title}
          </h3>
          {description ? (
            <p className="mt-1 text-[0.78rem] leading-[1.45] text-muted-foreground">
              {description}
            </p>
          ) : null}
        </div>
        {tag ? <span className={SETTINGS_SECTION_TAG_CLASS}>{tag}</span> : null}
      </header>
      <div className="px-4 py-4">{children}</div>
    </article>
  );
}
