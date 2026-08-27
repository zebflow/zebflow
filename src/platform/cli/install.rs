//! `zeb project install <ref>`, and its `zeb install <ref>` alias.
//!
//! Platform-scope install: it materialises a whole project, which
//! `distribution.md` §0 separates from the project-scope `install` that takes
//! on a managed dependency. The two are not variants of one operation, and
//! this command only ever performs the first — `interface.md` §7 makes bare
//! `install` an alias with one meaning rather than an inference from context.
//!
//! The review runs first and prints, every time. Creating a project cannot be
//! undone by an uninstall and the SQL a package carries reaches a real store,
//! so the person approving it is shown what would happen before it does.

use std::io::{self, IsTerminal, Write};

use serde_json::{Value, json};

use super::client::Instance;
use super::render;

/// The three consent flags the install API takes, in the shape it takes them.
///
/// Every flag defaults on, so a person who names none installs the whole
/// package — the same default the HTTP body has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentScope {
    pub include_code: bool,
    pub include_schema: bool,
    pub execute_schema: bool,
}

impl Default for ConsentScope {
    fn default() -> Self {
        Self {
            include_code: true,
            include_schema: true,
            execute_schema: true,
        }
    }
}

impl ConsentScope {
    /// Refuses the one combination that cannot mean anything, matching
    /// `HubInstallScope::validate` so the client and the API refuse together
    /// rather than the client sending a body the server will reject.
    fn validate(&self) -> Result<(), io::Error> {
        if self.execute_schema && !self.include_schema {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--execute-schema requires --include-schema: there is nothing to run when the \
                 schema files are not written",
            ));
        }
        Ok(())
    }
}

/// A parsed `zeb install` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallArgs {
    /// `<package-id>` or `<package-id>@<version>`.
    pub package_id: String,
    pub version: Option<String>,
    /// Which of the instance's hub repositories to take it from.
    ///
    /// Never needed to resolve a reference: the sources are searched in order
    /// and the first hit wins. It is how a person **overrides** that order when
    /// two sources carry one package id and they want the later one.
    pub repository: Option<String>,
    pub instance: Option<String>,
    pub assume_yes: bool,
    pub scope: ConsentScope,
}

pub fn parse_args(args: &[String]) -> Result<InstallArgs, io::Error> {
    let mut reference = None::<String>;
    let mut repository = None::<String>;
    let mut instance = None::<String>;
    let mut assume_yes = false;
    let mut scope = ConsentScope::default();

    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--repo" | "--instance" => {
                index += 1;
                let value = args.get(index).cloned().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("missing value for {arg}"),
                    )
                })?;
                if arg == "--repo" {
                    repository = Some(value);
                } else {
                    instance = Some(value);
                }
            }
            "--yes" | "-y" => assume_yes = true,
            "--include-code" => scope.include_code = true,
            "--no-include-code" => scope.include_code = false,
            "--include-schema" => scope.include_schema = true,
            "--no-include-schema" => scope.include_schema = false,
            "--execute-schema" => scope.execute_schema = true,
            "--no-execute-schema" => scope.execute_schema = false,
            other if other.starts_with('-') => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown install flag '{other}'"),
                ));
            }
            other => {
                if reference.is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unexpected argument '{other}'; install takes one reference"),
                    ));
                }
                reference = Some(other.to_string());
            }
        }
        index += 1;
    }

    let reference = reference.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage: {} install <package-id>[@version] [--repo <repository-id>] [--yes]",
                super::program()
            ),
        )
    })?;
    scope.validate()?;

    let (package_id, version) = match reference.rsplit_once('@') {
        Some((id, version)) if !id.is_empty() && !version.is_empty() => {
            (id.to_string(), Some(version.to_string()))
        }
        _ => (reference, None),
    };

    Ok(InstallArgs {
        package_id,
        version,
        repository,
        instance,
        assume_yes,
        scope,
    })
}

/// One hub package as the instance's asset listing reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetMatch {
    pub repository_id: String,
    pub package_id: String,
    pub version: String,
}

/// Whether a listed package id answers to `reference`.
///
/// A hub package id is always `{publisher}.{slug}`, so `kids-educational-games`
/// and `acme.kids-educational-games` name the same thing. The short form is
/// accepted because it is what a person types. It is a name and not a
/// preference order: two *publishers* in one source answering to it are refused
/// rather than ranked, because nothing about where they sit tells them apart.
fn reference_names(package_id: &str, reference: &str) -> bool {
    package_id == reference
        || package_id
            .split_once('.')
            .is_some_and(|(_, slug)| slug == reference)
}

/// One configured source, as the listing reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceRow {
    repository_id: String,
    title: String,
    kind: String,
    base_url: String,
    ok: bool,
    error: String,
}

fn source_rows(sources: &[Value]) -> Vec<SourceRow> {
    sources
        .iter()
        .map(|item| SourceRow {
            repository_id: string_field(item, "repository_id"),
            title: string_field(item, "title"),
            kind: string_field(item, "kind"),
            base_url: string_field(item, "base_url"),
            ok: item.get("ok").and_then(Value::as_bool).unwrap_or(true),
            error: string_field(item, "error"),
        })
        .collect()
}

/// Names every source that was searched, in the order it was searched.
///
/// A reference that no source carries is **one** failure, not one per source,
/// so it is reported as one error that names them all. A source that could not
/// be reached says so here rather than being indistinguishable from a source
/// that simply does not publish the package.
fn searched_sources(sources: &[SourceRow]) -> String {
    if sources.is_empty() {
        return "  (this instance has no hub sources configured)".to_string();
    }
    sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            let kind = if source.kind.is_empty() {
                "api".to_string()
            } else {
                source.kind.clone()
            };
            let status = if source.ok {
                String::new()
            } else if source.error.is_empty() {
                "  [did not answer]".to_string()
            } else {
                format!("  [{}]", source.error)
            };
            let title = if source.title.trim().is_empty() {
                String::new()
            } else {
                format!(" — {}", source.title)
            };
            format!(
                "  {}. {} ({kind}) {}{title}{status}",
                index + 1,
                source.repository_id,
                source.base_url
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Picks the one package a reference names, or says why it cannot.
///
/// **The configured sources are searched in order and the first hit wins.**
/// That order is the instance's own -- `PlatformHubRepository::priority`,
/// ascending -- and the listing returns the sources in it, so this walks the
/// list the server handed back rather than ranking anything itself.
///
/// Ordering settles only the question it can settle. Two *sources* offering one
/// package id is answered by which source is searched first; two *publishers*
/// within one source offering one slug is not, because they are equally near
/// and the reference is genuinely ambiguous. The first is resolved, the second
/// is refused with both full ids, and `--repo` overrides the first.
pub fn resolve_asset(
    items: &[Value],
    sources: &[Value],
    package_id: &str,
    version: Option<&str>,
    repository: Option<&str>,
) -> Result<AssetMatch, io::Error> {
    let sources = source_rows(sources);
    // An instance that returned no source list is walked in the order its rows
    // arrived, which is the order it walked them in.
    let order = if sources.is_empty() {
        let mut seen = Vec::new();
        for item in items {
            let repository_id = string_field(item, "repository_id");
            if !seen.contains(&repository_id) {
                seen.push(repository_id);
            }
        }
        seen
    } else {
        sources
            .iter()
            .map(|source| source.repository_id.clone())
            .collect::<Vec<_>>()
    };

    if let Some(wanted) = repository
        && !order.iter().any(|item| item == wanted)
    {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no hub source named '{wanted}' on this instance. Searched, in order:\n{}",
                searched_sources(&sources)
            ),
        ));
    }

    for repository_id in &order {
        if repository.is_some_and(|wanted| wanted != repository_id) {
            continue;
        }
        let mut matches = items
            .iter()
            .filter(|item| string_field(item, "repository_id") == *repository_id)
            .filter(|item| {
                item.get("package_id")
                    .and_then(Value::as_str)
                    .is_some_and(|listed| reference_names(listed, package_id))
            })
            .map(|item| AssetMatch {
                repository_id: string_field(item, "repository_id"),
                package_id: string_field(item, "package_id"),
                version: version
                    .map(str::to_string)
                    .unwrap_or_else(|| string_field(item, "latest_version")),
            })
            .collect::<Vec<_>>();
        matches.dedup_by(|left, right| left.package_id == right.package_id);
        match matches.len() {
            0 => continue,
            1 => return Ok(matches.remove(0)),
            _ => {
                let offers = matches
                    .iter()
                    .map(|item| format!("{} in {}", item.package_id, item.repository_id))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "'{package_id}' names more than one package in {repository_id} ({offers}); \
                         say which with the full package id"
                    ),
                ));
            }
        }
    }

    let known = items
        .iter()
        .filter_map(|item| item.get("package_id").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let offered = if known.is_empty() {
        String::new()
    } else {
        format!("\nThose sources offer: {}", known.join(", "))
    };
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "no hub package named '{package_id}'. Searched, in order:\n{}{offered}",
            searched_sources(&sources)
        ),
    ))
}

/// Runs review, then confirmation, then install.
pub async fn run(instance: &Instance, args: &InstallArgs) -> Result<(), io::Error> {
    let listing = instance.get_json("/api/platform/hub/assets").await?;
    let empty = Vec::new();
    let items = listing
        .get("items")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    // The sources travel with the rows, in the order the instance searched
    // them, so the client resolves by that order rather than inventing one.
    let sources = listing
        .get("sources")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let asset = resolve_asset(
        items,
        sources,
        &args.package_id,
        args.version.as_deref(),
        args.repository.as_deref(),
    )?;

    let body = json!({
        "repository_id": asset.repository_id,
        "package_id": asset.package_id,
        "version": asset.version,
        "include_code": args.scope.include_code,
        "include_schema": args.scope.include_schema,
        "execute_schema": args.scope.execute_schema,
    });

    println!(
        "Reviewing {}@{} from {} on {}",
        asset.package_id,
        asset.version,
        asset.repository_id,
        instance.base_url()
    );
    println!();

    let response = instance
        .post_json("/api/platform/hub/install/review", &body)
        .await?;
    let review = response.get("review").cloned().unwrap_or(Value::Null);
    print!("{}", render::review_report(&review));

    if !review
        .get("installable")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "refusing to install {}@{}: the package review reports findings that no approval \
                 overrides. Nothing was installed.",
                asset.package_id, asset.version
            ),
        ));
    }

    if !args.assume_yes {
        let destination = render::destination(&review);
        if !confirm(&format!(
            "Install {}@{} as {destination}?",
            asset.package_id, asset.version
        ))? {
            println!("Cancelled. Nothing was installed.");
            return Ok(());
        }
        println!();
    }

    let response = instance
        .post_json("/api/platform/hub/install", &body)
        .await?;
    let result = response.get("install").cloned().unwrap_or(Value::Null);
    print!("{}", render::install_report(&result, instance.base_url()));

    // An install that ran in-process left nothing listening, by design
    // (`interface.md` §6). The project exists and its URL was just printed, so
    // the one thing left to say is what turns that URL into a live address.
    if instance.is_local() {
        let zeb = super::program();
        println!(
            "\n  Nothing is serving it yet. Run `{zeb} run {}` for this project alone, or \
             `{zeb}` for the whole instance at {}.",
            render::destination(&result),
            instance.base_url()
        );
    }
    Ok(())
}

/// Asks before an irreversible act.
///
/// A non-interactive shell is refused rather than defaulted: a script that
/// meant to install says `--yes`, and one that did not should not have the
/// answer chosen for it.
fn confirm(question: &str) -> Result<bool, io::Error> {
    println!("Creating a project is irreversible; no uninstall undoes it.");
    if !io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "stdin is not a terminal and this install was not confirmed; pass --yes to confirm it \
             non-interactively",
        ));
    }
    print!("{question} [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn a_bare_reference_installs_everything() {
        let parsed = parse_args(&args(&["kids-educational-games"])).expect("parse");
        assert_eq!(parsed.package_id, "kids-educational-games");
        assert_eq!(parsed.version, None);
        assert_eq!(parsed.scope, ConsentScope::default());
        assert!(!parsed.assume_yes);
    }

    #[test]
    fn a_pinned_reference_keeps_its_version() {
        let parsed = parse_args(&args(&["acme-tools@1.2.0", "--yes"])).expect("parse");
        assert_eq!(parsed.package_id, "acme-tools");
        assert_eq!(parsed.version.as_deref(), Some("1.2.0"));
        assert!(parsed.assume_yes);
    }

    #[test]
    fn executing_schema_without_writing_it_is_refused_before_the_request() {
        let err = parse_args(&args(&["pkg", "--no-include-schema", "--execute-schema"]))
            .expect_err("contradiction");
        assert!(
            err.to_string()
                .contains("--execute-schema requires --include-schema"),
            "{err}"
        );
    }

    #[test]
    fn narrowing_the_scope_turns_execution_off_with_it() {
        let parsed = parse_args(&args(&[
            "pkg",
            "--no-include-schema",
            "--no-execute-schema",
        ]))
        .expect("parse");
        assert!(parsed.scope.include_code);
        assert!(!parsed.scope.include_schema);
        assert!(!parsed.scope.execute_schema);
    }

    fn asset(repository: &str, package: &str) -> Value {
        json!({
            "repository_id": repository,
            "package_id": package,
            "latest_version": "1.0.0",
            "title": "Kids Educational Games",
            "asset_kind": "project_bundle",
        })
    }

    /// The two official sources, in the order a fresh instance searches them:
    /// static first, API hub second (`distribution.md` §2, decided 2026-08-27).
    fn official_sources() -> Vec<Value> {
        vec![
            json!({
                "repository_id": "zebflow-hub",
                "title": "Zebflow Hub (static)",
                "kind": "static",
                "base_url": "https://raw.githubusercontent.com/zebflow/hub/main",
                "priority": 10,
                "ok": true,
            }),
            json!({
                "repository_id": "zebflow-com",
                "title": "Zebflow Hub",
                "kind": "api",
                "base_url": "https://hub.zebflow.com/api",
                "priority": 20,
                "ok": true,
            }),
        ]
    }

    #[test]
    fn one_offer_resolves_to_its_latest_version() {
        let items = vec![asset("zebflow-com", "acme.kids-educational-games")];
        let found = resolve_asset(
            &items,
            &official_sources(),
            "acme.kids-educational-games",
            None,
            None,
        )
        .expect("resolve");
        assert_eq!(found.repository_id, "zebflow-com");
        assert_eq!(found.version, "1.0.0");
    }

    #[test]
    fn the_short_name_finds_the_publisher_qualified_package() {
        let items = vec![asset("zebflow-com", "acme.kids-educational-games")];
        let found = resolve_asset(
            &items,
            &official_sources(),
            "kids-educational-games",
            None,
            None,
        )
        .expect("resolve");
        assert_eq!(found.package_id, "acme.kids-educational-games");
    }

    #[test]
    fn two_publishers_of_one_slug_in_one_source_ask_for_the_full_id() {
        let items = vec![
            asset("zebflow-com", "acme.quiz"),
            asset("zebflow-com", "globex.quiz"),
        ];
        let sources = official_sources();
        let err = resolve_asset(&items, &sources, "quiz", None, None).expect_err("ambiguous");
        assert!(
            err.to_string().contains("acme.quiz in zebflow-com"),
            "{err}"
        );
        assert!(
            err.to_string().contains("globex.quiz in zebflow-com"),
            "{err}"
        );
        let found = resolve_asset(&items, &sources, "globex.quiz", None, None).expect("resolve");
        assert_eq!(found.package_id, "globex.quiz");
    }

    #[test]
    fn an_explicit_version_overrides_the_latest() {
        let items = vec![asset("zebflow-com", "acme.kids-educational-games")];
        let found = resolve_asset(
            &items,
            &official_sources(),
            "kids-educational-games",
            Some("0.9.0"),
            None,
        )
        .expect("resolve");
        assert_eq!(found.version, "0.9.0");
    }

    /// Two sources offering one id is settled by the order they are searched in.
    ///
    /// This replaces an assertion that the same case was refused. Refusing it
    /// made the two official sources unusable together: the static repository
    /// exists to carry what the API hub does not, and a package present in both
    /// -- which is the normal state during a migration between them -- would
    /// have failed rather than resolved. `--repo` still names the other one.
    #[test]
    fn two_sources_offering_one_id_resolve_to_the_first_searched() {
        let items = vec![
            asset("zebflow-com", "acme.shared"),
            asset("zebflow-hub", "acme.shared"),
        ];
        let sources = official_sources();
        let found = resolve_asset(&items, &sources, "shared", None, None).expect("resolve");
        assert_eq!(found.repository_id, "zebflow-hub");
        let overridden =
            resolve_asset(&items, &sources, "shared", None, Some("zebflow-com")).expect("resolve");
        assert_eq!(overridden.repository_id, "zebflow-com");
    }

    #[test]
    fn a_package_only_the_second_source_carries_still_resolves() {
        let items = vec![asset("zebflow-hub", "acme.only-static")];
        let found =
            resolve_asset(&items, &official_sources(), "only-static", None, None).expect("resolve");
        assert_eq!(found.repository_id, "zebflow-hub");
    }

    #[test]
    fn a_reference_in_neither_source_is_one_error_naming_both() {
        let items = vec![asset("zebflow-com", "acme.kids-educational-games")];
        let err =
            resolve_asset(&items, &official_sources(), "typo", None, None).expect_err("missing");
        let text = err.to_string();
        assert!(text.contains("zebflow-com"), "{text}");
        assert!(text.contains("zebflow-hub"), "{text}");
        assert!(text.contains("acme.kids-educational-games"), "{text}");
        assert_eq!(text.matches("no hub package named").count(), 1, "{text}");
    }

    #[test]
    fn a_source_that_did_not_answer_says_so_rather_than_looking_empty() {
        let mut sources = official_sources();
        sources[1]["ok"] = json!(false);
        sources[1]["error"] = json!("connection refused");
        let err = resolve_asset(&[], &sources, "anything", None, None).expect_err("missing");
        assert!(err.to_string().contains("connection refused"), "{err}");
    }

    #[test]
    fn an_unknown_repo_flag_names_the_sources_that_do_exist() {
        let items = vec![asset("zebflow-com", "acme.kids-educational-games")];
        let err = resolve_asset(
            &items,
            &official_sources(),
            "kids-educational-games",
            None,
            Some("acme-mirror"),
        )
        .expect_err("unknown repo");
        assert!(err.to_string().contains("acme-mirror"), "{err}");
        assert!(err.to_string().contains("zebflow-hub"), "{err}");
    }
}
