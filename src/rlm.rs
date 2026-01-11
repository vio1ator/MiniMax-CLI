use crate::config::Config;
use anyhow::{Context, Result};
use std::path::PathBuf;

// Local types for RLM commands (previously in main.rs)
#[allow(dead_code)]
pub struct RlmLoadArgs {
    pub path: PathBuf,
    pub context_id: String,
}

#[allow(dead_code)]
pub struct RlmSearchArgs {
    pub context_id: String,
    pub pattern: String,
    pub context_lines: usize,
    pub max_results: usize,
}

#[allow(dead_code)]
pub struct RlmExecArgs {
    pub context_id: String,
    pub code: String,
}

#[allow(dead_code)]
pub struct RlmStatusArgs {
    pub context_id: Option<String>,
}

#[allow(dead_code)]
pub struct RlmSaveSessionArgs {
    pub path: PathBuf,
    pub context_id: String,
}

#[allow(dead_code)]
pub struct RlmLoadSessionArgs {
    pub path: PathBuf,
}

#[allow(dead_code)]
pub struct RlmReplArgs {
    pub context_id: String,
    pub load: Option<PathBuf>,
}

#[allow(dead_code)]
pub enum RlmCommand {
    Load(RlmLoadArgs),
    Search(RlmSearchArgs),
    Exec(RlmExecArgs),
    Status(RlmStatusArgs),
    SaveSession(RlmSaveSessionArgs),
    LoadSession(RlmLoadSessionArgs),
    Repl(RlmReplArgs),
}
use colored::Colorize;
use regex::Regex;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SystemResources {
    pub available_memory_mb: Option<u64>,
    pub recommended_max_context: usize,
}

impl SystemResources {
    pub fn detect() -> Self {
        let available_memory_mb = Self::get_available_memory_mb();

        // Recommend context size based on available memory
        // Rule of thumb: use ~10% of available RAM for context
        let recommended_max_context = match available_memory_mb {
            Some(mem) if mem >= 32000 => 100_000_000,  // 100MB for 32GB+ RAM
            Some(mem) if mem >= 16000 => 50_000_000,   // 50MB for 16GB+ RAM
            Some(mem) if mem >= 8000 => 25_000_000,    // 25MB for 8GB+ RAM
            Some(mem) if mem >= 4000 => 10_000_000,    // 10MB for 4GB+ RAM
            _ => 5_000_000,                             // 5MB default
        };

        Self {
            available_memory_mb,
            recommended_max_context,
        }
    }

    #[cfg(target_os = "macos")]
    fn get_available_memory_mb() -> Option<u64> {
        use std::process::Command;

        // Try to get memory from sysctl on macOS
        Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .map(|bytes| bytes / (1024 * 1024))
            })
    }

    #[cfg(target_os = "linux")]
    fn get_available_memory_mb() -> Option<u64> {
        use std::fs;

        // Read from /proc/meminfo on Linux
        fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.starts_with("MemTotal:"))
                    .and_then(|line| {
                        line.split_whitespace()
                            .nth(1)
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(|kb| kb / 1024)
                    })
            })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn get_available_memory_mb() -> Option<u64> {
        None
    }

    pub fn print_info(&self) {
        println!("{}", "System Resources".cyan().bold());
        if let Some(mem) = self.available_memory_mb {
            println!("  Available RAM: {} MB ({:.1} GB)", mem, mem as f64 / 1024.0);
        } else {
            println!("  Available RAM: Unknown");
        }
        println!("  Recommended max context: {} chars ({:.1} MB)",
            self.recommended_max_context,
            self.recommended_max_context as f64 / (1024.0 * 1024.0));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmContext {
    pub id: String,
    pub content: String,
    pub source_path: Option<String>,
    pub line_count: usize,
    pub char_count: usize,
    pub variables: HashMap<String, String>,
}

impl RlmContext {
    pub fn new(id: &str, content: String, source_path: Option<String>) -> Self {
        let line_count = content.lines().count();
        let char_count = content.len();
        Self {
            id: id.to_string(),
            content,
            source_path,
            line_count,
            char_count,
            variables: HashMap::new(),
        }
    }

    pub fn peek(&self, start: usize, end: Option<usize>) -> &str {
        let end = end.unwrap_or(self.content.len()).min(self.content.len());
        &self.content[start.min(self.content.len())..end]
    }

    pub fn lines(&self, start: usize, end: Option<usize>) -> Vec<(usize, &str)> {
        let lines: Vec<&str> = self.content.lines().collect();
        let end = end.unwrap_or(lines.len()).min(lines.len());
        lines[start.min(lines.len())..end]
            .iter()
            .enumerate()
            .map(|(i, line)| (start + i + 1, *line))
            .collect()
    }

    pub fn search(&self, pattern: &str, context_lines: usize, max_results: usize) -> Result<Vec<SearchResult>> {
        let regex = Regex::new(pattern).context("Invalid regex pattern")?;
        let lines: Vec<&str> = self.content.lines().collect();
        let mut results = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if regex.is_match(line) {
                let start = i.saturating_sub(context_lines);
                let end = (i + context_lines + 1).min(lines.len());
                let context: Vec<String> = lines[start..end]
                    .iter()
                    .enumerate()
                    .map(|(j, l)| {
                        let line_num = start + j + 1;
                        if start + j == i {
                            format!("{:>5} > {}", line_num, l)
                        } else {
                            format!("{:>5}   {}", line_num, l)
                        }
                    })
                    .collect();

                results.push(SearchResult {
                    line_num: i + 1,
                    match_line: line.to_string(),
                    context,
                });

                if results.len() >= max_results {
                    break;
                }
            }
        }

        Ok(results)
    }

    pub fn chunk(&self, chunk_size: usize, overlap: usize) -> Vec<ChunkInfo> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < self.content.len() {
            let end = (start + chunk_size).min(self.content.len());
            let preview_end = (start + 100).min(end);
            let preview = self.content[start..preview_end].to_string();

            chunks.push(ChunkInfo {
                index: chunk_index,
                start_char: start,
                end_char: end,
                preview: preview.replace('\n', " "),
            });

            start = if end == self.content.len() {
                end
            } else {
                (end - overlap).max(start + 1)
            };
            chunk_index += 1;
        }

        chunks
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub line_num: usize,
    pub match_line: String,
    pub context: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub index: usize,
    pub start_char: usize,
    pub end_char: usize,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmSession {
    pub contexts: HashMap<String, RlmContext>,
    pub active_context: String,
}

impl Default for RlmSession {
    fn default() -> Self {
        Self {
            contexts: HashMap::new(),
            active_context: "default".to_string(),
        }
    }
}

impl RlmSession {
    pub fn load_context(&mut self, id: &str, content: String, source_path: Option<String>) {
        let ctx = RlmContext::new(id, content, source_path);
        self.contexts.insert(id.to_string(), ctx);
        self.active_context = id.to_string();
    }

    pub fn get_context(&self, id: &str) -> Option<&RlmContext> {
        self.contexts.get(id)
    }

    #[allow(dead_code)]
    pub fn get_context_mut(&mut self, id: &str) -> Option<&mut RlmContext> {
        self.contexts.get_mut(id)
    }
}

#[allow(dead_code)]
pub fn handle_command(command: RlmCommand, _config: &Config) -> Result<()> {
    let mut session = RlmSession::default();

    match command {
        RlmCommand::Load(args) => {
            let content = fs::read_to_string(&args.path)
                .with_context(|| format!("Failed to read file: {}", args.path.display()))?;
            let source = args.path.to_string_lossy().to_string();
            session.load_context(&args.context_id, content, Some(source));

            let ctx = session.get_context(&args.context_id).unwrap();
            println!("{}", "Context loaded successfully!".green());
            println!("  ID: {}", ctx.id.cyan());
            println!("  Source: {}", ctx.source_path.as_deref().unwrap_or("N/A"));
            println!("  Lines: {}", ctx.line_count);
            println!("  Characters: {}", ctx.char_count);
        }
        RlmCommand::Search(args) => {
            let content = load_context_from_stdin_or_error(&args.context_id)?;
            let ctx = RlmContext::new(&args.context_id, content, None);

            let results = ctx.search(&args.pattern, args.context_lines, args.max_results)?;

            if results.is_empty() {
                println!("{}", "No matches found.".yellow());
            } else {
                println!("{} matches found:\n", results.len().to_string().green());
                for result in results {
                    println!("{}", "─".repeat(60).dimmed());
                    for line in &result.context {
                        println!("{}", line);
                    }
                }
                println!("{}", "─".repeat(60).dimmed());
            }
        }
        RlmCommand::Exec(args) => {
            let content = load_context_from_stdin_or_error(&args.context_id)?;
            let ctx = RlmContext::new(&args.context_id, content, None);

            let result = execute_expr(&ctx, &args.code)?;
            println!("{}", result);
        }
        RlmCommand::Status(args) => {
            if let Some(id) = args.context_id {
                println!("Context '{}' status: (no persistent session)", id);
            } else {
                println!("{}", "RLM Session Status".cyan().bold());
                println!("Note: For persistent sessions, use 'rlm repl' or save/load session.");
            }
        }
        RlmCommand::SaveSession(args) => {
            let json = serde_json::to_string_pretty(&session)?;
            fs::write(&args.path, json)?;
            println!("Session saved to {}", args.path.display());
        }
        RlmCommand::LoadSession(args) => {
            let content = fs::read_to_string(&args.path)?;
            session = serde_json::from_str(&content)?;
            println!("Session loaded from {}", args.path.display());
            println!("Contexts: {:?}", session.contexts.keys().collect::<Vec<_>>());
        }
        RlmCommand::Repl(args) => {
            run_repl(args.context_id, args.load)?;
        }
    }

    Ok(())
}

fn load_context_from_stdin_or_error(context_id: &str) -> Result<String> {
    // For now, return an error - real implementation would track sessions
    anyhow::bail!(
        "No context '{}' loaded. Use 'rlm load' first or 'rlm repl' for interactive mode.",
        context_id
    )
}

fn execute_expr(ctx: &RlmContext, code: &str) -> Result<String> {
    // Simple expression evaluator for RLM
    // Supports: len(ctx), lines(start, end), search("pattern"), peek(start, end), chunk(size)
    let code = code.trim();

    if code == "len(ctx)" || code == "len" {
        return Ok(format!("{}", ctx.char_count));
    }

    if code == "line_count" || code == "lines" {
        return Ok(format!("{}", ctx.line_count));
    }

    if code.starts_with("peek(") && code.ends_with(')') {
        let args = &code[5..code.len() - 1];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        let start: usize = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let end: Option<usize> = parts.get(1).and_then(|s| s.parse().ok());
        return Ok(ctx.peek(start, end).to_string());
    }

    if code.starts_with("lines(") && code.ends_with(')') {
        let args = &code[6..code.len() - 1];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        let start: usize = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let end: Option<usize> = parts.get(1).and_then(|s| s.parse().ok());
        let lines = ctx.lines(start, end);
        return Ok(lines
            .iter()
            .map(|(n, l)| format!("{:>5} {}", n, l))
            .collect::<Vec<_>>()
            .join("\n"));
    }

    if code.starts_with("search(") && code.ends_with(')') {
        let pattern = &code[7..code.len() - 1].trim_matches('"').trim_matches('\'');
        let results = ctx.search(pattern, 2, 20)?;
        if results.is_empty() {
            return Ok("No matches found.".to_string());
        }
        let mut output = Vec::new();
        for result in results {
            output.push(format!("Line {}: {}", result.line_num, result.match_line));
        }
        return Ok(output.join("\n"));
    }

    if code.starts_with("chunk(") && code.ends_with(')') {
        let args = &code[6..code.len() - 1];
        let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        let size: usize = parts.first().and_then(|s| s.parse().ok()).unwrap_or(2000);
        let overlap: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
        let chunks = ctx.chunk(size, overlap);
        let output: Vec<String> = chunks
            .iter()
            .map(|c| format!("Chunk {}: chars {}..{} - {}", c.index, c.start_char, c.end_char, &c.preview[..50.min(c.preview.len())]))
            .collect();
        return Ok(output.join("\n"));
    }

    if code == "head" || code == "head()" {
        let lines = ctx.lines(0, Some(10));
        return Ok(lines
            .iter()
            .map(|(n, l)| format!("{:>5} {}", n, l))
            .collect::<Vec<_>>()
            .join("\n"));
    }

    if code == "tail" || code == "tail()" {
        let start = ctx.line_count.saturating_sub(10);
        let lines = ctx.lines(start, None);
        return Ok(lines
            .iter()
            .map(|(n, l)| format!("{:>5} {}", n, l))
            .collect::<Vec<_>>()
            .join("\n"));
    }

    anyhow::bail!(
        "Unknown expression: {}. Supported: len, line_count, peek(start, end), lines(start, end), search(pattern), chunk(size, overlap), head, tail",
        code
    )
}

fn run_repl(context_id: String, initial_load: Option<std::path::PathBuf>) -> Result<()> {
    println!("{}", "MiniMax RLM Sandbox".bold().cyan());
    println!("Recursive Language Model - Local REPL Environment");
    println!("Type expressions or /help for commands.\n");

    // Detect and display system resources
    let resources = SystemResources::detect();
    resources.print_info();
    println!();

    let mut session = RlmSession::default();

    // Load initial file if provided
    if let Some(path) = initial_load {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        let source = path.to_string_lossy().to_string();
        session.load_context(&context_id, content, Some(source));

        let ctx = session.get_context(&context_id).unwrap();
        println!("{}", "Context loaded!".green());
        println!("  Lines: {} | Chars: {}\n", ctx.line_count, ctx.char_count);
    }

    let mut editor = Editor::<(), DefaultHistory>::new()?;
    let history_path = dirs::home_dir()
        .map(|h| h.join(".minimax").join("rlm_history"))
        .unwrap_or_default();
    let _ = editor.load_history(&history_path);

    loop {
        let prompt = format!("{}> ", "rlm".cyan());
        match editor.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                editor.add_history_entry(input)?;

                if input == "/exit" || input == "/quit" || input == "/q" {
                    break;
                }

                if input == "/help" {
                    print_repl_help();
                    continue;
                }

                if input == "/status" {
                    print_status(&session);
                    continue;
                }

                if let Some(rest) = input.strip_prefix("/load ") {
                    let path = Path::new(rest.trim());
                    match fs::read_to_string(path) {
                        Ok(content) => {
                            let source = path.to_string_lossy().to_string();
                            session.load_context(&context_id, content, Some(source));
                            let ctx = session.get_context(&context_id).unwrap();
                            println!("{}", "Loaded!".green());
                            println!("  Lines: {} | Chars: {}", ctx.line_count, ctx.char_count);
                        }
                        Err(e) => {
                            println!("{}: {}", "Error".red(), e);
                        }
                    }
                    continue;
                }

                if let Some(rest) = input.strip_prefix("/save ") {
                    let path = Path::new(rest.trim());
                    let json = serde_json::to_string_pretty(&session)?;
                    fs::write(path, json)?;
                    println!("Session saved to {}", path.display());
                    continue;
                }

                // Execute expression
                if let Some(ctx) = session.get_context(&context_id) {
                    match execute_expr(ctx, input) {
                        Ok(result) => println!("{}", result),
                        Err(e) => println!("{}: {}", "Error".red(), e),
                    }
                } else {
                    println!("{}: No context loaded. Use /load <path>", "Error".yellow());
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("{}: {}", "Error".red(), err);
                break;
            }
        }
    }

    let _ = editor.save_history(&history_path);
    Ok(())
}

fn print_repl_help() {
    println!("{}", "RLM Sandbox Commands".cyan().bold());
    println!();
    println!("  /load <path>   Load a file into context");
    println!("  /save <path>   Save session to file");
    println!("  /status        Show session status");
    println!("  /help          Show this help");
    println!("  /exit          Exit REPL");
    println!();
    println!("{}", "Expressions".cyan().bold());
    println!();
    println!("  len              Character count");
    println!("  line_count       Line count");
    println!("  head             First 10 lines");
    println!("  tail             Last 10 lines");
    println!("  peek(s, e)       Characters from s to e");
    println!("  lines(s, e)      Lines from s to e");
    println!("  search(pattern)  Regex search");
    println!("  chunk(size, overlap)  Split into chunks");
}

fn print_status(session: &RlmSession) {
    println!("{}", "Session Status".cyan().bold());
    println!("  Active context: {}", session.active_context);
    println!("  Loaded contexts: {}", session.contexts.len());
    for (id, ctx) in &session.contexts {
        println!("    {}: {} lines, {} chars", id, ctx.line_count, ctx.char_count);
        if let Some(ref source) = ctx.source_path {
            println!("      Source: {}", source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_lines(start: usize, end: usize) -> String {
        (start..=end)
            .map(|i| format!("{:>5} line {}", i, i))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn rlm_exec_len_head_tail_lines() -> Result<()> {
        let content = (1..=15)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let ctx = RlmContext::new("test", content, None);

        let len_output = execute_expr(&ctx, "len")?;
        assert_eq!(len_output, ctx.char_count.to_string());

        let head_output = execute_expr(&ctx, "head")?;
        assert_eq!(head_output, format_lines(1, 10));

        let tail_output = execute_expr(&ctx, "tail")?;
        assert_eq!(tail_output, format_lines(6, 15));

        let lines_output = execute_expr(&ctx, "lines(0, 10)")?;
        assert_eq!(lines_output, format_lines(1, 10));

        Ok(())
    }
}
