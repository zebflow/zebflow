//! Unified operation specifications for REST API, MCP tools, and assistant interfaces.
//!
//! Single source of truth for all project management operations.

use crate::platform::model::ProjectCapability;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Complete specification for one project management operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSpec {
    /// Stable operation ID (used in MCP tool names, logs, assistant prompts, and `/docs/operation` extraction).
    pub id: &'static str,
    /// Category grouping (pipelines, templates, credentials, tables, files, settings).
    pub category: &'static str,
    /// Human-readable description (used in MCP, API docs, assistant context).
    pub description: &'static str,
    /// Required project capability.
    pub capability: ProjectCapability,
    /// HTTP method for REST API (GET, POST, PUT, DELETE).
    pub method: &'static str,
    /// REST API path pattern.
    pub path: &'static str,
    /// Input parameters schema (JSON schema string or empty).
    pub params_schema: &'static str,
}

/// All project management operations.
pub const OPERATIONS: &[OperationSpec] = &[
    // Pipelines
    OperationSpec {
        id: "list_pipelines",
        category: "pipelines",
        description: "List all pipelines in the project with metadata (name, title, trigger_kind, active status)",
        capability: ProjectCapability::PipelinesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/pipelines",
        params_schema: "",
    },
    OperationSpec {
        id: "get_pipeline",
        category: "pipelines",
        description: "Get a specific pipeline definition source code by virtual path and name",
        capability: ProjectCapability::PipelinesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/pipelines/{virtual_path}/{name}",
        params_schema: r#"{"virtual_path":"string","name":"string"}"#,
    },
    OperationSpec {
        id: "upsert_pipeline",
        category: "pipelines",
        description: "Create or update a pipeline definition (YAML source code)",
        capability: ProjectCapability::PipelinesWrite,
        method: "PUT",
        path: "/api/projects/{owner}/{project}/pipelines/{virtual_path}/{name}",
        params_schema: r#"{"virtual_path":"string","name":"string","title":"string","description":"string","trigger_kind":"string","source":"string"}"#,
    },
    OperationSpec {
        id: "activate_pipeline",
        category: "pipelines",
        description: "Activate a pipeline (make it available for execution)",
        capability: ProjectCapability::PipelinesWrite,
        method: "POST",
        path: "/api/projects/{owner}/{project}/pipelines/{virtual_path}/{name}/activate",
        params_schema: r#"{"virtual_path":"string","name":"string"}"#,
    },
    OperationSpec {
        id: "deactivate_pipeline",
        category: "pipelines",
        description: "Deactivate a pipeline (remove from active registry)",
        capability: ProjectCapability::PipelinesWrite,
        method: "POST",
        path: "/api/projects/{owner}/{project}/pipelines/{virtual_path}/{name}/deactivate",
        params_schema: r#"{"virtual_path":"string","name":"string"}"#,
    },
    OperationSpec {
        id: "execute_pipeline",
        category: "pipelines",
        description: "Execute a pipeline with explicit trigger payload (webhook, schedule, or manual)",
        capability: ProjectCapability::PipelinesExecute,
        method: "POST",
        path: "/api/projects/{owner}/{project}/pipelines/{virtual_path}/{name}/execute",
        params_schema: r#"{"virtual_path":"string","name":"string","trigger":"webhook|schedule|manual","webhook_path":"string?","webhook_method":"string?","schedule_cron":"string?","input":"object"}"#,
    },
    // Templates
    OperationSpec {
        id: "list_templates",
        category: "templates",
        description: "List all templates in the project workspace (pages, components, scripts, styles)",
        capability: ProjectCapability::TemplatesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/templates",
        params_schema: "",
    },
    OperationSpec {
        id: "get_template",
        category: "templates",
        description: "Get a specific template file contents by relative path",
        capability: ProjectCapability::TemplatesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/templates/{path}",
        params_schema: r#"{"path":"string"}"#,
    },
    OperationSpec {
        id: "save_template",
        category: "templates",
        description: "Save (create or update) a template file with new contents",
        capability: ProjectCapability::TemplatesWrite,
        method: "PUT",
        path: "/api/projects/{owner}/{project}/templates/{path}",
        params_schema: r#"{"path":"string","contents":"string"}"#,
    },
    OperationSpec {
        id: "create_template",
        category: "templates",
        description: "Create a new template file or folder",
        capability: ProjectCapability::TemplatesCreate,
        method: "POST",
        path: "/api/projects/{owner}/{project}/templates",
        params_schema: r#"{"path":"string","kind":"string","contents":"string"}"#,
    },
    OperationSpec {
        id: "delete_template",
        category: "templates",
        description: "Delete a template file or folder",
        capability: ProjectCapability::TemplatesDelete,
        method: "DELETE",
        path: "/api/projects/{owner}/{project}/templates/{path}",
        params_schema: r#"{"path":"string"}"#,
    },
    // Credentials
    OperationSpec {
        id: "list_credentials",
        category: "credentials",
        description: "List all credential keys in the project",
        capability: ProjectCapability::CredentialsRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/credentials",
        params_schema: "",
    },
    OperationSpec {
        id: "get_credential",
        category: "credentials",
        description: "Get a specific credential value by key (masked in logs)",
        capability: ProjectCapability::CredentialsRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/credentials/{key}",
        params_schema: r#"{"key":"string"}"#,
    },
    OperationSpec {
        id: "upsert_credential",
        category: "credentials",
        description: "Create or update a credential (store secrets securely)",
        capability: ProjectCapability::CredentialsWrite,
        method: "PUT",
        path: "/api/projects/{owner}/{project}/credentials/{key}",
        params_schema: r#"{"key":"string","title":"string","kind":"string","secret":"object","notes":"string"}"#,
    },
    // Assistant
    OperationSpec {
        id: "get_project_assistant_config",
        category: "settings",
        description: "Get project assistant runtime configuration (bound LLM credentials and limits)",
        capability: ProjectCapability::SettingsRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/assistant/config",
        params_schema: "",
    },
    OperationSpec {
        id: "upsert_project_assistant_config",
        category: "settings",
        description: "Create or update project assistant runtime configuration",
        capability: ProjectCapability::SettingsWrite,
        method: "PUT",
        path: "/api/projects/{owner}/{project}/assistant/config",
        params_schema: r#"{"llm_high_credential_id":"string?","llm_general_credential_id":"string?","max_steps":"number?","max_replans":"number?","enabled":"bool?"}"#,
    },
    OperationSpec {
        id: "project_assistant_chat",
        category: "assistant",
        description: "Send one project-assistant chat message over SSE stream (internal web assistant path)",
        capability: ProjectCapability::ProjectRead,
        method: "POST",
        path: "/api/projects/{owner}/{project}/assistant/chat",
        params_schema: r#"{"message":"string","history":"[{role,content}]?","use_high_model":"bool?"}"#,
    },
    OperationSpec {
        id: "prepare_project_assets",
        category: "libraries",
        description: "Vendor selected web library assets into project workspace and build project-scoped chunk manifest for runtime serving",
        capability: ProjectCapability::LibrariesInstall,
        method: "POST",
        path: "/api/projects/{owner}/{project}/assets/prepare",
        params_schema: r#"{"library":"string?","version":"string?","entries":"string[]?"}"#,
    },
    // DB connections (sjtable + credential-backed external engines)
    OperationSpec {
        id: "list_db_connections",
        category: "db",
        description: "List all DB connections for the project",
        capability: ProjectCapability::TablesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/db/connections",
        params_schema: "",
    },
    OperationSpec {
        id: "get_db_connection",
        category: "db",
        description: "Get one DB connection by slug",
        capability: ProjectCapability::TablesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_slug}",
        params_schema: r#"{"connection_slug":"string"}"#,
    },
    OperationSpec {
        id: "upsert_db_connection",
        category: "db",
        description: "Create one DB connection",
        capability: ProjectCapability::TablesWrite,
        method: "POST",
        path: "/api/projects/{owner}/{project}/db/connections",
        params_schema: r#"{"connection_slug":"string","connection_label":"string","database_kind":"string","credential_id":"string?","config":"object?"}"#,
    },
    OperationSpec {
        id: "update_db_connection",
        category: "db",
        description: "Update one DB connection by slug",
        capability: ProjectCapability::TablesWrite,
        method: "PUT",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_slug}",
        params_schema: r#"{"connection_slug":"string","connection_label":"string","database_kind":"string","credential_id":"string?","config":"object?"}"#,
    },
    OperationSpec {
        id: "delete_db_connection",
        category: "db",
        description: "Delete one DB connection by slug",
        capability: ProjectCapability::TablesWrite,
        method: "DELETE",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_slug}",
        params_schema: r#"{"connection_slug":"string"}"#,
    },
    OperationSpec {
        id: "test_db_connection",
        category: "db",
        description: "Validate one DB connection (existing by slug or draft payload)",
        capability: ProjectCapability::TablesRead,
        method: "POST",
        path: "/api/projects/{owner}/{project}/db/connections/test",
        params_schema: r#"{"connection_slug":"string?","database_kind":"string?","credential_id":"string?","config":"object?"}"#,
    },
    OperationSpec {
        id: "describe_db_connection",
        category: "db",
        description: "Describe database objects for one DB connection by immutable id (schemas/tables/functions/tree)",
        capability: ProjectCapability::TablesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_id}/describe",
        params_schema: r#"{"connection_id":"string","scope":"tree|schemas|tables|functions?","schema":"string?","include_system":"boolean?"}"#,
    },
    OperationSpec {
        id: "query_db_connection",
        category: "db",
        description: "Execute one query against DB connection by immutable id (read-only by default)",
        capability: ProjectCapability::TablesRead,
        method: "POST",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_id}/query",
        params_schema: r#"{"connection_id":"string","sql":"string?","params":"array?","table":"string?","limit":"number?","read_only":"boolean?"}"#,
    },
    OperationSpec {
        id: "list_db_connection_schemas",
        category: "db",
        description: "List schemas for one DB connection by immutable id",
        capability: ProjectCapability::TablesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_id}/schemas",
        params_schema: r#"{"connection_id":"string","include_system":"boolean?"}"#,
    },
    OperationSpec {
        id: "list_db_connection_tables",
        category: "db",
        description: "List tables for one DB connection by immutable id (optionally filtered by schema)",
        capability: ProjectCapability::TablesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_id}/tables",
        params_schema: r#"{"connection_id":"string","schema":"string?","include_system":"boolean?"}"#,
    },
    OperationSpec {
        id: "list_db_connection_functions",
        category: "db",
        description: "List functions for one DB connection by immutable id (optionally filtered by schema)",
        capability: ProjectCapability::TablesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_id}/functions",
        params_schema: r#"{"connection_id":"string","schema":"string?","include_system":"boolean?"}"#,
    },
    OperationSpec {
        id: "preview_db_connection_table",
        category: "db",
        description: "Preview table rows for one DB connection by immutable id",
        capability: ProjectCapability::TablesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/db/connections/{connection_id}/table-preview",
        params_schema: r#"{"connection_id":"string","table":"string","limit":"number?"}"#,
    },
    // Docs (project docs: ERD, README.md, AGENTS.md, use cases)
    OperationSpec {
        id: "list_project_docs",
        category: "docs",
        description: "List project doc files (e.g. ERD, README.md, AGENTS.md, use case diagrams) under app/docs",
        capability: ProjectCapability::ProjectRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/docs",
        params_schema: "",
    },
    OperationSpec {
        id: "read_project_doc",
        category: "docs",
        description: "Read one project doc by path (e.g. README.md, AGENTS.md, erd.svg)",
        capability: ProjectCapability::ProjectRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/docs/{path}",
        params_schema: r#"{"path":"string"}"#,
    },
    OperationSpec {
        id: "create_project_doc",
        category: "docs",
        description: "Create or update one project doc file under app/docs",
        capability: ProjectCapability::FilesWrite,
        method: "POST",
        path: "/api/projects/{owner}/{project}/docs",
        params_schema: r#"{"path":"string","content":"string"}"#,
    },
    // Tables (simple tables in default Sekejap connection)
    OperationSpec {
        id: "list_tables",
        category: "tables",
        description: "List simple table names in the default Sekejap connection (sjtable/default)",
        capability: ProjectCapability::TablesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/tables",
        params_schema: "",
    },
    // Files
    OperationSpec {
        id: "list_files",
        category: "files",
        description: "List all files in the project app directory",
        capability: ProjectCapability::FilesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/files",
        params_schema: "",
    },
    OperationSpec {
        id: "read_file",
        category: "files",
        description: "Read a specific file contents by path",
        capability: ProjectCapability::FilesRead,
        method: "GET",
        path: "/api/projects/{owner}/{project}/files/{path}",
        params_schema: r#"{"path":"string"}"#,
    },
    OperationSpec {
        id: "write_file",
        category: "files",
        description: "Write or update a file with new contents",
        capability: ProjectCapability::FilesWrite,
        method: "PUT",
        path: "/api/projects/{owner}/{project}/files/{path}",
        params_schema: r#"{"path":"string","contents":"string"}"#,
    },
];

/// REST channel contract for one operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationRestContract {
    /// HTTP method.
    pub method: String,
    /// HTTP path pattern.
    pub path: String,
    /// Whether route is enabled.
    pub enabled: bool,
}

/// Project assistant channel contract for one operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationAssistantContract {
    /// Stable operation id exposed to assistant context.
    pub operation_id: String,
    /// Whether channel is enabled.
    pub enabled: bool,
}

/// MCP channel contract for one operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationMcpContract {
    /// Tool name (currently same as operation id).
    pub tool_name: String,
    /// Capability mapping key if mapped.
    pub capability_mapped: Option<String>,
    /// Whether this operation is exposed in MCP.
    pub enabled: bool,
}

/// Extractable operation contract item for `/docs/operation`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationContractItem {
    /// Stable operation id.
    pub id: String,
    /// Category group.
    pub category: String,
    /// Human description.
    pub description: String,
    /// Required project capability key.
    pub capability: String,
    /// REST channel contract.
    pub rest: OperationRestContract,
    /// Project assistant channel contract.
    pub project_assistant: OperationAssistantContract,
    /// MCP channel contract.
    pub mcp: OperationMcpContract,
    /// Parsed params schema.
    pub params_schema: Value,
}

/// Root operation contract document served at `/docs/operation`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationContractDocument {
    /// Marker for successful extraction.
    pub ok: bool,
    /// Stable contract schema version.
    pub schema_version: &'static str,
    /// Source anchor for traceability.
    pub source: &'static str,
    /// Operation contract entries.
    pub items: Vec<OperationContractItem>,
}

/// Builds operation contract items as a single source for docs extraction.
pub fn operation_contract_items() -> Vec<OperationContractItem> {
    OPERATIONS
        .iter()
        .map(|op| {
            let params_schema = if op.params_schema.trim().is_empty() {
                Value::Null
            } else {
                serde_json::from_str::<Value>(op.params_schema)
                    .unwrap_or_else(|_| Value::String(op.params_schema.to_string()))
            };
            let capability_mapped = crate::platform::model::mcp_tool_capability(op.id)
                .map(|capability| capability.key().to_string());
            OperationContractItem {
                id: op.id.to_string(),
                category: op.category.to_string(),
                description: op.description.to_string(),
                capability: op.capability.key().to_string(),
                rest: OperationRestContract {
                    method: op.method.to_string(),
                    path: op.path.to_string(),
                    enabled: true,
                },
                project_assistant: OperationAssistantContract {
                    operation_id: op.id.to_string(),
                    enabled: true,
                },
                mcp: OperationMcpContract {
                    tool_name: op.id.to_string(),
                    capability_mapped: capability_mapped.clone(),
                    enabled: capability_mapped.is_some(),
                },
                params_schema,
            }
        })
        .collect()
}

/// Get operation spec by ID.
pub fn get_operation(id: &str) -> Option<&'static OperationSpec> {
    OPERATIONS.iter().find(|op| op.id == id)
}

/// Get all operations by category.
pub fn operations_by_category(category: &str) -> Vec<&'static OperationSpec> {
    OPERATIONS
        .iter()
        .filter(|op| op.category == category)
        .collect()
}
