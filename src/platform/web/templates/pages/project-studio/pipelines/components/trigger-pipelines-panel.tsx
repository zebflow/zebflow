import { Link } from "zeb/react";
import Badge from "@/components/ui/badge";
import WebhookRouteTree from "@/components/ui/webhook-route-tree";
import { PipelineIcon } from "@/pages/project-studio/pipelines/registry/components/editor-icons";
import PipelineTabHeader from "@/pages/project-studio/pipelines/components/pipeline-tab-header";

/**
 * One trigger kind's pipelines: webhooks as a route tree, every other kind as
 * a list of rows that open the editor.
 */
export default function TriggerPipelinesPanel({ title, subtitle, items, webhooks }) {
  const list = Array.isArray(items) ? items : [];
  return (
    <div className="flex-1 min-h-0 overflow-auto flex flex-col">
      <PipelineTabHeader title={title} subtitle={subtitle} trailing={<Badge variant="secondary">{list.length}</Badge>} />
      {webhooks ? (
        <div className="flex-1 min-h-0 overflow-auto px-[0.875rem] py-3">
          <WebhookRouteTree items={list} />
        </div>
      ) : (
        <div className="flex flex-col py-2 px-3 gap-1">
          {list.map((item, index) => (
            <Link
              key={`${item?.name ?? "pipeline"}-${index}`}
              href={item?.editor_href ?? "#"}
              className="pipeline-registry-row"
            >
              <span className="shrink-0 flex items-center text-muted-foreground"><PipelineIcon className="w-4 h-4" /></span>
              <span className="flex-1 min-w-0 flex flex-col gap-[0.15rem]">
                <span className="flex items-center gap-2 flex-wrap">
                  <span className="pipeline-registry-row-name">{item?.title || item?.name}</span>
                  {item?.virtual_path && item?.virtual_path !== "/" ? (
                    <span className="text-[0.65rem] text-muted-foreground font-mono">{item?.virtual_path}</span>
                  ) : null}
                </span>
                {item?.description ? (
                  <span className="text-[0.72rem] text-muted-foreground leading-snug">{item?.description}</span>
                ) : null}
              </span>
            </Link>
          ))}
          {list.length === 0 ? (
            <p className="p-6 text-center text-[0.78rem] text-muted-foreground">
              No {String(title ?? "pipelines").toLowerCase()} yet. Create one in the Registry tab.
            </p>
          ) : null}
        </div>
      )}
    </div>
  );
}
