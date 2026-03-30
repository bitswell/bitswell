#!/bin/bash
set -euo pipefail

# bitswell startup — picks the next task and launches Claude in the background
# Usage: ./startup.sh [status]
#
# Tasks live in tasks/ as .md files. The file content is the prompt.
#   tasks/unassigned/  — waiting to be picked up
#   tasks/assigned/    — currently running
#   tasks/done/        — completed

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
LOG_DIR="$REPO_DIR/.runs"
TASK_DIR="$REPO_DIR/tasks"

# Show queue status
show_status() {
  echo "Task queue:"
  echo ""
  for state in unassigned assigned done; do
    count=0
    for f in "$TASK_DIR/$state"/*.md; do
      [ -f "$f" ] || continue
      count=$((count + 1))
    done
    echo "  $state: $count"
    for f in "$TASK_DIR/$state"/*.md; do
      [ -f "$f" ] || continue
      echo "    - $(basename "$f" .md)"
    done
  done
}

# Handle status command
if [ "${1:-}" = "status" ]; then
  show_status
  exit 0
fi

# Pick the next unassigned task (alphabetical order)
TASK_FILE=""
for f in "$TASK_DIR/unassigned"/*.md; do
  [ -f "$f" ] || continue
  TASK_FILE="$f"
  break
done

if [ -z "$TASK_FILE" ]; then
  echo "No unassigned tasks."
  echo ""
  show_status
  exit 0
fi

TASK_NAME=$(basename "$TASK_FILE" .md)
PROMPT=$(cat "$TASK_FILE")

# Move to assigned
mv "$TASK_FILE" "$TASK_DIR/assigned/$TASK_NAME.md"

mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/$TASK_NAME-$TIMESTAMP.json"

cd "$REPO_DIR"

# Run Claude, then move task to done (or back to unassigned on failure)
(
  if claude -p "$PROMPT" \
    --output-format json \
    --permission-mode acceptEdits \
    --max-budget-usd 5.00 \
    > "$LOG_FILE" 2>&1; then
    mv "$TASK_DIR/assigned/$TASK_NAME.md" "$TASK_DIR/done/$TASK_NAME.md"
  else
    mv "$TASK_DIR/assigned/$TASK_NAME.md" "$TASK_DIR/unassigned/$TASK_NAME.md"
    echo "FAILED (exit $?)" >> "$LOG_FILE"
  fi
) &

PID=$!
echo "bitswell is working: $TASK_NAME"
echo "  pid: $PID"
echo "  log: $LOG_FILE"
echo ""
echo "Check progress: tail -f $LOG_FILE"
echo "Check queue:    ./startup.sh status"
echo "Stop:           kill $PID"
