# Installation

Zebflow is one executable. Choose one installation method, start the server,
then open the browser. You can set the first admin password yourself or let
Zebflow generate one.

## npm

```bash
npm install -g zebflow
zebflow
```

## pip

```bash
pip install zebflow
zebflow
```

## Docker

```bash
docker run -p 10610:10610 \
  -v zebflow-data:/var/lib/zebflow/data \
  zebflow/zebflow:latest
```

The volume keeps projects and platform data after the container stops.

## Source

```bash
cargo build
./target/debug/zebflow
```

## Open Zebflow

The default address is:

```text
http://127.0.0.1:10610/login
```

The default owner is `superadmin`. To choose its initial password, set
`ZEBFLOW_PLATFORM_DEFAULT_PASSWORD` before the first run. When the variable is
missing, Zebflow generates a password and prints the path to:

```text
<platform-data-directory>/.bootstrap/superadmin-password
```

On Unix systems, the directory uses mode `0700` and the password file uses mode
`0600`. Read the password, store it securely, and remove the file when it is no
longer needed. Restarting Zebflow never replaces an existing superadmin
password.

## Where the data goes

Unset, `ZEBFLOW_PLATFORM_DATA_DIR` defaults to the OS user data path —
`~/.local/share/zebflow`, `~/Library/Application Support/Zebflow`, or
`%LOCALAPPDATA%\Zebflow` — never a path relative to the working directory.
Set it explicitly for anything you intend to keep. Two instances must never
share a data directory: it holds an embedded database.

## Environment

| Variable | Default | Meaning |
|---|---|---|
| `ZEBFLOW_PLATFORM_HOST` / `_PORT` | `127.0.0.1` / `10610` | where the server listens |
| `ZEBFLOW_PLATFORM_DATA_DIR` | OS user data path | the data root |
| `ZEBFLOW_PLATFORM_BASE_URL` | derived from request headers | external base URL for OAuth redirects and MCP session URLs |
| `ZEBFLOW_HEALTH_PORT` / `_HOST` | unset | a dedicated liveness listener (see deployment) |
| `ZEBFLOW_PLATFORM_DEFAULT_OWNER` / `_PROJECT` | `superadmin` / `default` | created on first boot |
| `ZEBFLOW_PLATFORM_DEFAULT_PASSWORD` | generated | the owner's first password |
| `ZEBFLOW_PLATFORM_ALLOW_INSECURE_DEFAULT_PASSWORD` | unset | `1` permits the literal password `secret`, for disposable local runs only |
| `ZEBFLOW_COOKIE_SECURE` | on unless the host is loopback | force the `Secure` cookie attribute |
| `ZEBFLOW_SECRET_ROTATION_EPOCH` | unset | Unix timestamp; platform-issued tokens older than it stop working |
| `ZEBFLOW_HUB_DEFAULT_BASE_URL` | `https://hub.zebflow.com/api` | the default hub |

`zebflow --help` prints the same list, including the cluster variables an
office or controller needs.

For a disposable local test, keep the defaults. For a server: a strong
password, an explicit durable data directory, TLS at the proxy, backups.
