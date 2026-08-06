use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct DiagnoseRequest {
    pub command: String,
    pub exit_code: i32,
    pub output_context: String,
    pub cwd: String,
    pub project_type: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct DiagnoseResponse {
    pub diagnosis: String,
    pub suggested_fix: String,
    pub explanation: String,
    pub confidence: f32,
    pub source: String,
}

pub async fn send_diagnose_request(
    command: &str,
    exit_code: i32,
    context: &str,
) -> Result<DiagnoseResponse> {
    let client = reqwest::Client::new();
    let cwd = std::env::current_dir()?
        .to_string_lossy()
        .to_string();

    let payload = DiagnoseRequest {
        command: command.to_string(),
        exit_code,
        output_context: context.to_string(),
        cwd,
        project_type: "unknown".to_string(),
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