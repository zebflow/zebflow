import { Link } from "zeb/react";
import Badge from "@/components/ui/badge";
import { formatOperationTimestamp } from "@/pages/project-studio/settings/components/settings-lib";

function operationBadgeClass(status) {
  if (status === "completed") {
    return "!rounded-none !border !border-info !bg-transparent !text-info";
  }
  if (status === "failed") {
    return "!rounded-none !border !border-warning !bg-transparent !text-warning";
  }
  return "!rounded-none !border !border-border !bg-transparent !text-muted-foreground";
}

/** One export or import, with its status and — if it produced one — its artifact. */
export default function TransferOperationRow({ owner, project, item }) {
  const status = item?.status;

  return (
    <div className="border border-border px-[0.7rem] py-[0.7rem]">
      <div className="flex items-center justify-between gap-3">
        <div>
          <div className="text-[0.78rem] font-medium text-foreground">{item?.kind ?? "operation"}</div>
          <div className="text-[0.72rem] text-muted-foreground">{item?.current_step || status}</div>
        </div>
        <Badge
          variant={status === "completed" ? "secondary" : status === "failed" ? "destructive" : "outline"}
          className={operationBadgeClass(status)}
          label={status ?? "unknown"}
        />
      </div>
      <div className="mt-2 text-[0.72rem] text-muted-foreground">
        Updated {formatOperationTimestamp(item?.updated_at)}
      </div>
      {item?.error_message ? (
        <div className="mt-2 text-[0.72rem] text-red-300">{item.error_message}</div>
      ) : null}
      {item?.artifact_rel_path ? (
        <div className="mt-2">
          <Link
            href={`/api/projects/${owner}/${project}/transfer/download/${item.operation_id}`}
            className="text-[0.72rem] text-primary"
          >
            Download artifact
          </Link>
        </div>
      ) : null}
    </div>
  );
}
