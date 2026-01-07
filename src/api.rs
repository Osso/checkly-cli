use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const BASE_URL: &str = "https://api.checklyhq.com";

pub struct Client {
    http: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub name: String,
    #[serde(rename = "checkType")]
    pub check_type: String,
    pub activated: bool,
    pub muted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckStatus {
    #[serde(rename = "checkId")]
    pub check_id: String,
    pub name: String,
    #[serde(rename = "hasFailures")]
    pub has_failures: bool,
    #[serde(rename = "hasErrors")]
    pub has_errors: bool,
    #[serde(rename = "isDegraded")]
    pub is_degraded: bool,
    #[serde(rename = "longestRun")]
    pub longest_run: Option<i64>,
    #[serde(rename = "shortestRun")]
    pub shortest_run: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CheckResultsResponse {
    entries: Vec<CheckResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub id: String,
    #[serde(rename = "checkId")]
    pub check_id: String,
    #[serde(rename = "hasFailures")]
    pub has_failures: bool,
    #[serde(rename = "hasErrors")]
    pub has_errors: bool,
    #[serde(rename = "isDegraded")]
    pub is_degraded: bool,
    #[serde(rename = "runLocation")]
    pub run_location: Option<String>,
    #[serde(rename = "startedAt")]
    pub started_at: Option<String>,
    #[serde(rename = "stoppedAt")]
    pub stopped_at: Option<String>,
    #[serde(rename = "responseTime")]
    pub response_time: Option<i64>,
    #[serde(rename = "checkRunId")]
    pub check_run_id: Option<i64>,
    // API check specific fields
    #[serde(rename = "statusCode")]
    pub status_code: Option<i32>,
    // Browser check specific fields
    #[serde(rename = "attempts")]
    pub attempts: Option<i32>,
    // Raw response for additional data
    #[serde(flatten)]
    pub extra: Value,
}

impl Client {
    pub fn new(api_key: &str, account_id: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .context("Invalid API key")?,
        );
        headers.insert(
            "X-Checkly-Account",
            HeaderValue::from_str(account_id).context("Invalid account ID")?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { http })
    }

    async fn get(&self, endpoint: &str) -> Result<Value> {
        let url = format!("{}{}", BASE_URL, endpoint);
        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {} - {}", status, body);
        }

        resp.json().await.context("Failed to parse JSON response")
    }

    pub async fn list_checks(&self) -> Result<Vec<Check>> {
        let value = self.get("/v1/checks").await?;
        let checks: Vec<Check> = serde_json::from_value(value)?;
        Ok(checks)
    }

    pub async fn get_statuses(&self) -> Result<Vec<CheckStatus>> {
        let value = self.get("/v1/check-statuses").await?;
        let statuses: Vec<CheckStatus> = serde_json::from_value(value)?;
        Ok(statuses)
    }

    pub async fn get_results(
        &self,
        check_id: &str,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<Vec<CheckResult>> {
        let mut endpoint = format!("/v2/check-results/{}", check_id);
        let mut params = vec![];

        if let Some(f) = from {
            params.push(format!("from={}", f));
        }
        if let Some(t) = to {
            params.push(format!("to={}", t));
        }

        if !params.is_empty() {
            endpoint = format!("{}?{}", endpoint, params.join("&"));
        }

        let value = self.get(&endpoint).await?;
        let response: CheckResultsResponse = serde_json::from_value(value)?;
        Ok(response.entries)
    }
}
