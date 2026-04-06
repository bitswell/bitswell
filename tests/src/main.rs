use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod loom;

// ── Configuration ───────────────────────────────────────────────────────────

const REQUIRED_AGENT_FILES: &[&str] = &[
    "values.md",
    "identity.md",
    "preferences.md",
    "seed-answers.md",
    "review.md",
];

const REQUIRED_MEMORY_FILES: &[&str] = &[
    "values.md",
    "identity.md",
    "preferences.md",
    "README.md",
];

const REQUIRED_MEMORY_DIRS: &[&str] = &["journal", "conversations"];

// Aspirational targets — used for informational reporting only, not pass/fail.
const EXPECTED_DOMAINS: usize = 25;
const QUESTIONS_PER_DOMAIN: usize = 40;
const QUESTIONS_PER_BATCH: usize = 50;
const TOTAL_BATCHES: usize = 20;

/// Minimum byte length for a review to be considered substantive.
const MIN_REVIEW_BYTES: usize = 200;

// ── Result tracking ─────────────────────────────────────────────────────────

#[derive(Default)]
struct Report {
    passed: usize,
    failed: usize,
    warnings: usize,
    details: Vec<String>,
}

impl Report {
    fn pass(&mut self, msg: &str) {
        self.passed += 1;
        self.details.push(format!("  PASS  {msg}"));
    }

    fn fail(&mut self, msg: &str) {
        self.failed += 1;
        self.details.push(format!("  FAIL  {msg}"));
    }

    fn warn(&mut self, msg: &str) {
        self.warnings += 1;
        self.details.push(format!("  WARN  {msg}"));
    }

    fn print_section(&self, title: &str) {
        println!("\n── {title} ──");
        for line in &self.details {
            println!("{line}");
        }
    }

    fn merge(&mut self, other: Report) {
        self.passed += other.passed;
        self.failed += other.failed;
        self.warnings += other.warnings;
        self.details.extend(other.details);
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Find the repo root by checking BITSWELL_ROOT env var, then walking up
/// from the current directory looking for a `.git` directory.
fn root() -> PathBuf {
    if let Ok(env_root) = std::env::var("BITSWELL_ROOT") {
        let p = PathBuf::from(env_root);
        if p.join(".git").exists() {
            return p;
        }
    }

    let mut dir = std::env::current_dir().expect("cannot determine current directory");
    loop {
        if dir.join(".git").exists() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }

    // Last resort: try parent of executable directory (original behavior)
    PathBuf::from("..")
}

fn read_file(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// List subdirectories of `dir` (non-recursive, immediate children only).
fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs
}

/// Extract the numbered values from memory/values.md.
/// Returns a map of value number -> value title.
fn parse_canonical_values(content: &str) -> HashMap<u32, String> {
    let mut values = HashMap::new();
    for line in content.lines() {
        // Match lines like "### 1. Be Fair"
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("### ") {
            if let Some(dot_pos) = rest.find(". ") {
                if let Ok(num) = rest[..dot_pos].trim().parse::<u32>() {
                    let title = rest[dot_pos + 2..].trim().to_string();
                    values.insert(num, title);
                }
            }
        }
    }
    values
}

/// Extract value numbers from agent values.md — same format.
fn parse_agent_values(content: &str) -> HashMap<u32, String> {
    parse_canonical_values(content) // same markdown format
}

/// Find all `[connects-to: value-N]` references in a string and return the N values.
fn find_value_references(content: &str) -> Vec<(u32, usize)> {
    let mut refs = vec![];
    for (line_no, line) in content.lines().enumerate() {
        let mut search = line;
        while let Some(start) = search.find("[connects-to: value-") {
            let after = &search[start + 20..];
            if let Some(end) = after.find(']') {
                if let Ok(num) = after[..end].trim().parse::<u32>() {
                    refs.push((num, line_no + 1));
                }
            }
            // Advance past this match
            search = &search[start + 20..];
        }
    }
    refs
}

/// Count the `## Q` headers in a batch file to approximate answered questions.
fn count_questions_in_file(content: &str) -> usize {
    content
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("## Q") || t.starts_with("## q")
        })
        .count()
}

/// Count occurrences of a tag in content.
fn count_tags(content: &str, tag: &str) -> usize {
    content.matches(tag).count()
}

// ── Validators ──────────────────────────────────────────────────────────────

/// 1. Structure Validation
///    - Every agent directory has all required files
///    - Memory directory has required files and subdirectories
///    - Questions directory has required files
fn validate_structure() -> Report {
    let mut r = Report::default();
    let base = root();

    // ── Agents ──
    // Discover agents from two locations:
    //   1. agents/{name}/ subdirectories (full format with values.md, identity.md, etc.)
    //   2. .claude/agents/{name}.md flat files (Claude agent format)
    let agents_dir = base.join("agents");
    let dir_agents = subdirs(&agents_dir);
    let mut dir_agent_names: Vec<String> = dir_agents
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect();
    dir_agent_names.sort();

    let claude_agents_dir = base.join(".claude/agents");
    let mut flat_agent_names: Vec<String> = vec![];
    if claude_agents_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&claude_agents_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    if let Some(stem) = path.file_stem() {
                        let name = stem.to_string_lossy().to_string();
                        if !dir_agent_names.contains(&name) {
                            flat_agent_names.push(name);
                        }
                    }
                }
            }
        }
    }
    flat_agent_names.sort();

    let total_agents = dir_agent_names.len() + flat_agent_names.len();
    if total_agents == 0 {
        r.fail("No agents found under agents/ or .claude/agents/");
    } else {
        r.pass(&format!(
            "Found {} agent(s) total: {} in agents/ dirs, {} in .claude/agents/ flat files",
            total_agents,
            dir_agent_names.len(),
            flat_agent_names.len(),
        ));
        if !dir_agent_names.is_empty() {
            r.pass(&format!("  Directory agents: {}", dir_agent_names.join(", ")));
        }
        if !flat_agent_names.is_empty() {
            r.pass(&format!("  Flat-file agents: {}", flat_agent_names.join(", ")));
        }

        // Validate directory-format agents (full file checks)
        for agent in &dir_agents {
            let name = agent.file_name().unwrap().to_string_lossy();
            for file in REQUIRED_AGENT_FILES {
                if agent.join(file).exists() {
                    r.pass(&format!("agents/{name}/{file} exists"));
                } else {
                    r.fail(&format!("agents/{name}/{file} MISSING"));
                }
            }
        }

        // Validate flat-file agents (just confirm the file exists and is non-empty)
        for name in &flat_agent_names {
            let path = claude_agents_dir.join(format!("{name}.md"));
            if let Some(content) = read_file(&path) {
                if content.len() > 0 {
                    r.pass(&format!(".claude/agents/{name}.md exists ({} bytes)", content.len()));
                } else {
                    r.warn(&format!(".claude/agents/{name}.md is empty"));
                }
            } else {
                r.fail(&format!(".claude/agents/{name}.md unreadable"));
            }
        }
    }

    // ── Memory ──
    let memory_dir = base.join("memory");
    for file in REQUIRED_MEMORY_FILES {
        if memory_dir.join(file).exists() {
            r.pass(&format!("memory/{file} exists"));
        } else {
            r.fail(&format!("memory/{file} MISSING"));
        }
    }
    for dir in REQUIRED_MEMORY_DIRS {
        if memory_dir.join(dir).is_dir() {
            r.pass(&format!("memory/{dir}/ exists"));
        } else {
            r.warn(&format!("memory/{dir}/ MISSING (expected directory)"));
        }
    }

    // ── Questions ──
    let q_dir = base.join("questions");
    for file in &[
        "README.md",
        "all-questions.md",
        "seed-answers.md",
        "favorites.md",
    ] {
        if q_dir.join(file).exists() {
            r.pass(&format!("questions/{file} exists"));
        } else {
            r.fail(&format!("questions/{file} MISSING"));
        }
    }
    if q_dir.join("answers").is_dir() {
        r.pass("questions/answers/ directory exists");
    } else {
        r.fail("questions/answers/ directory MISSING");
    }

    r
}

/// 2. Cross-Reference Integrity
///    - `[connects-to: value-N]` tags in answers and identity files reference
///      values that actually exist in memory/values.md
fn validate_cross_references() -> Report {
    let mut r = Report::default();
    let base = root();

    let values_path = base.join("memory/values.md");
    let Some(values_content) = read_file(&values_path) else {
        r.fail("Cannot read memory/values.md — skipping cross-reference checks");
        return r;
    };

    let canonical = parse_canonical_values(&values_content);
    if canonical.is_empty() {
        r.fail("No numbered values found in memory/values.md");
        return r;
    }
    r.pass(&format!(
        "Canonical values: {}",
        {
            let mut items: Vec<_> = canonical.iter().collect();
            items.sort_by_key(|(k, _)| *k);
            items
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));

    // Files to scan for [connects-to: value-N]
    let mut files_to_scan: Vec<PathBuf> = vec![
        base.join("memory/identity.md"),
        base.join("memory/preferences.md"),
        base.join("questions/seed-answers.md"),
        base.join("questions/favorites.md"),
    ];

    // Add all batch answer files
    let answers_dir = base.join("questions/answers");
    if answers_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&answers_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    files_to_scan.push(path);
                }
            }
        }
    }

    // Add agent seed-answers
    for agent in subdirs(&base.join("agents")) {
        files_to_scan.push(agent.join("seed-answers.md"));
    }

    let mut total_refs = 0usize;
    let mut broken_refs = 0usize;

    for path in &files_to_scan {
        let Some(content) = read_file(path) else {
            continue;
        };
        let refs = find_value_references(&content);
        if refs.is_empty() {
            continue;
        }
        let rel = path.strip_prefix(&base).unwrap_or(path);
        for (num, line) in &refs {
            total_refs += 1;
            if canonical.contains_key(num) {
                r.pass(&format!(
                    "{}: line {} references value-{num} ({})",
                    rel.display(),
                    line,
                    canonical[num]
                ));
            } else {
                broken_refs += 1;
                r.fail(&format!(
                    "{}: line {} references value-{num} which DOES NOT EXIST",
                    rel.display(),
                    line
                ));
            }
        }
    }

    if total_refs == 0 {
        r.warn("No [connects-to: value-N] references found in any scanned file");
    } else {
        let summary = format!(
            "Cross-reference summary: {total_refs} references found, {broken_refs} broken"
        );
        if broken_refs == 0 {
            r.pass(&summary);
        } else {
            r.fail(&summary);
        }
    }

    r
}

/// 3. Agent Review Completeness
///    - Every agent has a review.md
///    - Review is non-empty and substantive (> MIN_REVIEW_BYTES)
///    - Review contains structural markers (headings, bold text)
fn validate_reviews() -> Report {
    let mut r = Report::default();
    let base = root();

    let agents = subdirs(&base.join("agents"));
    if agents.is_empty() {
        r.fail("No agent directories found");
        return r;
    }

    for agent in &agents {
        let name = agent.file_name().unwrap().to_string_lossy();
        let review_path = agent.join("review.md");

        let Some(content) = read_file(&review_path) else {
            r.fail(&format!("{name}: review.md missing or unreadable"));
            continue;
        };

        let bytes = content.len();
        if bytes == 0 {
            r.fail(&format!("{name}: review.md is empty"));
            continue;
        }

        if bytes < MIN_REVIEW_BYTES {
            r.warn(&format!(
                "{name}: review.md is only {bytes} bytes (minimum {MIN_REVIEW_BYTES} for substantive review)"
            ));
        } else {
            r.pass(&format!("{name}: review.md is {bytes} bytes"));
        }

        // Check for structural markers
        let has_headings = content
            .lines()
            .any(|l| l.starts_with('#') || l.starts_with("### "));
        let has_bold = content.contains("**");
        let has_summary = content.to_lowercase().contains("summary");

        if has_headings {
            r.pass(&format!("{name}: review has section headings"));
        } else {
            r.warn(&format!("{name}: review lacks section headings"));
        }

        if has_bold {
            r.pass(&format!("{name}: review uses emphasis/bold markup"));
        } else {
            r.warn(&format!("{name}: review lacks bold/emphasis markup"));
        }

        if has_summary {
            r.pass(&format!("{name}: review contains a summary"));
        } else {
            r.warn(&format!("{name}: review has no summary section"));
        }
    }

    r
}

/// 4. Question Coverage Tracking
///    - Is all-questions.md populated?
///    - How many batch files exist in questions/answers/?
///    - How many questions are answered per batch?
///    - How many total questions answered vs expected 1000?
///    - Are self-note tags present?
fn validate_question_coverage() -> Report {
    let mut r = Report::default();
    let base = root();

    // ── all-questions.md ──
    let all_q_path = base.join("questions/all-questions.md");
    if let Some(content) = read_file(&all_q_path) {
        if content.contains("Not yet generated") || content.contains("not yet generated") {
            r.warn("questions/all-questions.md has not been populated yet");
        } else {
            let domain_count = content
                .lines()
                .filter(|l| l.starts_with("## ") || l.starts_with("### "))
                .count();
            let question_count = content
                .lines()
                .filter(|l| l.starts_with("- ") || l.starts_with("1."))
                .count();
            r.pass(&format!(
                "all-questions.md: ~{domain_count} sections, ~{question_count} question lines"
            ));
            r.pass(&format!(
                "Domain progress: {domain_count}/{EXPECTED_DOMAINS} target"
            ));
        }
    } else {
        r.fail("Cannot read questions/all-questions.md");
    }

    // ── seed-answers.md ──
    let seed_path = base.join("questions/seed-answers.md");
    if let Some(content) = read_file(&seed_path) {
        let q_count = count_questions_in_file(&content);
        if q_count > 0 {
            r.pass(&format!("seed-answers.md: {q_count} questions answered"));
        } else {
            r.warn("seed-answers.md: no question headers (## Q) detected");
        }

        let self_notes = count_tags(&content, "**Self-note**");
        if self_notes > 0 {
            r.pass(&format!("seed-answers.md: {self_notes} self-notes found"));
        } else {
            r.warn("seed-answers.md: no **Self-note** tags found");
        }
    }

    // ── Batch answer files ──
    let answers_dir = base.join("questions/answers");
    let mut batch_files: Vec<PathBuf> = vec![];
    if answers_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&answers_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    batch_files.push(path);
                }
            }
        }
    }
    batch_files.sort();

    if batch_files.is_empty() {
        r.pass(&format!(
            "No batch answer files in questions/answers/ yet (target: {TOTAL_BATCHES})"
        ));
    } else {
        r.pass(&format!(
            "Found {} batch file(s) (target: {TOTAL_BATCHES})",
            batch_files.len()
        ));

        let mut total_answered = 0usize;
        let mut total_tags_trait = 0usize;
        let mut total_tags_tension = 0usize;
        let mut total_tags_confidence = 0usize;
        let mut total_tags_connects = 0usize;

        for batch in &batch_files {
            let fname = batch.file_name().unwrap().to_string_lossy();
            if let Some(content) = read_file(batch) {
                let q_count = count_questions_in_file(&content);
                total_answered += q_count;

                total_tags_trait += count_tags(&content, "[trait]");
                total_tags_tension += count_tags(&content, "[tension]");
                total_tags_confidence += count_tags(&content, "[confidence:");
                total_tags_connects += count_tags(&content, "[connects-to:");

                r.pass(&format!(
                    "{fname}: {q_count} questions answered (target: {QUESTIONS_PER_BATCH})"
                ));
            }
        }

        let target_total = EXPECTED_DOMAINS * QUESTIONS_PER_DOMAIN;
        r.pass(&format!(
            "Total questions answered in batches: {total_answered} (target: {target_total})"
        ));

        r.pass(&format!(
            "Tags across batches — [trait]: {total_tags_trait}, [tension]: {total_tags_tension}, \
             [confidence]: {total_tags_confidence}, [connects-to]: {total_tags_connects}"
        ));
        if total_tags_confidence < total_answered {
            r.warn(&format!(
                "Only {total_tags_confidence}/{total_answered} answers have [confidence:] tags"
            ));
        }
    }

    // ── favorites.md ──
    let fav_path = base.join("questions/favorites.md");
    if let Some(content) = read_file(&fav_path) {
        if content.contains("None yet") || content.contains("none yet") {
            r.warn("favorites.md: no favorites curated yet");
        } else {
            let fav_count = content
                .lines()
                .filter(|l| l.starts_with("## Q") || l.starts_with("- Q"))
                .count();
            if fav_count > 0 {
                r.pass(&format!("favorites.md: {fav_count} favorites curated"));
            } else {
                r.warn("favorites.md: could not detect curated favorites");
            }
        }
    }

    r
}

/// 5. Consistency Checker
///    - Compare each agent's values.md against canonical memory/values.md
///    - Report value count mismatches and title drift
///    - Cross-check identity.md attributions against known agent names
fn validate_consistency() -> Report {
    let mut r = Report::default();
    let base = root();

    let canonical_path = base.join("memory/values.md");
    let Some(canonical_content) = read_file(&canonical_path) else {
        r.fail("Cannot read memory/values.md — skipping consistency checks");
        return r;
    };
    let canonical = parse_canonical_values(&canonical_content);
    r.pass(&format!(
        "Canonical values: {} value(s) defined",
        canonical.len()
    ));

    let agents = subdirs(&base.join("agents"));
    let agent_names: Vec<String> = agents
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect();

    for agent in &agents {
        let name = agent.file_name().unwrap().to_string_lossy();
        let values_path = agent.join("values.md");

        let Some(content) = read_file(&values_path) else {
            r.fail(&format!("{name}: cannot read values.md"));
            continue;
        };

        let agent_values = parse_agent_values(&content);
        let agent_count = agent_values.len();
        let canonical_count = canonical.len();

        if agent_count == 0 {
            r.warn(&format!(
                "{name}: no numbered values found (may use different format)"
            ));
            continue;
        }

        r.pass(&format!(
            "{name}: {agent_count} value(s) defined (canonical has {canonical_count})"
        ));

        if agent_count != canonical_count {
            r.warn(&format!(
                "{name}: value count differs from canonical ({agent_count} vs {canonical_count})"
            ));
        }

        // Check for shared numbering with different titles
        for (num, canonical_title) in &canonical {
            if let Some(agent_title) = agent_values.get(num) {
                if agent_title.to_lowercase() != canonical_title.to_lowercase() {
                    r.warn(&format!(
                        "{name}: value {num} differs — canonical: \"{canonical_title}\", \
                         agent: \"{agent_title}\""
                    ));
                }
            }
        }

        // Check for values the agent has that canonical doesn't
        for (num, title) in &agent_values {
            if !canonical.contains_key(num) {
                r.warn(&format!(
                    "{name}: has value {num} (\"{title}\") not present in canonical"
                ));
            }
        }
    }

    // ── Cross-check identity.md attributions ──
    let identity_path = base.join("memory/identity.md");
    if let Some(content) = read_file(&identity_path) {
        let common_words: &[&str] = &[
            "but", "not", "the", "and", "for", "with", "from", "this", "that", "also", "only",
            "yes", "see", "pre", "revised", "initial", "new", "old", "added", "each", "more",
            "less", "none", "some", "all", "any", "was", "has", "had", "are", "may", "can",
            "will", "should",
        ];

        for (line_no, line) in content.lines().enumerate() {
            let mut search = line;
            while let Some(open) = search.find('(') {
                if let Some(close) = search[open..].find(')') {
                    let inner = &search[open + 1..open + close];
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    for part in &parts {
                        let lower = part.to_lowercase();
                        // Only check single alphabetic words that look like agent names
                        if part.len() >= 3
                            && part.len() <= 10
                            && part.chars().all(|c| c.is_alphabetic())
                            && part.chars().next().is_some_and(|c| c.is_uppercase())
                            && !common_words.contains(&lower.as_str())
                            && !agent_names.iter().any(|a| a.eq_ignore_ascii_case(part))
                        {
                            r.warn(&format!(
                                "identity.md line {}: \"({part})\" may reference unknown agent \
                                 (known: {})",
                                line_no + 1,
                                agent_names.join(", ")
                            ));
                        }
                    }
                    search = &search[open + close + 1..];
                } else {
                    break;
                }
            }
        }
        r.pass("identity.md attribution cross-check complete");
    }

    r
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║        bitswell identity framework — test suite        ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let mut total = Report::default();

    let validators: Vec<(&str, fn() -> Report)> = vec![
        ("1. Structure Validation", validate_structure),
        ("2. Cross-Reference Integrity", validate_cross_references),
        ("3. Agent Review Completeness", validate_reviews),
        ("4. Question Coverage", validate_question_coverage),
        ("5. Consistency (Agent <-> Canonical)", validate_consistency),
    ];

    for (title, validator) in validators {
        let section = validator();
        section.print_section(title);
        total.merge(section);
    }

    // ── Summary ──
    println!("\n══════════════════════════════════════════════════════════");
    println!(
        "  TOTAL: {} passed, {} failed, {} warnings",
        total.passed, total.failed, total.warnings
    );
    println!("══════════════════════════════════════════════════════════");

    if total.failed > 0 {
        println!("\nResult: FAIL");
        ExitCode::FAILURE
    } else if total.warnings > 0 {
        println!("\nResult: PASS (with warnings)");
        ExitCode::SUCCESS
    } else {
        println!("\nResult: PASS");
        ExitCode::SUCCESS
    }
}
