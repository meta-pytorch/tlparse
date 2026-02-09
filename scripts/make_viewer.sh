#!/bin/bash
# make_viewer.sh - Create a standalone viewer HTML from a tlparse output directory.
#
# Usage:
#   ./scripts/make_viewer.sh <output_dir>
#
# This injects raw.jsonl and compile_directory.json into the viewer HTML template
# so it works with the file:// protocol (no server needed). Just open viewer.html.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
TEMPLATE="$REPO_DIR/frontend/dist/index.html"

if [ $# -lt 1 ]; then
    echo "Usage: $0 <tlparse_output_dir>"
    echo ""
    echo "Creates viewer.html in the output directory by injecting raw.jsonl and"
    echo "compile_directory.json into the viewer template. The resulting file works"
    echo "offline (file:// protocol)."
    exit 1
fi

OUTPUT_DIR="$1"
RAW_JSONL="$OUTPUT_DIR/raw.jsonl"
COMPILE_DIR="$OUTPUT_DIR/compile_directory.json"
VIEWER="$OUTPUT_DIR/viewer.html"

if [ ! -f "$TEMPLATE" ]; then
    echo "Error: Viewer template not found at $TEMPLATE"
    echo "Build it first: cd frontend && npm run build"
    exit 1
fi

if [ ! -f "$RAW_JSONL" ]; then
    echo "Error: raw.jsonl not found in $OUTPUT_DIR"
    exit 1
fi

if [ ! -f "$COMPILE_DIR" ]; then
    echo "Error: compile_directory.json not found in $OUTPUT_DIR"
    exit 1
fi

# Use python to do the replacement (handles multiline content correctly)
python3 -c "
import sys
template = open(sys.argv[1]).read()
raw_jsonl = open(sys.argv[2]).read()
compile_dir = open(sys.argv[3]).read()
# Escape </ to prevent breaking out of script tag
raw_jsonl = raw_jsonl.replace('</','<\\\\/')
compile_dir = compile_dir.replace('</','<\\\\/')
result = template.replace('__RAW_JSONL__', raw_jsonl)
result = result.replace('__COMPILE_DIRECTORY__', compile_dir)
open(sys.argv[4], 'w').write(result)
" "$TEMPLATE" "$RAW_JSONL" "$COMPILE_DIR" "$VIEWER"

echo "Created $VIEWER"
echo "Open it in your browser: open $VIEWER"
