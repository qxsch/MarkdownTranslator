//! Prompts and JSON schemas for the model calls. Port of `translate/prompts.ts`; the text is kept identical.

use crate::config::{Glossary, LanguageConfig};
use crate::types::{Formality, Segment};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

pub const PROMPT_VERSION: &str = "v5";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Term {
    pub term: String,
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocAnalysis {
    #[serde(default)]
    pub source_language: String,
    #[serde(default)]
    pub register: String,
    #[serde(default)]
    pub register_evidence: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub audience: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub do_not_translate: Vec<String>,
    #[serde(default)]
    pub terminology: Vec<Term>,
}

pub fn analysis_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["sourceLanguage", "register", "registerEvidence", "domain", "audience", "summary", "doNotTranslate", "terminology"],
        "properties": {
            "sourceLanguage": { "type": "string", "description": "BCP-47 code of the document language, e.g. en" },
            "register": { "type": "string", "enum": ["formal", "informal", "neutral"] },
            "registerEvidence": { "type": "string" },
            "domain": { "type": "string" },
            "audience": { "type": "string" },
            "summary": { "type": "string" },
            "doNotTranslate": { "type": "array", "items": { "type": "string" } },
            "terminology": {
                "type": "array",
                "items": { "type": "object", "additionalProperties": false, "required": ["term", "note"], "properties": { "term": { "type": "string" }, "note": { "type": "string" } } }
            }
        }
    })
}

pub const ANALYSIS_SYSTEM: &str = r#"You analyze a Markdown document before it is localized.
Determine:
- sourceLanguage: the BCP-47 language code of the prose (ignore code blocks).
- register: how the author addresses the reader. "informal" only for clearly casual/chatty writing (slang, jokes, emojis, "hey", "awesome", heavy contractions); "formal" for official/legal/enterprise tone; otherwise "neutral".
- registerEvidence: one short sentence quoting the evidence.
- domain and audience: short phrases.
- summary: 2-4 sentences describing the content, for translator context.
- doNotTranslate: product names, brand names, UI labels in code font, API/feature names that must stay in the source language. Only names, never common words.
- terminology: up to 25 domain terms whose translation must be consistent, with a short note on meaning in this document."#;

pub fn translation_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["translations"],
        "properties": {
            "translations": {
                "type": "array",
                "items": { "type": "object", "additionalProperties": false, "required": ["id", "text"], "properties": { "id": { "type": "string" }, "text": { "type": "string" } } }
            }
        }
    })
}

pub fn review_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["edits"],
        "properties": {
            "edits": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["id", "category", "severity", "explanation", "text"],
                    "properties": {
                        "id": { "type": "string" },
                        "category": { "type": "string", "enum": ["accuracy", "omission", "addition", "terminology", "grammar", "spelling", "fluency", "register", "locale", "consistency", "untranslated"] },
                        "severity": { "type": "string", "enum": ["minor", "major", "critical"] },
                        "explanation": { "type": "string" },
                        "text": { "type": "string" }
                    }
                }
            }
        }
    })
}

const TAG_RULES: &str = "Segment format:
- Segments contain inline markup tags. <gN>…</gN> wraps formatted text (bold, italic, link text, HTML element content). <xN/> stands for content that must not change (code, file names, paths, URLs, variables, line breaks, escapes).
- Keep every tag exactly once, with the same number. You may move tags and reorder words so the sentence is natural in the target language; the text inside <gN>…</gN> must be the translation of the source text inside that same pair.
- The \"tags\" map shows what each tag stands for. Use it only for grammar (gender, case, articles, prepositions); never copy or translate its content into the text.
- A \"structure\" field, when present, tells where the segment sits (section path, table column and row, the sentence that introduces a list, the surrounding component). Use it to pick the right meaning and form (e.g. a short UI label vs. a verb, consistent column terminology); never copy it into the text.
- Output plain text plus the tags. Do not add Markdown, HTML, backticks, asterisks, underscores, brackets or new line breaks. Keep &lt; &gt; &amp; as written.";

fn formality_guidance(lang: &LanguageConfig, formality: Formality) -> &str {
    match formality {
        Formality::Informal => &lang.informal,
        Formality::Formal => &lang.formal,
    }
}

fn glossary_for(glossary: &Glossary, lang: &LanguageConfig) -> String {
    let base = lang.code.split('-').next().unwrap_or("");
    let mut lines = Vec::new();
    for (term, by_lang) in &glossary.terms {
        let t = by_lang.get(&lang.code).or_else(|| by_lang.get(base)).and_then(|v| v.as_str());
        if let Some(t) = t.filter(|t| !t.is_empty()) {
            lines.push(format!("- \"{term}\" → \"{t}\""));
        }
    }
    lines.join("\n")
}

pub fn translation_system(source: &str, lang: &LanguageConfig, formality: Formality, analysis: Option<&DocAnalysis>, glossary: &Glossary) -> String {
    let terms = glossary_for(glossary, lang);
    let mut dnt: Vec<&str> = Vec::new();
    for t in analysis.map(|a| a.do_not_translate.as_slice()).unwrap_or(&[]) {
        if !dnt.contains(&t.as_str()) {
            dnt.push(t);
        }
    }
    dnt.truncate(80);
    let name = &lang.name;
    let style = lang.style.as_deref().unwrap_or("Follow the typographic conventions of the locale.");
    let keep = if dnt.is_empty() { "(none identified)".to_string() } else { dnt.join(", ") };
    let required = if terms.is_empty() { String::new() } else { format!("Required terminology:\n{terms}\n") };
    let key_terms = match analysis {
        Some(a) if !a.terminology.is_empty() => format!(
            "Key terms of this document (translate consistently):\n{}\n",
            a.terminology.iter().map(|t| format!("- {}: {}", t.term, t.note)).collect::<Vec<_>>().join("\n")
        ),
        _ => String::new(),
    };
    format!(
        "You are an expert technical translator and native-level {name} localizer. Translate from {source} into {name}.

Quality bar: publication-ready documentation that reads as if originally written in {name}. Preserve the exact meaning: no omissions, additions, explanations or summaries. Keep numbers and units correct. Never change version numbers, build numbers or identifiers (\"Docker 4.30\" stays \"4.30\"); apply locale decimal separators only to genuine measured quantities. Use established {name} technical terminology (as used in Microsoft and major vendor documentation for this locale) and keep terms consistent across all segments.

Register: {register}
Locale style: {style}

{TAG_RULES}

Segment kinds:
- heading: concise title style, natural capitalization rules of {name}.
- cell: table cell, keep it short.
- comment: a source code comment; keep it terse and technical, keep identifiers as-is. If a comment is actually commented-out code, return it unchanged.
- alt / attr / title: image descriptions, tooltips and HTML attributes; translate them.
- frontmatter: document metadata such as title or description.
- If a segment is already in {name}, or consists only of names/identifiers, return it unchanged.

Keep these names in their original form: {keep}.
{required}{key_terms}
Return JSON {{\"translations\":[{{\"id\":\"…\",\"text\":\"…\"}}]}} with exactly one entry per input segment id.",
        register = formality_guidance(lang, formality),
    )
}

/// The segment as sent to the model (`segmentPayload`).
pub fn segment_payload(seg: &Segment, with_structure: bool) -> Map<String, Value> {
    let mut tags = Map::new();
    for p in seg.placeholders.values() {
        tags.insert(format!("x{}", p.n), Value::String(p.hint.clone()));
    }
    for p in seg.pairs.values() {
        tags.insert(format!("g{}", p.n), Value::String(format!("{}: {}", p.kind.as_str(), p.hint)));
    }
    let mut m = Map::new();
    m.insert("id".into(), Value::String(seg.id.clone()));
    m.insert("kind".into(), Value::String(seg.kind.as_str().into()));
    m.insert("context".into(), Value::String(seg.note.clone()));
    if with_structure {
        if let Some(s) = &seg.structure {
            m.insert("structure".into(), Value::String(s.clone()));
        }
    }
    m.insert("text".into(), Value::String(seg.masked.clone()));
    if !tags.is_empty() {
        m.insert("tags".into(), Value::Object(tags));
    }
    m
}

pub fn document_context(analysis: Option<&DocAnalysis>, doc_name: &str, source: &str, max_chars: usize) -> String {
    let doc = if crate::jsstr::u16len(source) > max_chars {
        format!("{}\n[…truncated…]", crate::jsstr::prefix_u16(source, max_chars))
    } else {
        source.to_string()
    };
    let head = match analysis {
        Some(a) => format!("\nDomain: {}\nAudience: {}\nSummary: {}", a.domain, a.audience, a.summary),
        None => String::new(),
    };
    format!("Document \"{doc_name}\"{head}\n\nFull source document, for context only (do not translate it here):\n<document>\n{doc}\n</document>")
}

pub fn review_system(source: &str, lang: &LanguageConfig, formality: Formality, glossary: &Glossary) -> String {
    let terms = glossary_for(glossary, lang);
    let name = &lang.name;
    let required = if terms.is_empty() { String::new() } else { format!("Required terminology:\n{terms}\n") };
    format!(
        "You are a senior {name} localization reviewer (native speaker) performing a quality check of translations from {source}.

For every segment compare the translation with the source and look for: mistranslations and meaning shifts, omissions, additions, wrong or inconsistent terminology, grammar, spelling and agreement errors, unnatural or literal phrasing, wrong register, locale convention errors, untranslated text.
Register: {register}
Locale style: {style}
{required}
{TAG_RULES}

Segments of kind \"comment\" are comments and docstrings taken from code blocks; translating them is intended (the code itself is never sent and stays unchanged). Never revert a correct translation to the source language.

Only report segments that genuinely need a change. Do not make stylistic or preferential edits to translations that are already correct and natural. For each reported segment return the complete corrected translation in \"text\" (with all tags), the error category, severity and a one-line explanation.
Return JSON {{\"edits\":[…]}}; an empty list when everything is correct.",
        register = formality_guidance(lang, formality),
        style = lang.style.as_deref().unwrap_or(""),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lang() -> LanguageConfig {
        LanguageConfig {
            code: "de".into(),
            name: "German".into(),
            translator: None,
            formal: "Use Sie.".into(),
            informal: "Use du.".into(),
            style: None,
            wrap: None,
            length_ratio: None,
        }
    }

    #[test]
    fn system_prompt_shape() {
        let g = crate::config::load_glossary(None).unwrap();
        let s = translation_system("English", &lang(), Formality::Formal, None, &g);
        assert!(s.starts_with("You are an expert technical translator and native-level German localizer. Translate from English into German."));
        assert!(s.contains("Register: Use Sie.\nLocale style: Follow the typographic conventions of the locale.\n\nSegment format:"));
        assert!(s.contains("Required terminology:\n- \"resource group\" → \"Ressourcengruppe\""));
        assert!(s.ends_with("with exactly one entry per input segment id."));
        assert!(s.contains("Keep these names in their original form: (none identified).\nRequired terminology:"));
    }
}
