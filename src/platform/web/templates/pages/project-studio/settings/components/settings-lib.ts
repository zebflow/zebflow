/**
 * Wording and tone shared by the settings panels.
 *
 * `formatOperationTimestamp` is deliberately not `formatTs` from
 * `@/components/lib/format`: settings shows an operator their own locale's
 * time, the platform-wide helper shows a fixed ISO slice. Same shape, two
 * different promises to the reader.
 *
 * Wart worth naming: `AssistantPanel` and `GitBranchPanel` still inline their
 * own tone ternaries instead of calling `settingsStatusToneClass`. Left alone
 * here because fixing it changes what those two panels render.
 */

export function formatOperationTimestamp(ts) {
  if (!ts) return "unknown";
  try {
    return new Date(Number(ts) * 1000).toLocaleString();
  } catch (_err) {
    return String(ts);
  }
}

export function settingsStatusToneClass(tone) {
  if (tone === "ok") {
    return "text-info";
  }
  if (tone === "error") {
    return "text-warning";
  }
  return "text-muted-foreground";
}
