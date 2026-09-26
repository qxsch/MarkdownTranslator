//! Azure OpenAI v1 chat completions with strict JSON-schema structured output, and Azure Translator (NMT) v3.
//! Ports of `azure/openai.ts` and `azure/nmt.ts`.

use super::auth::{AzureAuth, Service};
use super::http::post_json;
use crate::config::AppConfig;
use crate::regexutil::RegexExt;
use crate::rx;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub calls: u64,
}

impl Usage {
    fn add(&mut self, prompt: u64, completion: u64) {
        self.prompt_tokens += prompt;
        self.completion_tokens += completion;
        self.calls += 1;
    }

    pub fn total(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Token usage per deployment and per purpose (the JSON schema name: `translations`, `review`, `analysis`).
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMeter {
    pub by_deployment: BTreeMap<String, Usage>,
    pub by_purpose: BTreeMap<String, Usage>,
}

#[derive(Debug)]
pub enum ChatError {
    /// The model hit the output limit; the caller may split the batch.
    Truncated,
    Other(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatError::Truncated => f.write_str("model output truncated"),
            ChatError::Other(m) => f.write_str(m),
        }
    }
}

pub struct ChatClient {
    cfg: AppConfig,
    auth: Arc<AzureAuth>,
    limiter: Arc<Semaphore>,
    client: reqwest::Client,
    pub usage: Mutex<UsageMeter>,
}

impl ChatClient {
    pub fn new(cfg: &AppConfig, auth: Arc<AzureAuth>, limiter: Arc<Semaphore>, client: reqwest::Client) -> Self {
        ChatClient { cfg: cfg.clone(), auth, limiter, client, usage: Mutex::new(UsageMeter::default()) }
    }

    pub fn available(&self) -> bool {
        self.cfg.openai_endpoint.is_some()
    }

    pub async fn json(&self, deployment: &str, messages: Vec<(&str, String)>, schema_name: &str, schema: Value, reasoning: Option<&str>) -> Result<Value, ChatError> {
        let endpoint = self.cfg.openai_endpoint.as_deref().ok_or_else(|| ChatError::Other("AZURE_OPENAI_ENDPOINT is not configured".into()))?;
        let mut body = json!({
            "model": deployment,
            "messages": messages.iter().map(|(role, content)| json!({ "role": role, "content": content })).collect::<Vec<_>>(),
            "response_format": { "type": "json_schema", "json_schema": { "name": schema_name, "strict": true, "schema": schema } },
            "max_completion_tokens": 64000,
        });
        if let Some(r) = reasoning.filter(|r| !r.is_empty()) {
            body["reasoning_effort"] = Value::String(r.into());
        }
        let res = {
            let _permit = self.limiter.acquire().await.map_err(|e| ChatError::Other(e.to_string()))?;
            let headers = self.auth.headers(Service::OpenAi).await.map_err(ChatError::Other)?;
            post_json(&self.client, &format!("{endpoint}/openai/v1/chat/completions"), &headers, &body, Duration::from_millis(self.cfg.request_timeout_ms), self.cfg.max_attempts)
                .await
                .map_err(|e| ChatError::Other(e.to_string()))?
        };
        let prompt = res["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
        let completion = res["usage"]["completion_tokens"].as_u64().unwrap_or(0);
        {
            let mut u = self.usage.lock().unwrap();
            u.by_deployment.entry(deployment.to_string()).or_default().add(prompt, completion);
            u.by_purpose.entry(schema_name.to_string()).or_default().add(prompt, completion);
        }
        let choice = &res["choices"][0];
        if choice["finish_reason"].as_str() == Some("length") {
            return Err(ChatError::Truncated);
        }
        let content = choice["message"]["content"].as_str().filter(|c| !c.is_empty()).ok_or_else(|| {
            let refusal = choice["message"]["refusal"].as_str().map(|r| format!(": {r}")).unwrap_or_default();
            ChatError::Other(format!("empty model response{refusal}"))
        })?;
        serde_json::from_str(content).map_err(|e| ChatError::Other(format!("model returned invalid JSON: {e}")))
    }
}

/// Azure Translator (neural machine translation) v3, HTML mode so inline tags survive.
pub struct NmtClient {
    cfg: AppConfig,
    auth: Arc<AzureAuth>,
    limiter: Arc<Semaphore>,
    client: reqwest::Client,
}

impl NmtClient {
    pub fn new(cfg: &AppConfig, auth: Arc<AzureAuth>, limiter: Arc<Semaphore>, client: reqwest::Client) -> Self {
        NmtClient { cfg: cfg.clone(), auth, limiter, client }
    }

    pub fn available(&self) -> bool {
        self.cfg.translator_endpoint.is_some()
    }

    pub async fn translate(&self, texts: &[String], to: &str, from: Option<&str>) -> Result<Vec<String>, String> {
        let endpoint = self.cfg.translator_endpoint.as_deref().ok_or("AZURE_TRANSLATOR_ENDPOINT is not configured")?;
        let mut out = Vec::with_capacity(texts.len());
        // Service limits: 1000 elements and 50,000 characters per request.
        let mut batches: Vec<Vec<&String>> = Vec::new();
        let mut cur: Vec<&String> = Vec::new();
        let mut chars = 0;
        for t in texts {
            let len = crate::jsstr::u16len(t);
            if !cur.is_empty() && (cur.len() >= 100 || chars + len > 40000) {
                batches.push(std::mem::take(&mut cur));
                chars = 0;
            }
            cur.push(t);
            chars += len;
        }
        if !cur.is_empty() {
            batches.push(cur);
        }
        for batch in batches {
            let mut url = reqwest::Url::parse(&format!("{endpoint}/translator/text/v3.0/translate")).map_err(|e| e.to_string())?;
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("api-version", "3.0").append_pair("to", to).append_pair("textType", "html");
                if let Some(f) = from {
                    q.append_pair("from", f);
                }
            }
            let body = Value::Array(batch.iter().map(|t| json!({ "Text": t })).collect());
            let _permit = self.limiter.acquire().await.map_err(|e| e.to_string())?;
            let headers = self.auth.headers(Service::Translator).await?;
            let res = post_json(&self.client, url.as_str(), &headers, &body, Duration::from_millis(self.cfg.request_timeout_ms), self.cfg.max_attempts).await.map_err(|e| e.to_string())?;
            for r in res.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
                out.push(r["translations"][0]["text"].as_str().unwrap_or("").to_string());
            }
        }
        Ok(out)
    }
}

rx!(X_TAG, r"<x(\d+)\/>");
rx!(G_TAG, r"<(\/?)g(\d+)>");
rx!(NMT_SPAN, r#"<span class="notranslate" id="x(\d+)">[^<]*<\/span>"#);
rx!(NMT_B, r#"<b id="g(\d+)">|<\/b>"#);

/// Masked segment -> HTML that Azure Translator keeps intact.
pub fn masked_to_html(masked: &str) -> String {
    let s = X_TAG().replace_all_with(masked, |m| format!("<span class=\"notranslate\" id=\"x{0}\">x{0}</span>", m.g(1)));
    G_TAG().replace_all_with(&s, |m| if m.g(1) == "/" { "</b>".to_string() } else { format!("<b id=\"g{}\">", m.g(2)) })
}

pub fn html_to_masked(html: &str) -> String {
    let out = NMT_SPAN().replace_all_with(html, |m| format!("<x{}/>", m.g(1)));
    // Rebuild paired tags by matching open/close order.
    let mut stack: Vec<String> = Vec::new();
    NMT_B().replace_all_with(&out, |m| match m.group(1) {
        Some(n) => {
            stack.push(n.to_string());
            format!("<g{n}>")
        }
        None => stack.pop().map(|top| format!("</g{top}>")).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmt_round_trip() {
        let m = "Use <g1>the <x2/> file</g1> now";
        let h = masked_to_html(m);
        assert_eq!(h, "Use <b id=\"g1\">the <span class=\"notranslate\" id=\"x2\">x2</span> file</b> now");
        assert_eq!(html_to_masked(&h), m);
    }
}
