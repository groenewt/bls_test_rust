#!/usr/bin/env bash
set -euo pipefail

# Append standard settings block to any DAG files missing a top-level `settings:` section

BLOCK="

# Global DAG settings
settings:
  max_active_runs: 1
  catchup: false
  depends_on_past: false
  email_on_failure: true
  email_on_retry: false
  retries: 1
  retry_delay_minutes: 5
"

# Find dags.yml files missing a top-level settings: key and append the block
missing_list=$(grep -L '^settings:' config/surveys/*/dags.yml || true)

if [ -n "$missing_list" ]; then
  while IFS= read -r file; do
    printf "%s" "$BLOCK" >> "$file"
    echo "[ensure_dag_settings] Appended settings block to: $file"
  done <<< "$missing_list"
else
  echo "[ensure_dag_settings] No dags.yml files missing settings block."
fi
