//! Platform-aware tools for the project assistant agentic loop.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::automaton::llm_interface::ToolDef;
use crate::platform::model::{
    DescribeProjectDbConnectionRequest, QueryProjectDbConnectionRequest, SimpleTableQueryRequest,
    DbObjectNode,
};
use crate::platform::services::PlatformService;

/// Result of executing a tool — text answer plus optional browser interaction sequence.
pub struct ToolRunResult {
    /// Human-readable result shown in the chat tool bubble.
    pub text: String,
    /// If present, emitted as `interaction_sequence` SSE event for browser automation.
    pub interaction: Option<Value>,
}

impl ToolRunResult {
    fn text(s: impl Into<String>) -> Self {
        Self { text: s.into(), interaction: None }
    }
}

/// Platform-aware tool runner for the project assistant.
pub struct AssistantPlatformTools {
    platform: Arc<PlatformService>,
    owner: String,
    project: String,
}

impl AssistantPlatformTools {
    pub fn new(platform: Arc<PlatformService>, owner: &str, project: &str) -> Self {
        Self {
            platform,
            owner: owner.to_string(),
            project: project.to_string(),
        }
    }

    /// Tool definitions in OpenAI function calling schema format.
    pub fn tool_defs() -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "list_pipelines".to_string(),
                description: "List all pipelines in the current project.".to_string(),
                parameters: json!({"type":"object","properties":{},"required":[]}),
            },
            ToolDef {
                name: "get_pipeline".to_string(),
                description: "Read the source JSON of a pipeline file. Use the exact file_rel_path returned by list_pipelines.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "file_rel_path": {
                            "type": "string",
                            "description": "Exact file_rel_path from list_pipelines output (e.g. 'pipelines/contents/blog/list-posts.zf.json')"
                        }
                    },
                    "required": ["file_rel_path"]
                }),
            },
            ToolDef {
                name: "list_templates".to_string(),
                description: "List all templates in the project workspace.".to_string(),
                parameters: json!({"type":"object","properties":{},"required":[]}),
            },
            ToolDef {
                name: "read_template".to_string(),
                description: "Read the source of a template file.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "rel_path": {
                            "type": "string",
                            "description": "Relative path to the template file (e.g. 'pages/blog-home.tsx')"
                        }
                    },
                    "required": ["rel_path"]
                }),
            },
            ToolDef {
                name: "list_tables".to_string(),
                description: "List all simple tables in the project.".to_string(),
                parameters: json!({"type":"object","properties":{},"required":[]}),
            },
            ToolDef {
                name: "query_table".to_string(),
                description: "Query rows from a simple table.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "table": { "type": "string", "description": "Table slug (required)" },
                        "where_field": { "type": "string", "description": "Optional field name to filter by equality" },
                        "where_value": { "description": "Optional value to match (required if where_field is set)" },
                        "limit": { "type": "integer", "description": "Max rows to return (default 20)" }
                    },
                    "required": ["table"]
                }),
            },
            ToolDef {
                name: "list_project_docs".to_string(),
                description: "List all documentation files in the project.".to_string(),
                parameters: json!({"type":"object","properties":{},"required":[]}),
            },
            ToolDef {
                name: "read_project_doc".to_string(),
                description: "Read a project documentation file.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative path to the doc file" }
                    },
                    "required": ["path"]
                }),
            },
            ToolDef {
                name: "read_skill".to_string(),
                description: "Read a Zebflow platform skill document (pipeline-authoring, rwe-templates, sekejapql, etc.).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Skill name (e.g. 'pipeline-authoring', 'rwe-templates', 'sekejapql')" }
                    },
                    "required": ["name"]
                }),
            },
            ToolDef {
                name: "list_db_connections".to_string(),
                description: "List all database connections configured in the project (PostgreSQL, SjTable, etc.).".to_string(),
                parameters: json!({"type":"object","properties":{},"required":[]}),
            },
            ToolDef {
                name: "describe_db_connection".to_string(),
                description: "Describe the schema of a database connection — lists schemas, tables, and columns. Call list_db_connections first to get the connection_id.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "connection_id": { "type": "string", "description": "The connection_id from list_db_connections" },
                        "scope": { "type": "string", "description": "Optional: 'tree' (default), 'tables', 'schemas'" }
                    },
                    "required": ["connection_id"]
                }),
            },
            ToolDef {
                name: "get_table_columns".to_string(),
                description: "Get the column definitions (name, data_type, nullable, default) for a specific table in a database connection. Call this BEFORE writing any SQL so you know the exact column names and types — never guess them.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "connection_id": { "type": "string", "description": "The connection_id from list_db_connections" },
                        "table": { "type": "string", "description": "Table name (e.g. 'users')" },
                        "schema": { "type": "string", "description": "Schema name (default: 'public' for PostgreSQL)" }
                    },
                    "required": ["connection_id", "table"]
                }),
            },
            ToolDef {
                name: "update_memory".to_string(),
                description: "Replace your MEMORY.md with updated content. Call this when you discover important project information (architecture, data models, decisions, patterns) that is not yet recorded in Your Memory.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "Full updated MEMORY.md content (markdown)" }
                    },
                    "required": ["content"]
                }),
            },
            ToolDef {
                name: "run_db_query".to_string(),
                description: "Execute SQL on a database connection and return the results. Also shows the query running in the browser UI for the user. Call list_db_connections first to get the connection_id.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "connection_id": { "type": "string", "description": "The connection_id from list_db_connections" },
                        "sql": { "type": "string", "description": "SQL query to run (SELECT recommended)" }
                    },
                    "required": ["connection_id", "sql"]
                }),
            },
            ToolDef {
                name: "navigate_to".to_string(),
                description: "Navigate the user's browser to a project page. ALWAYS prefer this over fetching data with tools when the user wants to SEE something — pipelines, templates, tables, credentials, settings, etc. The page itself shows the data.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "Absolute URL path to navigate to, e.g. /projects/owner/project/pipelines" },
                        "label": { "type": "string", "description": "Human-readable page name, e.g. 'Pipelines'" }
                    },
                    "required": ["url"]
                }),
            },
            ToolDef {
                name: "fill_input".to_string(),
                description: "Fill a form field on the current page using a data attribute selector, and optionally submit the form. Use after navigate_to to pre-fill inputs like a SQL query editor.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "selector": { "type": "string", "description": "data-* attribute selector, e.g. '[data-query-input]' or '[data-pipeline-name]'" },
                        "value": { "type": "string", "description": "Value to fill into the field" },
                        "submit": { "type": "boolean", "description": "If true, submit the form after filling (default false)" }
                    },
                    "required": ["selector", "value"]
                }),
            },
        ]
    }

    /// Execute a named tool with JSON args, returning a result string.
    pub fn run(&self, name: &str, args: &Value) -> String {
        match name {
            "list_pipelines" => self.list_pipelines(),
            "get_pipeline" => {
                // Accept both field names — LLMs sometimes use rel_path
                let path = args["file_rel_path"]
                    .as_str()
                    .or_else(|| args["rel_path"].as_str())
                    .unwrap_or("");
                self.get_pipeline(path)
            }
            "list_templates" => self.list_templates(),
            "read_template" => {
                let path = args["rel_path"].as_str().unwrap_or("");
                self.read_template(path)
            }
            "list_tables" => self.list_tables(),
            "query_table" => {
                let table = args["table"].as_str().unwrap_or("").to_string();
                let where_field = args["where_field"].as_str().map(|s| s.to_string());
                let where_value = args.get("where_value").cloned();
                let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                self.query_table(table, where_field, where_value, limit)
            }
            "list_project_docs" => self.list_project_docs(),
            "read_project_doc" => {
                let path = args["path"].as_str().unwrap_or("");
                self.read_project_doc(path)
            }
            "read_skill" => {
                let skill_name = args["name"].as_str().unwrap_or("");
                self.read_skill(skill_name)
            }
            "list_db_connections" => self.list_db_connections(),
            "navigate_to" => {
                let url = args["url"].as_str().unwrap_or("");
                let label = args["label"].as_str().unwrap_or(url);
                format!("Navigating to {label} ({url})")
            }
            "fill_input" => {
                let selector = args["selector"].as_str().unwrap_or("");
                let value = args["value"].as_str().unwrap_or("");
                let submit = args["submit"].as_bool().unwrap_or(false);
                if submit {
                    format!("Filled '{selector}' with value and submitted the form")
                } else {
                    format!("Filled '{selector}' with: {value}")
                }
            }
            _ => format!("Unknown tool: '{name}'"),
        }
    }

    fn list_pipelines(&self) -> String {
        match self
            .platform
            .projects
            .list_pipeline_meta_rows(&self.owner, &self.project)
        {
            Ok(rows) => match serde_json::to_string_pretty(&rows) {
                Ok(s) => s,
                Err(e) => format!("Serialization error: {e}"),
            },
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn get_pipeline(&self, file_rel_path: &str) -> String {
        if file_rel_path.is_empty() {
            return "Error: file_rel_path is required".to_string();
        }
        match self
            .platform
            .projects
            .read_pipeline_source(&self.owner, &self.project, file_rel_path)
        {
            Ok(src) => src,
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn list_templates(&self) -> String {
        match self
            .platform
            .projects
            .list_template_workspace(&self.owner, &self.project)
        {
            Ok(listing) => match serde_json::to_string_pretty(&listing) {
                Ok(s) => s,
                Err(e) => format!("Serialization error: {e}"),
            },
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn read_template(&self, rel_path: &str) -> String {
        if rel_path.is_empty() {
            return "Error: rel_path is required".to_string();
        }
        match self
            .platform
            .projects
            .read_template_file(&self.owner, &self.project, rel_path)
        {
            Ok(content) => content,
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn list_tables(&self) -> String {
        match self
            .platform
            .simple_tables
            .list_tables(&self.owner, &self.project)
        {
            Ok(tables) => match serde_json::to_string_pretty(&tables) {
                Ok(s) => s,
                Err(e) => format!("Serialization error: {e}"),
            },
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn query_table(
        &self,
        table: String,
        where_field: Option<String>,
        where_value: Option<Value>,
        limit: usize,
    ) -> String {
        if table.is_empty() {
            return "Error: table is required".to_string();
        }
        let req = SimpleTableQueryRequest {
            table,
            where_field,
            where_value,
            limit: limit.clamp(1, 200),
        };
        match self
            .platform
            .simple_tables
            .query_rows(&self.owner, &self.project, &req)
        {
            Ok(result) => match serde_json::to_string_pretty(&result.rows) {
                Ok(s) => s,
                Err(e) => format!("Serialization error: {e}"),
            },
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn list_project_docs(&self) -> String {
        match self
            .platform
            .projects
            .list_project_docs(&self.owner, &self.project)
        {
            Ok(items) => match serde_json::to_string_pretty(&items) {
                Ok(s) => s,
                Err(e) => format!("Serialization error: {e}"),
            },
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn read_project_doc(&self, path: &str) -> String {
        if path.is_empty() {
            return "Error: path is required".to_string();
        }
        match self
            .platform
            .projects
            .read_project_doc(&self.owner, &self.project, path)
        {
            Ok(content) => content,
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn read_skill(&self, name: &str) -> String {
        if name.is_empty() {
            let names: Vec<&str> = crate::platform::skills::all_skills()
                .iter()
                .map(|s| s.name)
                .collect();
            return format!(
                "Error: name is required. Available skills: {}",
                names.join(", ")
            );
        }
        match crate::platform::skills::get_skill(name) {
            Some(skill) => format!("# {}\n\n{}", skill.title, skill.content),
            None => format!("Error: skill '{name}' not found"),
        }
    }

    fn list_db_connections(&self) -> String {
        match self
            .platform
            .db_connections
            .list_project_connections(&self.owner, &self.project)
        {
            Ok(items) => match serde_json::to_string_pretty(&items) {
                Ok(s) => s,
                Err(e) => format!("Serialization error: {e}"),
            },
            Err(e) => format!("Error: {}", e.message),
        }
    }

    /// Execute a named tool that may require async (DB tools). Falls back to sync `run()` for others.
    pub async fn run_async(&self, name: &str, args: &Value) -> ToolRunResult {
        match name {
            "update_memory" => {
                let content = args["content"].as_str().unwrap_or("");
                match self.platform.projects.upsert_agent_doc(&self.owner, &self.project, "MEMORY.md", content) {
                    Ok(_) => ToolRunResult::text("Memory updated."),
                    Err(e) => ToolRunResult::text(format!("Error updating memory: {}", e.message)),
                }
            }
            "describe_db_connection" => {
                let connection_id = args["connection_id"].as_str().unwrap_or("");
                let scope = args["scope"].as_str().map(|s| s.to_string());
                ToolRunResult::text(self.describe_db_connection(connection_id, scope).await)
            }
            "query_db_connection" => {
                let connection_id = args["connection_id"].as_str().unwrap_or("");
                let sql = args["sql"].as_str().unwrap_or("");
                let limit = args["limit"].as_u64().unwrap_or(50) as usize;
                ToolRunResult::text(self.query_db_connection(connection_id, sql, limit).await)
            }
            "get_table_columns" => {
                let connection_id = args["connection_id"].as_str().unwrap_or("");
                let table = args["table"].as_str().unwrap_or("");
                let schema = args["schema"].as_str().unwrap_or("public");
                ToolRunResult::text(self.get_table_columns(connection_id, schema, table).await)
            }
            "run_db_query" => {
                let connection_id = args["connection_id"].as_str().unwrap_or("").to_string();
                let sql = args["sql"].as_str().unwrap_or("").to_string();
                // Execute server-side so LLM sees actual results
                let result_text = self.query_db_connection(&connection_id, &sql, 50).await;
                // Also build browser interaction for user-visible animation
                let interaction = self.build_run_db_query_interaction(&connection_id, &sql);
                ToolRunResult { text: result_text, interaction }
            }
            _ => ToolRunResult::text(self.run(name, args)),
        }
    }

    async fn get_table_columns(&self, connection_id: &str, schema: &str, table: &str) -> String {
        if connection_id.is_empty() {
            return "Error: connection_id is required. Use list_db_connections first.".to_string();
        }
        if table.is_empty() {
            return "Error: table is required".to_string();
        }

        // Look up the connection's database_kind for adapter-specific column inspection.
        let db_kind = match self
            .platform
            .db_connections
            .list_project_connections(&self.owner, &self.project)
        {
            Ok(items) => items
                .into_iter()
                .find(|c| c.connection_id == connection_id)
                .map(|c| c.database_kind)
                .unwrap_or_default(),
            Err(_) => String::new(),
        };

        match db_kind.as_str() {
            // SjTable: columns come from hash_indexed_fields + range_indexed_fields in describe
            "sjtable" => self.get_sjtable_columns(connection_id, table).await,
            // PostgreSQL, MySQL, MariaDB — all support information_schema.columns
            "postgresql" | "mysql" | "mariadb" | "" => {
                self.get_sql_table_columns(connection_id, schema, table).await
            }
            other => {
                format!(
                    "Column inspection is not yet supported for '{other}'. \
                     Use describe_db_connection to see the schema tree."
                )
            }
        }
    }

    async fn get_sql_table_columns(&self, connection_id: &str, schema: &str, table: &str) -> String {
        // Sanitize identifiers — keep only safe chars to prevent SQL injection
        let safe_schema: String = schema.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
        let safe_table: String = table.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
        if safe_schema.is_empty() || safe_table.is_empty() {
            return "Error: invalid schema or table name".to_string();
        }
        let sql = format!(
            "SELECT column_name, data_type, character_maximum_length, \
             is_nullable, column_default \
             FROM information_schema.columns \
             WHERE table_schema = '{safe_schema}' AND table_name = '{safe_table}' \
             ORDER BY ordinal_position"
        );
        self.query_db_connection(connection_id, &sql, 200).await
    }

    async fn get_sjtable_columns(&self, connection_id: &str, table: &str) -> String {
        // For SjTable, describe the tree to find hash/range indexed fields.
        let req = DescribeProjectDbConnectionRequest {
            scope: Some("tree".to_string()),
            schema: None,
            include_system: Some(false),
        };
        match self
            .platform
            .db_runtime
            .describe_connection(&self.owner, &self.project, connection_id, &req)
            .await
        {
            Ok(result) => {
                let table_node: Option<&DbObjectNode> = result
                    .nodes
                    .iter()
                    .find(|n| n.kind == "table" && n.name == table);
                if let Some(node) = table_node {
                    let hash_fields: Vec<&str> = node.meta
                        .get("hash_indexed_fields")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    let range_fields: Vec<&str> = node.meta
                        .get("range_indexed_fields")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    let mut columns: Vec<Value> = Vec::new();
                    columns.push(json!({"column_name":"id","data_type":"string","index":"primary","note":"document key"}));
                    for f in &hash_fields {
                        columns.push(json!({"column_name": f, "data_type": "string", "index": "hash"}));
                    }
                    for f in &range_fields {
                        columns.push(json!({"column_name": f, "data_type": "string", "index": "range"}));
                    }
                    columns.push(json!({"column_name":"payload","data_type":"json","index":"none","note":"arbitrary document fields"}));
                    serde_json::to_string_pretty(&columns).unwrap_or_else(|e| format!("Serialization error: {e}"))
                } else {
                    let available: Vec<&str> = result
                        .nodes
                        .iter()
                        .filter(|n| n.kind == "table")
                        .map(|n| n.name.as_str())
                        .collect();
                    format!(
                        "Table '{table}' not found. Available tables: {}",
                        available.join(", ")
                    )
                }
            }
            Err(e) => format!("Error: {}", e.message),
        }
    }

    fn build_run_db_query_interaction(&self, connection_id: &str, sql: &str) -> Option<Value> {
        if connection_id.is_empty() || sql.is_empty() {
            return None;
        }
        let conn = self
            .platform
            .db_connections
            .list_project_connections(&self.owner, &self.project)
            .ok()?
            .into_iter()
            .find(|c| c.connection_id == connection_id)?;
        let db_kind = &conn.database_kind;
        let slug = &conn.connection_slug;
        let label = conn.connection_label.clone();
        let owner = &self.owner;
        let project = &self.project;
        let query_url = format!("/projects/{owner}/{project}/db/{db_kind}/{slug}/query");
        let steps = json!([
            { "action": "navigate", "url": query_url },
            { "action": "wait_for_selector", "selector": "[data-db-suite-query-editor]", "timeout_ms": 5000 },
            { "action": "set_editor", "selector": "[data-db-suite-query-editor]", "value": sql },
            { "action": "sleep", "ms": 200 },
            { "action": "click", "selector": "[data-db-suite-query-run]" }
        ]);
        Some(json!({
            "id": format!("run-db-query-{connection_id}"),
            "label": format!("Running SQL on {label}…"),
            "steps": steps
        }))
    }

    async fn describe_db_connection(&self, connection_id: &str, scope: Option<String>) -> String {
        if connection_id.is_empty() {
            return "Error: connection_id is required. Use list_db_connections first.".to_string();
        }
        let req = DescribeProjectDbConnectionRequest {
            scope,
            schema: None,
            include_system: Some(false),
        };
        match self
            .platform
            .db_runtime
            .describe_connection(&self.owner, &self.project, connection_id, &req)
            .await
        {
            Ok(result) => match serde_json::to_string_pretty(&result) {
                Ok(s) => s,
                Err(e) => format!("Serialization error: {e}"),
            },
            Err(e) => format!("Error: {}", e.message),
        }
    }

    async fn query_db_connection(&self, connection_id: &str, sql: &str, limit: usize) -> String {
        if connection_id.is_empty() {
            return "Error: connection_id is required. Use list_db_connections first.".to_string();
        }
        if sql.is_empty() {
            return "Error: sql is required".to_string();
        }
        // Enforce read-only: reject non-SELECT statements
        let trimmed = sql.trim_start().to_ascii_uppercase();
        if !trimmed.starts_with("SELECT") && !trimmed.starts_with("WITH") && !trimmed.starts_with("EXPLAIN") {
            return "Error: only SELECT, WITH (CTE), and EXPLAIN queries are allowed. Write statements are not permitted.".to_string();
        }
        let req = QueryProjectDbConnectionRequest {
            sql: sql.to_string(),
            params: vec![],
            table: None,
            limit: Some(limit.clamp(1, 200)),
            read_only: Some(true),
        };
        match self
            .platform
            .db_runtime
            .query_connection(&self.owner, &self.project, connection_id, &req)
            .await
        {
            Ok(result) => match serde_json::to_string_pretty(&result) {
                Ok(s) => s,
                Err(e) => format!("Serialization error: {e}"),
            },
            Err(e) => format!("Error: {}", e.message),
        }
    }
}
