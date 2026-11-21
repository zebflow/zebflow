// What a zebflow_control fence opens, in the words its form shows
// (docs/contracts/app-control.md).

export const TOOL_GROUPS = [
  {
    title: "Files in the folder",
    tools: [
      { id: "file_list", label: "List files" },
      { id: "file_read", label: "Read files" },
      { id: "file_write", label: "Write new files and replace files" },
      { id: "file_edit", label: "Edit part of a file" },
      { id: "file_delete", label: "Delete files" },
      { id: "file_stat", label: "See a file's version (to write only if unchanged)" },
      { id: "template_check", label: "Check a page compiles, without saving" },
    ],
  },
  {
    title: "Pipelines in the folder",
    tools: [
      { id: "pipeline_get", label: "Read pipelines" },
      { id: "pipeline_register", label: "Make and change pipelines" },
      { id: "pipeline_activate", label: "Switch pipelines on" },
      { id: "pipeline_deactivate", label: "Switch pipelines off" },
      { id: "pipeline_delete", label: "Delete pipelines" },
    ],
  },
  {
    title: "Credentials",
    tools: [
      { id: "credential_list", label: "See the credentials it may use (never their passwords)" },
      { id: "credential_create", label: "Create its own credentials" },
      { id: "credential_update", label: "Change the credentials it created" },
      { id: "credential_delete", label: "Delete the credentials it created" },
    ],
  },
];

export const DEFAULT_DENIED = ["sekejap.*", "sqlite.*", "kv.*", "fs.*", "table.*", "function.*"];

/** A list field as an array, whether it was saved as one or as text. */
export function asList(value: any): string[] {
  if (Array.isArray(value)) return value.map((item) => String(item ?? "").trim()).filter(Boolean);
  return String(value || "").split(/[\n,]+/).map((item) => item.trim()).filter(Boolean);
}

/** `list` with `item` added when `on`, removed when not. */
export function toggled(list: string[], item: string, on: boolean): string[] {
  const rest = list.filter((entry) => entry !== item);
  return on ? [...rest, item] : rest;
}
