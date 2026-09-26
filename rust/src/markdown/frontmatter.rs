//! YAML front matter: prose values become segments; `formality` selects the register. Port of `frontmatter.ts`.

use super::context::{ExtractContext, SegmentOptions};
use super::line_map::{column_u16, max_line_width};
use super::yaml::{self, Style, Value, YNode, FRONTMATTER_KEYS, FRONTMATTER_LIST_KEYS};
use crate::jsstr;
use crate::mask::masking::EscapeMode;
use crate::regexutil::RegexExt;
use crate::rx;
use crate::types::{Finalize, Formality, SegmentKind, TextContext, WrapSpec};

rx!(FORMALITY_KEYS, r"^(?:formality|translation_formality|translation-formality|tone)$", "i");
rx!(FOLDED_PARAGRAPH, r"(?:^|\n)((?:[ \t]*\S[^\n]*(?:\n|$))+)");

/// `node` is the front matter node: `start..end` covers the fences, `value` is the YAML between them.
pub fn process_frontmatter(ctx: &mut ExtractContext, start: usize, end: usize, value: &str, translate: bool) -> Option<Formality> {
    let src = ctx.source;
    let first_nl = src[start..].find('\n').map(|i| i + start)?;
    if first_nl >= end {
        return None;
    }
    let base = first_nl + 1;
    if src.get(base..base + value.len()) != Some(value) {
        ctx.notes.push("front matter left untranslated (could not map it to the source)".into());
        return None;
    }
    let doc = match yaml::parse(value) {
        Ok(d) => d,
        Err(e) => {
            ctx.notes.push(format!("front matter left untranslated (YAML error: {})", e.lines().next().unwrap_or("")));
            return None;
        }
    };
    if !matches!(doc.value, Value::Map(_)) {
        return None;
    }
    let previous = ctx.structure.clone();
    let mut formality = None;
    visit_map(ctx, &doc, &[], base, translate, &mut formality);
    ctx.structure = previous;
    formality
}

fn visit_map(ctx: &mut ExtractContext, map: &YNode, path: &[String], base: usize, translate: bool, formality: &mut Option<Formality>) {
    let Value::Map(pairs) = &map.value else {
        return;
    };
    for (k, v) in pairs {
        if !k.is_scalar() {
            continue;
        }
        let key = k.key_string();
        let lower = key.to_lowercase();
        if path.is_empty() && FORMALITY_KEYS().test(&key) && v.is_scalar() {
            let f = v.key_string().to_lowercase();
            if f == "formal" {
                *formality = Some(Formality::Formal);
            } else if f == "informal" {
                *formality = Some(Formality::Informal);
            }
            continue;
        }
        if !translate {
            continue;
        }
        let mut key_path = path.to_vec();
        key_path.push(key.clone());
        if FRONTMATTER_KEYS.contains(&lower.as_str()) && v.is_scalar() && v.as_str().is_some() {
            scalar_segments(ctx, v, base, &key_path.join("."));
        } else if FRONTMATTER_LIST_KEYS.contains(&lower.as_str()) && matches!(v.value, Value::Seq(_)) {
            let Value::Seq(items) = &v.value else { unreachable!() };
            for item in items {
                if item.is_scalar() && item.as_str().is_some() {
                    scalar_segments(ctx, item, base, &key_path.join("."));
                }
            }
        } else if matches!(v.value, Value::Map(_)) {
            visit_map(ctx, v, &key_path, base, translate, formality);
        } else if let Value::Seq(items) = &v.value {
            for item in items {
                if matches!(item.value, Value::Map(_)) {
                    visit_map(ctx, item, &key_path, base, translate, formality);
                }
            }
        }
    }
}

fn add(ctx: &mut ExtractContext, text: &str, r_from: usize, r_to: usize, note: &str, finalize: Finalize, wrap: bool) {
    let src = ctx.source;
    let mut mb = ctx.builder();
    mb.source(text, EscapeMode::None);
    let spec = match (&mb.soft_break, wrap) {
        (Some(sb), true) => Some(WrapSpec {
            width: max_line_width(src, r_from, r_to).max(40),
            first_column: column_u16(src, r_from),
            prefix: sb.prefix.clone(),
            eol: sb.eol.clone(),
        }),
        _ => None,
    };
    let mut o = SegmentOptions::new(SegmentKind::Frontmatter, TextContext::Yaml, &src[r_from..r_to], note);
    o.finalize = finalize;
    o.wrap = spec;
    if let Some(seg) = ctx.segment(mb, o) {
        ctx.replace(r_from, r_to, seg);
    }
}

fn scalar_segments(ctx: &mut ExtractContext, node: &YNode, base: usize, key_path: &str) {
    let src = ctx.source;
    let from = base + node.start;
    let to = base + node.end;
    let raw = jsstr::trim_end(&src[from..to]);
    let raw_len = raw.len();
    ctx.structure = Some(format!("front matter \"{key_path}\""));
    let note = format!("front matter \"{key_path}\"");
    let value = node.as_str().unwrap_or("");
    match node.style {
        Some(Style::DoubleQuoted) => add(ctx, value, from, from + raw_len, &note, Finalize::YamlDouble, false),
        Some(Style::SingleQuoted) => add(ctx, value, from, from + raw_len, &note, Finalize::YamlSingle, false),
        Some(Style::Plain) => add(ctx, value, from, from + raw_len, &note, Finalize::YamlPlain, false),
        Some(style @ (Style::Folded | Style::Literal)) => {
            let Some(header_end) = raw.find('\n') else {
                return;
            };
            let content_from = from + header_end + 1;
            let content = &src[content_from..from + raw_len];
            if style == Style::Literal {
                // Line breaks are meaningful in literal blocks: every line is its own segment.
                let mut pos = content_from;
                for line in content.split('\n') {
                    let body = line.strip_suffix('\r').unwrap_or(line);
                    let indent = jsstr::leading_blank(body).len();
                    let text = body[indent..].trim_end_matches([' ', '\t']);
                    if !text.is_empty() {
                        add(ctx, text, pos + indent, pos + indent + text.len(), &note, Finalize::None, false);
                    }
                    pos += line.len() + 1;
                }
                return;
            }
            // Folded: blank lines separate paragraphs; each paragraph is re-wrapped with the block indentation.
            for m in FOLDED_PARAGRAPH().all(content) {
                let block = jsstr::trim_end(m.g(1));
                let offset = m.start + (m.len() - m.g(1).len());
                let indent = jsstr::leading_blank(block).len();
                let p_from = content_from + offset + indent;
                add(ctx, &block[indent..], p_from, p_from + block.len() - indent, &note, Finalize::None, true);
            }
        }
        None => {}
    }
}
