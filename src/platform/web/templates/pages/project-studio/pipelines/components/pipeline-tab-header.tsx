/** A pipelines tab's title strip: what the tab lists, and a count or action on the right. */
export default function PipelineTabHeader({ title, subtitle, trailing }) {
  return (
    <div className="shrink-0 flex items-start justify-between gap-3 px-[0.875rem] py-[0.625rem] border-b border-border bg-card">
      <div>
        <p className="text-[0.68rem] uppercase tracking-[0.08em] text-muted-foreground">{title}</p>
        {subtitle ? (
          <p className="text-[0.72rem] text-muted-foreground mt-[0.2rem] leading-snug">{subtitle}</p>
        ) : null}
      </div>
      {trailing ? <div className="shrink-0 flex items-center gap-2">{trailing}</div> : null}
    </div>
  );
}
