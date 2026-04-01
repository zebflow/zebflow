#!/usr/bin/env bash
# Kill whatever is on port 10610, then cargo run on that port.
PORT=10610

export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="admin123"

pid=$(lsof -ti tcp:$PORT)
if [ -n "$pid" ]; then
  echo "Killing PID $pid on port $PORT..."
  kill -9 $pid
  sleep 0.5
fi

echo "Starting zebflow on port $PORT..."
exec cargo run --bin zebflow_platform
