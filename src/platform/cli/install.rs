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
    /// Which of the instance's hub repositories to take it from. Required only
    /// when more than one carries the same package id, because the channel is
    /// a trust decision and is never picked for the user.
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
/// accepted because it is what a person types; it is a name, not a preference
/// order, and two packages answering to it are refused rather than ranked.
fn reference_names(package_id: &str, reference: &str) -> bool {
    package_id == reference
        || package_id
            .split_once('.')
            .is_some_and(|(_, slug)| slug == reference)
}

/// Picks the one package a reference names, or says why it cannot.
///
/// A reference is never resolved by preference order: two hub sources carrying
/// the same id is a question only the user can answer, since the channel is the
/// trust decision.
pub fn resolve_asset(
    items: &[Value],
    package_id: &str,
    version: Option<&str>,
    repository: Option<&str>,
) -> Result<AssetMatch, io::Error> {
    let mut matches = items
        .iter()
        .filter(|item| {
            item.get("package_id")
                .and_then(Value::as_str)
                .is_some_and(|listed| reference_names(listed, package_id))
        })
        .filter(|item| match repository {
            Some(wanted) => item.get("repository_id").and_then(Value::as_str) == Some(wanted),
            None => true,
        })
        .map(|item| AssetMatch {
            repository_id: string_field(item, "repository_id"),
            package_id: string_field(item, "package_id"),
            version: version
                .map(str::to_string)
                .unwrap_or_else(|| string_field(item, "latest_version")),
        })
        .collect::<Vec<_>>();

    match matches.len() {
        0 => {
            let known = items
                .iter()
                .filter_map(|item| item.get("package_id").and_then(Value::as_str))
                .collect::<Vec<_>>();
            let suffix = if known.is_empty() {
                "this instance's hub sources list no packages".to_string()
            } else {
                format!("available: {}", known.join(", "))
            };
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no hub package named '{package_id}'; {suffix}"),
            ))
        }
        1 => Ok(matches.remove(0)),
        _ => {
            let offers = matches
                .iter()
                .map(|item| format!("{} in {}", item.package_id, item.repository_id))
                .collect::<Vec<_>>()
                .join(", ");
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "'{package_id}' names more than one package ({offers}); say which with the \
                     full package id or --repo <repository-id>"
                ),
            ))
        }
    }
}

/// Runs review, then confirmation, then install.
pub async fn run(instance: &Instance, args: &InstallArgs) -> Result<(), io::Error> {
    let listing = instance.get_json("/api/platform/hub/assets").await?;
    let empty = Vec::new();
    let items = listing
        .get("items")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let asset = resolve_asset(
        items,
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

    #[test]
    fn one_offer_resolves_to_its_latest_version() {
        let items = vec![asset("local", "acme.kids-educational-games")];
        let found =
            resolve_asset(&items, "acme.kids-educational-games", None, None).expect("resolve");
        assert_eq!(found.repository_id, "local");
        assert_eq!(found.version, "1.0.0");
    }

    #[test]
    fn the_short_name_finds_the_publisher_qualified_package() {
        let items = vec![asset("local", "acme.kids-educational-games")];
        let found = resolve_asset(&items, "kids-educational-games", None, None).expect("resolve");
        assert_eq!(found.package_id, "acme.kids-educational-games");
    }

    #[test]
    fn two_publishers_of_one_slug_ask_for_the_full_id() {
        let items = vec![asset("local", "acme.quiz"), asset("local", "globex.quiz")];
        let err = resolve_asset(&items, "quiz", None, None).expect_err("ambiguous");
        assert!(err.to_string().contains("acme.quiz in local"), "{err}");
        assert!(err.to_string().contains("globex.quiz in local"), "{err}");
        let found = resolve_asset(&items, "globex.quiz", None, None).expect("resolve");
        assert_eq!(found.package_id, "globex.quiz");
    }

    #[test]
    fn an_explicit_version_overrides_the_latest() {
        let items = vec![asset("local", "acme.kids-educational-games")];
        let found =
            resolve_asset(&items, "kids-educational-games", Some("0.9.0"), None).expect("resolve");
        assert_eq!(found.version, "0.9.0");
    }

    #[test]
    fn two_repositories_offering_one_id_ask_rather_than_choose() {
        let items = vec![
            asset("local", "acme.shared"),
            asset("zebflow-com", "acme.shared"),
        ];
        let err = resolve_asset(&items, "shared", None, None).expect_err("ambiguous");
        assert!(err.to_string().contains("--repo"), "{err}");
        let found = resolve_asset(&items, "shared", None, Some("zebflow-com")).expect("resolve");
        assert_eq!(found.repository_id, "zebflow-com");
    }

    #[test]
    fn an_unknown_reference_names_what_the_instance_does_offer() {
        let items = vec![asset("local", "acme.kids-educational-games")];
        let err = resolve_asset(&items, "typo", None, None).expect_err("missing");
        assert!(
            err.to_string().contains("acme.kids-educational-games"),
            "{err}"
        );
    }
}
