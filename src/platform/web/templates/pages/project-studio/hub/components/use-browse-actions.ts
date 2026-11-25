import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import { needsDestination, verbOf } from "@/components/hub/hub-kinds";

/**
 * The name the lock knows a library by.
 *
 * The hub calls it `zebflow.use`; the lock and every template import call it
 * `zeb/use`. Removal is addressed by the second, so a package id has to be
 * translated back before it is sent.
 */
export function libraryNameOf(item) {
  if (item?.name?.startsWith("zeb/")) return item.name;
  const id = String(item?.package_id ?? "");
  return id.startsWith("zebflow.") ? `zeb/${id.slice("zebflow.".length)}` : id;
}

/** Adding, removing and repairing packages from the Hub browser. */
export function useBrowseActions({ api, refresh, showStatus }) {
  // Where an added package lands. Only add-kinds ask for it; install-kinds go to
  // `data/` and have nowhere to choose.
  const [hubReviewTargetFolder, setHubReviewTargetFolder] = useState("");

  // What an add would overwrite, held while the reader decides. Null means
  // nothing is being asked.
  const [overwritePrompt, setOverwritePrompt] = useState(null);
  const [browseBusy, setBrowseBusy] = useState(false);

  /**
   * Acting on the selected package.
   *
   * Reviews before it writes. The review already knows which files an add would
   * replace, and replacing a file someone wrote is the one part of this that
   * cannot be undone — so it is the one part worth stopping for. Everything
   * else proceeds without a question.
   */
  async function onBrowseAct(item) {
    if (item?.repair_all) {
      await repairDependencies();
      return;
    }
    if (item?.intent === "remove") {
      await removeLibrary(item);
      return;
    }
    if (item?.intent === "restore-built-in") {
      await restoreBuiltIn(item);
      return;
    }
    const packageId = item?.package_id;
    const version = item?.latest_version;
    if (!packageId || !version) {
      showStatus("That package has no installable version.");
      return;
    }

    // Only add-kinds land in the repository, so only they have somewhere to
    // choose. Install-kinds go to `data/` and the folder is meaningless.
    const folder = needsDestination(item?.asset_kind) ? hubReviewTargetFolder : "";

    setBrowseBusy(true);
    try {
      const review = await requestJson(reviewUrl(item), {
        method: "POST",
        body: JSON.stringify({ target_folder: folder }),
      });
      const overwritten = review?.review?.files_overwritten ?? [];
      if (overwritten.length) {
        setOverwritePrompt({ item, folder, files: overwritten });
        return;
      }
      await commitAdd(item, folder);
    } catch (err) {
      showStatus(err?.message || err);
    } finally {
      setBrowseBusy(false);
    }
  }

  function reviewUrl(item) {
    const packageId = encodeURIComponent(item?.package_id);
    const version = encodeURIComponent(item?.latest_version);
    return item?.source === "remote"
      ? `${api.repositories}/${encodeURIComponent(item.repository_id)}/packs/${packageId}/${version}/review`
      : `${api.assets}/${packageId}/${version}/review`;
  }

  /** The write itself, after the reader has seen what it costs. */
  async function commitAdd(item, folder) {
    const packageId = item?.package_id;
    const version = item?.latest_version;
    const verb = verbOf(item?.asset_kind) === "add" ? "Adding" : "Installing";
    showStatus(`${verb} ${packageId}@${version}...`);
    const url = item?.source === "remote"
      ? `${api.repositories}/${encodeURIComponent(item.repository_id)}/packs/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/add`
      : `${api.assets}/${encodeURIComponent(packageId)}/${encodeURIComponent(version)}/add`;
    const payload = await requestJson(url, {
      method: "POST",
      body: JSON.stringify({ target_folder: folder }),
    });
    const result = payload?.result || {};
    // The row must now say "in project", which it only learns by re-reading.
    await refresh();
    showStatus(
      `${packageId}@${version} — ${result.files_written || 0} file(s) into ${result.install_root || "the project"}`,
    );
  }

  // What a removal would delete, held while the reader decides.
  const [removePrompt, setRemovePrompt] = useState(null);

  async function removeLibrary(item) {
    setRemovePrompt(item);
  }

  async function commitRemove(item) {
    const name = libraryNameOf(item);
    showStatus(`Removing ${name}...`);
    try {
      await requestJson(`${api.libraries_remove}?name=${encodeURIComponent(name)}`, {
        method: "DELETE",
      });
      await refresh();
      showStatus(`Removed ${name} — its lock entry and its files are gone.`);
    } catch (err) {
      showStatus(err?.message || err);
    }
  }

  /** Stops an uploaded library replacing this built-in one: pages load it again. */
  async function restoreBuiltIn(item) {
    const name = libraryNameOf(item);
    setBrowseBusy(true);
    try {
      await requestJson(`${api.libraries_override}?library=${encodeURIComponent(name)}`, { method: "DELETE" });
      await refresh();
      showStatus(`Pages load the built-in ${name} again.`);
    } catch (err) {
      showStatus(err?.message || err);
    } finally {
      setBrowseBusy(false);
    }
  }

  async function repairDependencies() {
    setBrowseBusy(true);
    showStatus("Repairing packages that no longer match zeb.lock...");
    try {
      await requestJson(api.dependencies, { method: "POST", body: "{}" });
      await refresh();
      showStatus("Repair finished.");
    } catch (err) {
      showStatus(err?.message || err);
    } finally {
      setBrowseBusy(false);
    }
  }

  async function confirmRemove() {
    const pending = removePrompt;
    setRemovePrompt(null);
    if (!pending) return;
    setBrowseBusy(true);
    try {
      await commitRemove(pending);
    } finally {
      setBrowseBusy(false);
    }
  }

  async function confirmOverwrite() {
    const pending = overwritePrompt;
    setOverwritePrompt(null);
    if (!pending) return;
    setBrowseBusy(true);
    try {
      await commitAdd(pending.item, pending.folder);
    } catch (err) {
      showStatus(err?.message || err);
    } finally {
      setBrowseBusy(false);
    }
  }

  return {
    busy: browseBusy,
    destination: hubReviewTargetFolder,
    setDestination: setHubReviewTargetFolder,
    onAct: onBrowseAct,
    removePrompt,
    closeRemove: () => setRemovePrompt(null),
    confirmRemove,
    overwritePrompt,
    closeOverwrite: () => setOverwritePrompt(null),
    confirmOverwrite,
  };
}
