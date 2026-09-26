//! Configuration from environment variables (the same `AZURE_*` / `MDT_*` variables as the TypeScript
//! implementation) plus the language catalog and glossary. The repository's `config/languages.json` and
//! `config/glossary.json` are compiled into the binary; `MDT_LANGUAGES_FILE` / `MDT_GLOSSARY_FILE` override them.

use crate::types::Formality;
use serde_json::Value;
use std::collections::HashMap;

pub const DEFAULT_LANGUAGES: &str = include_str!("../../config/languages.json");
pub const DEFAULT_GLOSSARY: &str = include_str!("../../config/glossary.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Gpt,
    Nmt,
    /// Deterministic fake translation for tests (no network).
    Pseudo,
}

impl Engine {
    pub fn parse(s: &str) -> Option<Engine> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gpt" => Some(Engine::Gpt),
            "nmt" => Some(Engine::Nmt),
            "pseudo" => Some(Engine::Pseudo),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Engine::Gpt => "gpt",
            Engine::Nmt => "nmt",
            Engine::Pseudo => "pseudo",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub openai_endpoint: Option<String>,
    pub translator_endpoint: Option<String>,
    pub translator_region: Option<String>,
    /// When set, API-key auth is used for all Azure AI calls; otherwise Microsoft Entra ID.
    pub api_key: Option<String>,
    pub managed_identity_client_id: Option<String>,
    pub tenant_id: Option<String>,
    /// Pre-acquired Entra access token (expires after about an hour).
    pub static_access_token: Option<String>,
    pub translate_deployment: String,
    pub review_deployment: String,
    pub analysis_deployment: String,
    pub translate_reasoning: String,
    pub review_reasoning: String,
    pub review: bool,
    pub nmt_fallback: bool,
    pub max_concurrency: usize,
    pub batch_max_segments: usize,
    pub batch_max_chars: usize,
    pub context_max_chars: usize,
    pub request_timeout_ms: u64,
    /// Attempts per Azure request, including retries of throttled or failed calls.
    pub max_attempts: u32,
    pub cache_dir: Option<String>,
    pub languages_file: Option<String>,
    pub glossary_file: Option<String>,
    pub preserve_anchors: bool,
    pub source_language: Option<String>,
    /// Send each segment's position (section, table column/row, list lead-in) to the model.
    pub structural_context: bool,
    pub math_single_dollar: bool,
    pub docstrings: bool,
    pub code_comments: bool,
    pub front_matter: bool,
    /// Force MDX parsing on or off; by default only `.mdx` files are parsed as MDX.
    pub mdx: Option<bool>,
    pub formality: Option<Formality>,
    pub engine: Engine,
}

/// `/^(1|true|yes|on)$/i`
pub fn parse_bool(v: &str) -> bool {
    matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

/// JavaScript `Number.parseInt(v, 10)` for positive values.
fn parse_int(v: &str) -> Option<u64> {
    let t = v.trim_start();
    let t = t.strip_prefix('+').unwrap_or(t);
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u64>().ok().filter(|n| *n > 0)
}

pub fn parse_formality(v: &str) -> Option<Formality> {
    match v.trim().to_ascii_lowercase().as_str() {
        "formal" => Some(Formality::Formal),
        "informal" => Some(Formality::Informal),
        _ => None,
    }
}

impl AppConfig {
    /// Reads the configuration from a variable lookup (the process environment, possibly merged with a `.env` file).
    pub fn from_lookup(get: &dyn Fn(&str) -> Option<String>) -> AppConfig {
        let env = |k: &str| get(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let flag = |k: &str, def: bool| env(k).map(|v| parse_bool(&v)).unwrap_or(def);
        let int = |k: &str, def: u64| env(k).and_then(|v| parse_int(&v)).unwrap_or(def);
        let trim_slash = |v: Option<String>| v.map(|s| s.trim_end_matches('/').to_string());
        let translate_deployment = env("MDT_TRANSLATE_DEPLOYMENT").unwrap_or_else(|| "translate".into());
        AppConfig {
            openai_endpoint: trim_slash(env("AZURE_OPENAI_ENDPOINT")),
            translator_endpoint: trim_slash(env("AZURE_TRANSLATOR_ENDPOINT")),
            translator_region: env("AZURE_TRANSLATOR_REGION"),
            api_key: env("AZURE_AI_API_KEY"),
            managed_identity_client_id: env("AZURE_CLIENT_ID"),
            tenant_id: env("AZURE_TENANT_ID"),
            static_access_token: env("AZURE_AI_ACCESS_TOKEN"),
            review_deployment: env("MDT_REVIEW_DEPLOYMENT").unwrap_or_else(|| translate_deployment.clone()),
            analysis_deployment: env("MDT_ANALYSIS_DEPLOYMENT").unwrap_or_else(|| translate_deployment.clone()),
            translate_deployment,
            translate_reasoning: env("MDT_TRANSLATE_REASONING").unwrap_or_else(|| "medium".into()),
            review_reasoning: env("MDT_REVIEW_REASONING").unwrap_or_else(|| "high".into()),
            review: flag("MDT_REVIEW", true),
            nmt_fallback: flag("MDT_NMT_FALLBACK", true),
            max_concurrency: int("MDT_MAX_CONCURRENCY", 16) as usize,
            batch_max_segments: int("MDT_BATCH_MAX_SEGMENTS", 40) as usize,
            batch_max_chars: int("MDT_BATCH_MAX_CHARS", 12000) as usize,
            context_max_chars: int("MDT_CONTEXT_MAX_CHARS", 60000) as usize,
            request_timeout_ms: int("MDT_REQUEST_TIMEOUT_MS", 600000),
            max_attempts: int("MDT_MAX_ATTEMPTS", 6).min(20) as u32,
            cache_dir: env("MDT_CACHE_DIR"),
            languages_file: env("MDT_LANGUAGES_FILE"),
            glossary_file: env("MDT_GLOSSARY_FILE"),
            preserve_anchors: flag("MDT_PRESERVE_ANCHORS", true),
            source_language: env("MDT_SOURCE_LANGUAGE"),
            structural_context: flag("MDT_STRUCTURAL_CONTEXT", false),
            math_single_dollar: flag("MDT_MATH_SINGLE_DOLLAR", false),
            docstrings: !matches!(env("MDT_DOCSTRINGS").unwrap_or_else(|| "on".into()).to_ascii_lowercase().as_str(), "off" | "false" | "0" | "no"),
            code_comments: flag("MDT_CODE_COMMENTS", true),
            front_matter: flag("MDT_FRONT_MATTER", true),
            mdx: env("MDT_MDX").map(|v| parse_bool(&v)),
            formality: env("MDT_FORMALITY").and_then(|v| parse_formality(&v)),
            engine: env("MDT_ENGINE").and_then(|v| Engine::parse(&v)).unwrap_or(Engine::Gpt),
        }
    }

    pub fn from_env() -> AppConfig {
        AppConfig::from_lookup(&|k| std::env::var(k).ok())
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct LanguageConfig {
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translator: Option<String>,
    pub formal: String,
    pub informal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<String>,
    #[serde(rename = "lengthRatio", skip_serializing_if = "Option::is_none")]
    pub length_ratio: Option<(f64, f64)>,
}

#[derive(Clone, Debug)]
pub struct LanguageCatalog {
    pub default_targets: Vec<String>,
    /// Languages in file order, keyed by lower-cased code.
    pub order: Vec<String>,
    pub languages: HashMap<String, LanguageConfig>,
}

impl LanguageCatalog {
    pub fn get(&self, code: &str) -> Option<&LanguageConfig> {
        self.languages.get(&code.trim().to_lowercase())
    }
}

pub fn parse_languages(text: &str, origin: &str) -> Result<LanguageCatalog, String> {
    let raw: Value = serde_json::from_str(text).map_err(|e| format!("{origin}: {e}"))?;
    let default_targets: Vec<String> = raw["defaultTargets"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    let mut languages = HashMap::new();
    let mut order = Vec::new();
    for (code, l) in raw["languages"].as_object().ok_or_else(|| format!("{origin}: missing \"languages\""))? {
        let s = |k: &str| l.get(k).and_then(|v| v.as_str()).map(String::from);
        let ratio = l.get("lengthRatio").and_then(|v| v.as_array()).and_then(|a| Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?)));
        let lc = LanguageConfig {
            code: code.clone(),
            name: s("name").unwrap_or_else(|| code.clone()),
            translator: s("translator"),
            formal: s("formal").unwrap_or_default(),
            informal: s("informal").unwrap_or_default(),
            style: s("style"),
            wrap: s("wrap"),
            length_ratio: ratio,
        };
        order.push(code.to_lowercase());
        languages.insert(code.to_lowercase(), lc);
    }
    for d in &default_targets {
        if !languages.contains_key(&d.to_lowercase()) {
            return Err(format!("default target \"{d}\" missing in {origin}"));
        }
    }
    Ok(LanguageCatalog { default_targets, order, languages })
}

#[derive(Clone, Debug)]
pub struct Glossary {
    pub do_not_translate: Vec<String>,
    /// term -> { language code -> translation }, in file order.
    pub terms: serde_json::Map<String, Value>,
}

impl Glossary {
    /// `JSON.stringify(glossary)` of the TypeScript `Glossary` object (part of cache keys).
    pub fn to_json(&self) -> Value {
        serde_json::json!({ "doNotTranslate": self.do_not_translate, "terms": self.terms })
    }
}

pub fn parse_glossary(text: &str, origin: &str) -> Result<Glossary, String> {
    let raw: Value = serde_json::from_str(text).map_err(|e| format!("{origin}: {e}"))?;
    Ok(Glossary {
        do_not_translate: raw["doNotTranslate"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
        terms: raw["terms"].as_object().cloned().unwrap_or_default(),
    })
}

pub fn load_languages(file: Option<&str>) -> Result<LanguageCatalog, String> {
    match file {
        Some(f) => parse_languages(&std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?, f),
        None => parse_languages(DEFAULT_LANGUAGES, "built-in languages.json"),
    }
}

pub fn load_glossary(file: Option<&str>) -> Result<Glossary, String> {
    match file {
        Some(f) => parse_glossary(&std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?, f),
        None => parse_glossary(DEFAULT_GLOSSARY, "built-in glossary.json"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_config_loads() {
        let c = load_languages(None).unwrap();
        assert_eq!(c.get("DE").unwrap().name, "German");
        assert_eq!(c.get("zh-hans").unwrap().wrap.as_deref(), Some("none"));
        assert!(load_glossary(None).unwrap().do_not_translate.contains(&"Azure".to_string()));
    }

    #[test]
    fn env_parsing() {
        let vars: HashMap<&str, &str> = [("MDT_REVIEW", "off"), ("MDT_MAX_CONCURRENCY", "8x"), ("MDT_DOCSTRINGS", "maybe"), ("AZURE_OPENAI_ENDPOINT", "https://x/")].into();
        let c = AppConfig::from_lookup(&|k| vars.get(k).map(|v| v.to_string()));
        assert!(!c.review);
        assert_eq!(c.max_concurrency, 8);
        assert!(c.docstrings);
        assert_eq!(c.openai_endpoint.as_deref(), Some("https://x"));
        assert_eq!(c.review_deployment, "translate");
    }
}
