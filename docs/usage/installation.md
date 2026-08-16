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

## Change the Address

Use `ZEBFLOW_PLATFORM_HOST` to change the listen address and
`ZEBFLOW_PLATFORM_PORT` to change the port.

For a disposable local test, keep the default host. For a server, use a strong
password, a durable data directory, TLS at the proxy, and regular backups.
