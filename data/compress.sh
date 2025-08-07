#!/usr/bin/env bash
set -euo pipefail

# ============================================================================
# Comprehensive Directory Compression Script
# Compresses subdirectories with maximum XZ compression
# ============================================================================

# Configuration
SOURCE_DIR="./raw/bls"
OUTPUT_DIR="compressed_parts"
FINAL_ARCHIVE="compressed_parts_bundle.zip"
LOG_FILE="compression_$(date +%Y%m%d_%H%M%S).log"
USE_PROGRESS=true

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ============================================================================
# Functions
# ============================================================================

log_message() {
    local message="$1"
    echo -e "$message"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $message" >> "$LOG_FILE"
}

error_exit() {
    log_message "${RED}[ERROR] $1${NC}"
    exit 1
}

check_dependencies() {
    local missing_deps=()
    
    # Check for required commands
    for cmd in tar xz zip basename du; do
        if ! command -v "$cmd" &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done
    
    # Check for optional pv command (for progress bars)
    if command -v pv &> /dev/null; then
        USE_PROGRESS=true
        log_message "${GREEN}[✓] Progress bars available (pv installed)${NC}"
    else
        log_message "${YELLOW}[!] Install 'pv' for progress bars: sudo apt-get install pv${NC}"
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        error_exit "Missing required dependencies: ${missing_deps[*]}"
    fi
}

get_size_human() {
    local bytes="$1"
    if command -v numfmt &> /dev/null; then
        numfmt --to=iec-i --suffix=B "$bytes"
    else
        echo "$((bytes / 1048576))MB"
    fi
}

compress_directory() {
    local dir="$1"
    local name="$2"
    local output_file="${OUTPUT_DIR}/${name}.tar.xz"
    
    if [ "$USE_PROGRESS" = true ] && command -v pv &> /dev/null; then
        local dir_size=$(du -sb "$dir" 2>/dev/null | awk '{print $1}')
        tar -cf - "$dir" | pv -s "$dir_size" -N "$name" | xz -9e -T0 > "$output_file"
    else
        tar -cf - "$dir" | xz -9e -T0 > "$output_file"
    fi
    
    # Calculate and log compression ratio
    local original_size=$(du -sb "$dir" 2>/dev/null | awk '{print $1}')
    local compressed_size=$(stat -c%s "$output_file" 2>/dev/null || stat -f%z "$output_file" 2>/dev/null)
    
    if [ -n "$original_size" ] && [ -n "$compressed_size" ] && [ "$original_size" -gt 0 ]; then
        local ratio=$(echo "scale=2; 100 - ($compressed_size * 100 / $original_size)" | bc 2>/dev/null || echo "N/A")
        log_message "   Compressed: $(get_size_human $original_size) → $(get_size_human $compressed_size) (${ratio}% reduction)"
    fi
}

# ============================================================================
# Main Script
# ============================================================================

# Initialize logging
log_message "${BLUE}========================================${NC}"
log_message "${BLUE}Directory Compression Script Started${NC}"
log_message "${BLUE}========================================${NC}"

# Check dependencies
check_dependencies

# Verify source directory exists
if [ ! -d "$SOURCE_DIR" ]; then
    error_exit "Source directory '$SOURCE_DIR' not found!"
fi

# Count subdirectories
dir_count=$(find "$SOURCE_DIR" -maxdepth 1 -type d -not -path "$SOURCE_DIR" | wc -l)
if [ "$dir_count" -eq 0 ]; then
    error_exit "No subdirectories found in '$SOURCE_DIR'"
fi

log_message "${GREEN}[✓] Found $dir_count subdirectories to compress${NC}"

# Check if output directory exists and handle cleanup
if [ -d "$OUTPUT_DIR" ]; then
    log_message "${YELLOW}[!] Output directory '$OUTPUT_DIR' already exists${NC}"
    read -p "Do you want to remove it? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$OUTPUT_DIR"
        log_message "${GREEN}[✓] Removed existing output directory${NC}"
    else
        error_exit "Cannot proceed with existing output directory"
    fi
fi

# Check if final archive exists
if [ -f "$FINAL_ARCHIVE" ]; then
    log_message "${YELLOW}[!] Final archive '$FINAL_ARCHIVE' already exists${NC}"
    read -p "Do you want to overwrite it? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -f "$FINAL_ARCHIVE"
        log_message "${GREEN}[✓] Removed existing final archive${NC}"
    else
        # Create backup with timestamp
        backup_name="${FINAL_ARCHIVE%.zip}_backup_$(date +%Y%m%d_%H%M%S).zip"
        mv "$FINAL_ARCHIVE" "$backup_name"
        log_message "${GREEN}[✓] Backed up existing archive to: $backup_name${NC}"
    fi
fi

# Step 1: Create output directory
log_message "${BLUE}[1/3] Creating output directory...${NC}"
mkdir -p "$OUTPUT_DIR"

# Step 2: Compress each subdirectory
log_message "${BLUE}[2/3] Compressing individual directories...${NC}"

# Track statistics
total_original_size=0
total_compressed_size=0
current_dir=0

for dir in "$SOURCE_DIR"/*; do
    [ -d "$dir" ] || continue  # skip non-directories
    
    current_dir=$((current_dir + 1))
    name=$(basename "$dir")
    
    log_message "${YELLOW}[$current_dir/$dir_count] Compressing: $name${NC}"
    
    # Get original size for statistics
    dir_size=$(du -sb "$dir" 2>/dev/null | awk '{print $1}' || echo 0)
    total_original_size=$((total_original_size + dir_size))
    
    # Compress the directory
    compress_directory "$dir" "$name"
    
    # Get compressed size for statistics
    compressed_file="${OUTPUT_DIR}/${name}.tar.xz"
    if [ -f "$compressed_file" ]; then
        file_size=$(stat -c%s "$compressed_file" 2>/dev/null || stat -f%z "$compressed_file" 2>/dev/null || echo 0)
        total_compressed_size=$((total_compressed_size + file_size))
        log_message "${GREEN}   ✓ Completed: ${name}.tar.xz${NC}"
    fi
done

log_message "${GREEN}[✓] All individual directories compressed${NC}"

# Step 3: Archive the compressed_parts folder
log_message "${BLUE}[3/3] Creating final ZIP bundle...${NC}"

# Change to parent directory to avoid including full path in zip
cd "$OUTPUT_DIR" 2>/dev/null || error_exit "Cannot change to output directory"

# Create ZIP archive with maximum compression (-9)
if zip -9 -r "../$FINAL_ARCHIVE" . ; then
    cd - > /dev/null
    log_message "${GREEN}[✓] ZIP bundle created successfully${NC}"
else
    cd - > /dev/null
    error_exit "Failed to create ZIP bundle"
fi

# Final statistics
log_message "${BLUE}========================================${NC}"
log_message "${GREEN}[✓] Compression Complete!${NC}"
log_message "${BLUE}========================================${NC}"

# Calculate overall statistics
if [ "$total_original_size" -gt 0 ]; then
    final_size=$(stat -c%s "$FINAL_ARCHIVE" 2>/dev/null || stat -f%z "$FINAL_ARCHIVE" 2>/dev/null || echo 0)
    
    log_message "${GREEN}Statistics:${NC}"
    log_message "  Original size:     $(get_size_human $total_original_size)"
    log_message "  Compressed parts:  $(get_size_human $total_compressed_size)"
    log_message "  Final bundle:      $(get_size_human $final_size)"
    
    if [ "$final_size" -gt 0 ]; then
        overall_ratio=$(echo "scale=2; 100 - ($final_size * 100 / $total_original_size)" | bc 2>/dev/null || echo "N/A")
        log_message "  Space saved:       ${overall_ratio}%"
    fi
fi

log_message ""
log_message "${GREEN}Output files:${NC}"
log_message "  Individual archives: ${OUTPUT_DIR}/"
log_message "  Final bundle:        ${FINAL_ARCHIVE}"
log_message "  Log file:           ${LOG_FILE}"

# Offer cleanup option
echo
read -p "Do you want to remove the individual compressed parts (keep only bundle)? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf "$OUTPUT_DIR"
    log_message "${GREEN}[✓] Removed individual compressed parts${NC}"
    log_message "Final archive available at: ${FINAL_ARCHIVE}"
else
    log_message "${GREEN}[✓] Keeping both individual parts and bundle${NC}"
fi

log_message "${BLUE}========================================${NC}"
log_message "${GREEN}Script completed successfully!${NC}"
