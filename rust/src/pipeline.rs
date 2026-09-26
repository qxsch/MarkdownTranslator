//! Per-file orchestration: extraction, document analysis, translation into each target language and
//! assembly. Port of `pipeline.ts` (`translateFiles`) for a single file.

use crate::assemble::{assemble_document, AssembleOptions};
use crate::azure::auth::AzureAuth;
use crate::azure::clients::{ChatClient, NmtClient};
use crate::config::{AppConfig, Engine, Glossary, LanguageCatalog, LanguageConfig};
use crate::markdown::extract::extract;
use crate::translate::cache::TranslationCache;
use crate::translate::engine::{review_pass, translate_document, EngineDeps, Outcomes, SegmentOutcome, TranslateTask};
use crate::translate::prompts::{analysis_schema, DocAnalysis, ANALYSIS_SYSTEM};
use crate::types::{ExtractOptions, Extraction, Formality, ParseOptions, Segment, TMap};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Per-request feature switches; anything left `None` falls back to the configuration (environment).
#[derive(Clone, Debug, Default)]
pub struct TranslateOptions {
    pub review: Option<bool>,
    pub structural_context: Option<bool>,
    pub nmt_fallback: Option<bool>,
    pub preserve_anchors: Option<bool>,
    pub docstrings: Option<bool>,
    pub code_comments: Option<bool>,
    pub front_matter: Option<bool>,
    pub math_single_dollar: Option<bool>,
    /// Force MDX parsing on or off; by default only `.mdx` files are parsed as MDX.
    pub mdx: Option<bool>,
    pub formality: Option<Formality>,
    pub source_language: Option<String>,
    pub do_not_translate: Vec<String>,
    pub engine: Option<Engine>,
    pub translate_deployment: Option<String>,
    pub review_deployment: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct KeptSource {
    pub id: String,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReport {
    pub file: String,
    pub language: String,
    pub formality: String,
    pub segments: usize,
    pub via: BTreeMap<String, usize>,
    pub retried: usize,
    pub review_edits: usize,
    pub untranslated_warnings: Vec<String>,
    pub kept_source: Vec<KeptSource>,
    pub reverted_for_structure: Vec<String>,
    pub anchors_added: Vec<String>,
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct LanguageResult {
    pub text: String,
    pub report: FileReport,
    pub tm: TMap,
    pub outcomes: Outcomes,
}

pub struct FileResult {
    pub extraction: Extraction,
    pub analysis: Option<DocAnalysis>,
    /// Why the document analysis is missing, when it was attempted and failed.
    pub analysis_error: Option<String>,
    pub source_language: String,
    pub formality: Formality,
    pub languages: Vec<LanguageResult>,
}

/// Outcome of one target language, as reflected in the exit code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Translated,
    /// Some segments were kept in the source language (service or validation failures).
    Partial,
    /// Nothing was translated: the document failed, or every segment was kept in the source language.
    Failed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Translated => "translated",
            Status::Partial => "partial",
            Status::Failed => "failed",
        }
    }
}

impl LanguageResult {
    pub fn status(&self) -> Status {
        let kept = self.report.kept_source.len();
        if self.report.error.is_some() || (kept > 0 && kept == self.outcomes.len()) {
            Status::Failed
        } else if kept > 0 {
            Status::Partial
        } else {
            Status::Translated
        }
    }

    /// First error recorded for a segment kept in the source language.
    pub fn first_segment_error(&self) -> Option<&str> {
        self.report.kept_source.iter().flat_map(|k| k.errors.iter()).map(String::as_str).next()
    }
}

/// Where translations come from when they are not produced by the engine.
pub enum Memory {
    /// Translate with the configured engine.
    Engine,
    /// Assemble from a saved translation memory; optionally run the review pass over it.
    Saved { tm: TMap, outcomes: Vec<SegmentOutcome>, review: bool },
}

pub struct MarkdownTranslator {
    pub cfg: AppConfig,
    pub catalog: LanguageCatalog,
    pub glossary: Glossary,
    pub auth: Arc<AzureAuth>,
    pub chat: ChatClient,
    pub nmt: NmtClient,
    pub cache: TranslationCache,
}

#[derive(Debug)]
pub struct UnknownLanguage(pub String);

impl MarkdownTranslator {
    pub fn new(cfg: AppConfig, catalog: LanguageCatalog, glossary: Glossary) -> Result<Self, String> {
        let client = reqwest::Client::builder().user_agent(concat!("mdtranslate/", env!("CARGO_PKG_VERSION"))).build().map_err(|e| e.to_string())?;
        let auth = Arc::new(AzureAuth::new(&cfg, client.clone()));
        let limiter = Arc::new(Semaphore::new(cfg.max_concurrency.max(1)));
        let chat = ChatClient::new(&cfg, auth.clone(), limiter.clone(), client.clone());
        let nmt = NmtClient::new(&cfg, auth.clone(), limiter, client);
        let cache = TranslationCache::new(cfg.cache_dir.as_ref().map(std::path::PathBuf::from));
        Ok(MarkdownTranslator { cfg, catalog, glossary, auth, chat, nmt, cache })
    }

    pub fn resolve_languages(&self, codes: &[String]) -> Result<Vec<LanguageConfig>, UnknownLanguage> {
        let list: Vec<String> = if codes.is_empty() { self.catalog.default_targets.clone() } else { codes.to_vec() };
        list.iter().map(|c| self.catalog.get(c).cloned().ok_or_else(|| UnknownLanguage(c.clone()))).collect()
    }

    pub fn do_not_translate(&self, extra: &[String]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for t in self.glossary.do_not_translate.iter().chain(extra.iter()) {
            if !out.contains(t) {
                out.push(t.clone());
            }
        }
        out
    }

    pub fn extract_options(&self, file_name: &str, opts: &TranslateOptions) -> ExtractOptions {
        ExtractOptions {
            parse: ParseOptions {
                mdx: opts.mdx.or(self.cfg.mdx).unwrap_or_else(|| file_name.to_lowercase().ends_with(".mdx")),
                math_single_dollar: opts.math_single_dollar.unwrap_or(self.cfg.math_single_dollar),
            },
            docstrings: opts.docstrings.unwrap_or(self.cfg.docstrings),
            code_comments: opts.code_comments.unwrap_or(self.cfg.code_comments),
            front_matter: opts.front_matter.unwrap_or(self.cfg.front_matter),
        }
    }

    pub fn extract(&self, file_name: &str, content: &str, opts: &TranslateOptions) -> Result<Extraction, String> {
        extract(content, &self.do_not_translate(&opts.do_not_translate), self.extract_options(file_name, opts))
    }

    /// Document analysis for translator context. `Ok(None)` when no model endpoint is configured.
    pub async fn analyze(&self, doc_name: &str, source: &str) -> Result<Option<DocAnalysis>, String> {
        if !self.chat.available() {
            return Ok(None);
        }
        let doc = if crate::jsstr::u16len(source) > self.cfg.context_max_chars { crate::jsstr::prefix_u16(source, self.cfg.context_max_chars) } else { source };
        let messages = vec![("system", ANALYSIS_SYSTEM.to_string()), ("user", format!("Document \"{doc_name}\":\n<document>\n{doc}\n</document>"))];
        let v = self.chat.json(&self.cfg.analysis_deployment, messages, "analysis", analysis_schema(), Some("low")).await.map_err(|e| format!("document analysis failed: {e}"))?;
        serde_json::from_value(v).map(Some).map_err(|e| format!("document analysis returned an unexpected shape: {e}"))
    }

    pub fn engine(&self, opts: &TranslateOptions) -> Engine {
        opts.engine.unwrap_or(self.cfg.engine)
    }

    /// Translates one file into each target language. `analysis` overrides the document analysis (for
    /// reproducible evaluations); `memory` selects the translation source.
    pub async fn translate_file(
        &self,
        file_name: &str,
        content: &str,
        targets: &[LanguageConfig],
        opts: &TranslateOptions,
        analysis: Option<Option<DocAnalysis>>,
        memory: &Memory,
    ) -> Result<FileResult, String> {
        let ex = self.extract(file_name, content, opts)?;
        let engine = self.engine(opts);
        let translatable = ex.active().next().is_some();
        let mut analysis_error = None;
        let analysis = match analysis {
            Some(a) => a,
            // Without an analysis the translation still works, with less context; the caller reports the problem.
            None if translatable && engine == Engine::Gpt && matches!(memory, Memory::Engine) => self.analyze(file_name, &ex.source).await.unwrap_or_else(|e| {
                analysis_error = Some(e);
                None
            }),
            None => None,
        };
        let source_code = opts
            .source_language
            .clone()
            .or_else(|| self.cfg.source_language.clone())
            .or_else(|| analysis.as_ref().map(|a| a.source_language.clone()).filter(|s| !s.trim().is_empty()))
            .unwrap_or_else(|| "en".into())
            .trim()
            .to_string();
        let formality = opts
            .formality
            .or(self.cfg.formality)
            .or(ex.frontmatter_formality)
            .unwrap_or(if analysis.as_ref().is_some_and(|a| a.register == "informal") { Formality::Informal } else { Formality::Formal });
        let deps = EngineDeps { cfg: &self.cfg, chat: &self.chat, nmt: &self.nmt, cache: &self.cache };
        let preserve_anchors = opts.preserve_anchors.unwrap_or(self.cfg.preserve_anchors);
        let jobs = targets.iter().map(|lang| {
            let ex = &ex;
            let analysis = analysis.as_ref();
            let source_code = source_code.as_str();
            let deps = &deps;
            async move {
                let mut report = FileReport {
                    file: file_name.to_string(),
                    language: lang.code.clone(),
                    formality: formality.as_str().into(),
                    segments: 0,
                    via: BTreeMap::new(),
                    retried: 0,
                    review_edits: 0,
                    untranslated_warnings: Vec::new(),
                    kept_source: Vec::new(),
                    reverted_for_structure: Vec::new(),
                    anchors_added: Vec::new(),
                    notes: ex.notes.clone(),
                    error: None,
                };
                if same_language(source_code, &lang.code) || !translatable {
                    return LanguageResult { text: content.to_string(), report, tm: TMap::new(), outcomes: Outcomes::default() };
                }
                let task = TranslateTask {
                    doc_name: file_name,
                    ex,
                    lang,
                    source_language: language_name(source_code),
                    source_language_code: Some(source_code.split('-').next().unwrap_or(source_code).to_string()),
                    analysis,
                    formality,
                    glossary: &self.glossary,
                    engine,
                    review: opts.review.unwrap_or(self.cfg.review),
                    translate_deployment: opts.translate_deployment.clone(),
                    review_deployment: opts.review_deployment.clone(),
                    structural_context: opts.structural_context,
                    nmt_fallback: opts.nmt_fallback,
                };
                let (tm, outcomes) = match memory {
                    Memory::Engine => translate_document(deps, &task).await,
                    Memory::Saved { tm, outcomes, review } => {
                        let mut tm = tm.clone();
                        let mut out = Outcomes::default();
                        for o in outcomes {
                            out.set(o.clone());
                        }
                        for s in ex.active() {
                            if tm.contains_key(&s.id) && out.get(&s.id).is_none() {
                                out.set(SegmentOutcome { id: s.id.clone(), kind: s.kind.as_str().into(), via: "tm".into(), retries: 0, errors: None, review: None, untranslated: None });
                            }
                        }
                        if *review && self.chat.available() {
                            let to_review: Vec<&Segment> = ex.active().filter(|s| tm.contains_key(&s.id)).collect();
                            review_pass(deps, &task, &mut tm, &mut out, &to_review).await;
                        }
                        (tm, out)
                    }
                };
                match assemble_document(ex, &tm, &AssembleOptions { wrap: lang.wrap.as_deref() != Some("none"), preserve_anchors }) {
                    Ok(assembled) => {
                        report.segments = outcomes.len();
                        for o in outcomes.values() {
                            *report.via.entry(o.via.clone()).or_default() += 1;
                            if o.retries > 0 {
                                report.retried += 1;
                            }
                            if o.review.is_some() {
                                report.review_edits += 1;
                            }
                            if o.untranslated == Some(true) {
                                report.untranslated_warnings.push(o.id.clone());
                            }
                            if o.via == "source" {
                                report.kept_source.push(KeptSource { id: o.id.clone(), errors: o.errors.clone().unwrap_or_default() });
                            }
                        }
                        report.reverted_for_structure = assembled.reverted;
                        report.anchors_added = assembled.anchors;
                        LanguageResult { text: assembled.text, report, tm, outcomes }
                    }
                    Err(e) => {
                        report.error = Some(e);
                        LanguageResult { text: content.to_string(), report, tm, outcomes }
                    }
                }
            }
        });
        let languages = futures::future::join_all(jobs).await;
        Ok(FileResult { extraction: ex, analysis, analysis_error, source_language: source_code, formality, languages })
    }
}

fn same_language(a: &str, b: &str) -> bool {
    let base = |x: &str| x.to_lowercase().split(['-', '_']).next().unwrap_or("").to_string();
    base(a) == base(b)
}

/// English display name of a BCP-47 code (`Intl.DisplayNames(['en'], { type: 'language' })` for common codes).
pub fn language_name(code: &str) -> String {
    let exact = match code.to_lowercase().as_str() {
        "en-us" => Some("American English"),
        "en-gb" => Some("British English"),
        "en-au" => Some("Australian English"),
        "en-ca" => Some("Canadian English"),
        "pt-pt" => Some("European Portuguese"),
        "pt-br" => Some("Brazilian Portuguese"),
        "es-es" => Some("European Spanish"),
        "es-mx" => Some("Mexican Spanish"),
        "es-419" => Some("Latin American Spanish"),
        "fr-ca" => Some("Canadian French"),
        "fr-ch" => Some("Swiss French"),
        "de-at" => Some("Austrian German"),
        "de-ch" => Some("Swiss High German"),
        "nl-be" => Some("Flemish"),
        "zh-hans" => Some("Simplified Chinese"),
        "zh-hant" => Some("Traditional Chinese"),
        "sr-latn" => Some("Serbian (Latin)"),
        "sr-cyrl" => Some("Serbian (Cyrillic)"),
        _ => None,
    };
    if let Some(n) = exact {
        return n.into();
    }
    let base = code.split(['-', '_']).next().unwrap_or(code).to_lowercase();
    let name = match base.as_str() {
        "af" => "Afrikaans",
        "ar" => "Arabic",
        "bg" => "Bulgarian",
        "bn" => "Bangla",
        "ca" => "Catalan",
        "cs" => "Czech",
        "cy" => "Welsh",
        "da" => "Danish",
        "de" => "German",
        "el" => "Greek",
        "en" => "English",
        "es" => "Spanish",
        "et" => "Estonian",
        "eu" => "Basque",
        "fa" => "Persian",
        "fi" => "Finnish",
        "fil" => "Filipino",
        "fr" => "French",
        "ga" => "Irish",
        "gl" => "Galician",
        "he" => "Hebrew",
        "hi" => "Hindi",
        "hr" => "Croatian",
        "hu" => "Hungarian",
        "hy" => "Armenian",
        "id" => "Indonesian",
        "is" => "Icelandic",
        "it" => "Italian",
        "ja" => "Japanese",
        "ka" => "Georgian",
        "kk" => "Kazakh",
        "ko" => "Korean",
        "lt" => "Lithuanian",
        "lv" => "Latvian",
        "mk" => "Macedonian",
        "ms" => "Malay",
        "mt" => "Maltese",
        "nb" => "Norwegian Bokmål",
        "nl" => "Dutch",
        "nn" => "Norwegian Nynorsk",
        "no" => "Norwegian",
        "pl" => "Polish",
        "pt" => "Portuguese",
        "ro" => "Romanian",
        "ru" => "Russian",
        "sk" => "Slovak",
        "sl" => "Slovenian",
        "sq" => "Albanian",
        "sr" => "Serbian",
        "sv" => "Swedish",
        "sw" => "Swahili",
        "ta" => "Tamil",
        "th" => "Thai",
        "tr" => "Turkish",
        "uk" => "Ukrainian",
        "ur" => "Urdu",
        "vi" => "Vietnamese",
        "zh" => "Chinese",
        _ => return code.to_string(),
    };
    name.into()
}
