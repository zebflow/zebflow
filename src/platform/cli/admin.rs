//! Offline instance recovery: `zeb admin reset-password`, `break-glass`, `detach`.
//!
//! Group 3 maintenance (`interface.md` §6): these go straight at the data
//! directory because they exist for the moment nothing else works — the
//! operator lost the password, or `offices.md` §4 has disabled local login and
//! the controller that would open the door is gone.
//!
//! ## Why these are commands and not routes
//!
//! `offices.md` §6 names the protection outright: "Filesystem access is the
//! protection, as it is for `pg_hba.conf`." A route would be reachable by
//! whoever can reach the port, which is the population break-glass exists to
//! exclude; and it would need a credential to authenticate, which is the
//! credential that is missing. So the check is *being able to write the data
//! root*, and the only surface with that property is a command run on the host.
//!
//! ## Why they fit the frozen CLI scope
//!
//! `interface.md` §1 freezes the **verbs** and lets the nouns grow, and §3's
//! Group 3 is offline maintenance run "when the server will not start" —
//! exactly this. `admin` is the noun group `reset-password` already
//! established, so nothing new is added at the top level and no alias is
//! claimed (§4's set stays at three).
//!
//! `office` would read better as the noun and is not available: `zeb office` is
//! a Group 1 server mode, so `zeb office detach` would be one word meaning two
//! things depending on what follows it — the docker wart §2 rejects by name.
//! `admin` it is, and the collision is recorded here so it is not re-proposed.
//!
//! ## What they read from the environment
//!
//! The data root and the listen address the running-server probe needs, and
//! nothing else. These are the commands somebody runs when the configuration is
//! wrong, so validating the configuration before helping would refuse exactly
//! when help is needed — see [`offline_data_root`].
//!
//! ## Why they refuse behind a running server
//!
//! The same reason `reset-password` does, and the same probe. A running server
//! holds the catalog open and every live session in memory; a write behind its
//! back is a state it will not see. Requiring the stop also answers a question
//! nobody then has to ask: a session that existed before the act does not
//! survive it, vouched or not, because platform web sessions live only in a
//! running process's memory.

use std::io;
use std::path::Path;
use std::sync::Arc;

use crate::platform::adapters::data::{DataAdapter, build_data_adapter};
use crate::platform::boot;
use crate::platform::model::{IDENTITY_WRITE_ACTION_CREATED, PlatformConfig, slug_segment};
use crate::platform::services::UserService;
use crate::platform::services::cluster::local_authority::{
    OfficeJoinFile, OfficeLocalAuthorityService,
};

/// The file whose presence means an instance exists at a data root. Kept in
/// sync with `local.rs`; an absent catalog means there is nothing to reset and
/// opening the adapter would create an empty one.
const CATALOG_REL: &str = "platform/catalog.db";

/// `zeb admin <subcommand>` dispatch.
pub async fn run(args: &[String]) -> Result<(), io::Error> {
    match args.first().map(String::as_str) {
        Some("reset-password") => run_reset_password(&args[1..]).await,
        Some("break-glass") => run_break_glass(&args[1..]).await,
        Some("detach") => run_detach(&args[1..]).await,
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, usage())),
    }
}

fn usage() -> String {
    let zeb = super::program();
    format!(
        "usage:\n  \
         {zeb} admin reset-password <owner>\n  \
         {zeb} admin break-glass [<owner>]\n  \
         {zeb} admin detach"
    )
}

/// `zeb admin reset-password <owner>` — offline, against the catalog.
async fn run_reset_password(args: &[String]) -> Result<(), io::Error> {
    let [owner] = args else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} admin reset-password <owner>", super::program()),
        ));
    };

    let data_root = offline_data_root("reset").await?;
    let password = reset_against(&data_root, owner)?;

    println!("New password for {owner}: {password}");
    println!(
        "Printed once, not stored. The account is marked `generated` again, so the next \
         browser login is forced to choose a new password. Sessions live only in a running \
         server's memory, and none is running here."
    );
    Ok(())
}

/// `zeb admin break-glass [<owner>]` — `offices.md` §6.
///
/// It re-enables local authority and does **not** detach. §6 says break-glass
/// "re-enables local authority"; §7 makes leaving a separate act. The only
/// reading that keeps both sentences true is that this command changes who may
/// open the door and leaves the membership alone.
///
/// The optional owner is a convenience with a rule attached. §6 says a password
/// reset "is not enough", not that it is unwanted: an operator running this at
/// 3am usually needs both, and doing them as one act means the record says so.
/// The rule is that a local account created *by the controller* is not a valid
/// target — see [`refuse_controller_created`].
async fn run_break_glass(args: &[String]) -> Result<(), io::Error> {
    let owner = match args {
        [] => None,
        [owner] => Some(owner.clone()),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("usage: {} admin break-glass [<owner>]", super::program()),
            ));
        }
    };

    let data_root = offline_data_root("break-glass").await?;
    let data = open_data(&data_root)?;
    let authority = OfficeLocalAuthorityService::new(data.clone(), data_root.clone());

    let join = authority.join_file();
    if let OfficeJoinFile::Absent = join {
        return Err(io::Error::other(format!(
            "{} has joined no controller, so its local authority was never disabled and there \
             is nothing to break out of. Local login already works here; use `{} admin \
             reset-password <owner>` if the password is what is missing.",
            data_root.display(),
            super::program()
        )));
    }

    // Both checks before anything is written. A break-glass row recorded and
    // then a rotation that failed because the account does not exist would
    // leave the office broken out with no credential and an operator reading a
    // message about the wrong problem.
    if let Some(owner) = owner.as_deref() {
        let users = UserService::new(data.clone());
        if users.get_user(owner).map_err(to_io)?.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "no local account '{owner}' on {}. Run without an owner to re-enable local \
                     authority on its own.",
                    data_root.display()
                ),
            ));
        }
        refuse_controller_created(data.as_ref(), owner)?;
    }

    let detail = match owner.as_deref() {
        Some(owner) => format!(
            "local authority re-enabled from the host; the password of local account '{owner}' \
             was rotated in the same act"
        ),
        None => "local authority re-enabled from the host".to_string(),
    };
    let event = authority
        .break_glass(owner.as_deref().unwrap_or_default(), &detail)
        .map_err(to_io)?;

    let rotated = match owner.as_deref() {
        Some(owner) => Some((
            owner,
            UserService::new(data.clone())
                .reset_password_generated(owner)
                .map_err(to_io)?,
        )),
        None => None,
    };

    println!(
        "Local authority re-enabled on {} (office '{}').",
        data_root.display(),
        event.office_id
    );
    match rotated {
        Some((owner, password)) => {
            println!("New password for {owner}: {password}");
            println!(
                "Printed once, not stored. The account is marked `generated` again, so the next \
                 browser login is forced to choose a new password."
            );
        }
        None => {
            println!(
                "No password was changed. Run `{} admin reset-password <owner>` if you also \
                 need a credential.",
                super::program()
            );
        }
    }
    println!(
        "This office is still joined: the controller may still place projects here and still \
         vouches for identities here. `{} admin detach` is what leaves.",
        super::program()
    );
    println!(
        "Recorded locally as {} and readable at GET /api/office/local-authority by this \
         office's own superadmin. It is reported to the controller on the next successful \
         registration; until then it stays here, and if this office never reconnects it stays \
         here for good.",
        event.event_id
    );
    Ok(())
}

/// `zeb admin detach` — `offices.md` §7.
///
/// The office keeps everything, because everything except the login term was
/// always its own (§4). Removing the join token is the entire act.
async fn run_detach(args: &[String]) -> Result<(), io::Error> {
    if !args.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} admin detach", super::program()),
        ));
    }

    let data_root = offline_data_root("detach").await?;
    let data = open_data(&data_root)?;
    let authority = OfficeLocalAuthorityService::new(data, data_root.clone());
    let office_id = authority.join_file().office_id().to_string();
    let event = authority
        .detach("left the controller from the host")
        .map_err(to_io)?;

    println!(
        "Detached {} from its controller{}.",
        data_root.display(),
        if office_id.is_empty() {
            String::new()
        } else {
            format!(" (office '{office_id}')")
        }
    );
    println!(
        "Nothing was removed but the join token. Projects, data/, files/, the blessed shelf, \
         and the public surface are unchanged, and local accounts are live again — they were \
         disabled, never deleted."
    );
    println!(
        "Unset ZEBFLOW_CLUSTER_JOIN_TOKEN before starting this instance again, or the variable \
         will join it straight back. Ask the controller to revoke this office's token as well: \
         detaching is this instance's act and the controller still holds the record."
    );
    println!("Recorded locally as {}.", event.event_id);
    Ok(())
}

/// Refuse to hand a local password to an account the controller created.
///
/// A vouch-created account exists because a controller said a person is who
/// they are; its password is 32 random bytes hashed and discarded unread, so it
/// is reachable by vouch and by nothing else. Re-enabling local authority must
/// not change that, and neither must the convenience half of this command:
/// giving that account a password would turn a controller-asserted identity
/// into a local one, quietly, in the same keystroke as an unrelated repair.
///
/// The refusal names a different target rather than a way around itself. Host
/// access is host access and `reset-password` is one command away, but a
/// backdoor is not something a refusal should hand over directions to.
fn refuse_controller_created(data: &dyn DataAdapter, owner: &str) -> Result<(), io::Error> {
    let owner = slug_segment(owner);
    // Asked of the whole log, not of its newest 500 rows. This is a refusal,
    // and a refusal that only sees a window stops refusing on the row after it:
    // an office with a busy identity log would have handed a local password to
    // a controller-created account, quietly, in the same keystroke as an
    // unrelated repair. The paginated read stays for display.
    let created_by_controller = data
        .office_identity_write_exists(&owner, IDENTITY_WRITE_ACTION_CREATED)
        .map_err(to_io)?;
    if created_by_controller {
        return Err(io::Error::other(format!(
            "'{owner}' exists on this office only because its controller vouched for it. That \
             account has no usable password by construction and is reachable by vouch and by \
             nothing else; re-enabling local authority does not change it. Break the glass on \
             one of this office's own local accounts instead."
        )));
    }
    Ok(())
}

/// The data root, once it is established that nothing is serving it.
///
/// One probe and one existence check, shared by all three subcommands so their
/// refusals read the same.
///
/// It resolves the data root **directly** rather than through
/// `boot::load_platform_config`, and that is load-bearing rather than tidy.
/// Loading a full configuration validates things these commands do not use — a
/// role's cluster variables, the shape of a join token, the insecure-default
/// password guard — and every one of those is a state a recovery command may
/// find itself in. §6 says an office "is never locked out of itself"; a
/// break-glass that refused to run because an unrelated variable was wrong
/// would be exactly that lockout, arriving from the one direction nobody
/// checks. The only environment these commands read is the data root
/// (`interface.md` §5's two cases) and the listen address the probe needs.
async fn offline_data_root(act: &str) -> Result<std::path::PathBuf, io::Error> {
    let url = boot::local_instance_url();
    if super::local::listening(&url).await {
        return Err(io::Error::other(format!(
            "a server is answering at {url} and owns this data root. Stop it first — this \
             {act} writes to the catalog the running server holds open, and every live session \
             lives in its memory."
        )));
    }
    let data_root = boot::default_data_root();
    if !data_root.join(CATALOG_REL).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no instance at {} (missing {CATALOG_REL}); nothing to {act}",
                data_root.display()
            ),
        ));
    }
    Ok(data_root)
}

fn open_data(data_root: &Path) -> Result<Arc<dyn DataAdapter>, io::Error> {
    build_data_adapter(PlatformConfig::default().data_adapter, data_root)
        .map_err(|err| io::Error::other(err.message))
}

/// A platform refusal, carried out to the terminal with its message intact.
///
/// The code is dropped on purpose: these are the commands somebody runs when
/// nothing else works, and an error code beside prose they can act on is one
/// more thing to look up at the worst moment.
fn to_io(err: crate::platform::error::PlatformError) -> io::Error {
    io::Error::other(err.message)
}

/// Opens the catalog directly and rotates the credential.
fn reset_against(data_root: &Path, owner: &str) -> Result<String, io::Error> {
    let data = open_data(data_root)?;
    UserService::new(data)
        .reset_password_generated(owner)
        .map_err(to_io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::model::{DataAdapterKind, PlatformOfficeIdentityWrite, now_ts};

    fn temp_root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "zebflow-admin-{name}-{}-{}",
            std::process::id(),
            now_ts()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn write(data: &dyn DataAdapter, id: &str, owner: &str, action: &str, at: i64) {
        data.put_office_identity_write(&PlatformOfficeIdentityWrite {
            write_id: id.to_string(),
            office_id: "office-a".to_string(),
            owner: owner.to_string(),
            action: action.to_string(),
            role: "superadmin".to_string(),
            source: "controller-vouch".to_string(),
            detail: String::new(),
            written_at: at,
        })
        .expect("identity write");
    }

    /// FIX 2's regression for the guard half.
    ///
    /// The refusal used to page the newest 500 identity writes and scan them.
    /// Past that many, a controller-created account stopped being recognised as
    /// one — and `break-glass <owner>` would hand it a local password, turning a
    /// controller-asserted identity into a local one in the same keystroke as an
    /// unrelated repair. A decision must not be windowed.
    #[test]
    fn a_controller_created_account_is_refused_however_long_the_log_grows() {
        let root = temp_root("deep-identity-log");
        let data = build_data_adapter(DataAdapterKind::Sqlite, &root).expect("adapter");
        let now = now_ts();
        write(
            data.as_ref(),
            "created",
            "remote-admin",
            IDENTITY_WRITE_ACTION_CREATED,
            now,
        );

        // A local account, for contrast: it must stay eligible throughout.
        refuse_controller_created(data.as_ref(), "local-admin").expect("a local account is fine");
        let err = refuse_controller_created(data.as_ref(), "remote-admin")
            .expect_err("a controller-created account is refused");
        assert!(err.to_string().contains("reachable by vouch"), "{err}");

        for index in 0..1_500 {
            write(
                data.as_ref(),
                &format!("later-{index}"),
                "someone-else",
                "linked",
                now + 1 + index,
            );
        }

        let err = refuse_controller_created(data.as_ref(), "remote-admin")
            .expect_err("still refused past any window");
        assert!(err.to_string().contains("reachable by vouch"), "{err}");
        refuse_controller_created(data.as_ref(), "local-admin")
            .expect("a local account is still eligible");
        let _ = std::fs::remove_dir_all(root);
    }
}
