#!/usr/bin/env bash
# Kill whatever is on port 10610, then cargo run on that port.
PORT=10610

export ZEBFLOW_PLATFORM_PORT="$PORT"
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="admin123"

# The data root is no longer relative to the working directory: unset, the
# binary uses the OS user data path (interface.md §5). This repo's instance is
# explicit configuration like everything else in this file, and keeping it here
# is what stops `./dev.sh` from sharing a data root with an installed zebflow.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export ZEBFLOW_PLATFORM_DATA_DIR="${ZEBFLOW_PLATFORM_DATA_DIR:-$REPO_ROOT/.zebflow-platform-data}"

pid=$(lsof -ti tcp:$PORT)
if [ -n "$pid" ]; then
  echo "Killing PID $pid on port $PORT..."
  kill -9 $pid
  sleep 0.5
fi

echo "Starting zebflow on port $PORT..."
exec cargo run --bin zebflow
