#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   bash scripts/run_e2e_dags_random.sh [COUNT]
#
# Defaults:
#   COUNT = 5
#   ENV   = $ENV or dev
#
# This script builds the release binary and runs the DAG pipeline for COUNT random surveys.
# It captures logs under logs/ and prints where processed outputs are written.

COUNT=15
ENVIRONMENT=${ENV:-dev}

echo "[INFO] Building release binary..."
cargo build --release

TS=$(date +%Y%m%d_%H%M%S)
mkdir -p logs
LOG_FILE="logs/run_e2e_dags_random_${TS}.log"

echo "[INFO] Running DAGs for ${COUNT} random surveys (env=${ENVIRONMENT})..."
set -o pipefail
./target/release/rusty process-dags-random --count "${COUNT}" --env "${ENVIRONMENT}" | tee "${LOG_FILE}"

RC=${PIPESTATUS[0]:-0}

if [ "${RC}" -eq 0 ]; then
  echo "[INFO] Completed successfully. Logs saved to ${LOG_FILE}"
  echo "[INFO] Processed outputs are under data/processed/<SURVEY>/"
  echo "       Files: series.csv, observations.csv, lookups.csv"
else
  echo "[ERROR] One or more DAG runs failed. See ${LOG_FILE} for details." >&2
fi

exit "${RC}"
