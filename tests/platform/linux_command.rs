//! `linux.command.run` against a real SSH server (`docs/contracts/linux-control.md`):
//! a private `sshd` on a high port, for the user running the test. Skipped where
//! no `sshd` or `ssh-keygen` is installed, or when the test runs as root.

use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use serde_json::{Value, json};

use zebflow::pipeline::nodes::basic::linux::command::{Config, Node};
use zebflow::pipeline::nodes::{NodeExecutionInput, NodeHandler};
use zebflow::platform::model::UpsertProjectCredentialRequest;
use zebflow::platform::{PlatformConfig, PlatformService};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";

struct Server {
    child: Child,
    dir: std::path::PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn keygen(path: &std::path::Path) {
    let status = Command::new("ssh-keygen").args(["-q", "-t", "ed25519", "-N", "", "-f"]).arg(path).status().expect("ssh-keygen");
    assert!(status.success());
}

/// A private sshd on a free port, accepting `client` for the current user.
fn start_server(dir: &std::path::Path) -> Option<(Server, u16, String)> {
    let sshd = ["/usr/sbin/sshd", "/usr/bin/sshd"].into_iter().find(|p| std::path::Path::new(p).exists())?;
    keygen(&dir.join("host"));
    keygen(&dir.join("client"));
    std::fs::copy(dir.join("client.pub"), dir.join("authorized_keys")).expect("authorized_keys");
    let port = std::net::TcpListener::bind("127.0.0.1:0").ok()?.local_addr().ok()?.port();
    let config = format!(
        "Port {port}\nListenAddress 127.0.0.1\nHostKey {d}/host\nAuthorizedKeysFile {d}/authorized_keys\nPidFile {d}/sshd.pid\n\
         PasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nStrictModes no\nPermitRootLogin no\n",
        d = dir.display()
    );
    std::fs::write(dir.join("sshd_config"), config).expect("config");
    let child = Command::new(sshd).args(["-D", "-e", "-f"]).arg(dir.join("sshd_config")).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()?;
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            let host_key = std::fs::read_to_string(dir.join("host.pub")).ok()?;
            let host_key = host_key.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
            return Some((Server { child, dir: dir.to_path_buf() }, port, host_key));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    None
}

async fn run(node: &Node) -> Result<Value, String> {
    let input = NodeExecutionInput {
        node_id: "n0".to_string(),
        input_pin: "in".to_string(),
        payload: json!({}),
        metadata: json!({ "owner": OWNER, "project": PROJECT, "pipeline": "test", "request_id": "r1" }),
        bus: None,
    };
    node.execute_async(input).await.map(|out| out.payload["command"].clone()).map_err(|err| format!("{}: {}", err.code, err.message))
}

#[tokio::test]
async fn a_command_runs_on_a_real_machine_pinned_by_its_host_key() {
    let user = std::env::var("USER").unwrap_or_default();
    if user.is_empty() || user == "root" || Command::new("ssh-keygen").arg("-?").stderr(Stdio::null()).status().is_err() {
        eprintln!("skipped: needs ssh-keygen and a non-root user");
        return;
    }
    let dir = std::env::temp_dir().join(format!("zebflow-linux-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    let Some((server, port, host_key)) = start_server(&dir) else {
        let _ = std::fs::remove_dir_all(&dir);
        eprintln!("skipped: no sshd could start");
        return;
    };
    let private_key = std::fs::read_to_string(dir.join("client")).unwrap();

    let mut config = PlatformConfig::default();
    config.data_root = dir.join("data");
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let save = |id: &str, secret: Value| {
        platform.credentials.upsert_project_credential(
            OWNER,
            PROJECT,
            &UpsertProjectCredentialRequest { credential_id: id.to_string(), title: id.to_string(), kind: "ssh".to_string(), secret, notes: String::new() },
        )
    };
    let secret = json!({ "host": "127.0.0.1", "port": port.to_string(), "user": user, "private_key": private_key, "host_key": host_key });
    save("machine", secret.clone()).expect("an ssh credential");
    let mut listed = secret.clone();
    listed["commands"] = json!(["echo", "cat"]);
    save("listed", listed).expect("a credential with commands");
    let mut wrong_key = secret.clone();
    wrong_key["host_key"] = json!(std::fs::read_to_string(dir.join("client.pub")).unwrap().split_whitespace().take(2).collect::<Vec<_>>().join(" "));
    save("impostor", wrong_key).expect("a credential pinned to another key");
    let mut root = secret.clone();
    root["user"] = json!("root");
    assert!(save("as-root", root).is_err(), "a root login is refused when saved");

    let node = |credential: &str, run: &str, stdin: Option<&str>, timeout: Option<&str>| {
        Node::new(
            Config { credential_id: credential.to_string(), run: run.to_string(), stdin: stdin.map(str::to_string), timeout: timeout.map(str::to_string), max_output: None },
            platform.credentials.clone(),
        )
        .expect("node")
    };

    // It ran: what it printed, how it exited.
    let answer = run(&node("machine", "echo hello; echo oops >&2; exit 3", None, None)).await.expect("ran");
    assert_eq!((answer["ok"].clone(), answer["exit_code"].clone()), (json!(false), json!(3)), "{answer}");
    assert_eq!(answer["stdout"], "hello\n");
    assert_eq!(answer["stderr"], "oops\n");

    // A file written from the payload, read back.
    let file = dir.join("written.txt");
    let answer = run(&node("machine", &format!("cat > {}", file.display()), Some("from a pipeline\n"), None)).await.expect("written");
    assert_eq!(answer["ok"], json!(true), "{answer}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "from a pipeline\n");

    // A listed program runs; anything else, or a chained second command, is refused.
    let answer = run(&node("listed", "echo listed", None, None)).await.expect("listed program");
    assert_eq!(answer["stdout"], "listed\n");
    for line in ["ls", "echo a; ls", "echo $(id)"] {
        let err = run(&node("listed", line, None, None)).await.expect_err(line);
        assert!(err.starts_with("FW_NODE_LINUX_COMMAND_RUN_REFUSED"), "{line}: {err}");
    }

    // A machine answering with another host key is refused.
    let err = run(&node("impostor", "echo never", None, None)).await.expect_err("host key");
    assert!(err.contains("host key") && err.starts_with("FW_NODE_LINUX_COMMAND_RUN_REFUSED"), "{err}");

    // The time limit ends the run.
    let err = run(&node("machine", "sleep 5", None, Some("1s"))).await.expect_err("timeout");
    assert!(err.contains("--timeout"), "{err}");

    // Nothing of the key is left behind on this machine.
    let leftovers = std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("zebflow-ssh-"))
        .count();
    assert_eq!(leftovers, 0, "the private folder for the key is removed after every run");
    drop(server);
}
