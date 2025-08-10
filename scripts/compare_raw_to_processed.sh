#!/usr/bin/env bash
set -euo pipefail

# Compare raw BLS inputs to processed outputs for one or more surveys.
#
# Usage:
#   bash scripts/compare_raw_to_processed.sh [SURVEY_CODE ...] [--details]
#   # If no SURVEY_CODE provided, runs across all directories in data/processed/*
#
# What it does:
# - Counts raw records (series/data/lookups) under data/raw/bls/<survey>/ recursively
# - Counts processed records in data/processed/<survey>/{series,observations,lookups}.csv
# - Prints a per-survey summary and PASS/FAIL markers for count parity
# - With --details, samples a few series_ids from processed series.csv and checks presence in raw series
#
# Notes:
# - Raw count logic subtracts 1 header line per file (typical BLS format)
# - Lookups combine .area/.item/.footnote/.period files into one processed lookups.csv
# - Differences may occur if rows are filtered/invalid; this script highlights, you decide if acceptable

DETAILS=false
SURVEYS=()
for arg in "$@"; do
  if [[ "$arg" == "--details" ]]; then
    DETAILS=true
  else
    SURVEYS+=("$arg")
  fi
done

proc_base="data/processed"
raw_base="data/raw/bls"

if [[ ${#SURVEYS[@]} -eq 0 ]]; then
  if [[ -d "$proc_base" ]]; then
    mapfile -t SURVEYS < <(ls -1 "$proc_base")
  else
    echo "[ERROR] $proc_base not found" >&2
    exit 1
  fi
fi

shopt -s nullglob nocaseglob

# Count records as number of lines excluding header, robust to missing trailing newlines
count_records_minus_header() {
  local file="$1"
  if [[ ! -f "$file" ]]; then echo 0; return; fi
  awk 'NR>1{c++} END{print c+0}' "$file"
}

# Sum records for files matching any of the provided -name patterns
sum_counts_for_names() {
  local total=0
  local name
  for name in "$@"; do
    while IFS= read -r -d '' f; do
      local c
      c=$(count_records_minus_header "$f")
      total=$((total + c))
    done < <(find . -type f -name "$name" -print0 2>/dev/null || true)
  done
  echo "$total"
}

# Pretty printer
hr() { local cols; cols=$(tput cols 2>/dev/null || echo 80); printf '%*s\n' "$cols" '' | tr ' ' '-'; }

TS=$(date +%Y-%m-%dT%H:%M:%S)

echo "[INFO] Raw vs Processed comparison at ${TS}"
hr

overall_fail=0

for survey in "${SURVEYS[@]}"; do
  code_up=$(echo "$survey" | tr '[:lower:]' '[:upper:]')
  code_lo=$(echo "$survey" | tr '[:upper:]' '[:lower:]')
  proc_dir="$proc_base/$code_up"
  raw_dir="$raw_base/$code_lo"

  if [[ ! -d "$proc_dir" ]]; then
    echo "[WARN] Processed dir missing: $proc_dir (skipping)"
    continue
  fi

  echo "Survey: $code_up"

  # Processed counts
  p_series=0; p_obs=0; p_lookups=0
  [[ -f "$proc_dir/series.csv" ]] && p_series=$(count_records_minus_header "$proc_dir/series.csv")
  [[ -f "$proc_dir/observations.csv" ]] && p_obs=$(count_records_minus_header "$proc_dir/observations.csv")
  [[ -f "$proc_dir/lookups.csv" ]] && p_lookups=$(count_records_minus_header "$proc_dir/lookups.csv")

  # Raw counts (search with find -name to cover nested structures)
  if [[ -d "$raw_dir" ]]; then
    pushd "$raw_dir" >/dev/null
    r_series=$(sum_counts_for_names "*.series")
    r_obs=$(sum_counts_for_names "*.data*")
    r_lookups=$(sum_counts_for_names "*.area*" "*.item*" "*.footnote*" "*.period*")
    popd >/dev/null
  else
    echo "  [WARN] Raw dir missing: $raw_dir"
    r_series=0; r_obs=0; r_lookups=0
  fi

  # Print summary
  printf "  Raw vs Processed counts:\n"
  printf "    Series       : %10d raw | %10d processed | %s\n" "$r_series" "$p_series" "$([[ $r_series -eq $p_series ]] && echo PASS || echo FAIL)"
  printf "    Observations : %10d raw | %10d processed | %s\n" "$r_obs" "$p_obs" "$([[ $r_obs -eq $p_obs ]] && echo PASS || echo FAIL)"
  printf "    Lookups      : %10d raw | %10d processed | %s\n" "$r_lookups" "$p_lookups" "$([[ $r_lookups -eq $p_lookups ]] && echo PASS || echo FAIL)"

  # Track failures for exit code
  if [[ $r_series -ne $p_series || $r_obs -ne $p_obs || $r_lookups -ne $p_lookups ]]; then
    overall_fail=1
  fi

  # Optional details: sample 5 series IDs and verify existence in raw .series
  if $DETAILS && [[ -f "$proc_dir/series.csv" && -d "$raw_dir" ]]; then
    echo "  [Details] Sampling series_id presence in raw .series files:"
    # Extract series_id from first column header-normally named 'series_id'
    mapfile -t sample_ids < <(awk -F'\t|,' 'NR>1 {print $1}' "$proc_dir/series.csv" | shuf -n 5 2>/dev/null || (tail -n +2 "$proc_dir/series.csv" | awk -F'\t|,' '{print $1}' | head -n 5))
    for sid in "${sample_ids[@]}"; do
      sid_trim=$(echo "$sid" | tr -d '"\r')
      if [[ -z "$sid_trim" ]]; then continue; fi
      if grep -Rqs -- "$sid_trim" "$raw_dir"/*.series "$raw_dir"/*/*.series 2>/dev/null; then
        echo "    ✓ $sid_trim found in raw series"
      else
        echo "    ✗ $sid_trim NOT found in raw series"
        overall_fail=1
      fi
    done
  fi

  hr
 done

if [[ $overall_fail -eq 0 ]]; then
  echo "[RESULT] All compared surveys show matching counts and sampled IDs present."
else
  echo "[RESULT] Discrepancies found. Review the above differences; this may be acceptable based on filtering rules."
fi

# Exit non-zero if discrepancies to help CI/manual attention
exit $overall_fail
