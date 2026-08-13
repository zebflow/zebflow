# Installation

Zebflow is one executable. Choose one installation method, set the first admin
password, start the server, then open the browser.

## npm

```bash
npm install -g zebflow
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="choose-a-strong-password"
zebflow
```

## pip

```bash
pip install zebflow
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="choose-a-strong-password"
zebflow
```

## Docker

```bash
docker run -p 10610:10610 \
  -e ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="choose-a-strong-password" \
  -v zebflow-data:/var/lib/zebflow/data \
  zebflow/zebflow:latest
```

The volume keeps projects and platform data after the container stops.

## Source

```bash
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="choose-a-strong-password"
cargo build
./target/debug/zebflow
```

## Open Zebflow

The default address is:

```text
http://127.0.0.1:10610/login
```

The default owner is `superadmin`. The password is the value of
`ZEBFLOW_PLATFORM_DEFAULT_PASSWORD`.

## Change the Address

Use `ZEBFLOW_PLATFORM_HOST` to change the listen address and
`ZEBFLOW_PLATFORM_PORT` to change the port.

For a disposable local test, keep the default host. For a server, use a strong
password, a durable data directory, TLS at the proxy, and regular backups.
