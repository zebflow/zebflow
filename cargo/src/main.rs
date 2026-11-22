//! `cargo install zebflow` installs this launcher, not Zebflow's source: the
//! first run downloads the ready-made Zebflow program for this computer from
//! the GitHub release of the same version, checks it against the release's
//! checksums, keeps it under `~/.zebflow/bin/<version>/`, and runs it. Every
//! later run starts it directly. The download uses the system's own `curl`
//! and `tar` (on every Mac, Linux and Windows 10 or later), so the launcher
//! has no dependencies.

use std::path::{Path, PathBuf};
use std::process::{Command, exit};

const REPO: &str = "zebflow/zebflow";

fn fail(message: &str) -> ! {
    eprintln!("[zebflow] {message}");
    exit(1);
}

/// The release asset and the program's file name for this computer.
fn asset() -> (&'static str, &'static str) {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => ("zebflow-linux-amd64.tar.gz", "zebflow"),
        ("linux", "aarch64") => ("zebflow-linux-arm64.tar.gz", "zebflow"),
        ("macos", "aarch64") => ("zebflow-darwin-arm64.tar.gz", "zebflow"),
        ("windows", "x86_64") => ("zebflow-windows-amd64.zip", "zebflow.exe"),
        (os, arch) => fail(&format!(
            "no ready-made Zebflow for {os}-{arch}; supported: linux x86_64 and aarch64, macos aarch64, windows x86_64. \
             To build it yourself: https://github.com/{REPO}"
        )),
    }
}

fn version() -> String {
    std::env::var("ZEBFLOW_VERSION")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")))
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| fail("cannot find the home folder (HOME / USERPROFILE)"))
}

fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program).args(args).status().map(|status| status.success()).unwrap_or(false)
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// The SHA-256 of a file, through whichever tool this system has.
fn sha256(file: &Path) -> Option<String> {
    let path = file.to_str()?;
    let first_word = |text: String| text.split_whitespace().next().map(str::to_lowercase);
    if cfg!(windows) {
        // certutil prints a header line, the hash, then a footer line.
        let text = output("certutil", &["-hashfile", path, "SHA256"])?;
        return text.lines().nth(1).map(|line| line.replace(' ', "").to_lowercase());
    }
    output("sha256sum", &[path]).and_then(first_word).or_else(|| output("shasum", &["-a", "256", path]).and_then(first_word))
}

fn install(dir: &Path, version: &str, asset: &str, program: &str) -> PathBuf {
    let base = format!("https://github.com/{REPO}/releases/download/{version}");
    let target = dir.join(program);
    std::fs::create_dir_all(dir).unwrap_or_else(|err| fail(&format!("cannot create {}: {err}", dir.display())));
    let archive = dir.join(asset);
    let sums = dir.join("zebflow-checksums.txt");
    let archive_s = archive.to_string_lossy().to_string();
    let sums_s = sums.to_string_lossy().to_string();

    eprintln!("[zebflow] downloading {asset} ({version}), about 70 MB, once");
    if !run("curl", &["-fsSL", "-o", &archive_s, &format!("{base}/{asset}")]) {
        fail(&format!("could not download {base}/{asset} (is curl installed, and does release {version} exist?)"));
    }
    if run("curl", &["-fsSL", "-o", &sums_s, &format!("{base}/zebflow-checksums.txt")]) {
        let expected = std::fs::read_to_string(&sums)
            .ok()
            .and_then(|text| text.lines().find(|line| line.ends_with(asset)).and_then(|line| line.split_whitespace().next().map(str::to_lowercase)));
        match (expected, sha256(&archive)) {
            (Some(expected), Some(actual)) if expected == actual => eprintln!("[zebflow] checksum verified"),
            (Some(_), Some(_)) => {
                let _ = std::fs::remove_file(&archive);
                fail("the download does not match the release's checksum; nothing was installed");
            }
            _ => eprintln!("[zebflow] warning: could not verify the checksum (no sha256 tool or no checksum line)"),
        }
        let _ = std::fs::remove_file(&sums);
    }

    let dir_s = dir.to_string_lossy().to_string();
    let flags = if asset.ends_with(".zip") { "-xf" } else { "-xzf" };
    if !run("tar", &[flags, &archive_s, "-C", &dir_s]) {
        fail("could not unpack the download (tar)");
    }
    let _ = std::fs::remove_file(&archive);
    if !target.exists() {
        fail(&format!("{} is missing after unpacking", target.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
    }
    eprintln!("[zebflow] installed {}", target.display());
    target
}

fn main() {
    let (asset, program) = asset();
    let version = version();
    let dir = home().join(".zebflow").join("bin").join(&version);
    let target = dir.join(program);
    let target = if target.exists() { target } else { install(&dir, &version, asset, program) };
    let status = Command::new(&target)
        .args(std::env::args_os().skip(1))
        .status()
        .unwrap_or_else(|err| fail(&format!("could not start {}: {err}", target.display())));
    exit(status.code().unwrap_or(1));
}
