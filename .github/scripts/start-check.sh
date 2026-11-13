#!/usr/bin/env bash
# Starts a built zebflow binary with an empty data folder and waits until /health answers.
#   start-check.sh native <binary>          on the build machine itself (macOS, Windows)
#   start-check.sh docker <image> <binary>  inside an older Linux, with only what a normal
#                                           install has (OpenSSL 3, git), so a binary that needs a
#                                           newer C library than the image has fails here
# On failure it prints the log and exits non-zero.
set -euo pipefail

mode=$1
shift
port=10699
# A throwaway admin password for this one start, from random bytes.
password=$(head -c 18 /dev/urandom | od -An -tx1 | tr -d ' \n')
work=$(mktemp -d)
log="$work/start.log"
limit=240
# A JavaScript heap shared out unevenly: on these machines each runtime gets
# between V8's 64 MB minimum and its code-area maximum, and not a whole number
# of pages, which once stopped a 12-core machine from starting. A binary that
# starts here starts on any machine's 'auto' share.
mkdir -p "$work/data/platform"
printf '{"javascript":{"heap":"1000MB"},"runs":{"record_queue":"auto"}}' > "$work/data/platform/settings.json"

if [[ "$mode" == "native" ]]; then
  bin=$1
  data="$work/data"
  if command -v cygpath >/dev/null 2>&1; then data=$(cygpath -w "$data"); fi
  ZEBFLOW_PLATFORM_HOST=127.0.0.1 ZEBFLOW_PLATFORM_PORT=$port ZEBFLOW_PLATFORM_DATA_DIR="$data" \
    ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$password" "$bin" >"$log" 2>&1 &
  pid=$!
  running() { kill -0 "$pid" 2>/dev/null; }
  stop() { kill "$pid" 2>/dev/null || true; }
  show() { cat "$log"; }
elif [[ "$mode" == "docker" ]]; then
  image=$1
  bin=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
  cid=$(docker run -d -p "127.0.0.1:$port:$port" -v "$bin:/zebflow:ro" -v "$work/data:/data" \
    -e ZEBFLOW_PLATFORM_HOST=0.0.0.0 -e ZEBFLOW_PLATFORM_PORT=$port -e ZEBFLOW_PLATFORM_DATA_DIR=/data \
    -e ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$password" "$image" sh -c \
    'apt-get update -qq >/dev/null && apt-get install -y -qq --no-install-recommends libssl3 ca-certificates git >/dev/null && exec /zebflow')
  running() { [[ "$(docker inspect -f '{{.State.Running}}' "$cid" 2>/dev/null)" == "true" ]]; }
  stop() { docker rm -f "$cid" >/dev/null 2>&1 || true; }
  show() { docker logs "$cid" 2>&1 || true; }
else
  echo "usage: start-check.sh native <binary> | docker <image> <binary>" >&2
  exit 2
fi

for ((i = 1; i <= limit; i++)); do
  if curl -fsS "http://127.0.0.1:$port/health" >/dev/null 2>&1; then
    echo "zebflow answered /health after ${i}s"
    stop
    exit 0
  fi
  if ! running; then
    echo "zebflow exited before answering /health; its output:"
    show
    stop
    exit 1
  fi
  sleep 1
done
echo "zebflow did not answer /health within ${limit}s; its output:"
show
stop
exit 1
