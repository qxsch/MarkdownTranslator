//! JSON dumps of extractions in the format of `rust/tools/golden.ts`, for the golden tests and
//! `-dumpGolden` / `-dumpExtraction`.

use crate::assemble::{assemble_document, AssembleOptions};
use crate::markdown::extract::extract;
use crate::translate::pseudo::pseudo_map;
use crate::types::{ExtractOptions, Extraction, ParseOptions, TextContext};
use serde_json::{json, Map, Value};

#[derive(Clone, Copy, Debug)]
pub struct Variant {
    pub mdx: bool,
    pub math_single_dollar: bool,
    pub docstrings: bool,
    pub code_comments: bool,
    pub front_matter: bool,
    pub preserve_anchors: bool,
}

impl Variant {
    pub fn extract_options(&self) -> ExtractOptions {
        ExtractOptions {
            parse: ParseOptions { mdx: self.mdx, math_single_dollar: self.math_single_dollar },
            docstrings: self.docstrings,
            code_comments: self.code_comments,
            front_matter: self.front_matter,
        }
    }
}

pub fn text_context_name(c: TextContext) -> &'static str {
    match c {
        TextContext::Markdown => "markdown",
        TextContext::Cell => "cell",
        TextContext::Html => "html",
        TextContext::Attr => "attr",
        TextContext::MdTitle => "mdtitle",
        TextContext::Comment => "comment",
        TextContext::Yaml => "yaml",
    }
}

pub fn dump_extraction(ex: &Extraction) -> Map<String, Value> {
    let segments: Vec<Value> = ex
        .segments()
        .iter()
        .map(|s| {
            let mut tags = Map::new();
            for p in s.placeholders.values() {
                tags.insert(format!("x{}", p.n), Value::String(format!("{}{}", p.hint, if p.hard_break { " [break]" } else { "" })));
            }
            for p in s.pairs.values() {
                let quote = p.quote.map(|q| format!(" [quote {q}]")).unwrap_or_default();
                let reference = p.ref_label.as_ref().map(|r| format!(" [ref {r}]")).unwrap_or_default();
                tags.insert(format!("g{}", p.n), Value::String(format!("{}: {}{quote}{reference}", p.kind.as_str(), p.hint)));
            }
            json!({
                "id": s.id,
                "kind": s.kind.as_str(),
                "textContext": text_context_name(s.text_context),
                "note": s.note,
                "structure": s.structure,
                "masked": s.masked,
                "original": s.original,
                "sourceText": s.source_text,
                "passive": s.passive,
                "embedded": s.embedded,
                "dependents": s.dependents,
                "tags": tags,
                "tagGroups": s.tag_groups.iter().map(|g| format!("{}: {}", if g.contiguous { "contiguous" } else { "ordered" }, g.tokens.join(" "))).collect::<Vec<_>>(),
                "wrap": s.wrap.as_ref().map(|w| json!({ "width": w.width, "firstColumn": w.first_column, "prefix": w.prefix, "eol": w.eol })),
                "forbidden": s.forbidden,
                "quote": s.quote.map(|q| q.to_string()),
                "inlineSignature": s.inline_signature,
            })
        })
        .collect();
    let mut m = Map::new();
    m.insert("segments".into(), Value::Array(segments));
    m.insert("replacements".into(), Value::Array(ex.replacements.iter().map(|r| json!([r.start, r.end, r.seg])).collect()));
    m.insert("notes".into(), json!(ex.notes));
    m.insert("definitionsText".into(), Value::String(ex.definitions_text.clone()));
    m.insert("frontmatterFormality".into(), ex.frontmatter_formality.map(|f| Value::String(f.as_str().into())).unwrap_or(Value::Null));
    m
}

/// Extraction, pseudo translation and assembly (the `run` function of `golden.ts`).
pub fn golden(src: &str, dnt: &[String], v: &Variant) -> Value {
    let ex = match extract(src, dnt, v.extract_options()) {
        Ok(ex) => ex,
        Err(e) => return json!({ "error": format!("extract: {e}") }),
    };
    let mut m = dump_extraction(&ex);
    match assemble_document(&ex, &pseudo_map(&ex), &AssembleOptions { wrap: true, preserve_anchors: v.preserve_anchors }) {
        Ok(out) => {
            m.insert("output".into(), Value::String(out.text));
            m.insert("reverted".into(), json!(out.reverted));
            m.insert("anchors".into(), json!(out.anchors));
        }
        Err(e) => {
            m.insert("error".into(), Value::String(format!("assemble: {e}")));
        }
    }
    Value::Object(m)
}
