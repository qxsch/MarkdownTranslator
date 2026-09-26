//! JSON POST with retries, `retry-after` handling and jittered exponential backoff. Port of `azure/http.ts`.

use serde_json::Value;
use std::time::Duration;

#[derive(Debug)]
pub enum HttpError {
    Status { status: u16, body: String },
    Other(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Status { status, body } => write!(f, "HTTP {status}: {}", body.chars().take(500).collect::<String>()),
            HttpError::Other(m) => f.write_str(m),
        }
    }
}

const RETRYABLE: [u16; 7] = [408, 409, 429, 500, 502, 503, 504];

fn backoff(attempt: u32) -> Duration {
    let base = (1000.0 * 2f64.powi(attempt as i32 - 1)).min(30_000.0);
    Duration::from_millis((base * (0.75 + rand::random::<f64>() * 0.5)) as u64)
}

fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let num = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<f64>().ok());
    let ms = num("retry-after-ms").filter(|v| *v > 0.0).or_else(|| num("retry-after").map(|s| s * 1000.0)).filter(|v| v.is_finite() && *v > 0.0)?;
    Some(Duration::from_millis(ms.min(60_000.0) as u64))
}

pub async fn post_json(
    client: &reqwest::Client,
    url: &str,
    headers: &[(String, String)],
    body: &Value,
    timeout: Duration,
    attempts: u32,
) -> Result<Value, HttpError> {
    let payload = serde_json::to_vec(body).map_err(|e| HttpError::Other(e.to_string()))?;
    let mut last = HttpError::Other("no attempt made".into());
    for attempt in 1..=attempts {
        let mut req = client.post(url).timeout(timeout).header("content-type", "application/json").body(payload.clone());
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        match req.send().await {
            Ok(res) => {
                let status = res.status().as_u16();
                let wait = retry_after(res.headers());
                match res.text().await {
                    Ok(text) if (200..300).contains(&status) => match serde_json::from_str(&text) {
                        Ok(v) => return Ok(v),
                        Err(e) => last = HttpError::Other(format!("invalid JSON response: {e}")),
                    },
                    Ok(text) => {
                        let err = HttpError::Status { status, body: text };
                        if !RETRYABLE.contains(&status) || attempt == attempts {
                            return Err(err);
                        }
                        last = err;
                        tokio::time::sleep(wait.unwrap_or_else(|| backoff(attempt))).await;
                        continue;
                    }
                    Err(e) => last = HttpError::Other(e.to_string()),
                }
            }
            Err(e) => last = HttpError::Other(e.to_string()),
        }
        if attempt == attempts {
            return Err(last);
        }
        tokio::time::sleep(backoff(attempt)).await;
    }
    Err(last)
}
