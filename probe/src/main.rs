use std::io::{BufRead, BufReader};
use std::process::{Command, ExitCode, Stdio};

use serde::Deserialize;

// ── Init event types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Event {
    #[serde(rename = "system")]
    System(SystemEvent),
    #[serde(rename = "result")]
    Result(ResultEvent),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct SystemEvent {
    subtype: Option<String>,
    tools: Option<Vec<String>>,
    #[serde(default)]
    mcp_servers: Vec<McpServer>,
    model: Option<String>,
    #[serde(rename = "permissionMode")]
    permission_mode: Option<String>,
    #[serde(default)]
    agents: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    plugins: Vec<Plugin>,
    claude_code_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct McpServer {
    name: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct Plugin {
    name: String,
    #[allow(dead_code)]
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResultEvent {
    #[allow(dead_code)]
    subtype: Option<String>,
    #[serde(default)]
    permission_denials: Vec<serde_json::Value>,
    total_cost_usd: Option<f64>,
    #[allow(dead_code)]
    #[serde(default)]
    usage: serde_json::Value,
}

// ── Capabilities snapshot ──────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
struct Capabilities {
    claude_version: String,
    model: String,
    permission_mode: String,
    tools: Vec<String>,
    mcp_servers: Vec<McpStatus>,
    agents: Vec<String>,
    skills: Vec<String>,
    plugins: Vec<String>,
    permission_denials: Vec<serde_json::Value>,
    cost_usd: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
struct McpStatus {
    name: String,
    status: String,
}

// ── CLI ────────────────────────────────────────────────────────────────────

struct Args {
    claude_cmd: String,
    extra_args: Vec<String>,
    json_output: bool,
    working_dir: Option<String>,
}

fn parse_args() -> Args {
    let mut args = Args {
        claude_cmd: String::from("claude"),
        extra_args: Vec::new(),
        json_output: false,
        working_dir: None,
    };

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--claude-cmd" => {
                args.claude_cmd = iter.next().expect("--claude-cmd requires a value");
            }
            "--json" => {
                args.json_output = true;
            }
            "--dir" | "-C" => {
                args.working_dir = Some(iter.next().expect("--dir requires a value"));
            }
            "--help" | "-h" => {
                eprintln!("claude-probe -- Probe Claude CLI capabilities");
                eprintln!();
                eprintln!("Usage: claude-probe [OPTIONS] [-- CLAUDE_ARGS...]");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  --claude-cmd CMD  Claude binary (default: claude)");
                eprintln!("  --json            Output as JSON instead of human-readable");
                eprintln!("  --dir, -C DIR     Working directory for claude session");
                eprintln!("  --help, -h        Show this help");
                eprintln!();
                eprintln!("Everything after -- is passed to claude as extra args.");
                eprintln!();
                eprintln!("Examples:");
                eprintln!("  claude-probe                        # default capabilities");
                eprintln!("  claude-probe -- --tools ''           # probe with no built-in tools");
                eprintln!("  claude-probe -- --permission-mode bypassPermissions");
                eprintln!("  claude-probe --json                  # machine-readable output");
                std::process::exit(0);
            }
            "--" => {
                args.extra_args.extend(iter.by_ref());
                break;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                eprintln!("Use --help for usage.");
                std::process::exit(1);
            }
        }
    }

    args
}

// ── Probe logic ────────────────────────────────────────────────────────────

fn probe(args: &Args) -> Result<Capabilities, String> {
    let mut cmd = Command::new(&args.claude_cmd);
    cmd.arg("--print")
        .arg("--output-format")
        .arg("json")
        .arg("--max-turns")
        .arg("1")
        .arg("-p")
        .arg("Say OK");

    for extra in &args.extra_args {
        cmd.arg(extra);
    }

    if let Some(dir) = &args.working_dir {
        cmd.current_dir(dir);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn {}: {e}", args.claude_cmd))?;

    let stdout = child.stdout.take().expect("stdout piped");
    let reader = BufReader::new(stdout);

    let mut init: Option<SystemEvent> = None;
    let mut result: Option<ResultEvent> = None;

    // Output is either a JSON array on one line or newline-delimited JSON objects.
    // Read all stdout, then try array parse first, fall back to JSONL.
    let mut raw = String::new();
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Read error: {e}"))?;
        raw.push_str(&line);
        raw.push('\n');
    }

    let events: Vec<Event> = if raw.trim_start().starts_with('[') {
        // JSON array format
        serde_json::from_str(raw.trim()).unwrap_or_default()
    } else {
        // JSONL format — one event per line
        raw.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    };

    for event in events {
        match event {
            Event::System(sys) if sys.subtype.as_deref() == Some("init") => {
                init = Some(sys);
            }
            Event::Result(res) => {
                result = Some(res);
            }
            _ => {}
        }
    }

    let status = child.wait().map_err(|e| format!("Wait error: {e}"))?;
    if !status.success() {
        // Non-zero exit is okay for probing — we still got the init event.
        // Only fail if we got no init at all.
    }

    let init = init.ok_or("No init event received from Claude CLI")?;

    Ok(Capabilities {
        claude_version: init.claude_code_version.unwrap_or_default(),
        model: init.model.unwrap_or_default(),
        permission_mode: init.permission_mode.unwrap_or_default(),
        tools: init.tools.unwrap_or_default(),
        mcp_servers: init
            .mcp_servers
            .into_iter()
            .map(|s| McpStatus {
                name: s.name,
                status: s.status,
            })
            .collect(),
        agents: init.agents,
        skills: init.skills,
        plugins: init.plugins.into_iter().map(|p| p.name).collect(),
        permission_denials: result
            .as_ref()
            .map(|r| r.permission_denials.clone())
            .unwrap_or_default(),
        cost_usd: result.as_ref().and_then(|r| r.total_cost_usd),
    })
}

// ── Display ────────────────────────────────────────────────────────────────

fn print_human(caps: &Capabilities) {
    println!("Claude CLI Capabilities");
    println!("=======================");
    println!();
    println!("Version:         {}", caps.claude_version);
    println!("Model:           {}", caps.model);
    println!("Permission mode: {}", caps.permission_mode);
    println!();

    println!("Tools ({}):", caps.tools.len());
    for tool in &caps.tools {
        println!("  - {tool}");
    }
    println!();

    println!("MCP Servers ({}):", caps.mcp_servers.len());
    for srv in &caps.mcp_servers {
        let marker = match srv.status.as_str() {
            "connected" => "+",
            "failed" => "!",
            "needs-auth" => "?",
            _ => " ",
        };
        println!("  [{marker}] {} ({})", srv.name, srv.status);
    }
    println!();

    println!("Agents ({}):", caps.agents.len());
    for agent in &caps.agents {
        println!("  - {agent}");
    }
    println!();

    println!("Skills ({}):", caps.skills.len());
    for skill in &caps.skills {
        println!("  - {skill}");
    }
    println!();

    println!("Plugins ({}):", caps.plugins.len());
    for plugin in &caps.plugins {
        println!("  - {plugin}");
    }
    println!();

    if !caps.permission_denials.is_empty() {
        println!("Permission Denials ({}):", caps.permission_denials.len());
        for denial in &caps.permission_denials {
            println!("  - {denial}");
        }
        println!();
    }

    if let Some(cost) = caps.cost_usd {
        println!("Probe cost: ${cost:.6}");
    }
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let args = parse_args();

    match probe(&args) {
        Ok(caps) => {
            if args.json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&caps).expect("serialize")
                );
            } else {
                print_human(&caps);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
