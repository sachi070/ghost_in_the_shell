use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize)]
pub struct DiagnoseRequest<'a> {
    pub command: &'a str,
    pub exit_code: i32,
    pub output_context: &'a str,
    pub cwd: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiagnoseResponse {
    pub diagnosis: String,
    pub suggested_fix: String,
    pub explanation: String,
    pub confidence: f32,
    pub source: String,
    pub workspace: String,
    pub recurrence_count: i32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct HistoryRecord {
    pub id: i32,
    pub timestamp: String,
    pub workspace: Option<String>,
    pub command: String,
    pub exit_code: i32,
    pub diagnosis: String,
    pub suggested_fix: String,
    pub engine_source: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct HistoryResponse {
    pub status: String,
    pub count: usize,
    pub history: Vec<HistoryRecord>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CommandStat {
    pub command: String,
    pub failure_count: i64,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WorkspaceStat {
    pub workspace: String,
    pub failure_count: i64,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DoctorStatsResponse {
    pub total_failures: i64,
    pub top_failing_commands: Vec<CommandStat>,
    pub workspace_breakdown: Vec<WorkspaceStat>,
    pub engine_sources: std::collections::HashMap<String, i64>,
}

pub async fn send_diagnose_request(
    command: &str,
    exit_code: i32,
    output_context: &str,
) -> Result<DiagnoseResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let current_dir = env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let payload = DiagnoseRequest {
        command,
        exit_code,
        output_context,
        cwd: current_dir,
    };

    let res = client
        .post("http://127.0.0.1:8000/diagnose")
        .json(&payload)
        .send()
        .await?
        .json::<DiagnoseResponse>()
        .await?;

    Ok(res)
}

pub async fn fetch_history(
    limit: usize,
    workspace: Option<&str>,
) -> Result<HistoryResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let mut url = format!("http://127.0.0.1:8000/history?limit={}", limit);
    if let Some(ws) = workspace {
        url.push_str(&format!("&workspace={}", urlencoding_simple(ws)));
    }

    let res = client
        .get(url)
        .send()
        .await?
        .json::<HistoryResponse>()
        .await?;

    Ok(res)
}

pub async fn search_history(
    query: &str,
    limit: usize,
) -> Result<HistoryResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!(
        "http://127.0.0.1:8000/search?q={}&limit={}",
        urlencoding_simple(query),
        limit
    );

    let res = client
        .get(url)
        .send()
        .await?
        .json::<HistoryResponse>()
        .await?;

    Ok(res)
}

pub async fn fetch_stats() -> Result<DoctorStatsResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let res = client
        .get("http://127.0.0.1:8000/stats")
        .send()
        .await?
        .json::<DoctorStatsResponse>()
        .await?;

    Ok(res)
}

fn urlencoding_simple(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "%20".to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}