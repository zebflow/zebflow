//! Shell execution tools for the automaton REPL.
//!
//! These are execution primitives — NOT AI capabilities.
//! AI-native capabilities (TTS, STT, vectorize, classify, etc.) live in `crate::automaton::intelligence`.

use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Resolves a tool's path argument inside `work_dir`.
///
/// `work_dir` is a boundary, not a starting point. An argument that lands
/// outside it is refused by name rather than clamped back inside, so a caller
/// that asked for the wrong path learns that instead of quietly being served a
/// different one.
pub fn resolve_in_work_dir(work_dir: &Path, path: &str) -> Result<PathBuf, String> {
    // The root itself may be relative (`.`), and comparing a relative root
    // against an absolute argument would refuse paths that are in fact inside
    // it. Canonicalize when the root exists; fall back to the literal root so a
    // missing directory fails on use rather than here.
    let root = work_dir
        .canonicalize()
        .unwrap_or_else(|_| work_dir.to_path_buf());

    let requested = path.trim();
    if requested.is_empty() || requested == "." {
        return Ok(root);
    }

    let candidate = {
        let p = Path::new(requested);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        }
    };

    // `..` is resolved lexically first, so a path that escapes is refused even
    // when it names a directory that does not exist and so cannot be
    // canonicalized.
    let lexical = lexically_normalize(&candidate);
    if !lexical.starts_with(&root) {
        return Err(escape_error(requested, &root));
    }

    // Then again against the real filesystem, which is what catches a symbolic
    // link inside the root pointing out of it.
    match lexical.canonicalize() {
        Ok(real) if real.starts_with(&root) => Ok(real),
        Ok(_) => Err(escape_error(requested, &root)),
        // Not yet on disk: the lexical check above already proved containment.
        Err(_) => Ok(lexical),
    }
}

fn escape_error(requested: &str, root: &Path) -> String {
    format!(
        "path '{}' resolves outside the project directory {} and was refused",
        requested,
        root.display()
    )
}

/// Resolves `.` and `..` without touching the filesystem.
fn lexically_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                // A `..` with nothing to pop is kept, so the containment check
                // sees the escape instead of a path that silently lost it.
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// One shell tool: name, description, and run(args, work_dir) -> output.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    #[allow(dead_code)]
    fn description(&self) -> &str;
    /// Args as JSON (e.g. {"path": "/tmp"} for ls). Run in work_dir when relevant.
    fn run(&self, args: &serde_json::Value, work_dir: &Path) -> Result<String, String>;
}

/// Registry of shell tools. Register built-in and external tools, then run by name.
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
        let target = resolve_in_work_dir(work_dir, path)?;
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
        let full = resolve_in_work_dir(work_dir, script)?;
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

/// Registry with built-in tools (ls, pwd, python).
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("zebflow-shell-tools-{name}-{now}"));
        std::fs::create_dir_all(&dir).expect("temp root");
        dir
    }

    #[test]
    fn resolve_keeps_paths_inside_the_work_dir() {
        let root = temp_root("inside");
        std::fs::create_dir_all(root.join("sub")).expect("sub dir");

        let resolved = resolve_in_work_dir(&root, "sub").expect("path inside the root");

        assert!(resolved.starts_with(root.canonicalize().expect("canonical root")));
        assert!(resolved.ends_with("sub"));
    }

    #[test]
    fn resolve_refuses_a_relative_escape_and_names_the_path() {
        let root = temp_root("relative-escape");

        let err = resolve_in_work_dir(&root, "../secrets.txt")
            .expect_err("a path leaving the root must be refused");

        assert!(
            err.contains("../secrets.txt"),
            "refusal must name the path: {err}"
        );
        assert!(
            err.contains(
                &root
                    .canonicalize()
                    .expect("canonical root")
                    .display()
                    .to_string()
            ),
            "refusal must name the root: {err}"
        );
    }

    #[test]
    fn resolve_refuses_an_absolute_path_outside_the_work_dir() {
        let root = temp_root("absolute-escape");

        let err = resolve_in_work_dir(&root, "/etc/passwd")
            .expect_err("an absolute path outside the root must be refused");

        assert!(
            err.contains("/etc/passwd"),
            "refusal must name the path: {err}"
        );
    }

    #[test]
    fn ls_refuses_an_escaping_path_rather_than_listing_it() {
        let root = temp_root("ls-escape");

        let err = LsTool
            .run(&serde_json::json!({ "path": ".." }), &root)
            .expect_err("ls must refuse a path outside its root");

        assert!(err.contains(".."), "refusal must name the path: {err}");
    }

    #[test]
    fn python_refuses_a_script_outside_the_work_dir() {
        let root = temp_root("python-escape");
        let outside = root
            .parent()
            .expect("temp parent")
            .join("zebflow-outside.py");
        std::fs::write(&outside, "print('hi')").expect("outside script");

        let err = PythonTool
            .run(
                &serde_json::json!({ "script": "../zebflow-outside.py" }),
                &root,
            )
            .expect_err("python must refuse a script outside its root");

        assert!(
            err.contains("../zebflow-outside.py"),
            "refusal must name the path: {err}"
        );
        let _ = std::fs::remove_file(&outside);
    }
}
