//! Pattern matching: maps a DSL command string to a navigation URL.

/// Match a DSL string against known patterns and return a navigation URL if matched.
/// Only inspects the first command when chained with `&&`.
pub fn match_patterns(dsl: &str, owner: &str, project: &str) -> Option<String> {
    // For multi-command DSL (&&), only match against the first command.
    let first_cmd = dsl.split("&&").next().unwrap_or(dsl).trim();
    let lower = first_cmd.to_lowercase();
    let tokens: Vec<&str> = first_cmd.split_whitespace().collect();

    // register <name> ...
    if lower.starts_with("register ") {
        let name = tokens.get(1).copied().unwrap_or("");
        if !name.is_empty() {
            return Some(format!(
                "/projects/{owner}/{project}/pipelines/editor?name={name}"
            ));
        }
    }

    // activate pipeline <name>
    if lower.starts_with("activate pipeline ") {
        let name = tokens.get(2).copied().unwrap_or("");
        if !name.is_empty() {
            return Some(format!(
                "/projects/{owner}/{project}/pipelines/editor?name={name}&tab=status"
            ));
        }
    }

    // describe pipeline <name>
    if lower.starts_with("describe pipeline ") {
        let name = tokens.get(2).copied().unwrap_or("");
        if !name.is_empty() {
            return Some(format!(
                "/projects/{owner}/{project}/pipelines/editor?name={name}"
            ));
        }
    }

    // patch pipeline <name>
    if lower.starts_with("patch pipeline ") {
        let name = tokens.get(2).copied().unwrap_or("");
        if !name.is_empty() {
            return Some(format!(
                "/projects/{owner}/{project}/pipelines/editor?name={name}"
            ));
        }
    }

    // get templates
    if lower.starts_with("get templates") {
        return Some(format!("/projects/{owner}/{project}/build/templates"));
    }

    // get pipelines
    if lower.starts_with("get pipelines") {
        return Some(format!("/projects/{owner}/{project}/pipelines/registry"));
    }

    // describe connection <slug>
    if lower.starts_with("describe connection ") {
        let slug = tokens.get(2).copied().unwrap_or("");
        if !slug.is_empty() {
            return Some(format!(
                "/projects/{owner}/{project}/db?slug={slug}"
            ));
        }
    }

    // contains sjtable.query — navigate to sekejap tables page
    if lower.contains("sjtable.query") {
        let table_name = tokens.windows(2)
            .find(|w| w[0] == "--table")
            .map(|w| w[1])
            .unwrap_or("");
        if !table_name.is_empty() {
            return Some(format!(
                "/projects/{owner}/{project}/db/sekejap/default/tables?table={table_name}"
            ));
        }
        return Some(format!("/projects/{owner}/{project}/db/sekejap/default/tables"));
    }

    // contains pg.query
    if lower.contains("pg.query") {
        return Some(format!("/projects/{owner}/{project}/db"));
    }

    // get connections
    if lower.starts_with("get connections") {
        return Some(format!("/projects/{owner}/{project}/db/connections"));
    }

    // get credentials
    if lower.starts_with("get credentials") {
        return Some(format!("/projects/{owner}/{project}/credentials"));
    }

    // get docs
    if lower.starts_with("get docs") {
        return Some(format!("/projects/{owner}/{project}/build/docs"));
    }

    // get nodes
    if lower.starts_with("get nodes") {
        return Some(format!("/projects/{owner}/{project}/build/nodes"));
    }

    // get tables
    if lower.starts_with("get tables") {
        return Some(format!("/projects/{owner}/{project}/db/sekejap/default/tables"));
    }

    // describe node <kind>
    if lower.starts_with("describe node ") {
        return Some(format!("/projects/{owner}/{project}/build/nodes"));
    }

    // execute pipeline <name>
    if lower.starts_with("execute pipeline ") {
        let name = tokens.get(2).copied().unwrap_or("");
        if !name.is_empty() {
            return Some(format!(
                "/projects/{owner}/{project}/pipelines/editor?name={name}&tab=logs"
            ));
        }
    }

    // deactivate pipeline <name>
    if lower.starts_with("deactivate pipeline ") {
        let name = tokens.get(2).copied().unwrap_or("");
        if !name.is_empty() {
            return Some(format!(
                "/projects/{owner}/{project}/pipelines/editor?name={name}&tab=status"
            ));
        }
    }

    // git <subcommand>
    if lower.starts_with("git ") {
        return Some(format!("/projects/{owner}/{project}/files"));
    }

    None
}
