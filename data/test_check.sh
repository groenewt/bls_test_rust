#!/usr/bin/env bash

# List your surveys here (no duplicates unless you really want them)
SURVEY_LIST=" AP  BD  BP  CB  CC  CF  CU    EB    EE  EI  EP  FI  FM  GP  II  IN  IP  IS  JL  JT  LA  LE  LN  LU  NB  NC  NW  OE  OR  PD  SM  SU  WD  WM  WP"

printf "%-10s %-12s %-12s %-10s %-10s %-10s\n" "Survey" "raw_size" "FINAL_SIZE" "Savings%" "Files" "Rows"
printf "%-10s %-12s %-12s %-10s %-10s %-10s\n" "------" "--------" "----------" "--------" "-----" "----"

for s in $SURVEY_LIST; do
  raw_dir="raw/bls/${s,,}" # lowercase for raw
  final_dir="final/${s^^}" # uppercase for final (no /bls)

  # Skip if both directories are missing
  if [ ! -d "$raw_dir" ] && [ ! -d "$final_dir" ]; then
    echo "Skipping $s (no raw or final directory)"
    continue
  fi

  # Get raw/final sizes (bytes)
  raw_size_bytes=$(du -sb "$raw_dir" 2>/dev/null | awk '{print $1}')
  final_size_bytes=$(du -sb "$final_dir" 2>/dev/null | awk '{print $1}')

  # Default to 0 if missing
  raw_size_bytes=${raw_size_bytes:-0}
  final_size_bytes=${final_size_bytes:-0}

  # Calculate savings %
  if [ "$raw_size_bytes" -gt 0 ]; then
    savings=$(awk -v r="$raw_size_bytes" -v f="$final_size_bytes" 'BEGIN { printf "%.1f", ((r - f) / r) * 100 }')
  else
    savings="N/A"
  fi

  # Human-readable sizes
  raw_h=$(numfmt --to=iec --suffix=B "$raw_size_bytes")
  final_h=$(numfmt --to=iec --suffix=B "$final_size_bytes")

  # Count parquet files
  file_count=$(find "$final_dir" -type f -name "*.parquet" 2>/dev/null | wc -l)

  # Count rows if possible
  if command -v parquet-tools >/dev/null; then
    row_count=$(find "$final_dir" -type f -name "*.parquet" -exec parquet-tools rowcount {} \; 2>/dev/null | awk '{sum += $1} END {print sum}')
  elif python3 -c "import pyarrow" 2>/dev/null; then
    row_count=$(
      python3 - <<EOF
import pyarrow.parquet as pq
import pathlib
total = 0
for f in pathlib.Path("$final_dir").rglob("*.parquet"):
    try:
        total += pq.read_table(f).num_rows
    except:
        pass
print(total)
EOF
    )
  else
    row_count="N/A"
  fi

  printf "%-10s %-12s %-12s %-10s %-10s %-10s\n" "$s" "$raw_h" "$final_h" "$savings" "$file_count" "$row_count"
done
