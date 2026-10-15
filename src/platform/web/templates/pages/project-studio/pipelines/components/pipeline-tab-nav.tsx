import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";

/**
 * The pipelines area's tabs: the registry, then one tab per trigger kind.
 *
 * Both the pipelines page and the unified editor draw it, from the `nav`
 * the server builds, so a new tab is added here once.
 */
export default function PipelineTabNav({ nav }) {
  const links = nav?.links ?? {};
  const classes = nav?.classes ?? {};
  return (
    <StudioTabNav>
      <StudioTabLink href={links.pipelines_registry ?? "#"} active={!!classes.pipeline_registry}>Registry</StudioTabLink>
      <StudioTabLink href={links.pipelines_webhooks ?? "#"} active={!!classes.pipeline_webhooks}>Webhooks</StudioTabLink>
      <StudioTabLink href={links.pipelines_schedules ?? "#"} active={!!classes.pipeline_schedules}>Schedules</StudioTabLink>
      <StudioTabLink href={links.pipelines_manual ?? "#"} active={!!classes.pipeline_manual}>Manual</StudioTabLink>
      <StudioTabLink href={links.pipelines_functions ?? "#"} active={!!classes.pipeline_functions}>Functions</StudioTabLink>
    </StudioTabNav>
  );
}
