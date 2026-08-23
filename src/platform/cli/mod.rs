//! The client half of the CLI: commands that talk to a running instance.
//!
//! `interface.md` groups the surface. Group 2 is what a person who has never
//! read the contract types — `install`, `list`, `status` — and Group 3 is the
//! offline maintenance that repairs a server which will not start. This module
//! is Group 2 only, and speaks HTTP for the reason §6 gives: an install
//! touches an in-memory registry inside the live server, so writing to its data
//! directory behind its back would leave it believing something false.
//!
//! `zebflow login` / `zebflow use` / `zebflow status` are the context commands
//! `interface.md` §5 names, filling in the client context store the same
//! section records as missing.

pub mod client;
pub mod context;
pub mod install;
pub mod render;

use std::io::{self, IsTerminal, Read, Write};

use serde_json::Value;

use client::Instance;
use context::ClientContext;

/// The client commands, as `zeb help` lists them.
pub fn help_section() -> &'static str {
    "Everyday Use (these talk to a running instance over its HTTP API):
  zebflow install <ref> [--repo <repository-id>] [--instance <url>] [--yes]
               Materialize a project from a hub package. Alias for
               `zebflow project install`; bare `install` always means this,
               whatever project context is set.
               <ref> is a package id, optionally @version. A publisher-qualified
               id (acme.kids-games) and its short name (kids-games) both work;
               a name two packages answer to is refused, never ranked.
               Reviews first and prints what would happen, including the SQL,
               then asks before creating the project. --yes skips the question.
               A package the review reports violations for is refused outright,
               and --yes does not override that.
               Consent flags, all on by default:
                 --no-include-code       do not write the package's source
                 --no-include-schema     do not write its .sql files
                 --no-execute-schema     write the SQL but do not run it
               `--execute-schema` without `--include-schema` is refused.
  zebflow project install <ref>
               The canonical spelling of the above.
  zebflow list [--owner <name>] [--instance <url>]
               Projects on this instance. Alias for `zebflow project list`.
  zebflow status [--instance <url>]
               Instance, user, project, and whether the server answers.

Client Context (interface.md §5: nothing is inferred from the working directory):
  zebflow login <instance-url> [--user <name>] [--password <pw>]
               Exchange a password for a session token and store the token.
               With no URL, re-authenticates the stored instance.
               The password is read from stdin when stdin is not a terminal, so
               `printf %s \"$PW\" | zebflow login <url> --user alice` keeps it out
               of the process table and shell history that --password lands in.
  zebflow use <owner>/<project>
               Set the stored default owner and project.
  zebflow logout
               Forget the stored context, token included.

  Context resolves as: explicit flag, then stored default, then error.
  Stored in ~/.zebflow/client/context.json, directory 0700 and file 0600.
  It holds the session token, never the password; a file readable by anyone
  else is reported on stderr and tightened on the next write."
}

/// `zebflow login [<instance-url>] [--user <name>] [--password <pw>]`
pub async fn run_login(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(args, &["--user", "--password"], &[])?;
    let path = context::default_context_path()?;
    let mut stored = context::load(&path)?;

    let instance = context::resolve(
        flags.positional.first().map(String::as_str),
        &stored.instance,
        "instance",
        "run `zebflow login <instance-url>`",
    )
    .map(|value| context::normalize_instance_url(&value))?;

    let user = context::resolve(
        flags.value("--user"),
        &stored.owner,
        "user",
        "pass `--user <name>`",
    )?;
    let password = read_password(flags.value("--password"), &user)?;

    let token = client::login(&instance, &user, &password).await?;
    stored.instance = instance.clone();
    stored.token = token;
    stored.owner = user.clone();
    context::save(&path, &stored)?;

    println!("Signed in to {instance} as {user}");
    if stored.project.trim().is_empty() {
        println!("No default project set. Run `zebflow use {user}/<project>` to set one.");
    }
    Ok(())
}

/// `zebflow logout`
pub fn run_logout(args: &[String]) -> Result<(), io::Error> {
    Flags::parse(args, &[], &[])?.expect_no_positionals("logout")?;
    let path = context::default_context_path()?;
    let stored = context::load(&path)?;
    context::clear(&path)?;
    if stored.is_empty() {
        println!("No stored context to forget.");
    } else {
        println!("Forgot the context for {}.", stored.instance);
    }
    Ok(())
}

/// `zebflow use <owner>/<project>`
pub fn run_use(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(args, &[], &[])?;
    let Some(target) = flags.positional.first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: zebflow use <owner>/<project>",
        ));
    };
    let Some((owner, project)) = target.split_once('/') else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("'{target}' is not <owner>/<project>"),
        ));
    };
    if owner.trim().is_empty() || project.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "both owner and project must be named",
        ));
    }
    let path = context::default_context_path()?;
    let mut stored = context::load(&path)?;
    stored.owner = owner.trim().to_string();
    stored.project = project.trim().to_string();
    context::save(&path, &stored)?;
    println!("Using {}/{}", stored.owner, stored.project);
    Ok(())
}

/// `zebflow status`
pub async fn run_status(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(args, &["--instance"], &[])?;
    let path = context::default_context_path()?;
    let stored = context::load(&path)?;

    let instance_url = match context::resolve(
        flags.value("--instance"),
        &stored.instance,
        "instance",
        "run `zebflow login <instance-url>`",
    ) {
        Ok(value) => context::normalize_instance_url(&value),
        Err(_) => {
            println!("Instance  none stored — run `zebflow login <instance-url>`");
            println!("Context   {}", path.display());
            return Ok(());
        }
    };

    let instance = Instance::new(&instance_url, &stored.token)?;
    let reachable = instance.reachable().await;
    println!(
        "Instance  {instance_url} ({})",
        if reachable {
            "reachable"
        } else {
            "not answering"
        }
    );

    // The stored token is checked rather than trusted: sessions live in the
    // server's memory, so a restart invalidates one that still looks fine here.
    if stored.token.trim().is_empty() {
        println!("User      not signed in — run `zebflow login {instance_url}`");
    } else if !reachable {
        println!(
            "User      {} (unverified: instance not answering)",
            stored.owner
        );
    } else {
        match instance.get_json("/api/profile").await {
            Ok(profile) => {
                let user = profile.get("user").cloned().unwrap_or(Value::Null);
                println!(
                    "User      {} ({})",
                    string_field(&user, "owner"),
                    string_field(&user, "role")
                );
            }
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                println!(
                    "User      stored token rejected — run `zebflow login {instance_url}` again"
                );
            }
            Err(err) => return Err(err),
        }
    }

    match (stored.owner.trim(), stored.project.trim()) {
        (owner, project) if !owner.is_empty() && !project.is_empty() => {
            println!("Project   {owner}/{project}");
        }
        _ => println!("Project   none set — run `zebflow use <owner>/<project>`"),
    }
    println!("Context   {}", path.display());
    Ok(())
}

/// `zeb project list`, aliased as `zebflow list`.
pub async fn run_list(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(args, &["--instance", "--owner"], &[])?;
    let stored = context::load(&context::default_context_path()?)?;
    let instance = open_instance(&stored, flags.value("--instance"))?;
    let owner = context::resolve(
        flags.value("--owner"),
        &stored.owner,
        "owner",
        "pass `--owner <name>` or run `zebflow use <owner>/<project>`",
    )?;

    let response = instance
        .get_json(&format!("/api/users/{owner}/projects"))
        .await?;
    let items = response
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    println!("Projects on {} for {owner}", instance.base_url());
    if items.is_empty() {
        println!("  (none)");
        return Ok(());
    }
    let width = items
        .iter()
        .map(|item| string_field(item, "project").chars().count())
        .max()
        .unwrap_or(0);
    for item in &items {
        let slug = string_field(item, "project");
        let title = string_field(item, "title");
        println!("  {slug:<width$}  {title}");
    }
    Ok(())
}

/// `zeb project install <ref>`, aliased as `zebflow install <ref>`.
pub async fn run_install(args: &[String]) -> Result<(), io::Error> {
    let parsed = install::parse_args(args)?;
    let stored = context::load(&context::default_context_path()?)?;
    let instance = open_instance(&stored, parsed.instance.as_deref())?;
    install::run(&instance, &parsed).await
}

/// Opens the instance the context names, refusing rather than guessing when
/// the URL asked for is not the one the stored token belongs to.
fn open_instance(stored: &ClientContext, flag: Option<&str>) -> Result<Instance, io::Error> {
    let url = context::resolve(
        flag,
        &stored.instance,
        "instance",
        "run `zebflow login <instance-url>`",
    )
    .map(|value| context::normalize_instance_url(&value))?;
    if stored.token.trim().is_empty() || url != stored.instance {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("no stored credential for {url}; run `zebflow login {url}`"),
        ));
    }
    Instance::new(&url, &stored.token)
}

/// Reads the password without putting it anywhere it can be read back.
///
/// A pipe wins over a prompt so a script never has to put the password in
/// `argv`, where it is visible in the process table and the shell history.
fn read_password(flag: Option<&str>, user: &str) -> Result<String, io::Error> {
    if let Some(value) = flag {
        return Ok(value.to_string());
    }
    if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        let password = buffer.trim_end_matches(['\n', '\r']).to_string();
        if password.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "stdin is not a terminal and supplied no password",
            ));
        }
        return Ok(password);
    }

    let echo_off = set_terminal_echo(false);
    if !echo_off {
        eprintln!("zeb: warning: could not disable terminal echo; the password will be visible.");
    }
    print!("Password for {user}: ");
    io::stdout().flush()?;
    let mut password = String::new();
    let read = io::stdin().read_line(&mut password);
    if echo_off {
        set_terminal_echo(true);
        println!();
    }
    read?;
    Ok(password.trim_end_matches(['\n', '\r']).to_string())
}

/// Turns terminal echo off and on through `stty`, reporting whether it worked.
///
/// Shelling out avoids a dependency for one prompt, and the caller says so
/// out loud when it fails rather than reading a password onto a visible line.
fn set_terminal_echo(on: bool) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("stty")
            .arg(if on { "echo" } else { "-echo" })
            .stdin(std::process::Stdio::inherit())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = on;
        false
    }
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// A small flag parser shared by the context commands.
///
/// `install` parses its own arguments because its consent flags are part of
/// what it means, not incidental options.
#[derive(Debug, Default)]
struct Flags {
    values: Vec<(String, String)>,
    positional: Vec<String>,
}

impl Flags {
    fn parse(args: &[String], valued: &[&str], bare: &[&str]) -> Result<Self, io::Error> {
        let mut parsed = Self::default();
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            if valued.contains(&arg) {
                index += 1;
                let value = args.get(index).cloned().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("missing value for {arg}"),
                    )
                })?;
                parsed.values.push((arg.to_string(), value));
            } else if bare.contains(&arg) {
                parsed.values.push((arg.to_string(), String::new()));
            } else if arg.starts_with('-') {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown flag '{arg}'"),
                ));
            } else {
                parsed.positional.push(arg.to_string());
            }
            index += 1;
        }
        Ok(parsed)
    }

    fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn expect_no_positionals(&self, command: &str) -> Result<(), io::Error> {
        match self.positional.first() {
            Some(extra) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("`zebflow {command}` takes no arguments, got '{extra}'"),
            )),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn flags_separate_values_from_positionals() {
        let parsed = Flags::parse(
            &args(&["superadmin/default", "--owner", "alice"]),
            &["--owner"],
            &[],
        )
        .expect("parse");
        assert_eq!(parsed.positional, vec!["superadmin/default".to_string()]);
        assert_eq!(parsed.value("--owner"), Some("alice"));
    }

    #[test]
    fn an_unknown_flag_is_refused_rather_than_ignored() {
        let err = Flags::parse(&args(&["--nope"]), &["--owner"], &[]).expect_err("unknown");
        assert!(err.to_string().contains("unknown flag '--nope'"), "{err}");
    }

    #[test]
    fn a_credential_for_another_instance_is_not_reused() {
        let stored = ClientContext {
            instance: "http://localhost:10610".to_string(),
            token: "tok".to_string(),
            ..ClientContext::default()
        };
        let err = open_instance(&stored, Some("https://elsewhere.example"))
            .map(|_| ())
            .expect_err("refused");
        assert!(err.to_string().contains("no stored credential"), "{err}");
    }

    #[test]
    fn no_instance_at_all_names_the_command_that_sets_one() {
        let err = open_instance(&ClientContext::default(), None)
            .map(|_| ())
            .expect_err("refused");
        assert!(err.to_string().contains("zebflow login"), "{err}");
    }
}
