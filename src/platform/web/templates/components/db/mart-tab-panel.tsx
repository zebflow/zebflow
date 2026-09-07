import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";

/** Data marts, listed as drafts. Nothing here is wired to an engine yet. */
const DRAFTS = [
  { name: "mart_sales_daily", description: "Daily aggregated sales mart" },
  { name: "mart_retention_cohort", description: "User retention cohort mart" },
];

export default function MartTabPanel() {
  return (
    <section className="db-suite-panel db-suite-panel-fill">
      <div className="db-suite-mart-full">
        <StudioTable>
          <StudioThead>
            <tr>
              <StudioTh>Name</StudioTh>
              <StudioTh>Description</StudioTh>
              <StudioTh>Status</StudioTh>
            </tr>
          </StudioThead>
          <tbody>
            {DRAFTS.map((mart) => (
              <tr key={mart.name}>
                <StudioTd>{mart.name}</StudioTd>
                <StudioTd>{mart.description}</StudioTd>
                <StudioTd>draft</StudioTd>
              </tr>
            ))}
          </tbody>
        </StudioTable>
      </div>
    </section>
  );
}
