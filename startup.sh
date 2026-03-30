#!/bin/bash
set -euo pipefail

# bitswell startup — launches Claude in the background for identity work
# Usage: ./startup.sh <task>
# Tasks are defined as .md files in tasks/ — the file content is the prompt.

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
TASK="${1:-}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
LOG_DIR="$REPO_DIR/.runs"
TASK_DIR="$REPO_DIR/tasks"

# Show available tasks if none specified or task not found
show_usage() {
  echo "Usage: ./startup.sh <task>"
  echo ""
  echo "Available tasks:"
  for f in "$TASK_DIR"/*.md; do
    [ -f "$f" ] || continue
    name=$(basename "$f" .md)
    [ "$name" = "README" ] && continue
    echo "  $name"
  done
  exit 1
}

[ -z "$TASK" ] && show_usage

TASK_FILE="$TASK_DIR/$TASK.md"

if [ ! -f "$TASK_FILE" ]; then
  echo "Unknown task: $TASK"
  echo "No file found at tasks/$TASK.md"
  echo ""
  show_usage
fi

PROMPT=$(cat "$TASK_FILE")

mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/$TASK-$TIMESTAMP.json"

cd "$REPO_DIR"

claude -p "$PROMPT" \
  --output-format json \
  --permission-mode acceptEdits \
  --max-budget-usd 5.00 \
  > "$LOG_FILE" 2>&1 &

PID=$!
echo "bitswell is working: $TASK"
echo "  pid: $PID"
echo "  log: $LOG_FILE"
echo ""
echo "Check progress: tail -f $LOG_FILE"
echo "Stop: kill $PID"
