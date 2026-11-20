//! The `ssh` credential and one run of the system `ssh` program
//! (`docs/contracts/linux-control.md`): the target is pinned by its host key,
//! the key lives in a private temporary folder for the one run, and the
//! program is started with an argument list, never through a local shell.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

/// The kind of credential the node reads.
pub const CREDENTIAL_KIND: &str = "ssh";

/// Characters that let a line run a second command; refused when the
/// credential lists its `commands`.
const CHAINING: &[&str] = &[";", "&", "|", "`", "$(", ">", "<", "\n", "\r"];

/// What an `ssh` credential holds, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub private_key: String,
    pub host_key: String,
    pub commands: Vec<String>,
    pub workdir: Option<String>,
}

fn text(secret: &Value, key: &str) -> String {
    match secret.get(key) {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn list(secret: &Value, key: &str) -> Vec<String> {
    let items: Vec<String> = match secret.get(key) {
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(text)) => text.split([',', '\n']).map(str::to_string).collect(),
        _ => Vec::new(),
    };
    items.into_iter().map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect()
}

/// A private key in the OpenSSH format, unencrypted: its decoded body starts
/// with the format's own marker, and the cipher it names is `none`. A key with
/// a passphrase could only be used by asking for it.
fn key_is_usable(key: &str) -> Result<(), String> {
    use base64::Engine as _;
    let not_openssh = || "the private key must be in the OpenSSH format (ssh-keygen -t ed25519)".to_string();
    let body: String = key
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .map(str::trim)
        .collect();
    let bytes = base64::engine::general_purpose::STANDARD.decode(body).map_err(|_| not_openssh())?;
    let magic = b"openssh-key-v1\0";
    if !bytes.starts_with(magic) || bytes.len() < magic.len() + 4 {
        return Err(not_openssh());
    }
    let at = magic.len();
    let len = u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize;
    match bytes.get(at + 4..at + 4 + len) {
        Some(b"none") => Ok(()),
        Some(_) => Err("the private key has a passphrase; use one without".to_string()),
        None => Err("the private key is not readable".to_string()),
    }
}

impl Target {
    /// Reads and checks an `ssh` credential's secret: a plain host, a port, a
    /// user that is not `root`, an unencrypted key and the pinned host key.
    pub fn from_secret(credential_id: &str, secret: &Value) -> Result<Self, String> {
        let refuse = |why: String| format!("credential '{credential_id}': {why}");
        let host = text(secret, "host");
        let host_ok = !host.is_empty()
            && !host.starts_with('-')
            && host.len() <= 253
            && host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
        if !host_ok {
            return Err(refuse("`host` must be a host name or address".to_string()));
        }
        let port = match text(secret, "port").as_str() {
            "" => 22,
            raw => raw.parse::<u16>().ok().filter(|port| *port > 0).ok_or_else(|| refuse("`port` must be a number from 1 to 65535".to_string()))?,
        };
        let user = text(secret, "user");
        let mut chars = user.chars();
        let user_ok = matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_')
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.'))
            && user.len() <= 32;
        if !user_ok {
            return Err(refuse("`user` must be a Linux user name".to_string()));
        }
        if user == "root" {
            return Err(refuse("`user` may not be root; give the machine an ordinary user".to_string()));
        }
        let private_key = match secret.get("private_key") {
            Some(Value::String(key)) => key.trim().to_string(),
            _ => String::new(),
        };
        key_is_usable(&private_key).map_err(refuse)?;
        let host_key = text(secret, "host_key");
        let mut parts = host_key.split_whitespace();
        let (kind, blob) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
        let kind_ok = kind == "ssh-ed25519" || kind == "ssh-rsa" || kind.starts_with("ecdsa-sha2-");
        if !kind_ok || blob.is_empty() || !blob.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')) {
            return Err(refuse("`host_key` must be the machine's public host key, such as `ssh-ed25519 AAAA…` (ssh-keyscan prints it)".to_string()));
        }
        let workdir = Some(text(secret, "workdir")).filter(|dir| !dir.is_empty());
        if let Some(dir) = &workdir
            && (!dir.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-')) || dir.split('/').any(|s| s == ".."))
        {
            return Err(refuse("`workdir` must be a plain path, relative to the user's home or absolute".to_string()));
        }
        let commands = list(secret, "commands");
        if let Some(bad) = commands.iter().find(|command| !command.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'))) {
            return Err(refuse(format!("`commands` entry '{bad}' must be a program name")));
        }
        Ok(Self {
            host,
            port,
            user,
            private_key,
            host_key: format!("{kind} {blob}"),
            commands,
            workdir,
        })
    }

    /// The line the machine's shell runs: refused when the credential lists
    /// its programs and the line starts with another or could chain one.
    pub fn line(&self, run: &str) -> Result<String, String> {
        let run = run.trim();
        if run.is_empty() {
            return Err("--run is empty".to_string());
        }
        if !self.commands.is_empty() {
            if let Some(found) = CHAINING.iter().find(|mark| run.contains(*mark)) {
                return Err(format!("this credential runs one listed program per line; '{}' is not allowed", found.escape_debug()));
            }
            let program = run.split_whitespace().next().unwrap_or_default();
            if !self.commands.iter().any(|allowed| allowed == program) {
                return Err(format!("'{program}' is not one of this credential's commands ({})", self.commands.join(", ")));
            }
        }
        Ok(match &self.workdir {
            Some(dir) => format!("cd '{dir}' && {run}"),
            None => run.to_string(),
        })
    }
}

/// What one run answered.
#[derive(Debug, Clone)]
pub struct Ran {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub truncated: bool,
    pub duration: Duration,
}

/// How a run failed before the command could answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The machine's host key is not the pinned one.
    HostKey,
    /// The connection, the login or the `ssh` program failed.
    Connect(String),
    /// The time limit passed.
    Timeout,
}

/// A private folder for one run, removed when it goes out of scope.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> std::io::Result<Self> {
        let dir = std::env::temp_dir().join(format!("zebflow-ssh-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self(dir))
    }

    fn write(&self, name: &str, text: &str) -> std::io::Result<PathBuf> {
        let path = self.0.join(name);
        std::fs::write(&path, text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Reads all of `reader`, keeping at most `max` bytes; the rest is read and
/// dropped so the program never blocks on a full pipe.
async fn capped(mut reader: impl AsyncRead + Unpin, max: usize) -> (Vec<u8>, bool) {
    let mut kept = Vec::new();
    let mut cut = false;
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let room = max.saturating_sub(kept.len());
                kept.extend_from_slice(&buffer[..read.min(room)]);
                cut |= read > room;
            }
        }
    }
    (kept, cut)
}

/// The arguments the system `ssh` is started with, every option fixed.
fn arguments(target: &Target, key: &Path, known_hosts: &Path, line: &str) -> Vec<String> {
    let option = |text: &str| ["-o".to_string(), text.to_string()];
    let mut args = vec!["-F".to_string(), "/dev/null".to_string(), "-i".to_string(), key.display().to_string()];
    for setting in [
        "IdentitiesOnly=yes".to_string(),
        "BatchMode=yes".to_string(),
        "StrictHostKeyChecking=yes".to_string(),
        format!("UserKnownHostsFile={}", known_hosts.display()),
        "GlobalKnownHostsFile=/dev/null".to_string(),
        "HostKeyAlias=zebflow-target".to_string(),
        "ConnectTimeout=10".to_string(),
        "ServerAliveInterval=15".to_string(),
        "ForwardAgent=no".to_string(),
        "ForwardX11=no".to_string(),
        "ClearAllForwardings=yes".to_string(),
        "RequestTTY=no".to_string(),
        "LogLevel=ERROR".to_string(),
    ] {
        args.extend(option(&setting));
    }
    args.extend(["-p".to_string(), target.port.to_string(), "-l".to_string(), target.user.clone(), "--".to_string(), target.host.clone(), line.to_string()]);
    args
}

/// Runs `line` on the machine. A command that ran answers whatever it
/// exited with; a refused host key, a failed connection or a timeout fails.
pub async fn run(target: &Target, line: &str, stdin: Option<&str>, timeout: Duration, max_output: usize) -> Result<Ran, Failure> {
    let scratch = Scratch::new().map_err(|err| Failure::Connect(format!("no private folder for the key: {err}")))?;
    let key = scratch
        .write("id", &format!("{}\n", target.private_key))
        .map_err(|err| Failure::Connect(format!("cannot write the key: {err}")))?;
    let known_hosts = scratch
        .write("known_hosts", &format!("zebflow-target {}\n", target.host_key))
        .map_err(|err| Failure::Connect(format!("cannot write the host key: {err}")))?;
    let started = Instant::now();
    let mut child = tokio::process::Command::new("ssh")
        .args(arguments(target, &key, &known_hosts, line))
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|err| Failure::Connect(format!("the ssh program could not start: {err}")))?;
    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let text = text.to_string();
        tokio::spawn(async move {
            let _ = pipe.write_all(text.as_bytes()).await;
            let _ = pipe.shutdown().await;
        });
    }
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    let work = async {
        let (out, err) = tokio::join!(capped(stdout, max_output), capped(stderr, max_output));
        let status = child.wait().await;
        (out, err, status)
    };
    let ((stdout, cut_out), (stderr, cut_err), status) = match tokio::time::timeout(timeout, work).await {
        Ok(done) => done,
        Err(_) => return Err(Failure::Timeout),
    };
    let exit_code = status.map_err(|err| Failure::Connect(err.to_string()))?.code().unwrap_or(-1);
    let said = String::from_utf8_lossy(&stderr).to_string();
    if exit_code == 255 && said.contains("Host key verification failed") {
        return Err(Failure::HostKey);
    }
    if exit_code == 255 && stdout.is_empty() {
        let reason = said.lines().filter(|line| !line.trim().is_empty()).last().unwrap_or("the connection failed").trim().to_string();
        return Err(Failure::Connect(reason));
    }
    Ok(Ran { exit_code, stdout, stderr, truncated: cut_out || cut_err, duration: started.elapsed() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// An OpenSSH key body's shape (`openssh-key-v1`, then the cipher name),
    /// built here so no key material is written into the source.
    fn plain_key(cipher: &str) -> String {
        use base64::Engine as _;
        let mut bytes = b"openssh-key-v1\0".to_vec();
        bytes.extend_from_slice(&(cipher.len() as u32).to_be_bytes());
        bytes.extend_from_slice(cipher.as_bytes());
        bytes.extend_from_slice(&[0; 32]);
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn secret() -> Value {
        json!({ "host": "compute-1.example.com", "user": "app", "private_key": plain_key("none"), "host_key": "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExample" })
    }

    #[test]
    fn a_credential_is_a_pinned_ordinary_user_with_an_unlocked_key() {
        let target = Target::from_secret("c", &secret()).expect("target");
        assert_eq!((target.port, target.user.as_str()), (22, "app"));
        for (key, value, word) in [
            ("user", json!("root"), "root"),
            ("user", json!("-oProxyCommand=x"), "Linux user"),
            ("host", json!("-oProxyCommand=x"), "host"),
            ("host", json!("a b"), "host"),
            ("port", json!("70000"), "port"),
            ("host_key", json!(""), "host_key"),
            ("host_key", json!("ssh-ed25519 AAAA;rm"), "host_key"),
            ("private_key", json!(plain_key("aes256-ctr")), "passphrase"),
            ("private_key", json!("bm90IGEga2V5IGF0IGFsbA=="), "OpenSSH format"),
            ("workdir", json!("/srv; rm"), "workdir"),
            ("workdir", json!("~/work"), "workdir"),
            ("workdir", json!("work/../.."), "workdir"),
            ("commands", json!(["ls -la"]), "program name"),
        ] {
            let mut bad = secret();
            bad[key] = value.clone();
            let err = Target::from_secret("c", &bad).expect_err(&format!("{key}: {value}"));
            assert!(err.contains(word), "{key}: {err}");
        }
    }

    #[test]
    fn a_listed_program_runs_alone() {
        let mut open = Target::from_secret("c", &secret()).expect("target");
        assert_eq!(open.line("ls -la | wc -l").unwrap(), "ls -la | wc -l", "no list: the user's own permissions decide");
        open.commands = vec!["bun".into(), "systemctl".into()];
        assert!(open.line("bun run build").is_ok());
        for line in ["rm -rf ~", "bun run x; rm -rf ~", "bun $(whoami)", "bun x > /etc/a", "bun x\nrm y", "bun `id`", "bun a && b"] {
            assert!(open.line(line).is_err(), "{line}");
        }
        open.workdir = Some("/srv/app".into());
        assert_eq!(open.line("bun test").unwrap(), "cd '/srv/app' && bun test");
    }

    #[test]
    fn ssh_is_started_with_every_option_fixed_and_the_host_last() {
        let target = Target::from_secret("c", &secret()).expect("target");
        let args = arguments(&target, Path::new("/k/id"), Path::new("/k/kh"), "echo hi");
        let joined = args.join(" ");
        for must in ["-F /dev/null", "IdentitiesOnly=yes", "BatchMode=yes", "StrictHostKeyChecking=yes", "ForwardAgent=no", "ClearAllForwardings=yes", "RequestTTY=no", "-l app"] {
            assert!(joined.contains(must), "{must}: {joined}");
        }
        assert_eq!(&args[args.len() - 3..], ["--", "compute-1.example.com", "echo hi"]);
    }
}
