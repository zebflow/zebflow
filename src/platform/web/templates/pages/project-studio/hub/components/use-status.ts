import { useEffect, useState } from "zeb/react";

export function describeStatus(value) {
  if (value == null) return "Unknown status";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value.map((item) => describeStatus(item)).filter(Boolean).join(", ");
  }
  if (typeof value === "object") {
    const direct = value.message || value.error || value.detail || value.reason;
    if (direct && direct !== value) return describeStatus(direct);
    try {
      return JSON.stringify(value, null, 2);
    } catch (_) {
      return String(value);
    }
  }
  return String(value);
}

/** The one status line at the top of the Hub page. It clears itself after 5 s. */
export function useStatus() {
  const [status, setStatus] = useState("");

  function showStatus(value) {
    setStatus(describeStatus(value));
  }

  useEffect(() => {
    if (!status) return;
    const timer = setTimeout(() => setStatus(""), 5000);
    return () => clearTimeout(timer);
  }, [status]);

  return { status, showStatus };
}
