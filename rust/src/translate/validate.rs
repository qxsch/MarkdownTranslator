//! Validation of model output per segment. Port of `translate/validate.ts`.

use crate::config::LanguageConfig;
use crate::jsstr;
use crate::markdown::ast::Kind;
use crate::markdown::document::html_skeleton;
use crate::markdown::inline::inline_signature;
use crate::markdown::parse::parse_markdown;
use crate::markdown::render::render_segment;
use crate::mask::masking::{check_tags, plain_text};
use crate::regexutil::RegexExt;
use crate::rx;
use crate::types::{Extraction, RenderOptions, Segment, SegmentKind, TMap};

rx!(NEWLINE_RUN, r"\s*\r?\n\s*");
rx!(CONTAINER_PREFIX, r"(\r?\n)(?:[ \t]*>)*[ \t]*");
rx!(WORD_3, r"\p{L}{3,}");
rx!(WS, r"\s+");

/// Normalizes model output: single line, source-like outer whitespace.
pub fn sanitize(seg: &Segment, text: &str) -> String {
    let lead = jsstr::leading_ws(&seg.masked);
    let trail = jsstr::trailing_ws(&seg.masked);
    let one = NEWLINE_RUN().replace_all_str(text, " ");
    format!("{lead}{}{trail}", jsstr::trim(&one))
}

pub fn validate_segment(seg: &Segment, translated: &str, ex: &Extraction, lang: &LanguageConfig) -> Vec<String> {
    let mut errors = check_tags(seg, translated);
    if !errors.is_empty() {
        return errors;
    }
    let text = plain_text(translated);
    let src_text = plain_text(&seg.masked);
    for f in &seg.forbidden {
        if text.contains(f.as_str()) {
            errors.push(format!("must not contain \"{f}\" (it would terminate the code comment)"));
        }
    }
    if seg.kind == SegmentKind::Comment && jsstr::trim(&text).ends_with('\\') {
        errors.push("a code comment must not end with a backslash".into());
    }
    let src_len = jsstr::u16len(jsstr::trim(&src_text));
    if src_len >= 25 {
        let (min, max) = lang.length_ratio.unwrap_or((0.4, 3.0));
        let ratio = jsstr::u16len(jsstr::trim(&text)) as f64 / src_len as f64;
        if ratio < min {
            errors.push(format!("translation looks incomplete ({}% of source length)", (ratio * 100.0).round()));
        }
        if ratio > max {
            errors.push(format!("translation looks too long ({}% of source length); do not add content", (ratio * 100.0).round()));
        }
    }
    if !errors.is_empty() {
        return errors;
    }
    let tm: TMap = [(seg.id.clone(), translated.to_string())].into_iter().collect();
    let rendered = render_segment(seg, &tm, &ex.store, RenderOptions::wrapped(false));
    if let Some(signature) = &seg.inline_signature {
        // Container prefixes after kept line breaks (blockquote "> ", list indentation) belong to the parent block.
        let flat = CONTAINER_PREFIX().replace_all_with(&rendered, |m| m.g(1).to_string());
        // Headings and cells never start a line, so shield them from block-start interpretation.
        let probe = if seg.kind == SegmentKind::Paragraph { flat } else { format!("x {flat}") };
        let root = match parse_markdown(&format!("{probe}\n\n{}", ex.definitions_text), ex.parse_options) {
            Ok(r) => r,
            Err(e) => return vec![format!("translation does not parse: {e}")],
        };
        let Some(first) = root.children.first().filter(|n| matches!(n.kind, Kind::Paragraph { .. })) else {
            return vec!["translation turns into a different Markdown block (e.g. list or heading); rephrase the start".into()];
        };
        if inline_signature(&first.children, seg.kind == SegmentKind::Cell) != *signature {
            return vec![
                "inline formatting changed after rendering; keep emphasis/link tags hugging whole words (no spaces directly inside tags, no letters directly attached outside underscore-emphasis tags) and do not introduce Markdown characters"
                    .into(),
            ];
        }
    }
    if seg.kind == SegmentKind::Html && html_skeleton(&rendered) != html_skeleton(&seg.original) {
        errors.push("HTML structure changed".into());
    }
    errors
}

/// Soft check used for reporting: long segments that came back unchanged.
pub fn looks_untranslated(seg: &Segment, translated: &str) -> bool {
    let src = plain_text(&seg.masked);
    let src = jsstr::trim(&src);
    let words = WS().replace_all_str(src, "\u{0}");
    words.split('\u{0}').filter(|w| WORD_3().test(w)).count() >= 4 && jsstr::trim(&plain_text(translated)) == src
}
