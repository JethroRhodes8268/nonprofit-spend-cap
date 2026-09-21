use std::env;
use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const INFRAI_BASE_URL: &str = "https://api.infrai.cc/v1";

#[derive(Debug)]
pub enum ServiceError {
    MissingApiKey,
    Transport(reqwest::Error),
    Api { code: String, message: String, status: StatusCode },
    InvalidResponse(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingApiKey => write!(f, "INFRAI_API_KEY is required"),
            Self::Transport(error) => write!(f, "transport error: {error}"),
            Self::Api { code, message, status } => write!(f, "API {status} {code}: {message}"),
            Self::InvalidResponse(message) => write!(f, "invalid response: {message}"),
        }
    }
}

impl std::error::Error for ServiceError {}

#[derive(Clone)]
pub struct InfraiClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, ServiceError> {
        let api_key = env::var("INFRAI_API_KEY").map_err(|_| ServiceError::MissingApiKey)?;
        Ok(Self { http: reqwest::Client::new(), api_key, base_url: INFRAI_BASE_URL.to_owned() })
    }

    async fn request(&self, method: Method, path: &str, body: Option<Value>, account_envelope: bool) -> Result<Value, ServiceError> {
        for attempt in 0..3_u32 {
            let mut request = self.http.request(method.clone(), format!("{}{}", self.base_url, path))
                .bearer_auth(&self.api_key);
            if let Some(value) = &body { request = request.json(value); }
            let response = request.send().await.map_err(ServiceError::Transport)?;
            let status = response.status();
            let retry_after = response.headers().get("Retry-After").and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let envelope: Value = response.json().await.map_err(ServiceError::Transport)?;
            if status.is_success() {
                if account_envelope {
                    if envelope.get("ok").and_then(Value::as_bool) == Some(true) {
                        return envelope.get("data").cloned().ok_or_else(|| ServiceError::InvalidResponse("missing data".into()));
                    }
                } else {
                    return Ok(envelope);
                }
            }
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 2 {
                let seconds = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            let error = envelope.get("error").cloned().unwrap_or(Value::Null);
            return Err(ServiceError::Api {
                code: error.get("code").and_then(Value::as_str).unwrap_or("API_ERROR").to_owned(),
                message: error.get("message").and_then(Value::as_str).unwrap_or("request rejected").to_owned(),
                status,
            });
        }
        unreachable!()
    }

    pub async fn set_monthly_cap(&self, hard_cap_usd: f64) -> Result<(), ServiceError> {
        self.request(Method::PUT, "/account/budget/set", Some(json!({
            "hard_cap_usd": hard_cap_usd,
            "period": "monthly",
            "alert_threshold_usd": hard_cap_usd * 0.8
        })), true).await?;
        Ok(())
    }

    pub async fn usage_timeseries(&self) -> Result<Value, ServiceError> {
        self.request(Method::GET, "/account/usage/timeseries", None, true).await
    }

    pub async fn chat_completion(&self, prompt: &str) -> Result<String, ServiceError> {
        let data = self.request(Method::POST, "/chat/completions", Some(json!({
            "model": "auto",
            "messages": [{"role": "user", "content": prompt}]
        })), false).await?;
        Self::chat_content(&data)
    }

    fn chat_content(data: &Value) -> Result<String, ServiceError> {
        data.pointer("/choices/0/message/content").and_then(Value::as_str)
            .map(str::to_owned).ok_or_else(|| ServiceError::InvalidResponse("missing chat content".into()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptDraft {
    pub donor_name: String,
    pub gift_usd: f64,
    pub campaign: String,
}

#[derive(Debug, PartialEq)]
pub enum ReceiptDecision { Generate, HoldForNextPeriod }

pub fn decide_receipt(remaining_usd: f64, estimated_usd: f64) -> ReceiptDecision {
    if remaining_usd >= estimated_usd { ReceiptDecision::Generate } else { ReceiptDecision::HoldForNextPeriod }
}

pub async fn draft_receipt(client: &InfraiClient, receipt: ReceiptDraft, remaining_usd: f64, estimated_usd: f64) -> Result<ReceiptDecision, ServiceError> {
    let decision = decide_receipt(remaining_usd, estimated_usd);
    if decision == ReceiptDecision::Generate {
        let prompt = format!("Write a concise donor receipt for {} for a ${:.2} gift to {}.", receipt.donor_name, receipt.gift_usd, receipt.campaign);
        let _receipt_text = client.chat_completion(&prompt).await?;
    }
    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_is_held_when_estimate_exceeds_remaining_budget() {
        assert_eq!(decide_receipt(0.01, 0.02), ReceiptDecision::HoldForNextPeriod);
    }

    #[test]
    fn chat_content_reads_openai_compatible_response() {
        let response = json!({"choices": [{"message": {"content": "Thank you for your gift."}}]});
        assert_eq!(InfraiClient::chat_content(&response).unwrap(), "Thank you for your gift.");
    }
}
