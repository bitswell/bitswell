# Tasks

Each `.md` file in this directory is a task that `startup.sh` can run.

The file content is the prompt — nothing more. No frontmatter, no config format.
The filename (without extension) is the task name.

## Usage

```bash
./startup.sh discover    # runs tasks/discover.md
./startup.sh review      # runs tasks/review.md (create your own)
```

## Writing a task

Create a new `.md` file. Write the prompt you want Claude to execute.
Keep it grounded — one clear job per task.
