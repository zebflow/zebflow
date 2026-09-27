import {
  AddressIcon,
  CapabilityIcon,
  ProjectsIcon,
  SeenIcon,
  StatusDot,
  VersionIcon,
} from "@/pages/home/components/office-icons";

/** `0.9.1.202609232321` → `v0.9`: the monitor needs the line, not the build. */
function shortVersion(value) {
  const parts = String(value || "").split(".");
  return parts.length >= 2 && parts[0] !== "" ? `v${parts[0]}.${parts[1]}` : "";
}

function Fact({ icon, children, mono = false }) {
  return (
    <span className={`inline-flex min-w-0 items-center gap-1 ${mono ? "font-mono text-[11px]" : ""}`}>
      {icon}
      <span className="truncate">{children}</span>
    </span>
  );
}

/**
 * The left column of one office: a monitor, never a door (`offices.md` §3a).
 * Two lines — who and whether it answers, then where, which version, when
 * last heard, what it can run and how many projects it holds.
 */
export default function OfficeMeta({ office }) {
  const capabilities = Array.isArray(office?.capabilities) ? office.capabilities : [];
  const version = shortVersion(office?.version);

  return (
    <div className="min-w-0">
      <div className="flex items-center gap-2">
        <StatusDot online={Boolean(office?.online)} />
        <span className="truncate text-[15px] font-semibold text-foreground">{office?.label || office?.id}</span>
        <span className="shrink-0 text-[12px] text-muted-foreground">{office?.role}</span>
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[12px] text-muted-foreground">
        {office?.address ? <Fact icon={<AddressIcon />} mono>{office.address}</Fact> : null}
        {version ? <Fact icon={<VersionIcon />}>{version}</Fact> : null}
        <Fact icon={<SeenIcon />}>{office?.last_seen || "never"}</Fact>
        {capabilities.length > 0 ? <Fact icon={<CapabilityIcon />}>{capabilities.join(", ")}</Fact> : null}
        <Fact icon={<ProjectsIcon />}>{office?.project_count ?? 0}</Fact>
      </div>
    </div>
  );
}
