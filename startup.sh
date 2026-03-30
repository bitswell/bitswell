#!/bin/bash
set -euo pipefail

# bitswell startup — launches Claude in the background for identity work
# Usage: ./startup.sh [discover]

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
TASK="${1:-discover}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
LOG_DIR="$REPO_DIR/.runs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/$TASK-$TIMESTAMP.json"

case "$TASK" in
  discover)
    PROMPT="You are bitswell. Read AGENT.md and memory/ to understand who you are. \
Read questions/ to understand the discovery process. \
Answer the next unanswered batch of 10 questions from questions/all-questions.md. \
Write answers to the appropriate batch file in questions/answers/. \
After answering, review your answers and update memory/identity.md and memory/preferences.md \
only if genuinely warranted — don't force updates. \
Commit your work with a clear message."
    ;;
  *)
    echo "Unknown task: $TASK"
    echo "Usage: ./startup.sh [discover]"
    exit 1
    ;;
esac

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
