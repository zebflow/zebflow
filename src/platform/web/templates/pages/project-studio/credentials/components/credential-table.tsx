import { cx } from "zeb/react";
import Button from "@/components/ui/button";
import Badge from "@/components/ui/badge";
import { StudioTable, StudioThead, StudioTh } from "@/components/ui/studio-data-table";

export default function CredentialTable({ items, onEdit }: { items: any[]; onEdit: (item: any) => void }) {
  function formatTs(ts) {
    if (!Number.isFinite(Number(ts))) return "-";
    return new Date(Number(ts) * 1000).toISOString().slice(0, 19).replace("T", " ");
  }

  return (
      <StudioTable>
        <StudioThead>
          <tr>
            <StudioTh>ID</StudioTh>
            <StudioTh>Title</StudioTh>
            <StudioTh>Kind</StudioTh>
            <StudioTh>Managed by</StudioTh>
            <StudioTh>Roles</StudioTh>
            <StudioTh>Secret</StudioTh>
            <StudioTh>Updated</StudioTh>
            <StudioTh>Action</StudioTh>
          </tr>
        </StudioThead>
        <tbody>
          {items.length === 0 ? (
            <tr>
              <td colSpan={8} className="px-3 py-4 text-sm text-muted-foreground text-center">No credentials yet</td>
            </tr>
          ) : items.map((item) => (
            <tr key={item.credential_id}>
              <td className="px-3 py-2 text-sm font-mono text-foreground">{item.credential_id}</td>
              <td className="px-3 py-2 text-sm text-foreground">{item.title}</td>
              <td className="px-3 py-2 text-sm text-muted-foreground">{item.kind}</td>
                  <td className="px-3 py-2 text-sm text-muted-foreground">
                    {item.managed_by ? <Badge variant="outline" className="text-xs" title="Created through this fence's door; only that door and maintainers may change it">door · {item.managed_by}</Badge> : "person"}
                  </td>
              <td className="px-3 py-2">
                <div className="flex flex-wrap gap-1">
                  {(item.auth_roles || []).map((r) => (
                    <Badge key={r} variant="outline" className="text-xs">{r}</Badge>
                  ))}
                </div>
              </td>
              <td className="px-3 py-2 text-sm text-muted-foreground">
                {item.kind === "oauth2" && item.oauth2_status ? (
                  <span className="inline-flex items-center gap-1.5">
                    <span className={cx("inline-block w-1.5 h-1.5 rounded-full",
                      item.oauth2_status === "authorized" ? "bg-green-500" :
                      item.oauth2_status === "expired" ? "bg-amber-500" : "bg-zinc-400"
                    )} />
                    {item.oauth2_status}
                  </span>
                ) : item.has_secret ? "yes" : "no"}
              </td>
              <td className="px-3 py-2 text-sm text-muted-foreground">{formatTs(item.updated_at)}</td>
              <td className="px-3 py-2">
                <Button size="xs" variant="outline" onClick={() => onEdit(item)}>Edit</Button>
              </td>
            </tr>
          ))}
        </tbody>
      </StudioTable>
  );
}
