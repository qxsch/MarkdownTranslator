//! Translation of one document into one language: cached segments first, then batched model calls with
//! validation-driven retries, Azure Translator as fallback, and an optional review pass. Port of
//! `translate/engine.ts`.

use super::cache::TranslationCache;
use super::prompts::{document_context, review_schema, review_system, segment_payload, translation_schema, translation_system, DocAnalysis, PROMPT_VERSION};
use super::pseudo::pseudo;
use super::validate::{looks_untranslated, sanitize, validate_segment};
use crate::azure::clients::{html_to_masked, masked_to_html, ChatClient, ChatError, NmtClient};
use crate::config::{AppConfig, Engine, Glossary, LanguageConfig};
use crate::types::{Extraction, Formality, Segment, SegmentKind, TMap};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

const MAX_RETRY_ROUNDS: u32 = 2;

pub struct TranslateTask<'a> {
    pub doc_name: &'a str,
    pub ex: &'a Extraction,
    pub lang: &'a LanguageConfig,
    /// Display name of the source language ("English").
    pub source_language: String,
    pub source_language_code: Option<String>,
    pub analysis: Option<&'a DocAnalysis>,
    pub formality: Formality,
    pub glossary: &'a Glossary,
    pub engine: Engine,
    pub review: bool,
    pub translate_deployment: Option<String>,
    pub review_deployment: Option<String>,
    /// Include each segment's structural position in model requests (default: config).
    pub structural_context: Option<bool>,
    /// Use Azure Translator for segments that fail validation (default: config).
    pub nmt_fallback: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReviewInfo {
    pub category: String,
    pub severity: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SegmentOutcome {
    pub id: String,
    pub kind: String,
    /// `cache`, `gpt`, `nmt`, `pseudo`, `tm` (given translation memory) or `source` (kept untranslated).
    pub via: String,
    pub retries: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<ReviewInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub untranslated: Option<bool>,
}

/// Outcomes in insertion order.
#[derive(Clone, Debug, Default)]
pub struct Outcomes {
    list: Vec<SegmentOutcome>,
    index: HashMap<String, usize>,
}

impl Outcomes {
    pub fn set(&mut self, o: SegmentOutcome) {
        match self.index.get(&o.id) {
            Some(&i) => self.list[i] = o,
            None => {
                self.index.insert(o.id.clone(), self.list.len());
                self.list.push(o);
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&SegmentOutcome> {
        self.index.get(id).map(|&i| &self.list[i])
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut SegmentOutcome> {
        self.index.get(id).map(|&i| &mut self.list[i])
    }

    pub fn values(&self) -> &[SegmentOutcome] {
        &self.list
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

pub struct EngineDeps<'a> {
    pub cfg: &'a AppConfig,
    pub chat: &'a ChatClient,
    pub nmt: &'a NmtClient,
    pub cache: &'a TranslationCache,
}

#[derive(Clone)]
struct Failure {
    seg: String,
    attempt: Option<String>,
    errors: Vec<String>,
}

enum BatchResult {
    Accepted { seg: String, text: String },
    Failed(Failure),
}

fn batches<'s>(segs: &[&'s Segment], max_segs: usize, max_chars: usize) -> Vec<Vec<&'s Segment>> {
    let mut out = Vec::new();
    let mut cur: Vec<&Segment> = Vec::new();
    let mut chars = 0;
    for s in segs {
        let len = crate::jsstr::u16len(&s.masked);
        if !cur.is_empty() && (cur.len() >= max_segs || chars + len > max_chars) {
            out.push(std::mem::take(&mut cur));
            chars = 0;
        }
        cur.push(s);
        chars += len;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn outcome(seg: &Segment, via: &str, retries: u32) -> SegmentOutcome {
    SegmentOutcome { id: seg.id.clone(), kind: seg.kind.as_str().into(), via: via.into(), retries, errors: None, review: None, untranslated: None }
}

struct Ctx<'a> {
    deps: &'a EngineDeps<'a>,
    task: &'a TranslateTask<'a>,
    system: String,
    context: String,
    deployment: String,
    with_structure: bool,
}

impl<'a> Ctx<'a> {
    fn call_batch(&'a self, batch: Vec<&'a Segment>, retry: Option<&'a HashMap<String, Failure>>) -> Pin<Box<dyn Future<Output = Vec<BatchResult>> + 'a>> {
        Box::pin(async move {
            let payload: Vec<Value> = batch
                .iter()
                .map(|s| {
                    let mut p = segment_payload(s, self.with_structure);
                    if let Some(f) = retry.and_then(|r| r.get(&s.id)) {
                        p.insert("previousAttempt".into(), f.attempt.clone().map(Value::String).unwrap_or(Value::Null));
                        p.insert("problems".into(), json!(f.errors));
                    }
                    Value::Object(p)
                })
                .collect();
            let intro = if retry.is_some() {
                "These segments failed automatic validation. Translate them again and fix the listed problems."
            } else {
                "Translate these segments."
            };
            let user = format!("{intro}\n{}", serde_json::to_string(&json!({ "segments": payload })).unwrap());
            let messages = vec![("system", self.system.clone()), ("user", self.context.clone()), ("user", user)];
            let reasoning = self.deps.cfg.translate_reasoning.clone();
            let res = self.deps.chat.json(&self.deployment, messages, "translations", translation_schema(), Some(&reasoning)).await;
            let res = match res {
                Ok(v) => v,
                Err(ChatError::Truncated) if batch.len() > 1 => {
                    let mid = batch.len().div_ceil(2);
                    let (a, b) = (batch[..mid].to_vec(), batch[mid..].to_vec());
                    let mut first = self.call_batch(a, retry).await;
                    first.extend(self.call_batch(b, retry).await);
                    return first;
                }
                Err(e) => {
                    return batch.iter().map(|s| BatchResult::Failed(Failure { seg: s.id.clone(), attempt: None, errors: vec![format!("model call failed: {e}")] })).collect();
                }
            };
            let by_id: HashMap<&str, &str> = res["translations"]
                .as_array()
                .map(|a| a.iter().filter_map(|t| Some((t["id"].as_str()?, t["text"].as_str()?))).collect())
                .unwrap_or_default();
            batch
                .iter()
                .map(|seg| match by_id.get(seg.id.as_str()) {
                    None => BatchResult::Failed(Failure { seg: seg.id.clone(), attempt: None, errors: vec!["segment missing from the response".into()] }),
                    Some(raw) => {
                        let text = sanitize(seg, raw);
                        let errors = validate_segment(seg, &text, self.task.ex, self.task.lang);
                        if errors.is_empty() {
                            BatchResult::Accepted { seg: seg.id.clone(), text }
                        } else {
                            BatchResult::Failed(Failure { seg: seg.id.clone(), attempt: Some(text), errors })
                        }
                    }
                })
                .collect()
        })
    }
}

pub async fn translate_document(deps: &EngineDeps<'_>, task: &TranslateTask<'_>) -> (TMap, Outcomes) {
    let cfg = deps.cfg;
    let segs: Vec<&Segment> = task.ex.active().collect();
    let mut tm = TMap::new();
    let mut outcomes = Outcomes::default();
    if task.engine == Engine::Pseudo {
        for s in &segs {
            tm.insert(s.id.clone(), pseudo(&s.masked));
            outcomes.set(outcome(s, "pseudo", 0));
        }
        return (tm, outcomes);
    }
    let translate_deployment = task.translate_deployment.clone().unwrap_or_else(|| cfg.translate_deployment.clone());
    let review_deployment = task.review_deployment.clone().unwrap_or_else(|| cfg.review_deployment.clone());
    let use_gpt = task.engine == Engine::Gpt && deps.chat.available();
    let review = task.review && use_gpt;
    let with_structure = task.structural_context.unwrap_or(cfg.structural_context);
    let glossary_key = TranslationCache::key(&json!([
        task.glossary.to_json(),
        task.analysis.map(|a| json!(a.do_not_translate)).unwrap_or(json!([])),
        task.analysis.map(|a| json!(a.terminology)).unwrap_or(json!([])),
    ]));
    let index: HashMap<&str, usize> = segs.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
    let key_of = |seg: &Segment| {
        let i = index[seg.id.as_str()];
        let prev = if i > 0 { segs[i - 1].masked.as_str() } else { "" };
        let next = segs.get(i + 1).map(|s| s.masked.as_str()).unwrap_or("");
        TranslationCache::key(&json!([
            PROMPT_VERSION,
            task.engine.as_str(),
            if use_gpt { translate_deployment.as_str() } else { "nmt" },
            if review { review_deployment.as_str() } else { "" },
            task.lang.code,
            task.formality.as_str(),
            seg.kind.as_str(),
            seg.note,
            seg.masked,
            prev,
            next,
            glossary_key,
            if with_structure { Value::String(seg.structure.clone().unwrap_or_default()) } else { Value::Bool(false) },
        ]))
    };

    let mut pending: Vec<&Segment> = Vec::new();
    for seg in &segs {
        match deps.cache.get(&key_of(seg)) {
            Some(hit) if validate_segment(seg, &hit, task.ex, task.lang).is_empty() => {
                tm.insert(seg.id.clone(), hit);
                outcomes.set(outcome(seg, "cache", 0));
            }
            _ => pending.push(seg),
        }
    }

    let by_id: HashMap<&str, &Segment> = segs.iter().map(|s| (s.id.as_str(), *s)).collect();
    let accept = |tm: &mut TMap, outcomes: &mut Outcomes, seg: &Segment, text: String, via: &str, retries: u32| {
        let mut o = outcome(seg, via, retries);
        if looks_untranslated(seg, &text) {
            o.untranslated = Some(true);
        }
        tm.insert(seg.id.clone(), text);
        outcomes.set(o);
    };

    let mut failures: Vec<Failure>;
    if use_gpt && !pending.is_empty() {
        let ctx = Ctx {
            deps,
            task,
            system: translation_system(&task.source_language, task.lang, task.formality, task.analysis, task.glossary),
            context: document_context(task.analysis, task.doc_name, &task.ex.source, cfg.context_max_chars),
            deployment: translate_deployment.clone(),
            with_structure,
        };
        let first: Vec<Vec<BatchResult>> = futures::future::join_all(batches(&pending, cfg.batch_max_segments, cfg.batch_max_chars).into_iter().map(|b| ctx.call_batch(b, None))).await;
        failures = Vec::new();
        for r in first.into_iter().flatten() {
            match r {
                BatchResult::Accepted { seg, text } => accept(&mut tm, &mut outcomes, by_id[seg.as_str()], text, "gpt", 0),
                BatchResult::Failed(f) => failures.push(f),
            }
        }
        for round in 1..=MAX_RETRY_ROUNDS {
            if failures.is_empty() {
                break;
            }
            let info: HashMap<String, Failure> = failures.iter().map(|f| (f.seg.clone(), f.clone())).collect();
            let retry_segs: Vec<&Segment> = failures.iter().map(|f| by_id[f.seg.as_str()]).collect();
            let info_ref = &info;
            let results: Vec<Vec<BatchResult>> =
                futures::future::join_all(batches(&retry_segs, 10, cfg.batch_max_chars).into_iter().map(|b| ctx.call_batch(b, Some(info_ref)))).await;
            failures = Vec::new();
            for r in results.into_iter().flatten() {
                match r {
                    BatchResult::Accepted { seg, text } => accept(&mut tm, &mut outcomes, by_id[seg.as_str()], text, "gpt", 1),
                    BatchResult::Failed(f) => failures.push(f),
                }
            }
            for seg in &segs {
                if info.contains_key(&seg.id) {
                    if let Some(o) = outcomes.get_mut(&seg.id) {
                        o.retries = round;
                    }
                }
            }
        }
    } else {
        failures = pending.iter().map(|s| Failure { seg: s.id.clone(), attempt: None, errors: Vec::new() }).collect();
    }

    if !failures.is_empty() && deps.nmt.available() && (task.nmt_fallback.unwrap_or(cfg.nmt_fallback) || task.engine == Engine::Nmt) {
        let htmls: Vec<String> = failures.iter().map(|f| masked_to_html(&by_id[f.seg.as_str()].masked)).collect();
        let to = task.lang.translator.clone().unwrap_or_else(|| task.lang.code.clone());
        match deps.nmt.translate(&htmls, &to, task.source_language_code.as_deref()).await {
            Ok(out) => {
                let mut remaining = Vec::new();
                for (i, f) in failures.into_iter().enumerate() {
                    let seg = by_id[f.seg.as_str()];
                    let text = sanitize(seg, &html_to_masked(out.get(i).map(String::as_str).unwrap_or("")));
                    let errors = validate_segment(seg, &text, task.ex, task.lang);
                    if errors.is_empty() {
                        accept(&mut tm, &mut outcomes, seg, text, "nmt", if task.engine == Engine::Nmt { 0 } else { MAX_RETRY_ROUNDS });
                    } else {
                        let mut all = f.errors.clone();
                        all.extend(errors.iter().map(|e| format!("nmt: {e}")));
                        remaining.push(Failure { errors: all, ..f });
                    }
                }
                failures = remaining;
            }
            Err(e) => {
                for f in failures.iter_mut() {
                    f.errors.push(format!("nmt failed: {e}"));
                }
            }
        }
    }
    for f in &failures {
        let mut o = outcome(by_id[f.seg.as_str()], "source", MAX_RETRY_ROUNDS);
        o.errors = Some(f.errors.clone());
        outcomes.set(o);
    }

    if review {
        let to_review: Vec<&Segment> = segs.iter().copied().filter(|s| tm.contains_key(&s.id) && outcomes.get(&s.id).is_some_and(|o| o.via != "cache")).collect();
        review_pass(deps, task, &mut tm, &mut outcomes, &to_review).await;
    }

    for s in &segs {
        if let (Some(t), Some(o)) = (tm.get(&s.id), outcomes.get(&s.id)) {
            if o.via != "cache" && o.via != "source" {
                deps.cache.set(&key_of(s), t);
            }
        }
    }
    (tm, outcomes)
}

/// Second pass: a reviewer model checks each translation against its source and returns corrections.
pub async fn review_pass(deps: &EngineDeps<'_>, task: &TranslateTask<'_>, tm: &mut TMap, outcomes: &mut Outcomes, to_review: &[&Segment]) {
    let cfg = deps.cfg;
    if to_review.is_empty() {
        return;
    }
    let deployment = task.review_deployment.clone().unwrap_or_else(|| cfg.review_deployment.clone());
    let system = review_system(&task.source_language, task.lang, task.formality, task.glossary);
    let context = document_context(task.analysis, task.doc_name, &task.ex.source, cfg.context_max_chars);
    let with_structure = task.structural_context.unwrap_or(cfg.structural_context);
    let batches = batches(to_review, cfg.batch_max_segments, cfg.batch_max_chars);
    let snapshot = tm.clone();
    let calls = batches.iter().map(|batch| {
        let payload: Vec<Value> = batch
            .iter()
            .map(|s| {
                let mut p = segment_payload(s, with_structure);
                p.insert("translation".into(), snapshot.get(&s.id).cloned().map(Value::String).unwrap_or(Value::Null));
                Value::Object(p)
            })
            .collect();
        let user = format!("Review these translations.\n{}", serde_json::to_string(&json!({ "segments": payload })).unwrap());
        let messages = vec![("system", system.clone()), ("user", context.clone()), ("user", user)];
        let deployment = deployment.clone();
        let reasoning = cfg.review_reasoning.clone();
        async move { deps.chat.json(&deployment, messages, "review", review_schema(), Some(&reasoning)).await.ok() }
    });
    let results = futures::future::join_all(calls).await;
    for (batch, res) in batches.iter().zip(results) {
        let Some(res) = res else { continue };
        let in_batch: HashMap<&str, &Segment> = batch.iter().map(|s| (s.id.as_str(), *s)).collect();
        for edit in res["edits"].as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
            let Some(seg) = edit["id"].as_str().and_then(|id| in_batch.get(id)) else { continue };
            let text = sanitize(seg, edit["text"].as_str().unwrap_or(""));
            if Some(&text) == tm.get(&seg.id) || !validate_segment(seg, &text, task.ex, task.lang).is_empty() {
                continue;
            }
            // Code comments are extracted because they are prose; restoring the English source is never a fix.
            if seg.kind == SegmentKind::Comment && text == seg.masked && tm.get(&seg.id) != Some(&seg.masked) {
                continue;
            }
            let before = tm.insert(seg.id.clone(), text.clone());
            let Some(o) = outcomes.get_mut(&seg.id) else { continue };
            o.review = Some(ReviewInfo {
                category: edit["category"].as_str().unwrap_or("").into(),
                severity: edit["severity"].as_str().unwrap_or("").into(),
                explanation: edit["explanation"].as_str().unwrap_or("").into(),
                before,
            });
            o.untranslated = looks_untranslated(seg, &text).then_some(true);
        }
    }
}
