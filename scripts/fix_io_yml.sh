#!/usr/bin/env bash
set -euo pipefail

# Fix missing sections in config/surveys/*/io.yml based on AP reference
# Adds minimal defaults while preserving existing content.

append_block() {
  local file="$1"
  shift
  # Ensure file ends with a newline and a separator newline before appending
  if [ -s "$file" ] && [ -n "$(tail -c1 "$file" || true)" ]; then
    printf "\n" >> "$file"
  fi
  printf "\n%s\n" "$*" >> "$file"
}

for f in config/surveys/*/io.yml; do
  [ -f "$f" ] || continue
  dir="$(dirname "$f")"
  code="$(basename "$dir")"
  code_lc="${code,,}"

  # config_version
  if ! grep -q "^config_version:" "$f"; then
    append_block "$f" "config_version: 1"
  fi

  # discovery
  if ! grep -q "^discovery:" "$f"; then
    append_block "$f" \
"discovery:
  root: \"data/raw/bls/${code_lc}\"
  include: [\"${code_lc}.series\", \"${code_lc}.data.*\"]
  exclude: [\"*.tmp\", \\".*\\", \"*~\", \"*.bak\"]"
  fi

  # combining
  if ! grep -q "^combining:" "$f"; then
    append_block "$f" \
"combining:
  enabled: false
  strategy: \"none\"
  thresholds:
    small_file_mb: 10"
  fi

  # partition_hints
  if ! grep -q "^partition_hints:" "$f"; then
    append_block "$f" \
"partition_hints:
  strategy: \"size_based\"
  max_partition_size_mb: 256
  partition_by: []"
  fi

  # mmap
  if ! grep -q "^mmap:" "$f"; then
    append_block "$f" \
"mmap:
  enable_for_mb_greater_than: 100"
  fi

  # lookup_strategy
  if ! grep -q "^lookup_strategy:" "$f"; then
    append_block "$f" \
"# Lookup file loading strategy
lookup_strategy:
  preload: []
  lazy: []
  mmap: []
  cache_all: true"
  fi

done

echo "[fix_io_yml] Completed updating io.yml files."