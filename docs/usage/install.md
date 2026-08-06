# Install

Most users should start with `npm` or `pip`.

## npm

Use this when you already have Node.js:

```bash
npm install -g zebflow
zebflow
```

Open:

```text
http://localhost:10610/login
```

## pip

Use this when you already have Python:

```bash
pip install zebflow
zebflow
```

Open:

```text
http://localhost:10610/login
```

## Docker

Use Docker for repeatable local or server deployments:

```bash
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$(openssl rand -base64 32)"
docker run --name zebflow \
  -p 10610:10610 \
  -v zebflow-data:/var/lib/zebflow/data \
  -e ZEBFLOW_PLATFORM_DEFAULT_PASSWORD \
  zebflow/zebflow:latest
```

## Source

Use this when you are developing Zebflow itself:

```bash
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$(openssl rand -base64 32)"
cargo run --bin zebflow
```
