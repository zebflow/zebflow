import { cx } from "zeb/react";
import Badge from "@/components/ui/badge";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";

/**
 * Where this project keeps its files, and where it could move.
 *
 * Lives with the Files page rather than in Settings: it describes the store you
 * are looking at, and showing it somewhere you cannot act on it was the reason
 * it read as decoration. The two icons are used nowhere else, so they stay here.
 */
export function DiskIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <ellipse cx="12" cy="6" rx="8" ry="3" />
      <path d="M4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6" />
      <path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" />
    </svg>
  );
}

export function BucketIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 8V16a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8"/>
      <path d="M3 8l9-5 9 5"/>
      <path d="M12 3v18"/>
    </svg>
  );
}

export default function StorageBackendPanel({ backend, backendLabel, declared, field }) {
  const active = String(backend ?? "zebfs");
  const activeLabel = String(backendLabel ?? "Local disk");
  return (
    <div className="project-settings-panel">
      <div className="project-settings-panel-head">
        <p className="project-card-label">File Storage Backend</p>
        <Badge variant="outline">{active}</Badge>
      </div>
      <div className="project-settings-panel-body flex flex-col gap-6 pt-2">

        <p className="text-[0.78rem] text-body-soft">
          Where this project keeps its own files. Every object under{" "}
          <code className="font-mono text-[0.75rem]">files/</code> lives in this store, and every FileRef a
          pipeline passes around carries its name in <code className="font-mono text-[0.75rem]">backend</code>.
          A bucket a pipeline reads from is a different thing — that is a connection with a credential,
          chosen per pipeline, and the bytes it fetches still land here.
        </p>

        {/* Current selection */}
        <Card>
          <CardContent className="flex items-start gap-4 pt-5">
            <div className="mt-0.5 rounded bg-accent/10 p-2 text-accent">
              <DiskIcon />
            </div>
            <div className="flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <p className="text-[0.88rem] font-semibold text-body">{activeLabel}</p>
                <Badge variant="secondary" className="text-[0.72rem]">Selected</Badge>
              </div>
              <p className="mt-0.5 text-[0.78rem] text-body-soft">
                Objects are stored on this machine&apos;s disk and served through{" "}
                <code className="font-mono text-[0.75rem]">/fs/...</code> under this project&apos;s access rules.
              </p>
              <p className="mt-2 text-[0.75rem] text-body-soft">
                <code className="font-mono text-[0.75rem]">{field ?? "spec.files.backend"}: {active}</code>{" "}
                {declared ? "declared in" : "— the default; not declared in"}{" "}
                <code className="font-mono text-[0.75rem]">repo/zebflow.yaml</code>
              </p>
            </div>
          </CardContent>
        </Card>

        {/* Not yet available */}
        <Card className="opacity-60">
          <CardContent className="flex items-start gap-4 pt-5">
            <div className="mt-0.5 rounded bg-accent/10 p-2 text-accent">
              <BucketIcon />
            </div>
            <div className="flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <p className="text-[0.88rem] font-semibold text-body">Object store</p>
                <Badge variant="outline" className="text-[0.72rem]">Coming soon</Badge>
              </div>
              <p className="mt-0.5 text-[0.78rem] text-body-soft">
                Replace disk as this project&apos;s store, so the same objects live in a bucket instead.
                Selecting one will be a change of this same setting plus the connection that holds the
                endpoint and credential — stored FileRefs keep their shape.
              </p>
              <div className="mt-3 flex flex-wrap gap-2">
                {["AWS S3", "Cloudflare R2", "MinIO", "Backblaze B2", "Tigris"].map((label) => (
                  <Badge key={label} variant="secondary" className="text-[0.72rem]">{label}</Badge>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
