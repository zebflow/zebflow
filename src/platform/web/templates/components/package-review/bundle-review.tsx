import Badge from "@/components/ui/badge";
import InstallConsent from "@/components/package-review/install-consent";
import ReviewList from "@/components/package-review/review-list";
import SqlReport from "@/components/package-review/sql-report";
import { ViolationNotice, WarningNotice } from "@/components/package-review/findings";

/**
 * A project bundle install, described before any of it happens.
 *
 * The review answers for the consent flags it was handed, so the flags are part
 * of the report rather than a separate form beside it: change one and every list
 * below changes with it.
 */
export default function BundleReview({ review, scope, onScopeChange, busy = false, dirty = false }) {
  const installable = !!review?.installable;
  const executeSchema = scope?.include_schema !== false && scope?.execute_schema !== false;

  return (
    <div className="space-y-3">
      <InstallConsent scope={scope} onChange={onScopeChange} disabled={busy} />

      {dirty ? (
        <p className="m-0 rounded-lg border border-border bg-accent/40 px-3 py-2 text-xs text-muted-foreground">
          The consent flags changed. The report below still answers for the previous ones — refresh
          the review before installing.
        </p>
      ) : null}

      {review ? (
        <>
          <div className="rounded-lg border border-border bg-popover p-3">
            <div className="flex flex-wrap items-center gap-2">
              <p className="m-0 font-mono text-sm font-semibold text-foreground">
                {String(review.package_id || "")}@{String(review.version || "")}
              </p>
              <Badge variant="outline" label={String(review.asset_kind || "")} />
              <Badge
                variant={installable ? "secondary" : "destructive"}
                label={installable ? `risk ${String(review.risk_level || "")}` : `blocked · risk ${String(review.risk_level || "")}`}
              />
            </div>
            <p className="m-0 mt-2 text-xs text-muted-foreground">
              Would create the project <span className="font-mono text-foreground">{String(review.owner || "")}/{String(review.project || "")}</span>.
              That name is a prediction of the first free slug: a name free now can be claimed before
              you install, and the install moves on to the next one.
            </p>
            <p className="m-0 mt-1 text-xs text-muted-foreground">
              Schema execution for this review: <span className="font-mono text-foreground">{review.schema_executed ? "the SQL runs" : "nothing runs"}</span>.
            </p>
          </div>

          <ViolationNotice items={review.violations} subject="This bundle" />
          <WarningNotice items={review.warnings} />

          <SqlReport
            reports={review.database_initialization}
            executeSchema={executeSchema}
            emptyNote={
              scope?.include_schema === false
                ? "No SQL is described because you turned the schema off. Any .sql this package carries is listed under files skipped by your consent, and is neither written nor run."
                : "This bundle carries no install-time SQL. Nothing is replayed into any store."
            }
          />

          <div className="grid gap-2 md:grid-cols-2">
            <ReviewList title="Files written" items={review.files_written} />
            <ReviewList
              title="Files skipped by your consent"
              items={review.skipped_files}
              emptyNote="Nothing skipped."
            />
            <ReviewList title="Pipelines registered" items={review.pipelines_registered} />
            <ReviewList title="Pipelines activated" items={review.pipelines_activated} />
            <ReviewList
              title="Named active, not activated"
              items={review.pipelines_not_activated}
              tone="danger"
              emptyNote="None — every pipeline the bundle names active is activated."
            />
            <ReviewList
              title="SQL written, left for you to run"
              items={review.unexecuted_initial_data}
              emptyNote="None."
            />
            <ReviewList title="Nodes used" items={review.nodes_used} />
            <ReviewList title="Credentials required" items={review.credentials_required} tone="danger" />
            <ReviewList title="External URLs contacted" items={review.external_urls} tone="danger" />
            <ReviewList title="Public endpoints created" items={review.public_endpoints} tone="danger" />
            <ReviewList title="Database effects" items={review.database_effects} tone="danger" />
            <ReviewList title="Filesystem effects" items={review.filesystem_effects} />
            <ReviewList title="Outbound connections" items={review.network_effects} tone="danger" />
            <ReviewList title="Runs supplied code" items={review.code_execution} tone="danger" />
            <ReviewList title="Schedules" items={review.schedules} tone="danger" />
            <ReviewList title="Large files" items={review.large_files} />
            <ReviewList title="Seed / demo data" items={review.seed_data} />
          </div>
        </>
      ) : (
        <p className="m-0 rounded-lg border border-border bg-popover px-3 py-6 text-center text-sm text-muted-foreground">
          {busy ? "Reading the package…" : "No review yet. Refresh it to see what this install would do."}
        </p>
      )}
    </div>
  );
}
