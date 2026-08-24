//! `zeb project remove <ref>`, and its `zeb remove <ref>` alias.
//!
//! This is not the undo of `install`. `distribution.md` §0 is explicit that a
//! platform-scope install "produces a project and cannot be undone by an
//! uninstall": what arrived is now the receiving instance's own project, edited
//! freely, and there is no record of which bytes came from the package. So the
//! only removal the platform offers is deleting the project itself, which is
//! what this command does — the catalog record and the whole directory tree.
//!
//! Project-scope uninstall, which does remove a tracked dependency, exists as
//! an API for node bundles and has no CLI verb today. `distribution.md` §0a
//! says so.

use std::io::{self, IsTerminal, Write};

use serde_json::{Value, json};

use super::client::Instance;
use super::program;

/// A parsed `zeb remove` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveArgs {
    pub owner: String,
    pub project: String,
    pub instance: Option<String>,
    pub password: Option<String>,
    pub assume_yes: bool,
}

/// Splits `<owner>/<project>`, falling back to the owner already resolved.
///
/// A bare `<project>` is the everyday form, and the owner it belongs to comes
/// from the same explicit-flag-then-stored-default order §5 fixes for every
/// other command. Nothing is inferred from the working directory.
pub fn split_reference(reference: &str, owner: &str) -> Result<(String, String), io::Error> {
    let (found_owner, project) = match reference.split_once('/') {
        Some((left, right)) => (left.trim(), right.trim()),
        None => (owner.trim(), reference.trim()),
    };
    if found_owner.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "'{reference}' names no owner; write <owner>/<project>, pass `--owner <name>`, or \
                 run `{} use <owner>/<project>`",
                program()
            ),
        ));
    }
    if project.is_empty() || project.contains('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("'{reference}' is not <owner>/<project>"),
        ));
    }
    Ok((found_owner.to_string(), project.to_string()))
}

/// Runs the existence check, then the confirmation, then the delete.
pub async fn run(instance: &Instance, args: &RemoveArgs) -> Result<(), io::Error> {
    let RemoveArgs { owner, project, .. } = args;

    let listing = instance
        .get_json(&format!("/api/users/{owner}/projects"))
        .await?;
    let empty = Vec::new();
    let items = listing
        .get("items")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let found = items
        .iter()
        .find(|item| item.get("project").and_then(Value::as_str) == Some(project.as_str()));
    let Some(found) = found else {
        let known = items
            .iter()
            .filter_map(|item| item.get("project").and_then(Value::as_str))
            .collect::<Vec<_>>();
        let suffix = if known.is_empty() {
            format!("{owner} has no projects on this instance")
        } else {
            format!("{owner} has: {}", known.join(", "))
        };
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no project '{owner}/{project}'; {suffix}"),
        ));
    };

    let title = found
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default();
    println!("Removing {owner}/{project} from {}", instance.base_url());
    if !title.is_empty() {
        println!("  {title}");
    }
    println!();
    println!("This deletes the project record and everything under it: its source,");
    println!("its pipelines, its database, and the files it stores for its users.");
    println!("It is irreversible, and it is not an uninstall — nothing is restored");
    println!("by installing the package it came from again.");
    println!();

    if !args.assume_yes && !confirm(project)? {
        println!("Cancelled. Nothing was removed.");
        return Ok(());
    }

    // The password is the server's own gate on this route, re-verified there
    // rather than trusted from the session, so the client asks for it rather
    // than trying to satisfy the check with the token it already holds.
    let password = super::read_password(args.password.as_deref(), owner)?;
    instance
        .delete_json(
            &format!("/api/users/{owner}/projects/{project}"),
            &json!({ "project_name": project, "password": password }),
        )
        .await?;

    println!("Removed {owner}/{project}.");
    Ok(())
}

/// Asks for the project slug back before an irreversible delete.
///
/// Typing the name is what the API itself requires in the body, so asking for
/// it here is the same confirmation rather than a second invented one.
fn confirm(project: &str) -> Result<bool, io::Error> {
    if !io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "stdin is not a terminal and this removal was not confirmed; pass --yes to confirm \
             it non-interactively (the password is still required, on --password or stdin)",
        ));
    }
    print!("Type '{project}' to confirm: ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(answer.trim() == project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_name_takes_the_resolved_owner() {
        let (owner, project) = split_reference("shop", "superadmin").expect("split");
        assert_eq!(owner, "superadmin");
        assert_eq!(project, "shop");
    }

    #[test]
    fn a_qualified_reference_overrides_the_resolved_owner() {
        let (owner, project) = split_reference("alice/shop", "superadmin").expect("split");
        assert_eq!(owner, "alice");
        assert_eq!(project, "shop");
    }

    #[test]
    fn a_bare_name_with_no_owner_anywhere_names_how_to_supply_one() {
        let err = split_reference("shop", "").expect_err("no owner");
        assert!(err.to_string().contains("--owner"), "{err}");
    }

    #[test]
    fn a_three_part_reference_is_refused_rather_than_truncated() {
        let err = split_reference("alice/shop/extra", "superadmin").expect_err("too deep");
        assert!(
            err.to_string().contains("is not <owner>/<project>"),
            "{err}"
        );
    }
}
