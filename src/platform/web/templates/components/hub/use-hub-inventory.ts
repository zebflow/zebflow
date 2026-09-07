import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import { buildHubItems } from "@/components/hub/hub-items";

/**
 * Keep installed libraries and lock health in one snapshot. Refreshing only
 * the library list lets an old lock entry mark a removed package installed
 * again; the catalogue cannot answer whether this project still owns it.
 */
export function useHubInventory(assets, initialInstalled, api) {
  const [inventory, setInventory] = useState({
    libraries: Array.isArray(initialInstalled?.libraries_available) ? initialInstalled.libraries_available : [],
    lock: initialInstalled?.dependencies?.status ?? {},
  });

  async function refreshInventory() {
    const [libraries, dependencies] = await Promise.all([
      api.libraries ? requestJson(api.libraries) : Promise.resolve(null),
      api.dependencies ? requestJson(api.dependencies) : Promise.resolve(null),
    ]);
    setInventory((previous) => ({
      libraries: Array.isArray(libraries) ? libraries : previous.libraries,
      lock: dependencies?.report ?? previous.lock,
    }));
  }

  return {
    browseItems: buildHubItems({ assets, libraries: inventory.libraries, lock: inventory.lock }),
    refreshInventory,
  };
}
