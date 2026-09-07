#!/usr/bin/env bash
# Kill whatever is on the chosen port, then cargo run on it.
#
#   ./dev.sh          the usual instance on 10610
#   ./dev.sh 10620    a second instance, isolated, for working in parallel
#
# A second instance needs its own store: two processes over one embedded
# database is not a thing you get away with. So any port other than the default
# gets its own data directory, and the default keeps the one you already have.
PORT="${1:-${ZEBFLOW_PLATFORM_PORT:-10610}}"
DEFAULT_PORT=10610

export ZEBFLOW_PLATFORM_PORT="$PORT"
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="admin123"

# The data root is no longer relative to the working directory: unset, the
# binary uses the OS user data path (interface.md §5). This repo's instance is
# explicit configuration like everything else in this file, and keeping it here
# is what stops `./dev.sh` from sharing a data root with an installed zebflow.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ "$PORT" = "$DEFAULT_PORT" ]; then
  DEFAULT_DATA_DIR="$REPO_ROOT/.zebflow-platform-data"
else
  DEFAULT_DATA_DIR="$REPO_ROOT/.zebflow-platform-data-$PORT"
fi
export ZEBFLOW_PLATFORM_DATA_DIR="${ZEBFLOW_PLATFORM_DATA_DIR:-$DEFAULT_DATA_DIR}"

pid=$(lsof -ti tcp:$PORT)
if [ -n "$pid" ]; then
  echo "Killing PID $pid on port $PORT..."
  kill -9 $pid
  sleep 0.5
fi

echo "Starting zebflow on port $PORT (data: $ZEBFLOW_PLATFORM_DATA_DIR)..."
exec cargo run --bin zebflow
