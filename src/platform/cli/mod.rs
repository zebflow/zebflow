//! The client half of the CLI: commands that talk to a running instance.
//!
//! `interface.md` groups the surface. Group 2 is what a person who has never
//! read the contract types — `install`, `list`, `status` — and Group 3 is the
//! offline maintenance that repairs a server which will not start. This module
//! is Group 2 only, and speaks HTTP for the reason §6 gives: an install
//! touches an in-memory registry inside the live server, so writing to its data
//! directory behind its back would leave it believing something false.
//!
//! `zeb login` / `zeb use` / `zeb status` are the context commands
//! `interface.md` §5 names, filling in the client context store the same
//! section records as missing.

pub mod admin;
pub mod client;
pub mod context;
pub mod install;
pub mod local;
pub mod remove;
pub mod render;

use std::ffi::OsStr;
use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;
use std::sync::OnceLock;

use serde_json::Value;

use client::Instance;
use context::ClientContext;

/// The name this binary was invoked as.
///
/// `distribution.md` §Binary name ships two names for one binary, `zeb` primary
/// and `zebflow` kept working. Hardcoding either one means half the users are
/// told to run a command they did not install, so every message asks argv[0]
/// instead. The fallback is the primary name, which is what an unnamed caller
/// should be taught.
static PROGRAM: OnceLock<String> = OnceLock::new();

/// Records argv[0] once, at the top of `main`.
pub fn set_program_name(argv0: Option<&str>) {
    let name = argv0
        .map(Path::new)
        .and_then(Path::file_stem)
        .and_then(OsStr::to_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("zeb")
        .to_string();
    let _ = PROGRAM.set(name);
}

/// What this program calls itself in help and in every "run `… login`" hint.
pub fn program() -> &'static str {
    PROGRAM.get().map(String::as_str).unwrap_or("zeb")
}

/// The client commands, as `zeb help` lists them.
pub fn help_section() -> String {
    let zeb = program();
    format!(
        "Everyday Use (these talk to a running instance over its HTTP API):
  {zeb} install <ref> [--repo <repository-id>] [--instance <url>] [--yes]
               Materialize a project from a hub package. Alias for
               `{zeb} project install`; bare `install` always means this,
               whatever project context is set.
               <ref> is a package id, optionally @version. A publisher-qualified
               id (acme.kids-games) and its short name (kids-games) both work;
               a name two packages answer to is refused, never ranked.
               --repo names one of the instance's own hub repositories by id,
               needed only when more than one carries the same package.
               Reviews first and prints what would happen, including the SQL,
               then asks before creating the project. --yes skips the question.
               A package the review reports violations for is refused outright,
               and --yes does not override that.
               Consent flags, all on by default:
                 --no-include-code       do not write the package's source
                 --no-include-schema     do not write its .sql files
                 --no-execute-schema     write the SQL but do not run it
               `--execute-schema` without `--include-schema` is refused.
  {zeb} project install <ref>
               The canonical spelling of the above.
  {zeb} remove <owner>/<project> [--instance <url>] [--password <pw>] [--yes]
               Delete a project: its record, its source, its database, and the
               files it stores. Alias for `{zeb} project remove`.
               This is not an uninstall. A platform-scope install produces a
               project the instance then owns, so nothing tracks which bytes
               came from a package and nothing is put back by reinstalling one.
               Asks for the project name back, then for the password the API
               re-verifies. --yes skips the name, never the password.
  {zeb} list [--owner <name>] [--instance <url>]
               Projects on this instance. Alias for `{zeb} project list`.
  {zeb} status [--instance <url>]
               Instance, user, project, and whether the server answers.

Client Context (interface.md §5: nothing is inferred from the working directory):
  {zeb} login <instance-url> [--user <name>] [--password <pw>]
               Exchange a password for a session token and store the token.
               With no URL, re-authenticates the stored instance.
               The password is read from stdin when stdin is not a terminal, so
               `printf %s \"$PW\" | {zeb} login <url> --user alice` keeps it out
               of the process table and shell history that --password lands in.
  {zeb} use <owner>/<project>
               Set the stored default owner and project.
  {zeb} logout
               Forget the stored context, token included.

  Context resolves as: explicit flag, then stored default, then error.
  Stored in ~/.zebflow/client/context.json, directory 0700 and file 0600.
  It holds the session token, never the password; a file readable by anyone
  else is reported on stderr and tightened on the next write."
    )
}

/// `zeb login [<instance-url>] [--user <name>] [--password <pw>]`
pub async fn run_login(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(args, &["--user", "--password"], &[])?;
    let path = context::default_context_path()?;
    let mut stored = context::load(&path)?;

    let instance = context::resolve(
        flags.positional.first().map(String::as_str),
        &stored.instance,
        "instance",
        &format!("run `{} login <instance-url>`", program()),
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
        println!(
            "No default project set. Run `{} use {user}/<project>` to set one.",
            program()
        );
    }
    Ok(())
}

/// `zeb logout`
pub async fn run_logout(args: &[String]) -> Result<(), io::Error> {
    Flags::parse(args, &[], &[])?.expect_no_positionals("logout")?;
    let path = context::default_context_path()?;
    let stored = context::load(&path)?;
    if stored.is_empty() {
        context::clear(&path)?;
        println!("No stored context to forget.");
        return Ok(());
    }

    // End the session before forgetting the token, so a failure here is still
    // reported. Forgetting a token does not revoke it: the server's session
    // stands until it expires, and anyone who copied the file meanwhile keeps
    // the account. The local context is cleared either way -- refusing to
    // forget an unreachable instance would strand the one command that exists
    // to get out of it.
    let revoked = match client::Instance::new(&stored.instance, &stored.token) {
        Ok(instance) => instance.end_session().await,
        Err(_) => false,
    };
    context::clear(&path)?;

    if revoked {
        println!("Signed out of {} and forgot the context.", stored.instance);
    } else {
        println!("Forgot the context for {}.", stored.instance);
        eprintln!(
            "{}: warning: {} could not be reached, so the session was not ended. \
             The stored token stays valid there until it expires.",
            program(),
            stored.instance
        );
    }
    Ok(())
}

/// `zeb use <owner>/<project>`
pub fn run_use(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(args, &[], &[])?;
    let Some(target) = flags.positional.first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} use <owner>/<project>", program()),
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

/// `zeb status`
pub async fn run_status(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(args, &["--instance"], &[])?;
    let path = context::default_context_path()?;
    let stored = context::load(&path)?;

    let instance_url = match context::resolve(
        flags.value("--instance"),
        &stored.instance,
        "instance",
        &format!("run `{} login <instance-url>`", program()),
    ) {
        Ok(value) => context::normalize_instance_url(&value),
        Err(_) => {
            println!(
                "Instance  none stored — run `{} login <instance-url>`",
                program()
            );
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
        println!(
            "User      not signed in — run `{} login {instance_url}`",
            program()
        );
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
                    "User      stored token rejected — run `{} login {instance_url}` again",
                    program()
                );
            }
            Err(err) => return Err(err),
        }
    }

    match (stored.owner.trim(), stored.project.trim()) {
        (owner, project) if !owner.is_empty() && !project.is_empty() => {
            println!("Project   {owner}/{project}");
        }
        _ => println!(
            "Project   none set — run `{} use <owner>/<project>`",
            program()
        ),
    }
    println!("Context   {}", path.display());
    Ok(())
}

/// `zeb project list`, aliased as `zeb list`.
pub async fn run_list(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(args, &["--instance", "--owner"], &[])?;
    let mut stored = context::load(&context::default_context_path()?)?;
    let instance = open_instance(&mut stored, flags.value("--instance")).await?;
    let owner = context::resolve(
        flags.value("--owner"),
        &stored.owner,
        "owner",
        &format!(
            "pass `--owner <name>` or run `{} use <owner>/<project>`",
            program()
        ),
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

/// `zeb project install <ref>`, aliased as `zeb install <ref>`.
pub async fn run_install(args: &[String]) -> Result<(), io::Error> {
    let parsed = install::parse_args(args)?;
    let mut stored = context::load(&context::default_context_path()?)?;
    let instance = open_instance(&mut stored, parsed.instance.as_deref()).await?;
    install::run(&instance, &parsed).await
}

/// `zeb project remove <ref>`, aliased as `zeb remove <ref>`.
pub async fn run_remove(args: &[String]) -> Result<(), io::Error> {
    let flags = Flags::parse(
        args,
        &["--instance", "--owner", "--password"],
        &["--yes", "-y"],
    )?;
    let mut stored = context::load(&context::default_context_path()?)?;
    let instance = open_instance(&mut stored, flags.value("--instance")).await?;
    let Some(reference) = flags.positional.first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} remove <owner>/<project>", program()),
        ));
    };
    // An owner named in the reference wins over both, so the flag and the
    // stored default only have to answer the bare `<project>` form.
    let owner = flags.value("--owner").unwrap_or(&stored.owner);
    let (owner, project) = remove::split_reference(reference, owner)?;

    remove::run(
        &instance,
        &remove::RemoveArgs {
            owner,
            project,
            instance: flags.value("--instance").map(str::to_string),
            password: flags.value("--password").map(str::to_string),
            assume_yes: flags.value("--yes").is_some() || flags.value("-y").is_some(),
        },
    )
    .await
}

/// Opens the instance the context names, refusing rather than guessing when
/// the URL asked for is not the one the stored token belongs to.
///
/// One exception, and it is a rule rather than a special case: when the
/// resolved URL is *this machine's own*, `local` opens it — over HTTP if a
/// server is listening, in this process if none is, creating the instance if
/// there is not one yet. `interface.md` §6 states it as HTTP when a server owns
/// the state and direct when none does, which is why a cold machine needs no
/// `login` and no server started by hand.
///
/// A remote instance is untouched by that. An unreachable one is an error, not
/// a reason to quietly install somewhere else.
async fn open_instance(
    stored: &mut ClientContext,
    flag: Option<&str>,
) -> Result<Instance, io::Error> {
    let local_url = crate::platform::boot::local_instance_url();
    let url = resolve_instance_url(stored, flag, &local_url);

    if url == local_url {
        return local::open(stored).await;
    }
    if stored.token.trim().is_empty() || url != stored.instance {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "no stored credential for {url}; run `{} login {url}`",
                program()
            ),
        ));
    }
    Instance::new(&url, &stored.token)
}

/// Which instance a command is about: the flag, then the stored default, then
/// this machine.
///
/// The last step is not a guess. §5 says nothing is inferred from the working
/// directory, and this infers nothing from it: with no flag and no stored
/// context there is exactly one instance a person can mean, and it is the one
/// on the machine they are typing on.
fn resolve_instance_url(stored: &ClientContext, flag: Option<&str>, local_url: &str) -> String {
    match context::resolve(flag, &stored.instance, "instance", "") {
        Ok(value) => context::normalize_instance_url(&value),
        Err(_) => local_url.to_string(),
    }
}

/// Reads the password without putting it anywhere it can be read back.
///
/// A pipe wins over a prompt so a script never has to put the password in
/// `argv`, where it is visible in the process table and the shell history.
pub(crate) fn read_password(flag: Option<&str>, user: &str) -> Result<String, io::Error> {
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
                format!(
                    "`{} {command}` takes no arguments, got '{extra}'",
                    program()
                ),
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

    #[tokio::test]
    async fn a_credential_for_another_instance_is_not_reused() {
        let mut stored = ClientContext {
            instance: "http://localhost:10610".to_string(),
            token: "tok".to_string(),
            ..ClientContext::default()
        };
        let err = open_instance(&mut stored, Some("https://elsewhere.example"))
            .await
            .map(|_| ())
            .expect_err("refused");
        assert!(err.to_string().contains("no stored credential"), "{err}");
    }

    #[test]
    fn no_stored_context_means_this_machine_rather_than_an_error() {
        // This replaces a test that asserted the opposite. It required a
        // `zeb login` before anything else could run, which is exactly the
        // first-use step interface.md §5 now says a cold machine does not take:
        // there is one instance a person can mean here, and this is it.
        let local = "http://127.0.0.1:10610";
        assert_eq!(
            resolve_instance_url(&ClientContext::default(), None, local),
            local
        );
        // A stored instance still wins over the local default, and a flag over
        // both, so nothing that was explicit becomes a guess.
        let stored = ClientContext {
            instance: "https://zeb.example".to_string(),
            ..ClientContext::default()
        };
        assert_eq!(
            resolve_instance_url(&stored, None, local),
            "https://zeb.example"
        );
        assert_eq!(
            resolve_instance_url(&stored, Some("https://other.example/"), local),
            "https://other.example"
        );
    }
}
