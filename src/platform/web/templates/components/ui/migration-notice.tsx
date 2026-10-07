import Alert from "@/components/ui/alert";

/**
 * A project the 0.11 migration at server start left for its owner: the plan
 * was not ready, or holds items to review. Links to the readable plan.
 * Renders nothing when there is no notice.
 */
export default function MigrationNotice({ notice, className }) {
  if (!notice) return null;
  const items = Number(notice.items) || 0;
  return (
    <Alert variant="warning" className={className}>
      <a href={notice.plan_href} target="_blank" rel="noopener" data-migration-notice className="underline">
        needs migration — {items} {items === 1 ? "item" : "items"} to review
      </a>
    </Alert>
  );
}
