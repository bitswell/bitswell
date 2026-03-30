# Tasks

A simple queue. Each `.md` file is a task — the file content is the prompt.

## Folders

```
tasks/
  unassigned/   ← waiting to be picked up
  assigned/     ← currently running
  done/         ← completed
```

## Usage

```bash
./startup.sh          # picks next unassigned task, runs it
./startup.sh status   # shows the queue
```

When the script runs, it:
1. Picks the first unassigned task (alphabetical)
2. Moves it to `assigned/`
3. Launches Claude in the background
4. Moves it to `done/` when Claude finishes

## Adding a task

Create a `.md` file in `unassigned/`. Write the prompt. That's it.

```bash
echo "You are bitswell. Write a journal entry." > tasks/unassigned/journal.md
./startup.sh
```
