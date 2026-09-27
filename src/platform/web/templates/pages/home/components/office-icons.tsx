/**
 * Small line icons for the office monitor. They stand in for the words
 * "address", "version" and the rest, so an office reads in two short lines.
 */

function Icon({ children, title }) {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-label={title}
      role="img"
      className="shrink-0"
    >
      <title>{title}</title>
      {children}
    </svg>
  );
}

export function AddressIcon() {
  return (
    <Icon title="Address">
      <circle cx="8" cy="8" r="6" />
      <path d="M2 8h12M8 2c1.8 2 1.8 10 0 12M8 2c-1.8 2-1.8 10 0 12" />
    </Icon>
  );
}

export function VersionIcon() {
  return (
    <Icon title="Version">
      <path d="M8.5 2H13a1 1 0 0 1 1 1v4.5L7.5 14 2 8.5z" />
      <circle cx="11" cy="5" r="0.8" />
    </Icon>
  );
}

export function SeenIcon() {
  return (
    <Icon title="Last seen">
      <circle cx="8" cy="8" r="6" />
      <path d="M8 4.5V8l2.5 1.5" />
    </Icon>
  );
}

export function CapabilityIcon() {
  return (
    <Icon title="Capabilities">
      <rect x="4" y="4" width="8" height="8" rx="1" />
      <path d="M6 1.5v2.5M10 1.5v2.5M6 12v2.5M10 12v2.5M1.5 6h2.5M1.5 10h2.5M12 6h2.5M12 10h2.5" />
    </Icon>
  );
}

export function ProjectsIcon() {
  return (
    <Icon title="Projects">
      <path d="M2 4.5A1.5 1.5 0 0 1 3.5 3H6l1.5 1.5h5A1.5 1.5 0 0 1 14 6v5.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5z" />
    </Icon>
  );
}

/** Green when the office answered its last heartbeat in time, red when not. */
export function StatusDot({ online }) {
  return (
    <span
      title={online ? "Online" : "Offline"}
      className="inline-block h-2 w-2 shrink-0 rounded-full"
      style={{ backgroundColor: online ? "var(--success)" : "var(--destructive)" }}
    />
  );
}
