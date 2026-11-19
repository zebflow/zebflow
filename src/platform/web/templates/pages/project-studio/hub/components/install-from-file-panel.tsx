import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import SqlReport from "@/components/package-review/sql-report";
import { ViolationNotice, WarningNotice } from "@/components/package-review/findings";
import InstallFromFileDialog from "@/pages/project-studio/hub/components/install-from-file-dialog";

function BundleReviewList({ title, items }) {
  const values = Array.isArray(items) ? items : [];
  return (
    <div className="rounded-lg border border-border bg-accent/20 px-3 py-2">
      <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{title}</p>
      {values.length ? (
        <ul className="mt-1 list-disc pl-5 text-sm text-foreground">
          {values.map((item, index) => (
            <li key={`${title}-${index}`}>{item}</li>
          ))}
        </ul>
      ) : (
        <p className="mt-1 text-sm text-muted-foreground">None</p>
      )}
    </div>
  );
}

/** Picking a node bundle file, reviewing it, and installing it. */
export default function InstallFromFilePanel({ open, onClose, api, showStatus }) {
  const [bundleFileName, setBundleFileName] = useState("");
  const [bundleArtifact, setBundleArtifact] = useState(null);
  const [bundleReview, setBundleReview] = useState(null);
  const [bundleBusy, setBundleBusy] = useState(false);

  const bundleIdentity = () => {
    const metadata = bundleArtifact?.metadata ?? {};
    return {
      package_id: String(metadata.name ?? ""),
      version: String(metadata.version ?? ""),
    };
  };

  const onPickBundleFile = (event) => {
    const file = event?.target?.files?.[0];
    if (!file) return;
    setBundleReview(null);
    setBundleArtifact(null);
    setBundleFileName(file.name);
    file
      .text()
      .then((text) => {
        const parsed = JSON.parse(text);
        setBundleArtifact(parsed);
        showStatus(`Loaded ${file.name}. Review it before installing.`);
      })
      .catch(() => {
        setBundleFileName("");
        showStatus("That file is not a readable Zebflow package document.");
      });
  };

  const reviewBundle = () => {
    const identity = bundleIdentity();
    if (!bundleArtifact || !identity.package_id || !identity.version) {
      showStatus("Choose a package file that declares metadata.name and metadata.version.");
      return;
    }
    setBundleBusy(true);
    requestJson(api.node_bundle_review, {
      method: "POST",
      body: JSON.stringify({ ...identity, target_folder: "", artifact: bundleArtifact }),
    })
      .then((payload) => {
        setBundleReview(payload?.review ?? null);
        showStatus("Reviewed. Check what this package does before installing.");
      })
      .catch((err) => showStatus(err?.message || err))
      .then(() => setBundleBusy(false));
  };

  const installBundle = () => {
    const identity = bundleIdentity();
    setBundleBusy(true);
    requestJson(api.node_bundle_install, {
      method: "POST",
      body: JSON.stringify({ ...identity, target_folder: "", artifact: bundleArtifact }),
    })
      .then(() => {
        setBundleReview(null);
        setBundleArtifact(null);
        setBundleFileName("");
        showStatus(`Installed ${identity.package_id} ${identity.version}.`);
      })
      .catch((err) => showStatus(err?.message || err))
      .then(() => setBundleBusy(false));
  };

  return (
    <InstallFromFileDialog open={open} onClose={onClose}>
      <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="project-content-subtitle">Node Bundle From File</p>
          <p className="text-sm text-muted-foreground">
            Install a node bundle you authored locally or received as a file. It runs the same
            review a published package runs: a Hub package is not safer, only published.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button as="label" type="button" variant="outline" size="sm">
            Choose file
            <input
              type="file"
              accept="application/json,.json"
              className="sr-only"
              onChange={onPickBundleFile}
            />
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={!bundleArtifact || bundleBusy}
            onClick={reviewBundle}
          >
            Review
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={!bundleReview?.installable || bundleBusy}
            onClick={installBundle}
          >
            Install
          </Button>
        </div>
      </div>

      {bundleFileName ? (
        <p className="mt-3 text-sm text-muted-foreground">
          Selected: <span className="font-medium text-foreground">{bundleFileName}</span>
        </p>
      ) : null}

      {bundleReview ? (
        <div className="mt-4 space-y-3">
          <ViolationNotice items={bundleReview.violations} subject="This bundle" />
          <WarningNotice items={bundleReview.warnings} />

          <SqlReport
            reports={bundleReview.database_initialization}
            emptyNote="This bundle carries no install-time SQL. Nothing is replayed into any store."
          />

          <div className="grid gap-3 md:grid-cols-2">
            <BundleReviewList title="Nodes provided" items={bundleReview.nodes_used} />
            <BundleReviewList title="Credentials requested" items={bundleReview.credentials_required} />
            <BundleReviewList title="External URLs contacted" items={bundleReview.external_urls} />
            <BundleReviewList title="Public endpoints created" items={bundleReview.public_endpoints} />
            <BundleReviewList title="Database effects" items={bundleReview.database_effects} />
            <BundleReviewList title="File effects" items={bundleReview.filesystem_effects} />
            <BundleReviewList title="Outbound connections" items={bundleReview.network_effects} />
            <BundleReviewList title="Runs supplied code" items={bundleReview.code_execution} />
            <BundleReviewList title="Schedules" items={bundleReview.schedules} />
            <BundleReviewList title="Files written" items={bundleReview.files_added} />
            <BundleReviewList title="Files overwritten" items={bundleReview.files_overwritten} />
          </div>

          <p className="text-sm text-muted-foreground">
            Risk level: <span className="font-medium text-foreground">{bundleReview.risk_level}</span>
          </p>
        </div>
      ) : null}
    </InstallFromFileDialog>
  );
}
