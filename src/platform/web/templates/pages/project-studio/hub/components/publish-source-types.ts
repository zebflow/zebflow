export const SOURCE_TYPES = [
  { value: "pipeline_with_dependencies", label: "Pipeline with dependencies", note: "Exports the pipeline plus referenced templates and local imports." },
  { value: "template_with_dependencies", label: "Template with dependencies", note: "Exports the selected TSX/TS/CSS file plus local imports." },
  { value: "folder_files", label: "Folder files", note: "Exports everything recursively under one folder." },
  { value: "project_files", label: "Project files", note: "Exports the full repo workspace." },
];

export function sourceLabel(value) {
  return SOURCE_TYPES.find((item) => item.value === value)?.label || value;
}

/** What the server says this project could put into a project bundle. */
export function publishOptionFacts(publishOptions) {
  const availableLibraries = Array.isArray(publishOptions?.libraries) ? publishOptions.libraries : [];
  const sekejapSchemaAvailable = !!publishOptions?.sekejap_schema?.available;
  const sqliteSchemaAvailable = !!publishOptions?.sqlite_schema?.available;
  const initialDataItems = Array.isArray(publishOptions?.initial_data?.items) ? publishOptions.initial_data.items : [];
  const initialDataAvailable = !!publishOptions?.initial_data?.available && initialDataItems.length > 0;
  return { availableLibraries, sekejapSchemaAvailable, sqliteSchemaAvailable, initialDataItems, initialDataAvailable };
}
