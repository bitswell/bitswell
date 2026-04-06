//! Integration tests for LOOM dispatch and hook scripts.
//!
//! Tests `loom/bin/loom-dispatch`, `loom/bin/loom-spawn`, and
//! `loom/scripts/{agent-start,agent-stop,worktree-create,worktree-remove}.sh`
//! against real git repos in temporary directories.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Returns the repo root (parent of the `tests/` directory).
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("tests/ should be inside the repo root")
        .to_path_buf()
}

/// Returns the path to a loom bin script.
fn loom_bin(name: &str) -> PathBuf {
    repo_root().join("loom").join("bin").join(name)
}

/// Returns the path to a loom hook script.
fn loom_script(name: &str) -> PathBuf {
    repo_root().join("loom").join("scripts").join(name)
}

/// Creates a fresh git repo in a temp dir. Returns the temp dir path.
fn create_test_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    run_git(tmp.path(), &["init", "--initial-branch=main"]);
    run_git(tmp.path(), &["config", "user.email", "test@test.com"]);
    run_git(tmp.path(), &["config", "user.name", "Test"]);
    // Create an initial commit so we have something to branch from
    run_git(
        tmp.path(),
        &["commit", "--allow-empty", "-m", "initial commit"],
    );
    tmp
}

/// Run a git command in the given directory. Panics on failure.
fn run_git(dir: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to run git {:?}: {}", args, e));
    if !output.status.success() {
        panic!(
            "git {:?} failed:\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    output
}


/// Creates a loom/* branch with an ASSIGNED commit in the test repo.
fn create_assigned_branch(dir: &Path, branch: &str, agent: &str, assignment: &str) {
    run_git(dir, &["checkout", "-b", branch]);
    let msg = format!(
        "task(loom): assignment for {agent}\n\n\
         Implement the thing.\n\n\
         Agent-Id: bitswell\n\
         Session-Id: test-session-001\n\
         Task-Status: ASSIGNED\n\
         Assigned-To: {agent}\n\
         Assignment: {assignment}\n\
         Scope: src/\n\
         Budget: 30"
    );
    run_git(dir, &["commit", "--allow-empty", "-m", &msg]);
    run_git(dir, &["checkout", "main"]);
}

/// Creates a loom/* branch that has progressed past ASSIGNED.
fn create_completed_branch(dir: &Path, branch: &str, agent: &str, assignment: &str) {
    create_assigned_branch(dir, branch, agent, assignment);
    run_git(dir, &["checkout", branch]);
    let impl_msg = format!(
        "chore(loom): implementing\n\n\
         Agent-Id: {agent}\n\
         Session-Id: test-session-001\n\
         Task-Status: IMPLEMENTING\n\
         Heartbeat: 2026-04-06T00:00:00Z"
    );
    run_git(dir, &["commit", "--allow-empty", "-m", &impl_msg]);
    let done_msg = format!(
        "task(loom): done\n\n\
         Agent-Id: {agent}\n\
         Session-Id: test-session-001\n\
         Task-Status: COMPLETED\n\
         Key-Finding: it worked\n\
         Files-Changed: src/lib.rs\n\
         Heartbeat: 2026-04-06T00:01:00Z"
    );
    run_git(dir, &["commit", "--allow-empty", "-m", &done_msg]);
    run_git(dir, &["checkout", "main"]);
}

/// Run a script with JSON on stdin and return its output.
fn run_script_with_json(script: &Path, json_input: &str, cwd: &Path) -> Output {
    use std::io::Write;
    let mut child = Command::new("bash")
        .arg(script)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", script, e));

    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(json_input.as_bytes())
            .expect("failed to write to stdin");
    }
    drop(child.stdin.take());

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {:?}: {}", script, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── loom-dispatch tests ─────────────────────────────────────────────

    #[test]
    fn dispatch_scan_finds_assigned_branches() {
        let repo = create_test_repo();
        create_assigned_branch(repo.path(), "loom/ratchet-fix", "ratchet", "ratchet/fix");
        create_assigned_branch(
            repo.path(),
            "loom/moss-migrate",
            "moss",
            "moss/migrate",
        );

        let output = Command::new("bash")
            .args([loom_bin("loom-dispatch").to_str().unwrap(), "--scan", "--dry-run"])
            .current_dir(repo.path())
            .output()
            .expect("failed to run loom-dispatch");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "dispatch --scan failed: {}", stdout);
        assert!(
            stdout.contains("loom/ratchet-fix"),
            "should find ratchet branch in:\n{}",
            stdout
        );
        assert!(
            stdout.contains("loom/moss-migrate"),
            "should find moss branch in:\n{}",
            stdout
        );
        assert!(
            stdout.contains("[DRY RUN]"),
            "should show dry run marker in:\n{}",
            stdout
        );
    }

    #[test]
    fn dispatch_scan_skips_completed_branches() {
        let repo = create_test_repo();
        create_completed_branch(repo.path(), "loom/ratchet-done", "ratchet", "ratchet/done");
        create_assigned_branch(
            repo.path(),
            "loom/moss-pending",
            "moss",
            "moss/pending",
        );

        let output = Command::new("bash")
            .args([loom_bin("loom-dispatch").to_str().unwrap(), "--scan", "--dry-run"])
            .current_dir(repo.path())
            .output()
            .expect("failed to run loom-dispatch");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success());
        // Only the pending branch should be found
        assert!(
            stdout.contains("loom/moss-pending"),
            "should find pending branch in:\n{}",
            stdout
        );
        // The completed branch should not trigger dispatch
        assert!(
            !stdout.contains("ratchet-done") || !stdout.contains("[DRY RUN] Would spawn ratchet"),
            "should not dispatch completed branch"
        );
    }

    #[test]
    fn dispatch_scan_none_found() {
        let repo = create_test_repo();
        // No loom branches at all

        let output = Command::new("bash")
            .args([loom_bin("loom-dispatch").to_str().unwrap(), "--scan", "--dry-run"])
            .current_dir(repo.path())
            .output()
            .expect("failed to run loom-dispatch");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success());
        assert!(
            stdout.contains("None found"),
            "should report none found in:\n{}",
            stdout
        );
    }

    #[test]
    fn dispatch_branch_mode_dry_run() {
        let repo = create_test_repo();
        create_assigned_branch(repo.path(), "loom/ratchet-task", "ratchet", "ratchet/task");

        let output = Command::new("bash")
            .args([
                loom_bin("loom-dispatch").to_str().unwrap(),
                "--branch",
                "loom/ratchet-task",
                "--dry-run",
            ])
            .current_dir(repo.path())
            .output()
            .expect("failed to run loom-dispatch");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "dispatch --branch failed:\nstdout: {}\nstderr: {}",
            stdout, String::from_utf8_lossy(&output.stderr));
        assert!(
            stdout.contains("Agent:        ratchet"),
            "should show agent in:\n{}",
            stdout
        );
        assert!(
            stdout.contains("[DRY RUN]"),
            "should be dry run in:\n{}",
            stdout
        );
    }

    #[test]
    fn dispatch_requires_mode_flag() {
        let repo = create_test_repo();

        let output = Command::new("bash")
            .arg(loom_bin("loom-dispatch").to_str().unwrap())
            .current_dir(repo.path())
            .output()
            .expect("failed to run loom-dispatch");

        assert!(
            !output.status.success(),
            "should fail without mode flag"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("specify --commit, --branch, or --scan"),
            "should show usage hint in:\n{}",
            stderr
        );
    }

    // ── loom-spawn tests ────────────────────────────────────────────────

    #[test]
    fn spawn_requires_prompt_file() {
        let repo = create_test_repo();

        let output = Command::new("bash")
            .arg(loom_spawn_path().to_str().unwrap())
            .current_dir(repo.path())
            .output()
            .expect("failed to run loom-spawn");

        assert!(!output.status.success(), "should fail without prompt file");
    }

    #[test]
    fn spawn_rejects_missing_prompt_file() {
        let repo = create_test_repo();

        let output = Command::new("bash")
            .args([
                loom_spawn_path().to_str().unwrap(),
                "/nonexistent/prompt.txt",
            ])
            .current_dir(repo.path())
            .output()
            .expect("failed to run loom-spawn");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("prompt file not found"),
            "should report missing file in:\n{}",
            stderr
        );
    }

    #[test]
    fn spawn_rejects_non_git_directory() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let prompt = tmp.path().join("prompt.txt");
        std::fs::write(&prompt, "test prompt").expect("write prompt");

        let output = Command::new("bash")
            .args([loom_spawn_path().to_str().unwrap(), prompt.to_str().unwrap()])
            .current_dir(tmp.path())
            // Override CLAUDE_CMD to something that won't be reached
            .env("CLAUDE_CMD", "echo")
            .output()
            .expect("failed to run loom-spawn");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not inside a git worktree"),
            "should reject non-git dir in:\n{}",
            stderr
        );
    }

    fn loom_spawn_path() -> PathBuf {
        loom_bin("loom-spawn")
    }

    // ── Hook script tests ────────��──────────────────────────────────────

    #[test]
    fn worktree_create_accepts_valid_loom_branch() {
        let repo = create_test_repo();
        let json = r#"{"branch": "loom/ratchet-fix"}"#;

        let output = run_script_with_json(
            &loom_script("worktree-create.sh"),
            json,
            repo.path(),
        );

        assert!(
            output.status.success(),
            "should accept valid branch:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("validated"), "should confirm validation");
    }

    #[test]
    fn worktree_create_rejects_invalid_loom_branch() {
        let repo = create_test_repo();
        let json = r#"{"branch": "loom/UPPERCASE-BAD"}"#;

        let output = run_script_with_json(
            &loom_script("worktree-create.sh"),
            json,
            repo.path(),
        );

        // Exit code 2 = block creation
        assert_eq!(
            output.status.code(),
            Some(2),
            "should exit 2 to block invalid branch:\nstderr: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Invalid branch name"),
            "should report invalid name in:\n{}",
            stderr
        );
    }

    #[test]
    fn worktree_create_ignores_non_loom_branch() {
        let repo = create_test_repo();
        let json = r#"{"branch": "feature/something"}"#;

        let output = run_script_with_json(
            &loom_script("worktree-create.sh"),
            json,
            repo.path(),
        );

        assert!(output.status.success(), "should pass through non-loom branch");
        // Should produce no LOOM output
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("LOOM"),
            "should be silent for non-loom branch"
        );
    }

    #[test]
    fn worktree_create_ignores_empty_branch() {
        let repo = create_test_repo();
        let json = r#"{}"#;

        let output = run_script_with_json(
            &loom_script("worktree-create.sh"),
            json,
            repo.path(),
        );

        assert!(output.status.success(), "should pass through empty branch");
    }

    #[test]
    fn worktree_remove_reports_completed_branch() {
        let repo = create_test_repo();
        create_completed_branch(repo.path(), "loom/ratchet-done", "ratchet", "ratchet/done");

        let json = r#"{"branch": "loom/ratchet-done", "path": "/tmp/test-worktree"}"#;
        let output = run_script_with_json(
            &loom_script("worktree-remove.sh"),
            json,
            repo.path(),
        );

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Terminal status confirmed"),
            "should confirm terminal status in:\n{}",
            stdout
        );
        assert!(
            stdout.contains("COMPLETED"),
            "should report COMPLETED in:\n{}",
            stdout
        );
    }

    #[test]
    fn worktree_remove_warns_non_terminal() {
        let repo = create_test_repo();
        create_assigned_branch(
            repo.path(),
            "loom/moss-stuck",
            "moss",
            "moss/stuck",
        );
        // Branch is in ASSIGNED state, not terminal
        run_git(repo.path(), &["checkout", "loom/moss-stuck"]);
        run_git(
            repo.path(),
            &[
                "commit",
                "--allow-empty",
                "-m",
                "chore(loom): implementing\n\nAgent-Id: moss\nSession-Id: s1\nTask-Status: IMPLEMENTING\nHeartbeat: 2026-04-06T00:00:00Z",
            ],
        );
        run_git(repo.path(), &["checkout", "main"]);

        let json = r#"{"branch": "loom/moss-stuck", "path": "/tmp/test-worktree"}"#;
        let output = run_script_with_json(
            &loom_script("worktree-remove.sh"),
            json,
            repo.path(),
        );

        assert!(output.status.success(), "should still exit 0 (non-blockable)");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Non-terminal Task-Status"),
            "should warn about non-terminal state in:\n{}",
            stderr
        );
    }

    #[test]
    fn worktree_remove_ignores_non_loom_branch() {
        let repo = create_test_repo();
        let json = r#"{"branch": "feature/something"}"#;

        let output = run_script_with_json(
            &loom_script("worktree-remove.sh"),
            json,
            repo.path(),
        );

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("LOOM"),
            "should be silent for non-loom branch"
        );
    }

    #[test]
    fn agent_start_logs_spawn() {
        let repo = create_test_repo();
        let json = r#"{"agent_id": "ratchet", "session_id": "sess-001"}"#;

        let output = run_script_with_json(
            &loom_script("agent-start.sh"),
            json,
            repo.path(),
        );

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SubagentStart"),
            "should log start event in:\n{}",
            stdout
        );
        assert!(
            stdout.contains("Agent-Id=ratchet"),
            "should include agent id in:\n{}",
            stdout
        );
    }

    #[test]
    fn agent_stop_confirms_completed() {
        let repo = create_test_repo();
        create_completed_branch(repo.path(), "loom/ratchet-done", "ratchet", "ratchet/done");

        // Create a worktree for the branch so cwd-based detection works
        let wt_path = repo.path().join("wt-done");
        run_git(
            repo.path(),
            &[
                "worktree",
                "add",
                wt_path.to_str().unwrap(),
                "loom/ratchet-done",
            ],
        );

        let json = format!(
            r#"{{"agent_id": "ratchet", "session_id": "sess-001", "cwd": "{}"}}"#,
            wt_path.display()
        );
        let output = run_script_with_json(
            &loom_script("agent-stop.sh"),
            &json,
            repo.path(),
        );

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Task-Status confirmed: COMPLETED"),
            "should confirm COMPLETED in:\n{}",
            stdout
        );
    }

    #[test]
    fn agent_stop_warns_no_trailer() {
        let repo = create_test_repo();
        // Create a loom branch with no Task-Status trailer
        run_git(repo.path(), &["checkout", "-b", "loom/moss-empty"]);
        run_git(
            repo.path(),
            &["commit", "--allow-empty", "-m", "chore: no trailers here"],
        );

        let wt_path = repo.path().join("wt-empty");
        // Need to go back to main before adding worktree
        run_git(repo.path(), &["checkout", "main"]);
        run_git(
            repo.path(),
            &[
                "worktree",
                "add",
                wt_path.to_str().unwrap(),
                "loom/moss-empty",
            ],
        );

        let json = format!(
            r#"{{"agent_id": "moss", "session_id": "sess-002", "cwd": "{}"}}"#,
            wt_path.display()
        );
        let output = run_script_with_json(
            &loom_script("agent-stop.sh"),
            &json,
            repo.path(),
        );

        assert!(output.status.success(), "should still exit 0 (warn only)");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("no Task-Status trailer"),
            "should warn about missing trailer in:\n{}",
            stderr
        );
    }

    #[test]
    fn agent_stop_ignores_non_loom_branch() {
        let repo = create_test_repo();
        // CWD on main branch
        let json = r#"{"agent_id": "ratchet", "session_id": "s1"}"#;

        let output = run_script_with_json(
            &loom_script("agent-stop.sh"),
            json,
            repo.path(),
        );

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("SubagentStop"),
            "should be silent for non-loom branch"
        );
    }

    // ── hooks.json structure test ───────────────────────────────────────

    #[test]
    fn hooks_json_valid_structure() {
        let hooks_path = repo_root().join("loom").join("hooks").join("hooks.json");
        let content = std::fs::read_to_string(&hooks_path)
            .unwrap_or_else(|e| panic!("cannot read hooks.json: {}", e));

        // Parse as JSON
        let value: serde_json::Value =
            serde_json::from_str(&content).expect("hooks.json is not valid JSON");

        // Required top-level fields
        assert!(value.get("hooks").is_some(), "must have 'hooks' key");
        assert!(value.get("version").is_some(), "must have 'version' key");

        let hooks = value["hooks"].as_object().expect("hooks should be an object");

        // All four hook events should be defined
        for event in &[
            "WorktreeCreate",
            "WorktreeRemove",
            "SubagentStart",
            "SubagentStop",
        ] {
            assert!(
                hooks.contains_key(*event),
                "hooks.json should define {} event",
                event
            );
        }
    }

    // ── Script executable permissions ───────────────────────────────────

    #[test]
    fn bin_scripts_are_executable() {
        use std::os::unix::fs::PermissionsExt;

        for name in &["loom-dispatch", "loom-spawn"] {
            let path = loom_bin(name);
            assert!(path.exists(), "{} should exist", name);
            let mode = std::fs::metadata(&path)
                .unwrap_or_else(|e| panic!("cannot stat {}: {}", name, e))
                .permissions()
                .mode();
            assert!(
                mode & 0o111 != 0,
                "{} should be executable (mode: {:o})",
                name,
                mode
            );
        }
    }

    #[test]
    fn hook_scripts_are_executable() {
        use std::os::unix::fs::PermissionsExt;

        for name in &[
            "agent-start.sh",
            "agent-stop.sh",
            "worktree-create.sh",
            "worktree-remove.sh",
        ] {
            let path = loom_script(name);
            assert!(path.exists(), "{} should exist", name);
            let mode = std::fs::metadata(&path)
                .unwrap_or_else(|e| panic!("cannot stat {}: {}", name, e))
                .permissions()
                .mode();
            assert!(
                mode & 0o111 != 0,
                "{} should be executable (mode: {:o})",
                name,
                mode
            );
        }
    }
}
