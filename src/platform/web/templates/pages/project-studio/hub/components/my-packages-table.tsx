import { requestJson } from "@/components/lib/http";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import Button from "@/components/ui/button";

function fmtTs(ts) {
  const n = Number(ts || 0);
  if (!n) return "-";
  const dt = new Date(n * 1000);
  if (Number.isNaN(dt.getTime())) return "-";
  return dt.toISOString().slice(0, 19).replace("T", " ");
}

/**
 * The packages this project has published, with Retract.
 *
 * Retracting needs the publisher token typed into the publish form, so the
 * page passes it in.
 */
export default function MyPackagesTable({ myPacks, api, publisherToken, refresh, showStatus }) {
  async function retractAsset(item) {
    const packageId = item?.package_id;
    if (!packageId) return;
    if (!publisherToken) {
      showStatus("Publisher token is required to retract a package.");
      return;
    }
    if (!window.confirm(`Retract ${packageId}? Its files are destroyed, its name and versions stay listed, and they can never be published again.`)) {
      return;
    }
    showStatus(`Retracting ${packageId}...`);
    try {
      const payload = await requestJson(`${api.assets}/${encodeURIComponent(packageId)}`, {
        method: "DELETE",
        headers: { Authorization: `Bearer ${publisherToken}` },
      });
      await refresh();
      showStatus(`Retracted ${packageId} (${payload?.retracted_versions || 0} version(s))`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  return (
    <StudioTable>
      <StudioThead>
        <tr>
          <StudioTh>Package</StudioTh>
          <StudioTh>Title</StudioTh>
          <StudioTh>Kind</StudioTh>
          <StudioTh>Version</StudioTh>
          <StudioTh>Visibility</StudioTh>
          <StudioTh>Updated</StudioTh>
          <StudioTh>Action</StudioTh>
        </tr>
      </StudioThead>
      <tbody>
        {myPacks.map((item, index) => (
          <tr key={`${item?.package_id ?? "mine"}-${index}`}>
            <StudioTd>
              <div className="flex items-center gap-3">
                {item?.image_url ? <img src={item.image_url} alt="" className="h-10 w-14 rounded-md object-cover border border-border" /> : null}
                <span>{item?.package_id}</span>
              </div>
            </StudioTd>
            <StudioTd>{item?.title}</StudioTd>
            <StudioTd>{item?.asset_kind}</StudioTd>
            <StudioTd>{item?.latest_version || "-"}</StudioTd>
            <StudioTd>{item?.retracted ? "retracted" : item?.visibility}</StudioTd>
            <StudioTd>{fmtTs(item?.updated_at)}</StudioTd>
            <StudioTd>
              {item?.retracted ? null : (
                <Button type="button" variant="ghost" size="sm" onClick={() => retractAsset(item)}>
                  Retract
                </Button>
              )}
            </StudioTd>
          </tr>
        ))}
        {!myPacks.length ? (
          <tr><StudioTd colSpan={7}>You have not published any packages yet.</StudioTd></tr>
        ) : null}
      </tbody>
    </StudioTable>
  );
}
