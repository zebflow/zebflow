//! Pattern matching: maps a DSL command string to a navigation URL.

use crate::platform::model::PIPELINE_DEFINITION_EXTENSION;
use crate::platform::services::project::virtual_path_from_file_rel_path;

/// The one page that opens a pipeline: the registry with the unified editor
/// scoped to the pipeline's folder and the pipeline selected. This is the URL
/// the editor itself navigates to after a save (`redirectUrl` in
/// `pipeline-editor/index.tsx`) and the registry rows link to; there is no
/// other pipeline page. `/pipelines/editor?name=…` used to be one and rendered
/// an empty outlet, which is where every agent following `navigate` landed.
pub fn pipeline_editor_url(owner: &str, project: &str, name: &str) -> String {
    let file_rel_path = pipeline_file_id(name);
    let virtual_path = virtual_path_from_file_rel_path(&file_rel_path);
    format!(
        "/projects/{owner}/{project}/pipelines/registry?type=pipeline&path={virtual_path}&file={file_rel_path}"
    )
}

/// The identity a DSL name resolves to, the way `register` spells it: the
/// `.zf.json` extension is replaced rather than appended, so `api/foo`,
/// `api/foo.json` and `api/foo.zf.json` are one pipeline.
fn pipeline_file_id(name: &str) -> String {
    let rel = name.trim().trim_start_matches('/');
    if rel.ends_with(PIPELINE_DEFINITION_EXTENSION) {
        return rel.to_string();
    }
    let stem = rel.strip_suffix(".json").unwrap_or(rel);
    format!("{stem}{PIPELINE_DEFINITION_EXTENSION}")
}

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
            return Some(pipeline_editor_url(owner, project, name));
        }
    }

    // activate | deactivate | describe | patch | execute pipeline <name>
    for verb in ["activate", "deactivate", "describe", "patch", "execute"] {
        if lower.starts_with(&format!("{verb} pipeline ")) {
            let name = tokens.get(2).copied().unwrap_or("");
            if !name.is_empty() {
                return Some(pipeline_editor_url(owner, project, name));
            }
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
            return Some(format!("/projects/{owner}/{project}/db?slug={slug}"));
        }
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
        return Some(format!("/projects/{owner}/{project}/db"));
    }

    // describe node <kind>
    if lower.starts_with("describe node ") {
        return Some(format!("/projects/{owner}/{project}/build/nodes"));
    }

    // git <subcommand>
    if lower.starts_with("git ") {
        return Some(format!("/projects/{owner}/{project}/files"));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pipeline_command_opens_the_pipeline_in_the_unified_editor() {
        let expected = "/projects/o/p/pipelines/registry?type=pipeline&path=/pipelines/test&file=pipelines/test/demo.zf.json";
        assert_eq!(
            match_patterns("register pipelines/test/demo | trigger.manual", "o", "p").as_deref(),
            Some(expected)
        );
        for dsl in [
            "activate pipeline pipelines/test/demo",
            "deactivate pipeline pipelines/test/demo.zf.json",
            "describe pipeline pipelines/test/demo.json",
            "patch pipeline pipelines/test/demo node b --optional",
            "execute pipeline pipelines/test/demo --input '{}'",
            "register pipelines/test/demo | trigger.manual && activate pipeline pipelines/test/demo",
        ] {
            assert_eq!(match_patterns(dsl, "o", "p").as_deref(), Some(expected), "{dsl}");
        }
        assert_eq!(
            pipeline_editor_url("o", "p", "demo"),
            "/projects/o/p/pipelines/registry?type=pipeline&path=/&file=demo.zf.json"
        );
        assert!(!match_patterns("register x | trigger.manual", "o", "p")
            .unwrap()
            .contains("pipelines/editor"));
    }
}
