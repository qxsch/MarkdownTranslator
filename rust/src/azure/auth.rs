//! Authentication for Azure AI services: API key, a static token, or Microsoft Entra ID with the credential
//! chain of `DefaultAzureCredential` (preceded by the user-assigned managed identity when `AZURE_CLIENT_ID` is
//! set, as in `azure/http.ts`). Tokens are cached until five minutes before they expire.

use crate::config::AppConfig;
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const SCOPE: &str = "https://cognitiveservices.azure.com/.default";
const RESOURCE: &str = "https://cognitiveservices.azure.com";

/// Azure PowerShell token script, after `AzurePowerShellCredential` in @azure/identity: resource and tenant come
/// from the environment (never spliced into the script), and Az.Accounts 2.17 up to 5.0 is asked for a
/// SecureString, since it otherwise announces the upcoming type change on stdout.
const POWERSHELL_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; $WarningPreference = 'SilentlyContinue'; \
    $m = Import-Module Az.Accounts -MinimumVersion 2.2.0 -PassThru | Select-Object -First 1; \
    $p = @{ ResourceUrl = $env:AZURE_IDENTITY_POWERSHELL_RESOURCE }; \
    if ($env:AZURE_IDENTITY_POWERSHELL_TENANT_ID) { $p['TenantId'] = $env:AZURE_IDENTITY_POWERSHELL_TENANT_ID }; \
    if ($m.Version -ge [version]'2.17.0' -and $m.Version -lt [version]'5.0.0') { $p['AsSecureString'] = $true }; \
    $t = Get-AzAccessToken @p; $v = $t.Token; \
    if ($v -is [System.Security.SecureString]) { $v = [System.Net.NetworkCredential]::new('', $v).Password }; \
    @{ token = $v; expires_on = $t.ExpiresOn.ToUnixTimeSeconds() } | ConvertTo-Json -Compress";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    OpenAi,
    Translator,
}

struct Token {
    value: String,
    expires: u64,
}

#[derive(Default)]
struct TokenState {
    token: Option<Token>,
    /// When the last acquisition failed, and why.
    failure: Option<(Instant, String)>,
}

pub struct AzureAuth {
    cfg: AppConfig,
    client: reqwest::Client,
    state: Mutex<TokenState>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Tenant ids are GUIDs or domain names (`checkTenantId` in @azure/identity). Anything else is rejected before it
/// reaches a URL or a command line.
fn valid_tenant(t: &str) -> bool {
    !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// Credential tools run in a trusted directory, as in @azure/identity: cmd.exe looks for `az` in the working
/// directory before PATH, and that is often the checkout being translated.
fn safe_working_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
    } else {
        PathBuf::from("/bin")
    }
}

/// The JSON object a credential tool printed. PowerShell modules can print warnings and banners on stdout too, so
/// when the whole output is not JSON, the last line that is a JSON object counts (like `parseJsonToken`).
fn json_in_output(out: &str) -> Option<Value> {
    let out = out.trim_start_matches('\u{feff}').trim();
    if let Ok(v) = serde_json::from_str::<Value>(out) {
        return Some(v);
    }
    out.lines().rev().map(str::trim).filter(|l| l.starts_with('{') && l.ends_with('}')).find_map(|l| serde_json::from_str::<Value>(l).ok().filter(Value::is_object))
}

/// Days from 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `2026-09-26T17:00:00Z`, `2026-09-26T17:00:00.123+02:00` -> Unix seconds.
fn parse_rfc3339(s: &str) -> Option<u64> {
    let s = s.trim();
    let num = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, se) = (num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let mut rest = &s[19..];
    if let Some(r) = rest.strip_prefix('.') {
        rest = r.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    let offset = match rest {
        "" | "Z" | "z" => 0,
        r if r.len() == 6 && (r.starts_with('+') || r.starts_with('-')) => {
            let sign = if r.starts_with('-') { -1 } else { 1 };
            sign * (r[1..3].parse::<i64>().ok()? * 3600 + r[4..6].parse::<i64>().ok()? * 60)
        }
        _ => return None,
    };
    let secs = days_from_civil(y, mo, d) * 86400 + h * 3600 + mi * 60 + se - offset;
    u64::try_from(secs).ok()
}

fn expiry_from(v: &Value) -> u64 {
    let field = |k: &str| v.get(k).and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.trim().parse::<u64>().ok())));
    if let Some(e) = field("expires_on") {
        return e;
    }
    if let Some(e) = field("expires_in") {
        return now() + e;
    }
    if let Some(s) = v.get("expiresOn").and_then(|x| x.as_str()) {
        if let Some(e) = parse_rfc3339(s) {
            return e;
        }
    }
    now() + 45 * 60
}

fn token_from(v: &Value) -> Option<String> {
    ["access_token", "accessToken", "token"].iter().find_map(|k| v.get(*k).and_then(|t| t.as_str())).map(String::from)
}

impl AzureAuth {
    pub fn new(cfg: &AppConfig, client: reqwest::Client) -> Self {
        AzureAuth { cfg: cfg.clone(), client, state: Mutex::new(TokenState::default()) }
    }

    pub fn mode(&self) -> &'static str {
        if self.cfg.api_key.is_some() {
            "api-key"
        } else {
            "entra"
        }
    }

    pub async fn headers(&self, service: Service) -> Result<Vec<(String, String)>, String> {
        if let Some(key) = &self.cfg.api_key {
            return Ok(match service {
                Service::OpenAi => vec![("api-key".into(), key.clone())],
                Service::Translator => {
                    let mut h = vec![("Ocp-Apim-Subscription-Key".into(), key.clone())];
                    if let Some(r) = &self.cfg.translator_region {
                        h.push(("Ocp-Apim-Subscription-Region".into(), r.clone()));
                    }
                    h
                }
            });
        }
        Ok(vec![("Authorization".into(), format!("Bearer {}", self.bearer().await?))])
    }

    async fn bearer(&self) -> Result<String, String> {
        if let Some(t) = &self.cfg.static_access_token {
            return Ok(t.clone());
        }
        let asked = Instant::now();
        let mut state = self.state.lock().await;
        if let Some(t) = state.token.as_ref() {
            if t.expires > now() + 5 * 60 {
                return Ok(t.value.clone());
            }
        }
        // Requests that queued up behind a failed acquisition share its answer instead of running the whole chain
        // again one after another; later requests (the engine's retry rounds) try again.
        if let Some((at, e)) = &state.failure {
            if *at >= asked {
                return Err(e.clone());
            }
        }
        match self.acquire().await {
            Ok(t) => {
                let value = t.value.clone();
                *state = TokenState { token: Some(t), failure: None };
                Ok(value)
            }
            Err(e) => {
                state.failure = Some((Instant::now(), e.clone()));
                Err(e)
            }
        }
    }

    async fn acquire(&self) -> Result<Token, String> {
        let mut errors = Vec::new();
        let client_id = self.cfg.managed_identity_client_id.clone();
        if client_id.is_some() {
            match self.managed_identity(client_id.as_deref()).await {
                Ok(t) => return Ok(t),
                Err(e) => errors.push(format!("ManagedIdentityCredential (client id): {e}")),
            }
        }
        macro_rules! attempt {
            ($name:expr, $f:expr) => {
                match $f.await {
                    Ok(t) => return Ok(t),
                    Err(e) => errors.push(format!("{}: {}", $name, e)),
                }
            };
        }
        attempt!("EnvironmentCredential", self.environment());
        attempt!("WorkloadIdentityCredential", self.workload_identity());
        if client_id.is_none() {
            attempt!("ManagedIdentityCredential", self.managed_identity(None));
        }
        attempt!("AzureCliCredential", self.azure_cli());
        attempt!("AzurePowerShellCredential", self.azure_powershell());
        attempt!("AzureDeveloperCliCredential", self.azure_developer_cli());
        Err(format!("could not acquire a Microsoft Entra token for Azure AI services:\n  {}", errors.join("\n  ")))
    }

    fn authority(&self) -> String {
        env("AZURE_AUTHORITY_HOST").unwrap_or_else(|| "https://login.microsoftonline.com".into()).trim_end_matches('/').to_string()
    }

    fn tenant(&self) -> Result<Option<String>, String> {
        match self.cfg.tenant_id.clone().or_else(|| env("AZURE_TENANT_ID")) {
            Some(t) if !valid_tenant(&t) => Err(format!("invalid tenant id \"{t}\" (only letters, digits, '-' and '.' are allowed)")),
            t => Ok(t),
        }
    }

    async fn token_request(&self, tenant: &str, form: &[(&str, &str)]) -> Result<Token, String> {
        let url = format!("{}/{tenant}/oauth2/v2.0/token", self.authority());
        let res = self.client.post(url).form(form).timeout(Duration::from_secs(30)).send().await.map_err(|e| e.to_string())?;
        let status = res.status();
        let v: Value = res.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(v.get("error_description").and_then(|d| d.as_str()).unwrap_or("token request failed").lines().next().unwrap_or("").to_string());
        }
        Ok(Token { value: token_from(&v).ok_or("no access_token in response")?, expires: expiry_from(&v) })
    }

    async fn environment(&self) -> Result<Token, String> {
        let (Some(tenant), Some(id), Some(secret)) = (self.tenant()?, env("AZURE_CLIENT_ID"), env("AZURE_CLIENT_SECRET")) else {
            return Err("AZURE_TENANT_ID, AZURE_CLIENT_ID and AZURE_CLIENT_SECRET are not all set".into());
        };
        self.token_request(&tenant, &[("client_id", &id), ("client_secret", &secret), ("scope", SCOPE), ("grant_type", "client_credentials")]).await
    }

    async fn workload_identity(&self) -> Result<Token, String> {
        let (Some(tenant), Some(id), Some(file)) = (self.tenant()?, env("AZURE_CLIENT_ID"), env("AZURE_FEDERATED_TOKEN_FILE")) else {
            return Err("AZURE_FEDERATED_TOKEN_FILE, AZURE_CLIENT_ID and AZURE_TENANT_ID are not all set".into());
        };
        let assertion = std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?;
        self.token_request(
            &tenant,
            &[
                ("client_id", &id),
                ("scope", SCOPE),
                ("grant_type", "client_credentials"),
                ("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer"),
                ("client_assertion", assertion.trim()),
            ],
        )
        .await
    }

    async fn managed_identity(&self, client_id: Option<&str>) -> Result<Token, String> {
        let client_id = client_id.map(String::from).or_else(|| env("AZURE_CLIENT_ID"));
        let mut query: Vec<(&str, String)> = vec![("resource", RESOURCE.into())];
        let req = if let (Some(endpoint), Some(header)) = (env("IDENTITY_ENDPOINT"), env("IDENTITY_HEADER")) {
            // App Service, Azure Functions, Azure Container Apps.
            query.push(("api-version", "2019-08-01".into()));
            if let Some(id) = &client_id {
                query.push(("client_id", id.clone()));
            }
            self.client.get(endpoint).query(&query).header("X-IDENTITY-HEADER", header).timeout(Duration::from_secs(30))
        } else if let Some(endpoint) = env("MSI_ENDPOINT") {
            match env("MSI_SECRET") {
                Some(secret) => {
                    query.push(("api-version", "2017-09-01".into()));
                    if let Some(id) = &client_id {
                        query.push(("clientid", id.clone()));
                    }
                    self.client.get(endpoint).query(&query).header("secret", secret).timeout(Duration::from_secs(30))
                }
                // Cloud Shell.
                None => {
                    let mut form = vec![("resource", RESOURCE.to_string())];
                    if let Some(id) = &client_id {
                        form.push(("client_id", id.clone()));
                    }
                    self.client.post(endpoint).form(&form).header("Metadata", "true").timeout(Duration::from_secs(30))
                }
            }
        } else {
            // Azure Instance Metadata Service (VMs, AKS nodes). A short timeout keeps developer machines fast.
            query.push(("api-version", "2018-02-01".into()));
            if let Some(id) = &client_id {
                query.push(("client_id", id.clone()));
            }
            let base = env("AZURE_POD_IDENTITY_AUTHORITY_HOST").unwrap_or_else(|| "http://169.254.169.254".into());
            let url = format!("{}/metadata/identity/oauth2/token", base.trim_end_matches('/'));
            self.client.get(url).query(&query).header("Metadata", "true").timeout(Duration::from_secs(3))
        };
        let res = req.send().await.map_err(|e| format!("managed identity endpoint unavailable ({e})"))?;
        let status = res.status();
        let text = res.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {}", text.chars().take(300).collect::<String>()));
        }
        let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        Ok(Token { value: token_from(&v).ok_or("no access_token in response")?, expires: expiry_from(&v) })
    }

    async fn run_json(&self, program: &str, args: &[&str], envs: &[(&str, &str)]) -> Result<Value, String> {
        // `az` is a batch file on Windows and must run through cmd; pwsh, powershell and azd are executables.
        let mut cmd = if cfg!(windows) && program == "az" {
            let mut c = tokio::process::Command::new("cmd");
            c.arg("/C").arg(program);
            c
        } else {
            tokio::process::Command::new(program)
        };
        cmd.args(args).envs(envs.iter().copied()).current_dir(safe_working_dir()).stdin(std::process::Stdio::null()).kill_on_drop(true);
        let out = tokio::time::timeout(Duration::from_secs(60), cmd.output())
            .await
            .map_err(|_| format!("{program} timed out"))?
            .map_err(|e| format!("{program} is not available ({e})"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(err.lines().find(|l| !l.trim().is_empty()).unwrap_or("failed").trim().to_string());
        }
        // The output is not quoted in the error: it may hold a token.
        json_in_output(&String::from_utf8_lossy(&out.stdout)).ok_or_else(|| format!("{program} printed no JSON object"))
    }

    async fn azure_cli(&self) -> Result<Token, String> {
        let tenant = self.tenant()?;
        let mut args = vec!["account", "get-access-token", "--output", "json", "--resource", RESOURCE];
        if let Some(t) = &tenant {
            args.extend(["--tenant", t.as_str()]);
        }
        let v = self.run_json("az", &args, &[]).await?;
        Ok(Token { value: token_from(&v).ok_or("no accessToken in az output")?, expires: expiry_from(&v) })
    }

    async fn azure_developer_cli(&self) -> Result<Token, String> {
        let tenant = self.tenant()?;
        let mut args = vec!["auth", "token", "--output", "json", "--scope", SCOPE];
        if let Some(t) = &tenant {
            args.extend(["--tenant-id", t.as_str()]);
        }
        let v = self.run_json("azd", &args, &[]).await?;
        Ok(Token { value: token_from(&v).ok_or("no token in azd output")?, expires: expiry_from(&v) })
    }

    async fn azure_powershell(&self) -> Result<Token, String> {
        let tenant = self.tenant()?;
        let envs = [
            ("AZURE_IDENTITY_POWERSHELL_RESOURCE", RESOURCE),
            ("AZURE_IDENTITY_POWERSHELL_TENANT_ID", tenant.as_deref().unwrap_or("")),
            ("SuppressAzurePowerShellBreakingChangeWarnings", "true"),
        ];
        let mut last = String::new();
        for shell in if cfg!(windows) { &["pwsh", "powershell"][..] } else { &["pwsh"][..] } {
            match self.run_json(shell, &["-NoProfile", "-NonInteractive", "-Command", POWERSHELL_SCRIPT], &envs).await {
                Ok(v) => return Ok(Token { value: token_from(&v).ok_or("no token in PowerShell output")?, expires: expiry_from(&v) }),
                Err(e) => last = e,
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339("2000-03-01T00:00:00Z"), Some(951868800));
        assert_eq!(parse_rfc3339("2000-03-01T02:00:00.5+02:00"), Some(951868800));
    }

    #[test]
    fn expiry_fields() {
        assert_eq!(expiry_from(&serde_json::json!({ "expires_on": "123" })), 123);
        assert_eq!(expiry_from(&serde_json::json!({ "expires_on": 456 })), 456);
    }

    #[test]
    fn json_after_powershell_warnings() {
        let out = "WARNING: Upcoming breaking changes in the cmdlet 'Get-AzAccessToken' :\r\n- The Token property will be a SecureString.\r\n{\"token\":\"abc\",\"expires_on\":123}\r\n";
        let v = json_in_output(out).unwrap();
        assert_eq!((token_from(&v).as_deref(), expiry_from(&v)), (Some("abc"), 123));
        let az = "{\n  \"accessToken\": \"xyz\",\n  \"expires_on\": 456\n}\n";
        assert_eq!(token_from(&json_in_output(az).unwrap()).as_deref(), Some("xyz"));
        assert!(json_in_output("WARNING: no token\n").is_none());
        assert!(json_in_output("").is_none());
    }

    #[test]
    fn tenant_ids() {
        assert!(valid_tenant("72f988bf-86f1-41af-91ab-2d7cd011db47"));
        assert!(valid_tenant("contoso.onmicrosoft.com"));
        for bad in ["", "x' ; Write-Host pwned ; '", "a&calc", "t/oauth2", "t?x=1", "t e"] {
            assert!(!valid_tenant(bad), "{bad}");
        }
    }
}
