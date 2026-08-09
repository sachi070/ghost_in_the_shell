use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct DiagnoseRequest<'a> {
    pub command: &'a str,
    pub exit_code: i32,
    pub output_context: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct DiagnoseResponse {
    pub diagnosis: String,
    pub suggested_fix: String,
    pub explanation: String,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Deserialize)]
pub struct HistoryRecord {
    pub id: i32,
    pub timestamp: String,
    pub command: String,
    pub exit_code: i32,
    pub diagnosis: String,
    pub suggested_fix: String,
}

#[derive(Debug, Deserialize)]
pub struct HistoryResponse {
    pub status: String,
    pub count: usize,
    pub history: Vec<HistoryRecord>,
}

pub async fn send_diagnose_request(
    command: &str,
    exit_code: i32,
    output_context: &str,
) -> Result<DiagnoseResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let payload = DiagnoseRequest {
        command,
        exit_code,
        output_context,
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

pub async fn fetch_history(limit: usize) -> Result<HistoryResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://127.0.0.1:8000/history?limit={}", limit))
        .send()
        .await?
        .json::<HistoryResponse>()
        .await?;

    Ok(res)
}