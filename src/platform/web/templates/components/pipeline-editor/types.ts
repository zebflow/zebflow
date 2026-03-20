// Pipeline editor type definitions.
// Used across all sub-components.

export interface SelectOptionDef {
  value: string;
  label: string;
}

export type NodeFieldType = "text" | "textarea" | "code_editor" | "select" | "datalist" | "method_buttons" | "copy_url" | "checkbox" | "section";

export type NodeFieldDataSource = "credentials_postgres" | "credentials_jwt" | "templates_pages";

export interface SidebarItem {
  label: string;
  type_hint?: string;
  description?: string;
}

export interface SidebarSection {
  title: string;
  items: SidebarItem[];
}

export interface NodeFieldDef {
  name: string;
  label: string;
  type: NodeFieldType;
  /** Live value — not part of the schema, enriched at render time */
  value?: unknown;
  help?: string;
  placeholder?: string;
  readonly?: boolean;
  rows?: number;
  language?: string;
  options?: (SelectOptionDef | string)[];
  data_source?: NodeFieldDataSource;
  default_value?: unknown;
  sidebar?: SidebarSection[];
  /** "full" | "half" — overrides default grid span */
  span?: string;
}

export type LayoutItem =
  | string                          // Field reference by name
  | { row: LayoutItem[] }           // Horizontal group
  | { col: LayoutItem[] };          // Vertical group (inside a row)

export interface NodeCatalogEntry {
  kind: string;
  title: string;
  description?: string;
  input_pins?: string[];
  output_pins?: string[];
  color?: string;
  fields?: NodeFieldDef[];
  layout?: LayoutItem[];
}

export interface EditorDataState {
  pgCredentials: any[];
  jwtCredentials: any[];
  pageTemplates: any[];
  owner: string;
  project: string;
}

export interface EditorApi {
  byId: string;
  definition: string;
  activate: string;
  deactivate: string;
  hits: string;
  invocations: string;
  nodes: string;
  credentials: string;
  templatesWorkspace: string;
  templateFile: string;
  templateSave: string;
}

/** Callback shape received by PipelineEditor when user clicks "E" on a node */
export interface PipelineNodeData {
  graphNodeId: number;
  zfKind: string;
  zfPipelineNodeId: string;
  zfConfig: Record<string, unknown>;
  title?: string;
  x: number;
  y: number;
  inputs: { name: string }[];
  outputs: { name: string }[];
  /** Live graph node — modify zfConfig/zfPipelineNodeId/title directly */
  _raw: any;
}

export interface GitFile {
  code: string;
  rel_path: string;
}

export interface PipelineMeta {
  name: string;
  title?: string;
  virtual_path: string;
  trigger_kind: string;
  file_rel_path: string;
  active_hash?: string;
  hash?: string;
}
