/** Filename rules for staged uploads: pure, so the queue and the dialog agree. */

export function extensionForMime(type: string): string {
  if (type === "image/jpeg") return "jpg";
  if (type === "image/webp") return "webp";
  if (type === "image/gif") return "gif";
  return "png";
}

export function extensionForUploadFile(file: any): string {
  const name = String(file?.name || "");
  const match = name.match(/\.([A-Za-z0-9]{1,12})$/);
  if (match) return match[1].toLowerCase();
  return extensionForMime(String(file?.type || ""));
}

export function sanitizeUploadName(name: string, fallback: string): string {
  const cleaned = String(name || "")
    .replace(/[\\/:*?"<>|]+/g, "-")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/^\.+/, "");
  return cleaned || fallback;
}

function splitFileName(name: string): { stem: string; ext: string } {
  const match = String(name).match(/^(.*?)(\.[^.]*)?$/);
  return {
    stem: match?.[1] || "file",
    ext: match?.[2] || "",
  };
}

export function uniqueUploadName(name: string, used: Set<string>): string {
  const clean = sanitizeUploadName(name, "upload.bin");
  if (!used.has(clean.toLowerCase())) return clean;
  const { stem, ext } = splitFileName(clean);
  let index = 1;
  while (true) {
    const next = `${stem}-${index}${ext}`;
    if (!used.has(next.toLowerCase())) return next;
    index += 1;
  }
}

export function buildCrumbs(currentPath: string): Array<{ label: string; path: string }> {
  if (!currentPath) return [];
  const out: Array<{ label: string; path: string }> = [];
  let acc = "";
  for (const part of currentPath.split("/")) {
    acc = acc ? `${acc}/${part}` : part;
    out.push({ label: part, path: acc });
  }
  return out;
}
