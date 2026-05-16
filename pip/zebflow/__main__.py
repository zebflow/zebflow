"""CLI entry point for zebflow. Downloads the binary on first run, then executes it."""

import os
import platform
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

REPO = "zebflow/zebflow"
VERSION = os.environ.get("ZEBFLOW_VERSION", f"v{__import__('zebflow').__version__}")
BIN_DIR = Path(__file__).parent / "bin"


def get_platform():
    system = platform.system().lower()
    machine = platform.machine().lower()

    if machine in ("x86_64", "amd64"):
        arch = "amd64"
    elif machine in ("aarch64", "arm64"):
        arch = "arm64"
    else:
        print(f"[zebflow] Unsupported architecture: {machine}", file=sys.stderr)
        sys.exit(1)

    platform_map = {
        ("linux", "amd64"): ("zebflow-linux-amd64.tar.gz", "zebflow"),
        ("linux", "arm64"): ("zebflow-linux-arm64.tar.gz", "zebflow"),
        ("darwin", "arm64"): ("zebflow-darwin-arm64.tar.gz", "zebflow"),
        ("windows", "amd64"): ("zebflow-windows-amd64.zip", "zebflow.exe"),
    }

    key = (system, arch)
    entry = platform_map.get(key)
    if not entry:
        print(f"[zebflow] Unsupported platform: {system}-{arch}", file=sys.stderr)
        print(f"[zebflow] Supported: {', '.join(f'{s}-{a}' for s, a in platform_map)}", file=sys.stderr)
        sys.exit(1)

    return entry


def download_binary():
    asset, binary_name = get_platform()
    binary_path = BIN_DIR / binary_name

    if binary_path.exists():
        return binary_path

    url = f"https://github.com/{REPO}/releases/download/{VERSION}/{asset}"
    print(f"[zebflow] Downloading {asset} ({VERSION})...")

    BIN_DIR.mkdir(parents=True, exist_ok=True)

    with tempfile.NamedTemporaryFile(delete=False, suffix=asset) as tmp:
        tmp_path = Path(tmp.name)
        urllib.request.urlretrieve(url, tmp_path)

    try:
        if asset.endswith(".tar.gz"):
            with tarfile.open(tmp_path, "r:gz") as tar:
                tar.extractall(path=BIN_DIR)
        elif asset.endswith(".zip"):
            with zipfile.ZipFile(tmp_path, "r") as z:
                z.extractall(path=BIN_DIR)
    finally:
        tmp_path.unlink(missing_ok=True)

    if not binary_path.exists():
        print(f"[zebflow] Binary not found after extraction: {binary_path}", file=sys.stderr)
        sys.exit(1)

    binary_path.chmod(0o755)
    print(f"[zebflow] Installed to {binary_path}")
    return binary_path


def main():
    binary_path = download_binary()
    result = subprocess.run([str(binary_path)] + sys.argv[1:])
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
