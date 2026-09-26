//! Rendering of (translated) segments. Port of `render.ts`.

use super::wrap::{join_pieces, wrap_pieces};
use crate::mask::masking::render_pieces;
use crate::regexutil::RegexExt;
use crate::rx;
use crate::types::{Finalize, Render, RenderOptions, Segment, SegmentKind, SegmentStore, TMap};

rx!(PARAGRAPH_BLOCK_START, r"^(?:[-+*](?=[ \t]|$)|#{1,6}(?=[ \t]|$)|>|\d{1,9}(?=[.)](?:[ \t]|$))|={2,}[ \t]*$|`{3}|~{3})");
rx!(STARTS_WITH_DIGIT, r"^\d");
rx!(PLAIN_ATTR_VALUE, r#"^[^\s"'=<>`]+$"#);
rx!(YAML_NEEDS_QUOTES, r#"^[\s\-?:,[\]{}#&*!|>'"%@`]|:\s|\s#|:$|^\s|\s$|^(?:true|false|yes|no|on|off|null|~|[-+]?\d[\d_.]*(?:e[-+]?\d+)?)$"#, "i");

fn is_translated(seg: &Segment, tm: &TMap) -> bool {
    tm.get(&seg.id).is_some_and(|t| *t != seg.masked)
}

pub fn needs_render(seg: &Segment, tm: &TMap, store: &SegmentStore) -> bool {
    if is_translated(seg, tm) {
        return true;
    }
    seg.dependents.iter().any(|id| store.get(id).is_some_and(|d| needs_render(d, tm, store)))
}

/// Resolves a placeholder/pair render against the current translations.
pub fn render_value(r: &Render, tm: &TMap, store: &SegmentStore) -> String {
    match r {
        Render::Literal(s) => s.clone(),
        Render::Spliced { text, parts } => {
            let mut out = String::with_capacity(text.len());
            let mut pos = 0;
            for p in parts {
                out.push_str(&text[pos..p.start]);
                match store.get(&p.seg) {
                    Some(seg) => out.push_str(&render_segment(seg, tm, store, RenderOptions::wrapped(true))),
                    None => out.push_str(&text[p.start..p.end]),
                }
                pos = p.end;
            }
            out.push_str(&text[pos..]);
            out
        }
        Render::Wrapped { before, seg, after } => {
            let inner = store.get(seg).map(|s| render_segment(s, tm, store, RenderOptions::wrapped(true))).unwrap_or_default();
            format!("{before}{inner}{after}")
        }
    }
}

pub fn render_segment(seg: &Segment, tm: &TMap, store: &SegmentStore, opts: RenderOptions) -> String {
    if !opts.force && !needs_render(seg, tm, store) {
        return seg.original.clone();
    }
    let translated = tm.get(&seg.id).unwrap_or(&seg.masked);
    let pieces = render_pieces(seg, translated, tm, store);
    let mut out = match &seg.wrap {
        Some(spec) if opts.wrap => wrap_pieces(&pieces, spec),
        _ => join_pieces(&pieces),
    };
    if seg.kind == SegmentKind::Paragraph {
        out = escape_paragraph_start(&out, &seg.original);
    }
    finalize(seg.finalize, out)
}

pub fn finalize(f: Finalize, s: String) -> String {
    match f {
        Finalize::None => s,
        Finalize::YamlDouble => crate::jsstr::json_string(&s),
        Finalize::YamlSingle => format!("'{}'", s.replace('\'', "''")),
        Finalize::YamlPlain => {
            if YAML_NEEDS_QUOTES().test(&s) {
                crate::jsstr::json_string(&s)
            } else {
                s
            }
        }
        Finalize::AttrUnquoted => {
            if PLAIN_ATTR_VALUE().test(&s) {
                s
            } else {
                format!("\"{}\"", s.replace('"', "&quot;"))
            }
        }
    }
}

/// A translated paragraph must not start with characters that would turn it into another block type.
pub fn escape_paragraph_start(out: &str, original: &str) -> String {
    let Some(m) = PARAGRAPH_BLOCK_START().exec(out) else {
        return out.to_string();
    };
    if PARAGRAPH_BLOCK_START().exec(original).is_some_and(|o| o.text == m.text) {
        return out.to_string();
    }
    if STARTS_WITH_DIGIT().test(m.text) {
        return format!("{}\\{}", &out[..m.end], &out[m.end..]);
    }
    format!("\\{out}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_start() {
        assert_eq!(escape_paragraph_start("- item", "Text"), "\\- item");
        assert_eq!(escape_paragraph_start("1. item", "Text"), "1\\. item");
        assert_eq!(escape_paragraph_start("# x", "# y"), "# x");
        assert_eq!(escape_paragraph_start("plain", "x"), "plain");
    }

    #[test]
    fn yaml_quoting() {
        assert_eq!(finalize(Finalize::YamlPlain, "Hallo Welt".into()), "Hallo Welt");
        assert_eq!(finalize(Finalize::YamlPlain, "Titel: Teil".into()), "\"Titel: Teil\"");
        assert_eq!(finalize(Finalize::YamlPlain, "Ja".into()), "Ja");
        assert_eq!(finalize(Finalize::YamlPlain, "yes".into()), "\"yes\"");
        assert_eq!(finalize(Finalize::YamlSingle, "it's".into()), "'it''s'");
    }
}
