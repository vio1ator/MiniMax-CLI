use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct McpServerInput {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct McpConfig {
    servers: HashMap<String, McpServer>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct McpServer {
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
}

pub fn list(path: PathBuf) -> Result<()> {
    let config = load(&path)?;
    if config.servers.is_empty() {
        println!("No MCP servers configured.");
        return Ok(());
    }

    for (name, server) in config.servers {
        println!("{} -> {} {}", name, server.command, server.args.join(" "));
    }
    Ok(())
}

pub fn add(path: PathBuf, input: McpServerInput) -> Result<()> {
    let mut config = load(&path)?;
    let env = parse_env(&input.env)?;
    config.servers.insert(
        input.name.clone(),
        McpServer {
            command: input.command,
            args: input.args,
            env,
        },
    );
    save(&path, &config)?;
    println!("Added MCP server: {}", input.name);
    Ok(())
}

pub fn remove(path: PathBuf, name: &str) -> Result<()> {
    let mut config = load(&path)?;
    if config.servers.remove(name).is_some() {
        save(&path, &config)?;
        println!("Removed MCP server: {}", name);
    } else {
        println!("No MCP server named {}.", name);
    }
    Ok(())
}

pub fn call_tool(path: PathBuf, server: &str, tool: &str, args: serde_json::Value) -> Result<String> {
    let config = load(&path)?;
    let Some(server_cfg) = config.servers.get(server) else {
        anyhow::bail!("MCP server not found: {}", server);
    };

    let mut cmd = Command::new(&server_cfg.command);
    cmd.args(&server_cfg.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in &server_cfg.env {
        cmd.env(key, value);
    }

    let mut child = cmd.spawn().with_context(|| "Failed to spawn MCP server")?;
    let mut stdin = child.stdin.take().context("Failed to open MCP stdin")?;
    let stdout = child.stdout.take().context("Failed to open MCP stdout")?;
    let mut reader = BufReader::new(stdout);

    let init_id = next_id();
    let init_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": init_id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": { "name": "minimax-cli", "version": "0.1.0" },
            "capabilities": {}
        }
    });
    send_request(&mut stdin, init_payload)?;
    let _init_response = read_response(&mut reader, init_id)?;
    let initialized_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    send_request(&mut stdin, initialized_payload)?;

    let call_id = next_id();
    let call_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": call_id,
        "method": "tools/call",
        "params": {
            "name": tool,
            "arguments": args
        }
    });
    send_request(&mut stdin, call_payload)?;
    let response = read_response(&mut reader, call_id)?;

    let _ = child.kill();

    if let Some(result) = response.get("result") {
        return Ok(serde_json::to_string_pretty(result)?);
    }
    if let Some(error) = response.get("error") {
        return Ok(serde_json::to_string_pretty(error)?);
    }
    Ok(serde_json::to_string_pretty(&response)?)
}

fn load(path: &PathBuf) -> Result<McpConfig> {
    if path.exists() {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(config)
    } else {
        Ok(McpConfig::default())
    }
}

fn save(path: &PathBuf, config: &McpConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(config)?;
    fs::write(path, contents)?;
    Ok(())
}

fn parse_env(items: &[String]) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();
    for item in items {
        let parts: Vec<&str> = item.splitn(2, '=').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid env format: {}", item);
        }
        env.insert(parts[0].to_string(), parts[1].to_string());
    }
    Ok(env)
}

fn send_request(stdin: &mut impl Write, payload: serde_json::Value) -> Result<()> {
    let line = serde_json::to_string(&payload)?;
    stdin
        .write_all(format!("{}\n", line).as_bytes())
        .with_context(|| "Failed to write MCP request")?;
    stdin.flush()?;
    Ok(())
}

fn read_response(reader: &mut BufReader<std::process::ChildStdout>, id: u64) -> Result<serde_json::Value> {
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            anyhow::bail!("MCP server closed output before responding.");
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if value.get("id").and_then(|v| v.as_u64()) == Some(id) {
                return Ok(value);
            }
        }
    }
}

fn next_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}
