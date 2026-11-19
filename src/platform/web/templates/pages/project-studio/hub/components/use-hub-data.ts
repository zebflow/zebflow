import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import { useHubInventory } from "@/components/hub/use-hub-inventory";

/**
 * What the Hub page reads from the server: the catalogue, this project's own
 * published packages, the hub sources, and the installed inventory. `refresh`
 * re-reads all of them at once.
 */
export function useHubData(input, api) {
  const [packs, setPacks] = useState(Array.isArray(input?.assets) ? input.assets : []);
  const { browseItems, refreshInventory } = useHubInventory(packs, input?.installed, api);
  const [myPacks, setMyPacks] = useState(Array.isArray(input?.my_assets) ? input.my_assets : []);
  const [hubSources, setHubSources] = useState([]);

  async function refresh() {
    const tasks = [
      requestJson(api.assets),
      requestJson(api.my_assets),
      api.access ? requestJson(api.access) : Promise.resolve(null),
      refreshInventory(),
    ];
    const [assetsRes, myRes, sourceRes] = await Promise.all(tasks);
    setPacks(Array.isArray(assetsRes?.items) ? assetsRes.items : []);
    setMyPacks(Array.isArray(myRes?.items) ? myRes.items : []);
    setHubSources(Array.isArray(sourceRes?.repositories) ? sourceRes.repositories : []);
  }

  useEffect(() => {
    refresh().catch(() => {});
  }, []);

  return { browseItems, myPacks, hubSources, refresh };
}
