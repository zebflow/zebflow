const DEFAULT_STYLESHEET = "/assets/libraries/zeb/devicons/0.1/runtime/devicons.css";

export function ensureDevicons(options = {}) {
  if (typeof document === "undefined") {
    return false;
  }
  const href = String(options.href || DEFAULT_STYLESHEET);
  const marker = `link[data-zeb-devicons='${href}']`;
  if (document.head.querySelector(marker)) {
    return true;
  }
  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = href;
  link.setAttribute("data-zeb-devicons", href);
  document.head.appendChild(link);
  return true;
}

export function dbKindIconClass(kind) {
  const value = String(kind || "").trim().toLowerCase();
  switch (value) {
    case "postgresql":
    case "postgres":
    case "pg":
      return "devicon-postgresql-plain colored";
    case "mysql":
      return "devicon-mysql-plain colored";
    case "sqlite":
      return "devicon-sqlite-plain colored";
    case "redis":
      return "devicon-redis-plain colored";
    case "mongodb":
      return "devicon-mongodb-plain colored";
    case "qdrant":
      return "devicon-vectorlogozone-plain";
    case "sjtable":
    case "sekejap":
      return "zf-icon-sjtable";
    default:
      return "zf-icon-default-db";
  }
}

export function dbObjectIconClass(kind) {
  const value = String(kind || "").trim().toLowerCase();
  switch (value) {
    case "schema":
      return "zf-icon-schema";
    case "table":
      return "zf-icon-table";
    case "function":
      return "zf-icon-function";
    case "file":
      return "zf-icon-file";
    case "folder":
      return "zf-icon-folder";
    case "node":
      return "zf-icon-node";
    default:
      return "zf-icon-default-db";
  }
}
