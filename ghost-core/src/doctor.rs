use crate::ipc_client::{self, DoctorStatsResponse, HistoryResponse};
use std::fs::File;
use std::io::Write;
use tokio::runtime::Handle;

pub enum DoctorCommand {
    Default { limit: usize },
    Search { query: String, limit: usize },
    Stats,
    Export { format: String, out_path: String },
}

impl DoctorCommand {
    pub fn parse(raw_cmd: &str) -> Self {
        let parts: Vec<&str> = raw_cmd.trim().split_whitespace().collect();

        // Check if "--stats" flag is present
        if parts.iter().any(|&p| p == "--stats" || p == "-s") {
            return DoctorCommand::Stats;
        }

        // Check for "--search <query>"
        if let Some(pos) = parts.iter().position(|&p| p == "--search" || p == "-q") {
            if pos + 1 < parts.len() {
                let query = parts[pos + 1].trim_matches('"').trim_matches('\'').to_string();
                let limit = Self::extract_limit(&parts).unwrap_or(10);
                return DoctorCommand::Search { query, limit };
            }
        }

        // Check for "--export <format>"
        if let Some(pos) = parts.iter().position(|&p| p == "--export" || p == "-e") {
            if pos + 1 < parts.len() {
                let format = parts[pos + 1].to_lowercase();
                let out_path = if let Some(out_pos) = parts.iter().position(|&p| p == "--out" || p == "-o") {
                    if out_pos + 1 < parts.len() {
                        parts[out_pos + 1].to_string()
                    } else {
                        format!("ghost_report.{}", if format == "json" { "json" } else { "md" })
                    }
                } else {
                    format!("ghost_report.{}", if format == "json" { "json" } else { "md" })
                };

                return DoctorCommand::Export { format, out_path };
            }
        }

        let limit = Self::extract_limit(&parts).unwrap_or(5);
        DoctorCommand::Default { limit }
    }

    fn extract_limit(parts: &[&str]) -> Option<usize> {
        if let Some(pos) = parts.iter().position(|&p| p == "--limit" || p == "-l" || p == "-n") {
            if pos + 1 < parts.len() {
                return parts[pos + 1].parse::<usize>().ok();
            }
        }
        None
    }
}

pub fn handle_doctor_cli(raw_cmd: &str, handle: &Handle) -> String {
    let command = DoctorCommand::parse(raw_cmd);

    match command {
        DoctorCommand::Default { limit } => {
            let mut out = String::from("\r\n\x1b[35;1m=== Ghost Doctor: Recent Interceptions ===\x1b[0m\r\n");
            match handle.block_on(ipc_client::fetch_history(limit, None)) {
                Ok(hist) => out.push_str(&format_history_table(&hist)),
                Err(_) => out.push_str("\x1b[31mFailed to connect to ghost_daemon at http://127.0.0.1:8000\x1b[0m\r\n"),
            }
            out
        }

        DoctorCommand::Search { query, limit } => {
            let mut out = format!("\r\n\x1b[35;1m=== Ghost Doctor: Search Results for '{}' ===\x1b[0m\r\n", query);
            match handle.block_on(ipc_client::search_history(&query, limit)) {
                Ok(hist) => {
                    if hist.history.is_empty() {
                        out.push_str(&format!("No historical failures matched '{}'.\r\n", query));
                    } else {
                        out.push_str(&format_history_table(&hist));
                    }
                }
                Err(_) => out.push_str("\x1b[31mFailed to reach ghost_daemon search endpoint.\x1b[0m\r\n"),
            }
            out
        }

        DoctorCommand::Stats => {
            let mut out = String::from("\r\n\x1b[35;1m=== Ghost Doctor: Diagnostic Analytics ===\x1b[0m\r\n");
            match handle.block_on(ipc_client::fetch_stats()) {
                Ok(stats) => out.push_str(&format_stats_summary(&stats)),
                Err(_) => out.push_str("\x1b[31mFailed to reach ghost_daemon stats endpoint.\x1b[0m\r\n"),
            }
            out
        }

        DoctorCommand::Export { format, out_path } => {
            let mut out = format!("\r\n\x1b[35;1m=== Ghost Doctor: Export Report ===\x1b[0m\r\n");
            match handle.block_on(ipc_client::fetch_history(100, None)) {
                Ok(hist) => {
                    let export_res = if format == "json" {
                        export_json(&hist, &out_path)
                    } else {
                        export_markdown(&hist, &out_path)
                    };

                    match export_res {
                        Ok(_) => out.push_str(&format!("\x1b[32mSuccessfully exported report to '{}'\x1b[0m\r\n", out_path)),
                        Err(e) => out.push_str(&format!("\x1b[31mFailed to write export file: {}\x1b[0m\r\n", e)),
                    }
                }
                Err(_) => out.push_str("\x1b[31mFailed to retrieve history for report generation.\x1b[0m\r\n"),
            }
            out
        }
    }
}

fn format_history_table(hist: &HistoryResponse) -> String {
    if hist.history.is_empty() {
        return "No intercepted failures found.\r\n".to_string();
    }

    let mut buf = String::new();
    for r in &hist.history {
        let engine = r.engine_source.as_deref().unwrap_or("unknown");
        let ws = r.workspace.as_deref().unwrap_or("global");

        buf.push_str(&format!(
            "\x1b[33m[{}]\x1b[0m \x1b[1mCommand:\x1b[0m {} \x1b[90m(Exit: {}, Engine: {}, Workspace: {})\x1b[0m\r\n  \x1b[36mDiagnosis:\x1b[0m {}\r\n  \x1b[32mSuggested Fix:\x1b[0m {}\r\n\r\n",
            r.timestamp, r.command, r.exit_code, engine, ws, r.diagnosis, r.suggested_fix
        ));
    }
    buf
}

fn format_stats_summary(stats: &DoctorStatsResponse) -> String {
    let mut buf = String::new();
    buf.push_str(&format!("\x1b[1mTotal Intercepted Failures:\x1b[0m {}\r\n\r\n", stats.total_failures));

    buf.push_str("\x1b[33;1mTop Failing Commands:\x1b[0m\r\n");
    if stats.top_failing_commands.is_empty() {
        buf.push_str("  (None recorded)\r\n");
    } else {
        for cmd in &stats.top_failing_commands {
            buf.push_str(&format!("  - \x1b[1m{}\x1b[0m ({} failures)\r\n", cmd.command, cmd.failure_count));
        }
    }

    buf.push_str("\r\n\x1b[34;1mWorkspace Failure Distribution:\x1b[0m\r\n");
    if stats.workspace_breakdown.is_empty() {
        buf.push_str("  (None recorded)\r\n");
    } else {
        for ws in &stats.workspace_breakdown {
            buf.push_str(&format!("  - \x1b[90m{}\x1b[0m: {} failures\r\n", ws.workspace, ws.failure_count));
        }
    }

    buf.push_str("\r\n\x1b[36;1mAI Engine Invocations:\x1b[0m\r\n");
    for (engine, count) in &stats.engine_sources {
        buf.push_str(&format!("  - \x1b[1m{}\x1b[0m: {} calls\r\n", engine, count));
    }
    buf.push_str("\r\n");
    buf
}

fn export_markdown(hist: &HistoryResponse, path: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "# 👻 Ghost in the Shell — Audit Report\n")?;
    writeln!(file, "Total Records: {}\n", hist.count)?;
    writeln!(file, "| Timestamp | Command | Exit Code | Engine | Diagnosis | Fix |")?;
    writeln!(file, "|---|---|---|---|---|---|")?;

    for r in &hist.history {
        let engine = r.engine_source.as_deref().unwrap_or("unknown");
        writeln!(
            file,
            "| {} | `{}` | {} | {} | {} | `{}` |",
            r.timestamp,
            r.command.replace('|', "\\|"),
            r.exit_code,
            engine,
            r.diagnosis.replace('|', "\\|"),
            r.suggested_fix.replace('|', "\\|")
        )?;
    }
    Ok(())
}

fn export_json(hist: &HistoryResponse, path: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let serialized = serde_json::to_string_pretty(hist).unwrap_or_default();
    file.write_all(serialized.as_bytes())?;
    Ok(())
}