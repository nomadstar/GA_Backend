#!/usr/bin/env bash
set -euo pipefail

# Sync quickshift into serverless/quickshift excluding build artifacts and git data
SRC_DIR="$(dirname "$0")/../quickshift"
DST_DIR="$(dirname "$0")/quickshift"

echo "Source: $SRC_DIR"
echo "Destination: $DST_DIR"

if [ ! -d "$SRC_DIR" ]; then
  echo "ERROR: source directory does not exist: $SRC_DIR" >&2
  exit 2
fi

# Use rsync if available for a safe copy; otherwise use cp -a
if command -v rsync >/dev/null 2>&1; then
  rsync -av --delete --exclude 'target' --exclude '.git' --exclude 'node_modules' --exclude '*.lock' "$SRC_DIR/" "$DST_DIR/"
else
  echo "rsync not found, falling back to cp -a" >&2
  mkdir -p "$DST_DIR"
  # copy everything except common large dirs using tar to preserve permissions
  (cd "$SRC_DIR" && tar --exclude='./target' --exclude='./.git' --exclude='./node_modules' -cf - .) | (cd "$DST_DIR" && tar -xf -)
fi

echo "Sync complete."