/**
 * The Hub's slug rule: every run of non-alphanumerics becomes one dash, capped
 * at 80 characters. Not the same rule as other pages' slug helpers.
 */
export function slugify(input) {
  return String(input || "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80);
}
