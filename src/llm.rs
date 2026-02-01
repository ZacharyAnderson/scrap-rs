use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ApiError {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
}

pub fn summarize_note(title: &str, content: &str) -> Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        anyhow::anyhow!(
            "ANTHROPIC_API_KEY not set. Add 'export ANTHROPIC_API_KEY=your_key' to your ~/.zshrc"
        )
    })?;

    let request = ApiRequest {
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 1024,
        system: "You are a note summarizer. Summarize the given note concisely. \
                 Return your summary as well-formatted markdown with bullet points, \
                 headers, and emphasis where appropriate. Keep it brief but informative."
            .to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: format!("Summarize this note titled \"{}\":\n\n{}", title, content),
        }],
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()?;

    let status = response.status();
    let body = response.text()?;

    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<ApiError>(&body) {
            bail!("Anthropic API error: {}", err.error.message);
        }
        bail!("Anthropic API error ({}): {}", status, body);
    }

    let api_response: ApiResponse = serde_json::from_str(&body)?;
    let summary = api_response
        .content
        .into_iter()
        .filter_map(|b| b.text)
        .collect::<Vec<_>>()
        .join("\n");

    if summary.is_empty() {
        bail!("Empty response from API");
    }

    Ok(summary)
}
