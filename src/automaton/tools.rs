//! Tool trait and registry for automaton REPL. Pipeline (or other hosts) can register tools.

use std::path::Path;
use std::process::Command;

/// One tool: name, description, and run(args, work_dir) -> output.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    #[allow(dead_code)]
    fn description(&self) -> &str;
    /// Args as JSON (e.g. {"path": "/tmp"} for ls). Run in work_dir when relevant.
    fn run(&self, args: &serde_json::Value, work_dir: &Path) -> Result<String, String>;
}

/// Registry of tools. Register built-in and external tools, then run by name.
#[derive(Default)]
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Add a tool. Later registrations with same name overwrite (first wins if we use find).
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name().to_string()).collect()
    }

    /// Run one tool by name. Args from LLM (e.g. {"path": "/tmp"}).
    pub fn run_tool(
        &self,
        name: &str,
        args: &serde_json::Value,
        work_dir: &Path,
    ) -> Option<Result<String, String>> {
        let t = self.get(name)?;
        Some(t.run(args, work_dir))
    }

    /// Run several tools (by name) and return combined output for context.
    pub fn run_auto(&self, enabled: &[String], work_dir: &Path) -> String {
        let mut out = String::new();
        for name in enabled {
            let args = serde_json::json!({});
            match self.run_tool(name, &args, work_dir) {
                Some(Ok(s)) => out.push_str(&format!("=== {} ===\n{}\n", name, s.trim())),
                Some(Err(e)) => out.push_str(&format!("=== {} ===\n(error: {})\n", name, e)),
                None => out.push_str(&format!("=== {} (unknown) ===\n(skipped)\n", name)),
            }
        }
        out
    }
}

/// Built-in: list directory. Optional args: "path" or "dir" (default: work_dir).
pub struct LsTool;

impl Tool for LsTool {
    fn name(&self) -> &str {
        "ls"
    }
    fn description(&self) -> &str {
        "list directory; optional args: path (or dir) = directory to list"
    }
    fn run(&self, args: &serde_json::Value, work_dir: &Path) -> Result<String, String> {
        let path = args
            .get("path")
            .or_else(|| args.get("dir"))
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let target = if path == "." || path.is_empty() {
            work_dir.to_path_buf()
        } else {
            let p = Path::new(path);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                work_dir.join(p)
            }
        };
        let result = Command::new("ls")
            .args(["-la"])
            .current_dir(&target)
            .output();
        match result {
            Ok(o) if o.status.success() => {
                Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
            }
            Ok(o) => Err(format!(
                "exit {:?}: {}",
                o.status,
                String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => Err(format!("{}", e)),
        }
    }
}

/// Built-in: current working directory (work_dir).
pub struct PwdTool;

impl Tool for PwdTool {
    fn name(&self) -> &str {
        "pwd"
    }
    fn description(&self) -> &str {
        "print working directory"
    }
    fn run(&self, _args: &serde_json::Value, work_dir: &Path) -> Result<String, String> {
        Ok(work_dir.display().to_string())
    }
}

/// Built-in: run a Python script. Args: "script" or "path" = path to .py (relative to work_dir).
pub struct PythonTool;

impl Tool for PythonTool {
    fn name(&self) -> &str {
        "python"
    }
    fn description(&self) -> &str {
        "run a Python script; args: script (or path) = path to .py file"
    }
    fn run(&self, args: &serde_json::Value, work_dir: &Path) -> Result<String, String> {
        let script = args
            .get("script")
            .or_else(|| args.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing script (or path)".to_string())?;
        let script = script.trim();
        if script.is_empty() {
            return Err("script path is empty".to_string());
        }
        let script_path = Path::new(script);
        let full = if script_path.is_absolute() {
            script_path.to_path_buf()
        } else {
            work_dir.join(script_path)
        };
        if !full.exists() {
            return Err(format!("script not found: {}", full.display()));
        }
        if full.extension().map(|e| e != "py").unwrap_or(true) {
            return Err("only .py files are allowed".to_string());
        }
        let result = Command::new("python3")
            .arg(&full)
            .current_dir(work_dir)
            .output();
        match result {
            Ok(o) if o.status.success() => {
                let out = String::from_utf8_lossy(&o.stdout);
                let err = String::from_utf8_lossy(&o.stderr);
                let mut s = out.trim().to_string();
                if !err.trim().is_empty() {
                    s.push_str("\n(stderr)\n");
                    s.push_str(err.trim());
                }
                Ok(s)
            }
            Ok(o) => Err(format!(
                "exit {:?}: {}",
                o.status,
                String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => Err(format!("{}", e)),
        }
    }
}

/// Registry with built-in tools (ls, pwd, python). Pipeline can clone and register more.
pub fn default_registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    r.register(Box::new(LsTool));
    r.register(Box::new(PwdTool));
    r.register(Box::new(PythonTool));
    r
}

/// Which tools to run automatically (from env ZEBTUNE_AUTO_COMMANDS). Default "ls,pwd".
pub fn enabled_auto_commands() -> Vec<String> {
    let s = std::env::var("ZEBTUNE_AUTO_COMMANDS").unwrap_or_else(|_| "ls,pwd".into());
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

/// Parse "RUN: ls /tmp" or "RUN: python scripts/foo.py" into (name, args).
pub fn parse_run_line(line: &str, allowed: &[String]) -> Option<(String, serde_json::Value)> {
    let t = line.trim();
    if !t.starts_with("RUN:") {
        return None;
    }
    let rest = t[4..].trim();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let name = parts.next()?.trim().to_string();
    if !allowed.contains(&name) {
        return None;
    }
    let tail = parts.next().map(str::trim).filter(|s| !s.is_empty());
    let args = match (name.as_str(), tail) {
        ("python", Some(t)) => serde_json::json!({ "script": t }),
        ("ls", Some(t)) | ("pwd", Some(t)) => serde_json::json!({ "path": t }),
        (_, Some(t)) => serde_json::json!({ "path": t }),
        (_, None) => serde_json::json!({}),
    };
    Some((name, args))
}
