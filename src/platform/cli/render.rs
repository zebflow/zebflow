//! Turning an install review into something a person reads.
//!
//! These are pure functions over the JSON the API answers with, so what the
//! terminal prints can be tested without a server. The SQL report is the part
//! that earns the command: a person should know what will touch their data
//! before it does, so every destructive statement is quoted in full and
//! nothing in that section is elided for length.

use std::fmt::Write as _;

use serde_json::Value;

/// How many entries of a plain list are shown before the rest are counted.
///
/// File lists run to hundreds of paths and reading them is not the decision
/// being made. The pipeline and SQL sections are never truncated, because they
/// are.
const LIST_PREVIEW: usize = 20;

/// The `owner/project` this install would create.
pub fn destination(review: &Value) -> String {
    format!("{}/{}", text(review, "owner"), text(review, "project"))
}

/// The whole pre-install report, as printed.
pub fn review_report(review: &Value) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "  Package   {} {} ({})",
        text(review, "package_id"),
        text(review, "version"),
        text(review, "asset_kind")
    );
    let _ = writeln!(out, "  Creates   project {}", destination(review));
    let _ = writeln!(out, "  Risk      {}", text(review, "risk_level"));
    let scope = review.get("scope").cloned().unwrap_or(Value::Null);
    let _ = writeln!(
        out,
        "  Consent   source code: {} · schema files: {} · run schema: {}",
        yes_no(flag(&scope, "include_code")),
        yes_no(flag(&scope, "include_schema")),
        yes_no(flag(&scope, "execute_schema"))
    );

    push_list(
        &mut out,
        "Files written",
        &list(review, "files_written"),
        true,
    );
    push_list(
        &mut out,
        "Files this consent leaves out",
        &list(review, "skipped_files"),
        true,
    );

    let registered = list(review, "pipelines_registered");
    let activated = list(review, "pipelines_activated");
    let inactive = list(review, "pipelines_not_activated");
    if !registered.is_empty() || !activated.is_empty() || !inactive.is_empty() {
        let _ = writeln!(out, "\nPipelines");
        push_sublist(&mut out, "registered", &registered);
        push_sublist(&mut out, "activated on install", &activated);
        push_sublist(&mut out, "named active but not registered", &inactive);
    }

    push_database(&mut out, review);

    push_list(
        &mut out,
        "Credentials the package expects",
        &list(review, "credentials_required"),
        true,
    );
    push_list(
        &mut out,
        "Outbound URLs",
        &list(review, "external_urls"),
        true,
    );
    push_list(
        &mut out,
        "Nodes that open a connection",
        &list(review, "network_effects"),
        true,
    );
    push_list(
        &mut out,
        "Nodes that run supplied code",
        &list(review, "code_execution"),
        true,
    );
    push_list(
        &mut out,
        "Public endpoints it would serve",
        &list(review, "public_endpoints"),
        true,
    );
    push_list(
        &mut out,
        "Schedules it would start",
        &list(review, "schedules"),
        true,
    );
    push_list(&mut out, "Warnings", &list(review, "warnings"), false);

    let violations = list(review, "violations");
    if !violations.is_empty() {
        let _ = writeln!(
            out,
            "\nViolations ({}) — a violation cannot be overridden; --yes does not apply",
            violations.len()
        );
        for item in &violations {
            let _ = writeln!(out, "  - {item}");
        }
    }

    out.push('\n');
    out
}

/// What the install actually did, and where the project ended up.
pub fn install_report(result: &Value, base_url: &str) -> String {
    let owner = text(result, "owner");
    let project = text(result, "project");
    let mut out = String::new();
    let _ = writeln!(out, "Installed {owner}/{project}");
    let _ = writeln!(
        out,
        "  {} file(s) written, {} pipeline(s) registered, {} activated",
        list(result, "files_written").len(),
        list(result, "pipelines_registered").len(),
        list(result, "pipelines_activated").len()
    );
    let _ = writeln!(
        out,
        "  schema executed: {}",
        yes_no(result.get("schema_executed").and_then(Value::as_bool) == Some(true))
    );
    push_list(
        &mut out,
        "SQL written but not run",
        &list(result, "unexecuted_initial_data"),
        true,
    );
    push_list(
        &mut out,
        "Named active but not registered",
        &list(result, "pipelines_not_activated"),
        true,
    );
    let _ = writeln!(out, "\n  Project   {base_url}/projects/{owner}/{project}");
    out
}

fn push_database(out: &mut String, review: &Value) {
    let reports = review
        .get("database_initialization")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if reports.is_empty() {
        return;
    }
    let executed = review.get("schema_executed").and_then(Value::as_bool) == Some(true);
    let _ = writeln!(
        out,
        "\nDatabase — {}",
        if executed {
            "this SQL runs against the new project's stores"
        } else {
            "this SQL is written into repo/ and left for you to run"
        }
    );
    for report in &reports {
        let _ = writeln!(
            out,
            "  {} -> {} ({})",
            text(report, "source"),
            text(report, "store"),
            text(report, "engine")
        );
        let unreadable = text(report, "unreadable");
        if !unreadable.is_empty() {
            let _ = writeln!(out, "      not read: {unreadable}");
            continue;
        }
        let statements = report
            .get("statements")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .map(|(verb, count)| format!("{verb} {}", count.as_u64().unwrap_or_default()))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        if !statements.is_empty() {
            let _ = writeln!(out, "      statements: {statements}");
        }
        let tables = list(report, "tables");
        if !tables.is_empty() {
            let _ = writeln!(out, "      tables: {}", tables.join(", "));
        }
        let _ = writeln!(
            out,
            "      existing data exposed to it: {}",
            yes_no(report.get("existing_data_at_risk").and_then(Value::as_bool) == Some(true))
        );
        let destructive = list(report, "destructive");
        if destructive.is_empty() {
            let _ = writeln!(out, "      destructive statements: none");
        } else {
            let _ = writeln!(out, "      destructive statements ({}):", destructive.len());
            for statement in &destructive {
                let _ = writeln!(out, "        {statement}");
            }
        }
    }
}

fn push_list(out: &mut String, heading: &str, items: &[String], truncate: bool) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n{heading} ({})", items.len());
    let shown = if truncate {
        items.len().min(LIST_PREVIEW)
    } else {
        items.len()
    };
    for item in &items[..shown] {
        let _ = writeln!(out, "  {item}");
    }
    if shown < items.len() {
        let _ = writeln!(out, "  ... and {} more", items.len() - shown);
    }
}

fn push_sublist(out: &mut String, heading: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "  {heading} ({})", items.len());
    for item in items {
        let _ = writeln!(out, "    {item}");
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| match item.as_str() {
                    Some(text) => text.to_string(),
                    None => item.to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn review() -> Value {
        json!({
            "package_id": "kids-educational-games",
            "version": "1.0.0",
            "asset_kind": "project_bundle",
            "owner": "superadmin",
            "project": "kids-educational-games",
            "risk_level": "medium",
            "scope": {"include_code": true, "include_schema": true, "execute_schema": true},
            "files_written": ["repo/pipelines/app.zf.json", "repo/schema/sekejap.sql"],
            "pipelines_registered": ["pipelines/app.zf.json"],
            "pipelines_activated": ["pipelines/app.zf.json"],
            "pipelines_not_activated": [],
            "schema_executed": true,
            "database_initialization": [{
                "engine": "sekejap",
                "source": "repo/schema/sekejap.sql",
                "store": "project store",
                "existing_data_at_risk": false,
                "statements": {"CREATE": 2, "INSERT": 1},
                "tables": ["lessons", "scores"],
                "destructive": ["DROP TABLE lessons"],
            }],
            "warnings": ["one warning"],
            "violations": [],
            "installable": true,
        })
    }

    #[test]
    fn the_report_names_the_package_and_where_it_lands() {
        let out = review_report(&review());
        assert!(
            out.contains("kids-educational-games 1.0.0 (project_bundle)"),
            "{out}"
        );
        assert!(
            out.contains("Creates   project superadmin/kids-educational-games"),
            "{out}"
        );
        assert_eq!(destination(&review()), "superadmin/kids-educational-games");
    }

    #[test]
    fn destructive_sql_is_quoted_rather_than_counted() {
        let out = review_report(&review());
        assert!(
            out.contains("this SQL runs against the new project's stores"),
            "{out}"
        );
        assert!(out.contains("statements: CREATE 2, INSERT 1"), "{out}");
        assert!(out.contains("tables: lessons, scores"), "{out}");
        assert!(out.contains("destructive statements (1):"), "{out}");
        assert!(out.contains("DROP TABLE lessons"), "{out}");
    }

    #[test]
    fn schema_left_unrun_says_so() {
        let mut value = review();
        value["schema_executed"] = json!(false);
        let out = review_report(&value);
        assert!(out.contains("left for you to run"), "{out}");
    }

    #[test]
    fn violations_say_that_nothing_overrides_them() {
        let mut value = review();
        value["violations"] = json!(["writes outside repo/"]);
        value["installable"] = json!(false);
        let out = review_report(&value);
        assert!(out.contains("Violations (1)"), "{out}");
        assert!(out.contains("cannot be overridden"), "{out}");
        assert!(out.contains("writes outside repo/"), "{out}");
    }

    #[test]
    fn long_file_lists_are_previewed_but_warnings_are_whole() {
        let mut value = review();
        value["files_written"] = json!((0..30).map(|i| format!("repo/f{i}")).collect::<Vec<_>>());
        value["warnings"] = json!((0..30).map(|i| format!("warning {i}")).collect::<Vec<_>>());
        let out = review_report(&value);
        assert!(out.contains("... and 10 more"), "{out}");
        assert!(out.contains("warning 29"), "{out}");
    }

    #[test]
    fn the_install_report_ends_with_the_project_url() {
        let result = json!({
            "owner": "superadmin",
            "project": "kids-educational-games-2",
            "files_written": ["a", "b"],
            "pipelines_registered": ["p"],
            "pipelines_activated": ["p"],
            "schema_executed": true,
        });
        let out = install_report(&result, "http://localhost:10610");
        assert!(
            out.contains("Installed superadmin/kids-educational-games-2"),
            "{out}"
        );
        assert!(
            out.contains("2 file(s) written, 1 pipeline(s) registered, 1 activated"),
            "{out}"
        );
        assert!(
            out.contains("http://localhost:10610/projects/superadmin/kids-educational-games-2"),
            "{out}"
        );
    }
}
