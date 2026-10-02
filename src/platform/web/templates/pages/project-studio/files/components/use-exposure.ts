import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";

/** Every exposed rule and what it exposes, re-read whenever `reload` is called. */
export function useExposure(api) {
  const [rules, setRules] = useState([]);
  const [fileHost, setFileHost] = useState("");
  const [version, setVersion] = useState(0);

  useEffect(() => {
    if (!api?.exposure) return;
    requestJson(api.exposure)
      .then((payload) => {
        setRules(Array.isArray(payload?.rules) ? payload.rules : []);
        setFileHost(payload?.file_host ?? "");
      })
      .catch(() => setRules([]));
  }, [api?.exposure, version]);

  return { rules, fileHost, reload: () => setVersion((v) => v + 1) };
}
