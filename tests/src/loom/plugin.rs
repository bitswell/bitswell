//! Plugin structure validation tests for the LOOM Claude Code plugin.
//!
//! Validates that `loom/` has the correct directory layout, manifest schema,
//! agent definitions, skill references, hooks, and bin scripts. Also checks
//! for v1 remnants that should no longer exist.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Returns the repo root (parent of the `tests/` directory).
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("tests/ should be inside the repo root")
        .to_path_buf()
}

/// Returns the loom plugin root directory.
fn loom_root() -> PathBuf {
    repo_root().join("loom")
}

/// Read a file to string, panicking with a clear message on failure.
fn read_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Plugin manifest ─────────────────────────────────────────────────

    #[test]
    fn plugin_json_exists() {
        let path = loom_root().join(".claude-plugin").join("plugin.json");
        assert!(path.exists(), "plugin.json should exist at {:?}", path);
    }

    #[test]
    fn plugin_json_valid_json() {
        let path = loom_root().join(".claude-plugin").join("plugin.json");
        let content = read_file(&path);
        let _: serde_json::Value =
            serde_json::from_str(&content).expect("plugin.json should be valid JSON");
    }

    #[test]
    fn plugin_json_required_fields() {
        let path = loom_root().join(".claude-plugin").join("plugin.json");
        let content = read_file(&path);
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Required fields
        let name = value.get("name").and_then(|v| v.as_str());
        assert_eq!(name, Some("loom"), "plugin name should be 'loom'");

        let version = value.get("version").and_then(|v| v.as_str());
        assert!(
            version.is_some() && !version.unwrap().is_empty(),
            "plugin should have a non-empty version"
        );

        let description = value.get("description").and_then(|v| v.as_str());
        assert!(
            description.is_some() && !description.unwrap().is_empty(),
            "plugin should have a non-empty description"
        );
    }

    #[test]
    fn plugin_json_version_is_semver() {
        let path = loom_root().join(".claude-plugin").join("plugin.json");
        let content = read_file(&path);
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();

        let version = value["version"].as_str().expect("version should be a string");
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "version '{}' should be semver (x.y.z)",
            version
        );
        for part in &parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "version component '{}' in '{}' should be a number",
                part,
                version
            );
        }
    }

    // ── Agent definition ────────────────────────────────────────────────

    #[test]
    fn loom_worker_agent_exists() {
        let path = loom_root().join("agents").join("loom-worker.md");
        assert!(
            path.exists(),
            "loom-worker.md should exist at {:?}",
            path
        );
    }

    #[test]
    fn loom_worker_has_valid_frontmatter() {
        let path = loom_root().join("agents").join("loom-worker.md");
        let content = read_file(&path);

        // Should start with YAML frontmatter
        assert!(
            content.starts_with("---"),
            "loom-worker.md should start with YAML frontmatter"
        );

        // Extract frontmatter
        let end = content[3..]
            .find("---")
            .expect("frontmatter should have closing ---");
        let frontmatter = &content[3..3 + end];

        // Required fields
        let required = ["name:", "description:", "model:", "isolation:", "maxTurns:"];
        for field in &required {
            assert!(
                frontmatter.contains(field),
                "frontmatter should contain '{}', got:\n{}",
                field,
                frontmatter
            );
        }
    }

    #[test]
    fn loom_worker_uses_worktree_isolation() {
        let path = loom_root().join("agents").join("loom-worker.md");
        let content = read_file(&path);
        let end = content[3..].find("---").unwrap();
        let frontmatter = &content[3..3 + end];

        assert!(
            frontmatter.contains("isolation: worktree")
                || frontmatter.contains("isolation: \"worktree\""),
            "loom-worker should use worktree isolation:\n{}",
            frontmatter
        );
    }

    #[test]
    fn loom_worker_body_references_commit_protocol() {
        let path = loom_root().join("agents").join("loom-worker.md");
        let content = read_file(&path);

        // Should reference commit-based protocol concepts
        assert!(
            content.contains("Agent-Id"),
            "worker should reference Agent-Id trailer"
        );
        assert!(
            content.contains("Session-Id"),
            "worker should reference Session-Id trailer"
        );
        assert!(
            content.contains("Task-Status"),
            "worker should reference Task-Status trailer"
        );
        assert!(
            content.contains("ASSIGNED"),
            "worker should reference ASSIGNED state"
        );
        assert!(
            content.contains("COMPLETED"),
            "worker should reference COMPLETED state"
        );
    }

    // ── Skill structure ─────────────────────────────────────────────────

    #[test]
    fn orchestrate_skill_exists() {
        let path = loom_root()
            .join("skills")
            .join("orchestrate")
            .join("SKILL.md");
        assert!(path.exists(), "SKILL.md should exist at {:?}", path);
    }

    #[test]
    fn orchestrate_skill_has_frontmatter() {
        let path = loom_root()
            .join("skills")
            .join("orchestrate")
            .join("SKILL.md");
        let content = read_file(&path);

        assert!(
            content.starts_with("---"),
            "SKILL.md should start with YAML frontmatter"
        );

        let end = content[3..].find("---").unwrap();
        let frontmatter = &content[3..3 + end];
        assert!(
            frontmatter.contains("name:"),
            "skill frontmatter should have 'name:'"
        );
    }

    #[test]
    fn skill_reference_docs_exist() {
        let refs_dir = loom_root()
            .join("skills")
            .join("orchestrate")
            .join("references");

        let required_refs = ["protocol.md", "schemas.md", "mcagent-spec.md", "examples.md"];
        for name in &required_refs {
            let path = refs_dir.join(name);
            assert!(
                path.exists(),
                "reference doc '{}' should exist at {:?}",
                name,
                path
            );
        }
    }

    #[test]
    fn skill_references_are_non_empty() {
        let refs_dir = loom_root()
            .join("skills")
            .join("orchestrate")
            .join("references");

        for name in &["protocol.md", "schemas.md", "mcagent-spec.md", "examples.md"] {
            let path = refs_dir.join(name);
            let content = read_file(&path);
            assert!(
                content.len() > 100,
                "{} should be substantial (got {} bytes)",
                name,
                content.len()
            );
        }
    }

    // ── Hooks ───────────────────────────────────────────────────────────

    #[test]
    fn hooks_json_exists() {
        let path = loom_root().join("hooks").join("hooks.json");
        assert!(path.exists(), "hooks.json should exist at {:?}", path);
    }

    #[test]
    fn hooks_json_valid_structure() {
        let path = loom_root().join("hooks").join("hooks.json");
        let content = read_file(&path);
        let value: serde_json::Value =
            serde_json::from_str(&content).expect("hooks.json should be valid JSON");

        assert!(value.get("hooks").is_some(), "must have 'hooks' key");
        assert!(value.get("version").is_some(), "must have 'version' key");

        let hooks = value["hooks"].as_object().expect("hooks should be an object");
        let expected_events = [
            "WorktreeCreate",
            "WorktreeRemove",
            "SubagentStart",
            "SubagentStop",
        ];
        for event in &expected_events {
            assert!(
                hooks.contains_key(*event),
                "hooks should define {} event",
                event
            );
        }
    }

    #[test]
    fn hook_scripts_exist() {
        let scripts_dir = loom_root().join("scripts");
        let expected = [
            "agent-start.sh",
            "agent-stop.sh",
            "worktree-create.sh",
            "worktree-remove.sh",
        ];
        for name in &expected {
            let path = scripts_dir.join(name);
            assert!(
                path.exists(),
                "hook script '{}' should exist at {:?}",
                name,
                path
            );
        }
    }

    #[test]
    fn hook_scripts_are_executable() {
        use std::os::unix::fs::PermissionsExt;

        let scripts_dir = loom_root().join("scripts");
        for name in &[
            "agent-start.sh",
            "agent-stop.sh",
            "worktree-create.sh",
            "worktree-remove.sh",
        ] {
            let path = scripts_dir.join(name);
            let mode = fs::metadata(&path)
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

    // ── Bin scripts ─────────────────────────────────────────────────────

    #[test]
    fn bin_scripts_exist() {
        let bin_dir = loom_root().join("bin");
        for name in &["loom-dispatch", "loom-spawn"] {
            let path = bin_dir.join(name);
            assert!(
                path.exists(),
                "bin script '{}' should exist at {:?}",
                name,
                path
            );
        }
    }

    #[test]
    fn bin_scripts_are_executable() {
        use std::os::unix::fs::PermissionsExt;

        let bin_dir = loom_root().join("bin");
        for name in &["loom-dispatch", "loom-spawn"] {
            let path = bin_dir.join(name);
            let mode = fs::metadata(&path)
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
    fn bin_scripts_have_shebang() {
        let bin_dir = loom_root().join("bin");
        for name in &["loom-dispatch", "loom-spawn"] {
            let content = read_file(&bin_dir.join(name));
            assert!(
                content.starts_with("#!/"),
                "{} should start with a shebang line",
                name
            );
        }
    }

    // ── Cross-reference consistency ─────────────────────────────────────

    #[test]
    fn protocol_references_schemas() {
        let content = read_file(
            &loom_root()
                .join("skills")
                .join("orchestrate")
                .join("references")
                .join("protocol.md"),
        );
        assert!(
            content.contains("schemas.md"),
            "protocol.md should reference schemas.md"
        );
    }

    #[test]
    fn schemas_references_protocol() {
        let content = read_file(
            &loom_root()
                .join("skills")
                .join("orchestrate")
                .join("references")
                .join("schemas.md"),
        );
        assert!(
            content.contains("protocol.md"),
            "schemas.md should reference protocol.md"
        );
    }

    #[test]
    fn protocol_version_consistent() {
        let refs_dir = loom_root()
            .join("skills")
            .join("orchestrate")
            .join("references");

        for name in &["protocol.md", "schemas.md"] {
            let content = read_file(&refs_dir.join(name));
            assert!(
                content.contains("loom/2") || content.contains("2.0.0"),
                "{} should reference protocol version 2",
                name
            );
        }
    }

    // ── No v1 remnants ──────────────────────────────────────────────────

    #[test]
    fn no_v1_protocol_file_references() {
        let v1_files = ["TASK.md", "STATUS.md", "PLAN.md"];
        // Note: MEMORY.md is excluded because it can legitimately appear in
        // agent memory system docs, not just v1 protocol references.

        let loom = loom_root();
        let mut violations: Vec<String> = Vec::new();

        // Walk all .md files in loom/
        fn find_md_files(dir: &Path, files: &mut Vec<PathBuf>) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        find_md_files(&path, files);
                    } else if path.extension().is_some_and(|e| e == "md") {
                        files.push(path);
                    }
                }
            }
        }

        let mut md_files = Vec::new();
        find_md_files(&loom, &mut md_files);

        for file in &md_files {
            let content = read_file(file);
            for v1_file in &v1_files {
                // Check for references like "TASK.md" or "creates a TASK.md"
                // but not "Task-Status" (which is a v2 trailer)
                if content.contains(v1_file) {
                    // Filter out references that are clearly v2 context
                    // (e.g., "no TASK.md" or "never create TASK.md")
                    let lines: Vec<&str> = content
                        .lines()
                        .filter(|l| l.contains(v1_file))
                        .collect();
                    for line in &lines {
                        let lower = line.to_lowercase();
                        // Allow negation/historical/explanatory references
                        if !lower.contains("no task.md")
                            && !lower.contains("no status.md")
                            && !lower.contains("no plan.md")
                            && !lower.contains("never")
                            && !lower.contains("not ")
                            && !lower.contains("don't")
                            && !lower.contains("do not")
                            && !lower.contains("there is no")
                            && !lower.contains("replaces")
                            && !lower.contains("previously")
                            && !lower.contains("was ")
                        {
                            violations.push(format!(
                                "{}: {}",
                                file.strip_prefix(&loom).unwrap_or(file).display(),
                                line.trim()
                            ));
                        }
                    }
                }
            }
        }

        assert!(
            violations.is_empty(),
            "v1 protocol file references found in loom/ docs:\n{}",
            violations.join("\n")
        );
    }

    // ── Directory structure ─────────────────────────────────────────────

    #[test]
    fn plugin_directory_structure() {
        let loom = loom_root();
        let required_dirs = [
            ".claude-plugin",
            "agents",
            "skills",
            "skills/orchestrate",
            "skills/orchestrate/references",
            "hooks",
            "scripts",
            "bin",
        ];

        for dir in &required_dirs {
            let path = loom.join(dir);
            assert!(
                path.is_dir(),
                "loom/{} should be a directory",
                dir
            );
        }
    }

    // ── No old skill location ───────────────────────────────────────────

    #[test]
    fn old_skill_location_mostly_removed() {
        // PR-8 moved the main skill to loom/. A few legacy reference files may
        // still exist in .claude/skills/loom/ but the SKILL.md itself should be gone.
        let old_skill_md = repo_root()
            .join(".claude")
            .join("skills")
            .join("loom")
            .join("SKILL.md");
        assert!(
            !old_skill_md.exists(),
            "old SKILL.md at .claude/skills/loom/SKILL.md should be removed (now at loom/skills/orchestrate/SKILL.md)"
        );
    }
}
