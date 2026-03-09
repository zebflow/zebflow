use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeMode {
    #[default]
    Inline,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityPolicy {
    pub import_allowlist: Vec<String>,
    pub blocked_globals: Vec<String>,
    pub allow_dynamic_import: bool,
    pub allow_raw_html: bool,
    pub network_allowlist: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            import_allowlist: vec![
                "./".to_string(),
                "../".to_string(),
                "@/".to_string(),
                "rwe".to_string(),
            ],
            blocked_globals: vec![
                "eval".to_string(),
                "Function".to_string(),
                "globalThis.Function".to_string(),
            ],
            allow_dynamic_import: false,
            allow_raw_html: false,
            network_allowlist: vec![], // empty = no restriction; populate to enforce domain allowlist
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CompileOptions {
    pub template_root: Option<String>,
    pub file_path: Option<String>,
    pub security: SecurityPolicy,
    pub runtime_mode: RuntimeMode,
    pub deno_timeout_ms: u64,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            template_root: None,
            file_path: None,
            security: SecurityPolicy::default(),
            runtime_mode: RuntimeMode::Inline,
            deno_timeout_ms: 3_000,
        }
    }
}
